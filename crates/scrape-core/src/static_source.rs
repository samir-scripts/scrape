//! Authoritative static pagination/detail configuration shared by save and execution.
use crate::{
    hydration::HydrationConfig,
    locator::{FieldLocator, QueryEngine},
    WorkerLimits,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlExtraction {
    Jsonld,
    Selectors,
    PublicHtml,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlConfig {
    #[serde(default = "crawl_pages")]
    pub max_pages: u32,
    #[serde(default = "crawl_depth")]
    pub max_depth: u32,
    pub extraction: CrawlExtraction,
}

fn crawl_pages() -> u32 {
    20
}
fn crawl_depth() -> u32 {
    2
}

impl CrawlConfig {
    pub fn parse(config: &Value) -> Result<Option<Self>, String> {
        let Some(value) = config.get("crawl") else {
            return Ok(None);
        };
        let crawl: Self =
            serde_json::from_value(value.clone()).map_err(|error| format!("crawl: {error}"))?;
        if !(1..=100).contains(&crawl.max_pages) {
            return Err("crawl.max_pages: expected a whole number from 1 to 100".into());
        }
        if crawl.max_depth > 5 {
            return Err("crawl.max_depth: expected a whole number from 0 to 5".into());
        }
        if crate::extraction::mode(config)? != crate::extraction::ExtractionMode::Static
            || config.get("script").is_some()
        {
            return Err(
                "crawl: requires static HTML extraction, not browser or script mode".into(),
            );
        }
        if ["next_page", "next_page_attribute", "next_page_regex"]
            .iter()
            .any(|key| config.get(key).is_some())
        {
            return Err("crawl: cannot be combined with next_page".into());
        }
        if crawl.extraction == CrawlExtraction::Selectors
            && ["item", "title", "link"].iter().any(|key| {
                config
                    .get(key)
                    .and_then(Value::as_str)
                    .is_none_or(|value| value.trim().is_empty())
            })
        {
            return Err(
                "crawl: selectors extraction requires item, title and link selectors".into(),
            );
        }
        if crawl.extraction == CrawlExtraction::Selectors {
            QueryEngine::parse_elements(config["item"].as_str().unwrap())
                .map_err(|error| format!("crawl.item: {error}"))?;
            for field in ["title", "link"] {
                FieldLocator::from_config(config, field, None)?;
            }
        }
        if let Some(url) = config.get("url") {
            let raw = url
                .as_str()
                .ok_or("crawl.url: expected HTTP(S) URL without credentials")?;
            if !crate::branding::http_url(raw) {
                return Err("crawl.url: expected HTTP(S) URL without credentials".into());
            }
        }
        Ok(Some(crawl))
    }
}

#[derive(Debug, Clone)]
pub struct StaticSourceConfig {
    pub crawl: Option<CrawlConfig>,
    pub next_page: Option<FieldLocator>,
    pub detail_link: Option<FieldLocator>,
    pub follow_detail: bool,
    pub detail_fields: BTreeMap<String, FieldLocator>,
    pub detail_container: Option<QueryEngine>,
    pub wait_for: Option<QueryEngine>,
    pub load_more: Option<QueryEngine>,
    pub load_more_raw: Option<String>,
    pub navigation_timeout_s: u64,
    pub max_pages: u32,
    pub duration_s: u64,
    pub per_host_delay_ms: Option<u64>,
    pub jitter_ms: Option<u64>,
    pub ignore_robots: bool,
}
impl StaticSourceConfig {
    pub fn parse(config: &Value, limits: &WorkerLimits) -> Result<Self, String> {
        let mode = crate::extraction::mode(config)?;
        let crawl = CrawlConfig::parse(config)?;
        let locator = |key| FieldLocator::from_config(config, key, None);
        let query = |key: &str| {
            config
                .get(key)
                .map(|v| {
                    let raw = v
                        .as_str()
                        .ok_or_else(|| format!("{key}: expected CSS/XPath locator"))?;
                    QueryEngine::parse_elements(raw).map_err(|e| format!("{key}: {e}"))
                })
                .transpose()
        };
        let source_limits = crate::extraction::limits(config)?;
        let navigation_timeout_s = if config.get("navigation_timeout_s").is_some() {
            source_limits.navigation_timeout_s
        } else {
            limits.navigation_timeout_s
        };
        let max_pages = if config.get("max_pages").is_some() {
            source_limits.max_pages_per_source
        } else {
            limits.max_pages_per_source
        };
        let duration_s = if config.get("max_source_duration_s").is_some() {
            source_limits.max_source_duration_s
        } else {
            limits.max_source_duration_s
        };
        let follow_detail = config
            .get("follow_detail")
            .map(|v| v.as_bool().ok_or("follow_detail must be true or false"))
            .transpose()?
            .unwrap_or(false);
        let detail_link = locator("detail")?;
        let mut detail_fields = BTreeMap::new();
        if let Some(fields) = config.get("detail_fields") {
            let fields = fields.as_object().ok_or(
                "detail_fields must be a mapping of location, salary, description or date locators",
            )?;
            for key in fields.keys() {
                if !["location", "salary", "description", "date"]
                    .iter()
                    .any(|field| {
                        key == field
                            || key == &format!("{field}_attribute")
                            || key == &format!("{field}_regex")
                    })
                {
                    return Err(format!(
                        "detail_fields.{key}: identity fields and unknown settings are not allowed"
                    ));
                }
            }
            for field in ["location", "salary", "description", "date"] {
                if let Some(locator) =
                    FieldLocator::from_config(&Value::Object(fields.clone()), field, None)?
                {
                    detail_fields.insert(field.into(), locator);
                }
            }
            if !follow_detail && detail_link.is_none() {
                return Err(
                    "detail_fields requires follow_detail: true or a detail link locator".into(),
                );
            }
        }
        if (follow_detail || detail_link.is_some()) && detail_fields.is_empty() {
            return Err(
                "Detail fetching requires nonempty detail_fields with explicit locators".into(),
            );
        }
        if detail_link.is_some() && HydrationConfig::parse(config)?.is_some() {
            return Err("Hydration detail fetching uses follow_detail: true and the candidate destination; a DOM detail link cannot select a JSON field".into());
        }
        if config.get("detail_container").is_some() && !follow_detail && detail_link.is_none() {
            return Err("detail_container requires enabled detail fetching".into());
        }
        if config.get("load_more").is_some() && mode == crate::extraction::ExtractionMode::Static {
            return Err(
                "load_more requires browser extraction; static HTTP cannot click controls".into(),
            );
        }
        let load_more = query("load_more")?;
        let load_more_raw = config
            .get("load_more")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string());
        Ok(Self {
            max_pages: crawl
                .as_ref()
                .map_or(max_pages, |crawl| max_pages.min(crawl.max_pages))
                .min(limits.max_pages_per_source),
            crawl,
            next_page: locator("next_page")?,
            detail_link,
            follow_detail,
            detail_fields,
            detail_container: query("detail_container")?,
            wait_for: query("wait_for")?,
            load_more,
            load_more_raw,
            navigation_timeout_s: navigation_timeout_s.min(limits.navigation_timeout_s),
            duration_s: duration_s.min(limits.max_source_duration_s),
            per_host_delay_ms: config.get("per_host_delay_ms").and_then(Value::as_u64),
            jitter_ms: source_limits.jitter_ms.or(limits.jitter_ms),
            ignore_robots: source_limits
                .ignore_robots
                .or(limits.ignore_robots)
                .unwrap_or(false),
        })
    }
    pub fn has_details(&self) -> bool {
        self.follow_detail || self.detail_link.is_some()
    }
}

#[cfg(test)]
mod crawl_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn crawl_defaults_and_worker_bounds_are_authoritative() {
        let limits = WorkerLimits {
            max_pages_per_source: 7,
            ..WorkerLimits::default()
        };
        let parsed =
            StaticSourceConfig::parse(&json!({"crawl":{"extraction":"jsonld"}}), &limits).unwrap();
        let crawl = parsed.crawl.unwrap();
        assert_eq!(crawl.max_pages, 20);
        assert_eq!(crawl.max_depth, 2);
        assert_eq!(crawl.extraction, CrawlExtraction::Jsonld);
        assert_eq!(parsed.max_pages, 7);
        assert!(StaticSourceConfig::parse(&json!({}), &limits)
            .unwrap()
            .crawl
            .is_none());
        for depth in [0, 5] {
            assert!(StaticSourceConfig::parse(
                &json!({"crawl":{"extraction":"jsonld","max_pages":100,"max_depth":depth}}),
                &WorkerLimits::default()
            )
            .is_ok());
        }
    }

    #[test]
    fn crawl_rejects_malformed_or_unbounded_configuration() {
        for crawl in [
            json!(null),
            json!(true),
            json!([]),
            json!({}),
            json!({"extraction":"automatic"}),
            json!({"extraction":"jsonld","unknown":true}),
            json!({"extraction":"jsonld","max_pages":0}),
            json!({"extraction":"jsonld","max_pages":101}),
            json!({"extraction":"jsonld","max_pages":1.5}),
            json!({"extraction":"jsonld","max_pages":"20"}),
            json!({"extraction":"jsonld","max_depth":-1}),
            json!({"extraction":"jsonld","max_depth":6}),
            json!({"extraction":"jsonld","max_depth":2.5}),
        ] {
            assert!(
                StaticSourceConfig::parse(&json!({"crawl":crawl}), &WorkerLimits::default())
                    .is_err(),
                "{crawl}"
            );
        }
    }

    #[test]
    fn crawl_rejects_other_navigation_modes_and_unsafe_urls() {
        for extra in [
            json!({"mode":"browser"}),
            json!({"mode":"script","script":{"runtime":"js","code":"ignored"}}),
            json!({"script":{"runtime":"js","code":"ignored"}}),
            json!({"next_page":"a.next"}),
            json!({"next_page_attribute":"href"}),
            json!({"next_page_regex":".*"}),
            json!({"url":"file:///tmp/careers"}),
            json!({"url":"javascript:alert(1)"}),
            json!({"url":"https://user:secret@careers.test/"}),
            json!({"url":"https://user@careers.test/"}),
            json!({"url":42}),
        ] {
            let mut config = extra;
            config["crawl"] = json!({"extraction":"jsonld"});
            assert!(
                StaticSourceConfig::parse(&config, &WorkerLimits::default()).is_err(),
                "{config}"
            );
        }
    }

    #[test]
    fn crawl_selector_mode_requires_explicit_valid_locators() {
        let limits = WorkerLimits::default();
        assert!(
            StaticSourceConfig::parse(&json!({"crawl":{"extraction":"selectors"}}), &limits)
                .is_err()
        );
        let config =
            json!({"crawl":{"extraction":"selectors"},"item":"article","title":"h2","link":"a"});
        assert!(StaticSourceConfig::parse(&config, &limits).is_ok());
        for key in ["item", "title", "link"] {
            let mut missing = config.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(
                StaticSourceConfig::parse(&missing, &limits).is_err(),
                "{key}"
            );
        }
    }
}
