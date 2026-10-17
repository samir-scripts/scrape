//! Deliberately bounded XPath 1.0 location-path subset on the existing HTML5 tree.
//! Unsupported expressions are errors, never CSS fallbacks or broadened queries.
use ego_tree::NodeRef;
use scraper::{ElementRef, Node};
use std::collections::HashSet;

type DomNode<'a> = NodeRef<'a, Node>;

#[derive(Debug, Clone)]
pub struct XPathExpr {
    absolute: bool,
    steps: Vec<Step>,
    group_predicates: Vec<Predicate>,
    attribute: Option<String>,
}
#[derive(Debug, Clone)]
struct Step {
    axis: Axis,
    test: String,
    predicates: Vec<Predicate>,
}
#[derive(Debug, Clone, Copy)]
enum Axis {
    Child,
    DescendantOrSelf,
    SelfNode,
    Parent,
    FollowingSibling,
}
#[derive(Debug, Clone)]
enum Predicate {
    Position(usize),
    Last,
    Attribute(String),
    Equals(String, String),
    Contains(String, String),
    StartsWith(String, String),
    TextContains(String),
    Normalized(String),
}

fn name(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}
fn literal(value: &str) -> Result<String, String> {
    let value = value.trim();
    let quote = value.chars().next().ok_or("Missing XPath literal")?;
    if !matches!(quote, '\'' | '"') || !value.ends_with(quote) || value.len() < 2 {
        return Err("XPath values must be quoted literals".into());
    }
    let inner = &value[1..value.len() - 1];
    if inner.contains(quote) {
        return Err("Unexpected content in XPath literal".into());
    }
    Ok(inner.to_string())
}

/// Split only outside quoted literals and nested parentheses/brackets.
fn split_top(value: &str, delimiter: char) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let (mut quote, mut depth, mut start) = (None, 0i32, 0);
    for (index, ch) in value.char_indices() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if matches!(ch, '(' | '[') {
            depth += 1;
        } else if matches!(ch, ')' | ']') {
            depth -= 1;
            if depth < 0 {
                return Err("Unbalanced XPath delimiters".into());
            }
        } else if ch == delimiter && depth == 0 {
            parts.push(&value[start..index]);
            start = index + ch.len_utf8();
        }
    }
    if quote.is_some() || depth != 0 {
        return Err("Unbalanced XPath delimiters or literal".into());
    }
    parts.push(&value[start..]);
    Ok(parts)
}
fn predicates(mut value: &str) -> Result<Vec<Predicate>, String> {
    let mut out = Vec::new();
    while !value.trim().is_empty() {
        value = value.trim();
        if !value.starts_with('[') {
            return Err("Unexpected XPath suffix".into());
        }
        let mut quote = None;
        let mut end = None;
        for (i, ch) in value[1..].char_indices() {
            if let Some(q) = quote {
                if ch == q {
                    quote = None;
                }
            } else if matches!(ch, '\'' | '"') {
                quote = Some(ch);
            } else if ch == ']' {
                end = Some(i + 1);
                break;
            } else if ch == '[' {
                return Err("Nested XPath predicates are unsupported".into());
            }
        }
        let end = end.ok_or("Unclosed XPath predicate")?;
        out.push(Predicate::parse(value[1..end].trim())?);
        if out.len() > 32 {
            return Err("XPath has more than 32 predicates".into());
        }
        value = &value[end + 1..];
    }
    Ok(out)
}
impl Predicate {
    fn parse(value: &str) -> Result<Self, String> {
        if let Ok(index) = value.parse::<usize>() {
            return Ok(Self::Position(index));
        }
        if value == "last()" {
            return Ok(Self::Last);
        }
        if let Some(rest) = value.strip_prefix('@') {
            let args = split_top(rest, '=')?;
            if name(args[0].trim()) {
                return match args.as_slice() {
                    [attr] => Ok(Self::Attribute(attr.trim().into())),
                    [attr, val] => Ok(Self::Equals(attr.trim().into(), literal(val)?)),
                    _ => Err("Unsupported XPath attribute predicate".into()),
                };
            }
        }
        if let Some(rest) = value.strip_prefix("normalize-space()") {
            if let Some(rhs) = rest.trim().strip_prefix('=') {
                return Ok(Self::Normalized(literal(rhs)?));
            }
        }
        for function in ["contains", "starts-with"] {
            if let Some(args) = value
                .strip_prefix(function)
                .and_then(|s| s.trim().strip_prefix('('))
                .and_then(|s| s.strip_suffix(')'))
            {
                let args = split_top(args, ',')?;
                if let [lhs, rhs] = args.as_slice() {
                    let rhs = literal(rhs)?;
                    if function == "contains" && lhs.trim() == "text()" {
                        return Ok(Self::TextContains(rhs));
                    }
                    if let Some(attr) = lhs.trim().strip_prefix('@').filter(|s| name(s)) {
                        return Ok(if function == "contains" {
                            Self::Contains(attr.into(), rhs)
                        } else {
                            Self::StartsWith(attr.into(), rhs)
                        });
                    }
                }
            }
        }
        Err(format!("Unsupported XPath predicate [{value}]; use supported attribute, text or positional predicates"))
    }
    fn matches(&self, node: DomNode<'_>, position: usize, size: usize) -> bool {
        let element = ElementRef::wrap(node);
        let attr = |key: &str| element.and_then(|e| e.attr(key));
        match self {
            Self::Position(index) => position == *index,
            Self::Last => position == size,
            Self::Attribute(key) => attr(key).is_some(),
            Self::Equals(key, value) => attr(key).is_some_and(|v| v == value),
            Self::Contains(key, value) => attr(key).is_some_and(|v| v.contains(value)),
            Self::StartsWith(key, value) => attr(key).is_some_and(|v| v.starts_with(value)),
            // XPath string(text()) uses the first direct text child, not descendant text.
            Self::TextContains(value) => node
                .children()
                .find_map(|n| n.value().as_text())
                .is_some_and(|s| s.contains(value)),
            Self::Normalized(value) => {
                string_value(node)
                    .split([' ', '\t', '\r', '\n'])
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ")
                    == *value
            }
        }
    }
}
fn filter(nodes: &mut Vec<DomNode<'_>>, predicates: &[Predicate]) {
    for predicate in predicates {
        let size = nodes.len();
        let mut position = 0;
        nodes.retain(|node| {
            position += 1;
            predicate.matches(*node, position, size)
        });
    }
}
pub fn string_value(node: DomNode<'_>) -> String {
    node.descendants()
        .filter_map(|n| n.value().as_text().map(|s| s.to_string()))
        .collect()
}
impl XPathExpr {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw
            .trim()
            .strip_prefix("xpath:")
            .unwrap_or(raw.trim())
            .trim();
        if raw.is_empty() || raw.len() > 4096 {
            return Err("XPath must contain 1–4096 bytes".into());
        }
        split_top(raw, '|')?
            .into_iter()
            .skip(1)
            .try_for_each(|_| Err::<(), _>("XPath unions are unsupported"))?;
        let (path, group_predicates, suffix) = if raw.starts_with('(') {
            let mut quote = None;
            let mut depth = 0;
            let mut end = None;
            for (i, ch) in raw.char_indices() {
                if let Some(q) = quote {
                    if ch == q {
                        quote = None;
                    }
                    continue;
                }
                if matches!(ch, '\'' | '"') {
                    quote = Some(ch);
                } else if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
            }
            let end = end.ok_or("Unclosed XPath group")?;
            let tail = &raw[end + 1..];
            let parts = split_top(tail, '/')?;
            if parts.len() > 2 {
                return Err(
                    "Only terminal attribute/text extraction can follow an XPath group".into(),
                );
            }
            (&raw[1..end], predicates(parts[0])?, parts.get(1).copied())
        } else {
            (raw, Vec::new(), None)
        };
        let absolute = path.starts_with('/');
        let parts = split_top(path, '/')?;
        let mut steps = Vec::new();
        let mut attribute = None;
        for (i, part) in parts.iter().enumerate() {
            let part = part.trim();
            if part.is_empty() {
                if i == parts.len() - 1 {
                    return Err("XPath cannot end in '/'".into());
                }
                if i != 0 {
                    if parts[i - 1].is_empty() && i != 1 {
                        return Err("Invalid XPath '///'".into());
                    }
                    steps.push(Step {
                        axis: Axis::DescendantOrSelf,
                        test: "node()".into(),
                        predicates: Vec::new(),
                    });
                }
                continue;
            }
            if let Some(attr) = part.strip_prefix('@') {
                if i != parts.len() - 1
                    || !name(attr)
                    || suffix.is_some()
                    || !group_predicates.is_empty()
                {
                    return Err("XPath attribute extraction must be terminal".into());
                }
                attribute = Some(attr.into());
                continue;
            }
            let (test, preds) = part
                .find('[')
                .map_or((part, ""), |idx| (&part[..idx], &part[idx..]));
            let (axis, test) = match test.trim() {
                "." => (Axis::SelfNode, "node()"),
                ".." => (Axis::Parent, "node()"),
                value => {
                    if let Some((axis, test)) = value.split_once("::") {
                        (
                            match axis.trim() {
                                "parent" => Axis::Parent,
                                "following-sibling" => Axis::FollowingSibling,
                                "child" => Axis::Child,
                                "self" => Axis::SelfNode,
                                _ => return Err(format!("Unsupported XPath axis '{axis}'")),
                            },
                            test.trim(),
                        )
                    } else {
                        (Axis::Child, value)
                    }
                }
            };
            if !name(test) && !matches!(test, "*" | "text()" | "node()") {
                return Err(format!("Unsupported XPath node test '{test}'"));
            }
            steps.push(Step {
                axis,
                test: test.into(),
                predicates: predicates(preds)?,
            });
        }
        if let Some(suffix) = suffix {
            if suffix == "text()" {
                // Group filtering must happen before the terminal text step.
                // Store this as a separate marker, evaluated below.
                attribute = Some("#text".into());
            } else if let Some(attr) = suffix.strip_prefix('@').filter(|s| name(s)) {
                attribute = Some(attr.into());
            } else {
                return Err("Unsupported XPath group suffix".into());
            }
        }
        if steps.len() > 64 || steps.is_empty() {
            return Err("XPath requires 1–64 location steps".into());
        }
        Ok(Self {
            absolute,
            steps,
            group_predicates,
            attribute,
        })
    }
    pub fn nodes<'a>(&self, context: DomNode<'a>) -> Vec<DomNode<'a>> {
        let root = if self.absolute {
            context.ancestors().last().unwrap_or(context)
        } else {
            context
        };
        let mut current = vec![root];
        let mut document_order = None;
        let ordering_root = if self.steps.iter().any(|s| matches!(s.axis, Axis::Parent)) {
            root.ancestors().last().unwrap_or(root)
        } else {
            root
        };
        for step in &self.steps {
            let context_count = current.len();
            let mut next = Vec::new();
            let mut seen = HashSet::new();
            for node in current {
                // The abbreviated // expansion has no predicates. Ancestor contexts
                // already cover nested contexts; avoid repeatedly walking that subtree.
                if matches!(step.axis, Axis::DescendantOrSelf) && seen.contains(&node.id()) {
                    continue;
                }
                let mut candidates: Vec<_> = match step.axis {
                    Axis::Child => node.children().collect(),
                    Axis::DescendantOrSelf => node.descendants().collect(),
                    Axis::SelfNode => vec![node],
                    Axis::Parent => node.parent().into_iter().collect(),
                    Axis::FollowingSibling => {
                        let mut out = Vec::new();
                        let mut cursor = node.next_sibling();
                        while let Some(n) = cursor {
                            out.push(n);
                            cursor = n.next_sibling();
                        }
                        out
                    }
                };
                candidates.retain(|n| match step.test.as_str() {
                    "node()" => true,
                    "text()" => n.value().is_text(),
                    "*" => n.value().is_element(),
                    tag => n.value().as_element().is_some_and(|e| e.name() == tag),
                });
                filter(&mut candidates, &step.predicates);
                next.extend(candidates.into_iter().filter(|n| seen.insert(n.id())));
            }
            // Parent/sibling axes can discover nodes out of order; XPath uses document order.
            if next.len() > 1 && context_count > 1 {
                let order = document_order.get_or_insert_with(|| {
                    ordering_root
                        .descendants()
                        .enumerate()
                        .map(|(i, n)| (n.id(), i))
                        .collect::<std::collections::HashMap<_, _>>()
                });
                next.sort_by_key(|n| order.get(&n.id()).copied().unwrap_or(0));
            }
            current = next;
        }
        filter(&mut current, &self.group_predicates);
        if let Some(attribute) = &self.attribute {
            current.retain(|node| {
                if attribute == "#text" {
                    node.children().any(|n| n.value().is_text())
                } else {
                    ElementRef::wrap(*node).is_some_and(|e| e.attr(attribute).is_some())
                }
            });
        }
        current
    }
    pub fn steps_return_text(&self) -> bool {
        self.steps.last().is_some_and(|step| step.test == "text()")
    }
    pub fn attribute(&self) -> Option<&str> {
        self.attribute.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::Html;
    fn values(html: &Html, expression: &str) -> Vec<String> {
        let expr = XPathExpr::parse(expression).unwrap();
        expr.nodes(html.tree.root())
            .into_iter()
            .map(|n| match expr.attribute() {
                Some(attr) => ElementRef::wrap(n)
                    .and_then(|e| e.attr(attr))
                    .unwrap_or("")
                    .into(),
                None => string_value(n),
            })
            .collect()
    }
    #[test]
    fn axes_predicates_and_position_follow_xpath_semantics() {
        let html = Html::parse_document("<section><div class='job-card' data-id='req-1'><h2>Rust <b>Engineer</b></h2><a href='/a'>A</a><a href='/b'>B</a><span> Remote </span></div><div class='job-card' data-id='req-2'><a href='/c'>C</a></div></section>");
        assert_eq!(values(&html, "//div[contains(@class, 'job-card')][@data-id][starts-with(@data-id,'req-')]/a[1]/@href"), ["/a", "/c"]);
        assert_eq!(values(&html, "(//a)[1]/@href"), ["/a"]);
        assert_eq!(values(&html, "//a[last()]/@href"), ["/b", "/c"]);
        let missing_attr = Html::parse_document("<a>Missing</a><a href='/valid'>Valid</a>");
        assert_eq!(values(&missing_attr, "//a/@href"), ["/valid"]);
        let nested = Html::parse_document("<div><div><a>A</a></div><a>B</a></div><a>C</a>");
        assert_eq!(values(&nested, "//div//a"), ["A", "B"]);
        assert_eq!(
            values(
                &html,
                "//h2/following-sibling::span[normalize-space()='Remote']/text()"
            ),
            [" Remote "]
        );
        assert_eq!(
            values(&html, "//h2/parent::div[@data-id='req-1']/a[last()]/@href"),
            ["/b"]
        );
        assert_eq!(
            values(&html, "//h2/../a[contains(text(),'B')]/text()"),
            ["B"]
        );
        assert!(values(&html, "//h2[contains(text(),'Engineer')]").is_empty());
        assert_eq!(
            values(&html, "/html/body/section/div[1]/h2"),
            ["Rust Engineer"]
        );
        let item = html
            .select(&scraper::Selector::parse("div").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            XPathExpr::parse(".//a[1]/@href")
                .unwrap()
                .nodes(*item)
                .len(),
            1
        );
    }
    #[test]
    fn malformed_and_unsupported_expressions_fail_without_broadening() {
        for expression in [
            "",
            "///a",
            "//a/",
            "//a[",
            "//a[@x='bad]",
            "//a[foo()]",
            "//a | //div",
            "//a/@href/div",
            "//a[1+1]",
            "//a[0 or 1]",
            "//a[unknown::x]",
            "//a[@x=bad]",
            "//a[ @x='a' junk ]",
            "//a[ancestor::div]",
        ] {
            assert!(
                XPathExpr::parse(expression).is_err(),
                "accepted {expression}"
            );
        }
    }
}
