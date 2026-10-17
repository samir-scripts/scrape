//! Authoritative static pagination/detail configuration shared by save and execution.
use crate::{
    hydration::HydrationConfig,
    locator::{FieldLocator, QueryEngine},
    WorkerLimits,
};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone)]
pub struct StaticSourceConfig {
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
}
impl StaticSourceConfig {
    pub fn parse(config: &Value, limits: &WorkerLimits) -> Result<Self, String> {
        let mode = crate::extraction::mode(config)?;
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
            next_page: locator("next_page")?,
            detail_link,
            follow_detail,
            detail_fields,
            detail_container: query("detail_container")?,
            wait_for: query("wait_for")?,
            load_more,
            load_more_raw,
            navigation_timeout_s: navigation_timeout_s.min(limits.navigation_timeout_s),
            max_pages: max_pages.min(limits.max_pages_per_source),
            duration_s: duration_s.min(limits.max_source_duration_s),
        })
    }
    pub fn has_details(&self) -> bool {
        self.follow_detail || self.detail_link.is_some()
    }
}
