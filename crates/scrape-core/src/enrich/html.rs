use scraper::{node::Node, ElementRef, Html};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;

const MAX_DESCRIPTION_INPUT_BYTES: usize = 512 * 1024;
const MAX_DESCRIPTION_NODES: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DescriptionInline {
    Text { text: String },
    Strong { children: Vec<DescriptionInline> },
    Emphasis { children: Vec<DescriptionInline> },
    Link { text: String, url: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DescriptionBlock {
    Paragraph {
        children: Vec<DescriptionInline>,
    },
    Heading {
        level: u8,
        children: Vec<DescriptionInline>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<DescriptionInline>>,
    },
    Break,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
pub struct JobDescription {
    pub plain_text: String,
    pub blocks: Vec<DescriptionBlock>,
    pub truncated: bool,
}

/// Normalize provider HTML into bounded plain text and a restricted display tree.
/// Some providers return one HTML-entity-encoded document. Decode exactly one
/// layer only when a recognized encoded tag is present; normal text is never
/// repeatedly decoded or interpreted as markup.
pub fn normalize_description(input: &str, max_bytes: usize) -> JobDescription {
    if input.trim().is_empty() {
        return JobDescription {
            plain_text: String::new(),
            blocks: Vec::new(),
            truncated: false,
        };
    }

    let bounded_input = truncate_utf8(input, MAX_DESCRIPTION_INPUT_BYTES);
    let input_truncated = bounded_input.len() < input.len();
    let decoded;
    let html = if has_encoded_html_tag(bounded_input) {
        decoded = decode_encoded_markup(bounded_input);
        decoded.as_str()
    } else {
        bounded_input
    };

    let document = Html::parse_fragment(html);
    let mut blocks = Vec::new();
    let mut visited = 0usize;
    for child in document.root_element().children() {
        collect_blocks(child, &mut blocks, &mut visited);
        if visited >= MAX_DESCRIPTION_NODES {
            break;
        }
    }

    let mut plain_text = blocks_to_text(&blocks);
    let limit = max_bytes.min(MAX_DESCRIPTION_INPUT_BYTES);
    let was_truncated = input_truncated || plain_text.len() > limit;
    if plain_text.len() > limit {
        plain_text = truncate_utf8(&plain_text, limit).trim_end().to_string();
    }
    JobDescription {
        plain_text,
        blocks,
        truncated: was_truncated,
    }
}

/// Compatibility helper for existing adapter callers. New paths should retain
/// the structured `JobDescription` returned by `normalize_description`.
pub fn to_plain_text(html: &str, max_bytes: usize) -> String {
    normalize_description(html, max_bytes).plain_text
}

fn truncate_utf8(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let end = input
        .char_indices()
        .map(|(i, c)| i + c.len_utf8())
        .take_while(|end| *end <= max_bytes)
        .last()
        .unwrap_or(0);
    &input[..end]
}

fn has_encoded_html_tag(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    [
        "&lt;p", "&lt;div", "&lt;h1", "&lt;h2", "&lt;h3", "&lt;ul", "&lt;ol", "&lt;li", "&lt;br",
    ]
    .iter()
    .any(|tag| lower.contains(tag))
}

fn decode_encoded_markup(input: &str) -> String {
    input
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn collect_blocks(
    node: ego_tree::NodeRef<'_, Node>,
    out: &mut Vec<DescriptionBlock>,
    visited: &mut usize,
) {
    *visited += 1;
    if *visited > MAX_DESCRIPTION_NODES {
        return;
    }
    if let Some(text) = node.value().as_text() {
        let children = vec![DescriptionInline::Text {
            text: text.to_string(),
        }];
        push_paragraph(out, children);
        return;
    }
    let Some(element) = ElementRef::wrap(node) else {
        for child in node.children() {
            collect_blocks(child, out, visited);
        }
        return;
    };
    let name = element.value().name();
    if matches!(
        name,
        "script" | "style" | "noscript" | "svg" | "iframe" | "object" | "template"
    ) {
        return;
    }
    if name.len() == 2 && name.starts_with('h') && name.as_bytes()[1].is_ascii_digit() {
        let level = name.as_bytes()[1].saturating_sub(b'0').clamp(1, 6);
        out.push(DescriptionBlock::Heading {
            level,
            children: inline_children(element, visited),
        });
    } else if name == "br" {
        out.push(DescriptionBlock::Break);
    } else if name == "p" || name == "blockquote" {
        push_paragraph(out, inline_children(element, visited));
    } else if name == "ul" || name == "ol" {
        let items = element
            .children()
            .filter_map(ElementRef::wrap)
            .filter(|child| child.value().name() == "li")
            .map(|child| inline_children(child, visited))
            .collect();
        out.push(DescriptionBlock::List {
            ordered: name == "ol",
            items,
        });
    } else if matches!(name, "div" | "section" | "article" | "main") {
        for child in element.children() {
            collect_blocks(child, out, visited);
        }
    } else {
        push_paragraph(out, inline_children(element, visited));
    }
}

fn push_paragraph(out: &mut Vec<DescriptionBlock>, children: Vec<DescriptionInline>) {
    if !inline_text(&children).trim().is_empty() {
        out.push(DescriptionBlock::Paragraph { children });
    }
}

fn inline_children(element: ElementRef<'_>, visited: &mut usize) -> Vec<DescriptionInline> {
    let mut out = Vec::new();
    for child in element.children() {
        *visited += 1;
        if *visited > MAX_DESCRIPTION_NODES {
            break;
        }
        if let Some(text) = child.value().as_text() {
            out.push(DescriptionInline::Text {
                text: text.to_string(),
            });
            continue;
        }
        let Some(child_element) = ElementRef::wrap(child) else {
            continue;
        };
        let name = child_element.value().name();
        if matches!(
            name,
            "script" | "style" | "noscript" | "svg" | "iframe" | "object" | "template"
        ) {
            continue;
        }
        let nested = inline_children(child_element, visited);
        match name {
            "strong" | "b" => out.push(DescriptionInline::Strong { children: nested }),
            "em" | "i" => out.push(DescriptionInline::Emphasis { children: nested }),
            "a" => {
                let label = inline_text(&nested).trim().to_string();
                let href = child_element
                    .attr("href")
                    .and_then(|href| safe_link(href, child_element.attr("data-base-url")));
                out.push(DescriptionInline::Link {
                    text: label,
                    url: href,
                });
            }
            "br" => out.push(DescriptionInline::Text { text: "\n".into() }),
            _ => out.extend(nested),
        }
    }
    out
}

fn safe_link(href: &str, _base: Option<&str>) -> Option<String> {
    let url = Url::parse(href).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

fn inline_text(items: &[DescriptionInline]) -> String {
    items
        .iter()
        .map(|item| match item {
            DescriptionInline::Text { text } => text.clone(),
            DescriptionInline::Strong { children } | DescriptionInline::Emphasis { children } => {
                inline_text(children)
            }
            DescriptionInline::Link { text, .. } => text.clone(),
        })
        .collect()
}

fn blocks_to_text(blocks: &[DescriptionBlock]) -> String {
    let mut output = String::new();
    for block in blocks {
        if matches!(block, DescriptionBlock::Break) {
            if !output.ends_with('\n') {
                output.push('\n');
            }
            continue;
        }
        let text = match block {
            DescriptionBlock::Paragraph { children }
            | DescriptionBlock::Heading { children, .. } => inline_text(children)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            DescriptionBlock::List { ordered, items } => items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let item_text = inline_text(item)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if *ordered {
                        format!("{}. {}", index + 1, item_text)
                    } else {
                        format!("• {}", item_text)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            DescriptionBlock::Break => String::new(),
        };
        let normalized = text;
        if !normalized.is_empty() {
            if !output.is_empty() && !output.ends_with('\n') {
                output.push_str("\n\n");
            }
            output.push_str(&normalized);
        }
    }
    output.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_plain_text() {
        let html = "<p>Hello <b>World</b>!</p><br/>&amp; welcome &quot;here&quot;.<script>var x = 1;</script>";
        assert_eq!(
            to_plain_text(html, 1024),
            "Hello World!\n& welcome \"here\"."
        );
    }

    #[test]
    fn encoded_markup_is_normalized_without_leaking_tags_or_attributes() {
        let input = "&lt;h2&gt;About Us&lt;/h2&gt;&lt;p class=&quot;content-intro&quot;&gt;A &amp; B&lt;/p&gt;&lt;script data-x=&quot;1&quot;&gt;alert(1)&lt;/script&gt;";
        let desc = normalize_description(input, 1024);
        assert_eq!(desc.plain_text, "About Us\n\nA & B");
        assert!(!desc.plain_text.contains("class="));
        assert!(!desc.plain_text.contains("alert"));
    }

    #[test]
    fn literal_mathematical_angle_brackets_are_preserved() {
        assert_eq!(
            to_plain_text("C++: 5 < 7 and 9 > 3", 1024),
            "C++: 5 < 7 and 9 > 3"
        );
    }

    #[test]
    fn structures_lists_and_only_http_links_are_retained() {
        let desc = normalize_description("<h2>Role</h2><p><strong>Build</strong> <a href=\"https://example.com/apply\">systems</a> <a href=\"javascript:alert(1)\">unsafe</a></p><ul><li>One</li><li>Two</li></ul>", 1024);
        assert_eq!(desc.blocks.len(), 3);
        assert!(desc.plain_text.contains("• One\n• Two"));
        let DescriptionBlock::Paragraph { children } = &desc.blocks[1] else {
            panic!("paragraph expected")
        };
        assert!(children.iter().any(|child| matches!(child, DescriptionInline::Link { url: Some(url), .. } if url == "https://example.com/apply")));
        assert!(children
            .iter()
            .any(|child| matches!(child, DescriptionInline::Link { url: None, .. })));
    }

    #[test]
    fn truncation_respects_utf8_boundaries_and_reports_it() {
        let desc = normalize_description("<p>café 👋🏽</p>", 8);
        assert!(desc.truncated);
        assert!(desc.plain_text.is_char_boundary(desc.plain_text.len()));
        assert!(desc.plain_text.len() <= 8);
    }
}
