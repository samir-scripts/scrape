//! Explicit JSON-pointer extraction of embedded hydration data. Never executes scripts.
use crate::locator::{apply_regex_capture, QueryEngine};
use serde_json::Value;
use std::collections::BTreeMap;

pub const POSTING_FIELDS: [&str; 7] = [
    "title",
    "link",
    "id",
    "location",
    "salary",
    "description",
    "date",
];
#[derive(Debug, Clone)]
pub struct HydrationConfig {
    pub script: QueryEngine,
    pub pointer: String,
    pub fields: BTreeMap<String, JsonField>,
    pub description_is_text: bool,
}
#[derive(Debug, Clone)]
pub struct JsonField {
    pointer: String,
    regex: Option<regex::Regex>,
}
impl JsonField {
    pub fn extract(&self, candidate: &Value, allow_number: bool) -> Option<String> {
        let value = candidate.pointer(&self.pointer)?;
        let text = match value {
            Value::String(text) => text.clone(),
            Value::Number(number) if allow_number => number.to_string(),
            _ => return None,
        };
        let text = match &self.regex {
            Some(regex) => apply_regex_capture(regex, &text)?,
            None => text.trim().into(),
        };
        (!text.is_empty()).then_some(text)
    }
}
/// Validate RFC 6901 syntax before serde_json evaluates the pointer.
pub fn validate_pointer(raw: &str) -> Result<(), String> {
    if !raw.is_empty() && !raw.starts_with('/') {
        return Err("JSON pointers must be empty (root) or start with '/'".into());
    }
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return Err("JSON pointer escapes must be ~0 or ~1".into());
        }
    }
    Ok(())
}
impl HydrationConfig {
    pub fn parse(config: &Value) -> Result<Option<Self>, String> {
        if !["hydration_script", "hydration_pointer", "hydration_fields"]
            .iter()
            .any(|key| config.get(key).is_some())
        {
            return Ok(None);
        }
        let script = config
            .get("hydration_script")
            .map(|v| {
                v.as_str()
                    .ok_or("hydration_script must be a CSS/XPath locator")
            })
            .transpose()?
            .unwrap_or("#__NEXT_DATA__, #__NUXT_DATA__");
        let pointer = config
            .get("hydration_pointer")
            .and_then(Value::as_str)
            .ok_or("hydration_pointer must be an explicit JSON pointer to a candidate array")?;
        validate_pointer(pointer).map_err(|e| format!("hydration_pointer: {e}"))?;
        let mappings = config
            .get("hydration_fields")
            .and_then(Value::as_object)
            .ok_or("hydration_fields must map posting fields to JSON pointers")?;
        let mut fields = BTreeMap::new();
        for (field, value) in mappings {
            if !POSTING_FIELDS.contains(&field.as_str()) {
                return Err(format!(
                    "hydration_fields.{field}: unsupported posting field"
                ));
            }
            let pointer = value
                .as_str()
                .ok_or_else(|| format!("hydration_fields.{field}: expected JSON pointer string"))?;
            validate_pointer(pointer).map_err(|e| format!("hydration_fields.{field}: {e}"))?;
            let regex_key = format!("{field}_regex");
            let regex = config
                .get(&regex_key)
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| format!("{regex_key}: expected regex string"))
                })
                .transpose()?;
            let regex = regex
                .filter(|s| !s.is_empty())
                .map(regex::Regex::new)
                .transpose()
                .map_err(|e| format!("{regex_key}: {e}"))?;
            fields.insert(
                field.clone(),
                JsonField {
                    pointer: pointer.into(),
                    regex,
                },
            );
        }
        for required in ["title", "link"] {
            if !fields.contains_key(required) {
                return Err(format!("hydration_fields.{required}: required"));
            }
        }
        for field in POSTING_FIELDS {
            if config.get(format!("{field}_regex")).is_some() && !fields.contains_key(field) {
                return Err(format!("{field}_regex requires hydration_fields.{field}"));
            }
            if field != "description" && config.get(format!("{field}_attribute")).is_some() {
                return Err(format!("{field}_attribute cannot be used for JSON values; use hydration_fields.{field}"));
            }
        }
        let format = config
            .get("description_attribute")
            .map(|v| {
                v.as_str()
                    .ok_or("description_attribute: expected text or html")
            })
            .transpose()?
            .unwrap_or("html");
        if !matches!(format, "text" | "html") {
            return Err(
                "description_attribute: hydration descriptions support text or html".into(),
            );
        }
        for key in [
            "item",
            "title",
            "link",
            "id",
            "location",
            "salary",
            "description",
            "date",
        ] {
            if config.get(key).is_some() {
                return Err(format!("{key}: DOM posting locators cannot be mixed with hydration_fields; use detail_fields for detail-page locators"));
            }
        }
        Ok(Some(Self {
            script: QueryEngine::parse_elements(script)?,
            pointer: pointer.into(),
            fields,
            description_is_text: format == "text",
        }))
    }
    pub fn extract(&self, candidate: &Value, field: &str) -> Option<String> {
        self.fields.get(field)?.extract(candidate, field == "id")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointers_preserve_escaped_names_and_reject_malformed_configuration() {
        let config = serde_json::json!({"hydration_pointer":"/jobs", "hydration_fields":{"title":"/a~1b", "link":"/m~0n"}});
        let parsed = HydrationConfig::parse(&config).unwrap().unwrap();
        assert_eq!(
            parsed
                .extract(
                    &serde_json::json!({"a/b":"Engineer", "m~n":"/job"}),
                    "title"
                )
                .as_deref(),
            Some("Engineer")
        );
        for value in ["jobs", "/bad~", "/bad~2"] {
            assert!(validate_pointer(value).is_err());
        }
        for (key, value) in [
            ("hydration_pointer", serde_json::json!(4)),
            ("hydration_fields", serde_json::json!({"title":"/title"})),
            ("title_regex", serde_json::json!("[")),
            ("title_attribute", serde_json::json!("href")),
        ] {
            let mut invalid = config.clone();
            invalid[key] = value;
            assert!(HydrationConfig::parse(&invalid).is_err(), "accepted {key}");
        }
    }
}
