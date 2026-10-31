use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;
use ts_rs::TS;

fn default_domains() -> u32 { 25 }
fn default_depth() -> u32 { 4 }
fn default_pages() -> u32 { 100 }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct SeedManifest {
    pub crawl_id: Option<String>,
    pub imported_at: String,
}

impl SeedManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.crawl_id.as_ref().is_some_and(|value| value.is_empty() || value.len() > 128 || value.chars().any(char::is_control)) {
            return Err("invalid crawl ID".into());
        }
        chrono::DateTime::parse_from_rfc3339(&self.imported_at)
            .map_err(|_| "expected RFC3339 import timestamp")?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebDiscoveryConfig {
    #[serde(default)]
    pub seed_urls: Vec<String>,
    #[serde(default)]
    pub seed_manifest: Option<SeedManifest>,
    #[serde(default = "default_domains")]
    pub max_domains: u32,
    #[serde(default = "default_depth")]
    pub max_depth: u32,
    #[serde(default = "default_pages")]
    pub max_pages: u32,
}

impl WebDiscoveryConfig {
    pub fn parse(value: &Value) -> Result<Self, String> {
        let empty = serde_json::json!({});
        let value = if value.is_null() { &empty } else { value };
        let object = value.as_object().ok_or("extractor.source_config: expected mapping")?;
        for key in object.keys() {
            if !matches!(key.as_str(), "seed_urls" | "seed_manifest" | "max_domains" | "max_depth" | "max_pages" | "url") {
                return Err(format!("extractor.source_config.{key}: unsupported discovery setting"));
            }
        }
        let mut public = object.clone();
        public.remove("url");
        for (key, min, max) in [("max_domains", 1, 100), ("max_depth", 0, 5), ("max_pages", 1, 100)] {
            if let Some(value) = public.get(key) {
                if !value.as_u64().is_some_and(|v| v >= min && v <= max) {
                    return Err(format!("extractor.source_config.{key}: expected integer between {min} and {max}"));
                }
            }
        }
        if let Some(value) = public.get("seed_urls") {
            let seeds = value.as_array().ok_or("extractor.source_config.seed_urls: expected array")?;
            for (i, seed) in seeds.iter().enumerate() {
                if !seed.is_string() { return Err(format!("extractor.source_config.seed_urls[{i}]: expected URL string")); }
            }
        }
        if let Some(metadata) = public.get("seed_manifest").filter(|value| !value.is_null()) {
            let manifest: SeedManifest = serde_json::from_value(metadata.clone())
                .map_err(|error| format!("extractor.source_config.seed_manifest: {error}"))?;
            manifest.validate().map_err(|error| format!("extractor.source_config.seed_manifest: {error}"))?;
        }
        serde_json::from_value(Value::Object(public)).map_err(|e| format!("extractor.source_config: {e}"))
    }
}

pub fn canonical_seeds(primary: &str, config: &WebDiscoveryConfig) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut seeds = Vec::with_capacity(config.seed_urls.len() + 1);
    for (i, raw) in std::iter::once(primary).chain(config.seed_urls.iter().map(String::as_str)).enumerate() {
        let path = if i == 0 { "extractor.url".to_string() } else { format!("extractor.source_config.seed_urls[{}]", i - 1) };
        let mut url = Url::parse(raw.trim()).map_err(|e| format!("{path}: invalid URL: {e}"))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
            return Err(format!("{path}: expected HTTP(S) URL without credentials"));
        }
        url.set_fragment(None);
        let canonical = url.to_string();
        if !seen.insert(canonical.clone()) { return Err(format!("{path}: duplicate canonical seed URL")); }
        seeds.push(canonical);
    }
    seeds.sort();
    Ok(seeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn configuration_and_seeds_are_bounded_and_canonical() {
        let config = WebDiscoveryConfig::parse(&json!({"seed_urls":["https://B.test/"]})).unwrap();
        assert_eq!((config.max_domains, config.max_depth, config.max_pages), (25, 4, 100));
        assert_eq!(canonical_seeds("https://a.test", &config).unwrap(), vec!["https://a.test/", "https://b.test/"]);
        assert!(canonical_seeds("https://b.test/#fragment", &config).is_err());
        assert!(canonical_seeds("https://user:pass@a.test/", &config).is_err());
        for value in [json!({"max_depth":6}), json!({"max_pages":0}), json!({"max_domains":101}), json!({"ignore_robots":true}), json!({"__proxy":"https://invalid.test"})] {
            assert!(WebDiscoveryConfig::parse(&value).is_err());
        }
    }
}
