//! Shared locator and field-transform validation for host configuration and worker execution.
use crate::xpath::{string_value, XPathExpr};
use scraper::{ElementRef, Html, Selector};

#[derive(Debug, Clone)]
pub enum QueryEngine {
    Css(Selector),
    XPath(XPathExpr),
    SelfNode,
}
impl QueryEngine {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        if raw == "." {
            return Ok(Self::SelfNode);
        }
        if is_xpath(raw) {
            return XPathExpr::parse(raw).map(Self::XPath);
        }
        Selector::parse(raw)
            .map(Self::Css)
            .map_err(|e| format!("Invalid CSS selector: {e}"))
    }
    pub fn parse_elements(raw: &str) -> Result<Self, String> {
        let query = Self::parse(raw)?;
        if let Self::XPath(expression) = &query {
            if expression.attribute().is_some() || expression.steps_return_text() {
                return Err(
                    "Container locators must select elements, not attributes or text()".into(),
                );
            }
        }
        Ok(query)
    }
    pub fn elements<'a>(
        &self,
        document: &'a Html,
        context: Option<ElementRef<'a>>,
    ) -> Vec<ElementRef<'a>> {
        match self {
            Self::Css(selector) => match context {
                Some(element) => element.select(selector).collect(),
                None => document.select(selector).collect(),
            },
            Self::XPath(expression) => expression
                .nodes(context.map_or(document.tree.root(), |e| *e))
                .into_iter()
                .filter_map(ElementRef::wrap)
                .collect(),
            Self::SelfNode => vec![context.unwrap_or_else(|| document.root_element())],
        }
    }
}
#[derive(Debug, Clone)]
pub struct FieldLocator {
    pub query: QueryEngine,
    pub attribute: Option<String>,
    pub regex: Option<regex::Regex>,
}
impl FieldLocator {
    pub fn parse(raw: &str, attribute: Option<&str>, regex: Option<&str>) -> Result<Self, String> {
        // CSS shorthand is only a terminal @name outside quotes/brackets.
        let mut suffix = None;
        let mut quote = None;
        let mut depth = 0;
        let mut escaped = false;
        if !is_xpath(raw.trim()) {
            for (i, c) in raw.char_indices() {
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if let Some(q) = quote {
                    if c == q {
                        quote = None;
                    }
                } else if matches!(c, '\'' | '"') {
                    quote = Some(c);
                } else if matches!(c, '[' | '(') {
                    depth += 1;
                } else if matches!(c, ']' | ')') {
                    depth -= 1;
                } else if c == '@' && depth == 0 {
                    suffix = Some(i);
                }
            }
        }
        let (query, shorthand) = suffix.map_or((raw, None), |i| (&raw[..i], Some(&raw[i + 1..])));
        let query = QueryEngine::parse(query)?;
        if attribute.is_some() && shorthand.is_some() && attribute != shorthand {
            return Err("Conflicting shorthand and explicit extraction attributes".into());
        }
        let attribute = attribute
            .or(shorthand)
            .map(|s| s.trim().strip_prefix('@').unwrap_or(s.trim()).to_string());
        if attribute.as_ref().is_some_and(|a| {
            a.is_empty()
                || !a
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':'))
        }) {
            return Err("Extraction attribute must be text, html or an attribute name".into());
        }
        if let QueryEngine::XPath(expr) = &query {
            if (expr.attribute().is_some() || expr.steps_return_text()) && attribute.is_some() {
                return Err(
                    "Use either XPath terminal attribute/text extraction or a field attribute, not both".into(),
                );
            }
        }
        let regex = regex
            .filter(|s| !s.is_empty())
            .map(regex::Regex::new)
            .transpose()
            .map_err(|e| format!("Invalid extraction regex: {e}"))?;
        Ok(Self {
            query,
            attribute,
            regex,
        })
    }
    pub fn from_config(
        config: &serde_json::Value,
        key: &str,
        default: Option<&str>,
    ) -> Result<Option<Self>, String> {
        let string = |key: &str| {
            config
                .get(key)
                .map(|value| {
                    value
                        .as_str()
                        .ok_or_else(|| format!("{key} must be a string"))
                })
                .transpose()
        };
        let attribute_key = format!("{key}_attribute");
        let regex_key = format!("{key}_regex");
        let attribute = string(&attribute_key)?;
        let regex = string(&regex_key)?;
        let query = string(key)?.or(default).or_else(|| regex.map(|_| "."));
        if query.is_none() && attribute.is_some() {
            return Err(format!("{attribute_key} requires a {key} locator"));
        }
        query
            .map(|query| {
                Self::parse(query, attribute, regex).map_err(|error| format!("{key}: {error}"))
            })
            .transpose()
    }
    pub fn extracts_html(&self) -> bool {
        self.attribute.as_deref().unwrap_or("html") == "html"
            && !matches!(&self.query, QueryEngine::XPath(expr) if expr.attribute().is_some() || expr.steps_return_text())
    }
    pub fn extract(
        &self,
        document: &Html,
        context: Option<ElementRef<'_>>,
        default_attribute: &str,
    ) -> Option<String> {
        let attribute = self
            .attribute
            .as_deref()
            .unwrap_or(if default_attribute == "date" {
                "#date"
            } else {
                default_attribute
            });
        let raw = if let QueryEngine::XPath(expression) = &self.query {
            let node = expression
                .nodes(context.map_or(document.tree.root(), |e| *e))
                .into_iter()
                .next()?;
            match expression.attribute() {
                Some("#text") => node
                    .children()
                    .find_map(|n| n.value().as_text().map(|s| s.to_string())),
                Some(attr) => ElementRef::wrap(node)
                    .and_then(|e| e.attr(attr))
                    .map(str::to_string),
                None if node.value().is_text() => Some(string_value(node)),
                None => value(ElementRef::wrap(node)?, attribute),
            }
        } else {
            value(
                self.query.elements(document, context).into_iter().next()?,
                attribute,
            )
        }?;
        let output = if let Some(regex) = &self.regex {
            apply_regex_capture(regex, &raw)?
        } else {
            raw.trim().to_string()
        };
        (!output.is_empty()).then_some(output)
    }
}
fn is_xpath(raw: &str) -> bool {
    raw.starts_with("xpath:")
        || raw.starts_with('/')
        || raw.starts_with("./")
        || raw.starts_with("(/")
        || raw == ".."
        || raw.starts_with("../")
}
fn value(element: ElementRef<'_>, attribute: &str) -> Option<String> {
    match attribute {
        "text" => Some(element.text().collect::<Vec<_>>().join(" ")),
        "html" => Some(element.html()),
        // Historical date selectors prefer datetime but retain text if absent.
        "#date" => Some(
            element
                .attr("datetime")
                .map(str::to_string)
                .unwrap_or_else(|| element.text().collect::<String>()),
        ),
        attr => element.attr(attr).map(str::to_string),
    }
}
pub fn apply_regex_capture(regex: &regex::Regex, value: &str) -> Option<String> {
    let captures = regex.captures(value)?;
    [
        "value",
        "title",
        "link",
        "location",
        "salary",
        "date",
        "description",
        "id",
    ]
    .iter()
    .find_map(|name| captures.name(name))
    .or_else(|| captures.get(1))
    .or_else(|| captures.get(0))
    .map(|m| m.as_str().trim().to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_attributes_and_named_regex_do_not_fall_back_after_mismatch() {
        let html = Html::parse_document(
            "<a data-url='/jobs/42' href='/ignored' title='Role: Rust'>Apply</a>",
        );
        let title = FieldLocator::parse("a@title", None, Some("Role: (?P<value>.+)")).unwrap();
        assert_eq!(title.extract(&html, None, "text").as_deref(), Some("Rust"));
        assert_eq!(
            FieldLocator::parse("//a/@data-url", None, None)
                .unwrap()
                .extract(&html, None, "href")
                .as_deref(),
            Some("/jobs/42")
        );
        assert_eq!(
            FieldLocator::parse("a", Some("title"), Some("missing (.+)"))
                .unwrap()
                .extract(&html, None, "text"),
            None
        );
        assert!(FieldLocator::parse("a@", None, None).is_err());
        assert!(FieldLocator::parse("//a/@href", Some("src"), None).is_err());
        assert!(FieldLocator::parse("a", None, Some("[")).is_err());
        assert!(FieldLocator::parse("a@href", Some("src"), None).is_err());
        assert!(FieldLocator::parse("a", Some("@@href"), None).is_err());
        assert!(FieldLocator::parse("//a/text()", Some("href"), None).is_err());
        let parent = Html::parse_document(
            "<article data-id='original'><a href='/apply'>Apply</a></article>",
        );
        let link = parent
            .select(&Selector::parse("a").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            FieldLocator::parse("../@data-id", None, None)
                .unwrap()
                .extract(&parent, Some(link), "text")
                .as_deref(),
            Some("original")
        );
        let escaped = Html::parse_document("<h2 id='role@title'>Engineer</h2>");
        assert_eq!(
            FieldLocator::parse(r"#role\@title", None, None)
                .unwrap()
                .extract(&escaped, None, "text")
                .as_deref(),
            Some("Engineer")
        );
    }
}
