use crate::db::sources::CreateSourceInput;
use crate::error::{AppError, Result};
use scrape_core::enrich::CompensationFilter;
use scrape_core::matching::PostingRules;
use scrape_core::{RoleType, SourceCadence, SourceKind, WorkerFilters};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

macro_rules! optional_fields {
    ($name:ident { $($field:ident: $kind:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
        #[serde(deny_unknown_fields)]
        #[ts(export)]
        pub struct $name { $(#[serde(default, skip_serializing_if="Option::is_none")] #[ts(optional = nullable)] pub $field: Option<$kind>),* }
    };
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WorkflowMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub company: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub enabled: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WorkflowExtractor {
    pub kind: SourceKind,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub source_config: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub job_container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub title_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub url_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub location_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub salary_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub description_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub date_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub title_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub link_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub location_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub salary_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub url_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub url_patterns: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub exclude_url_patterns: Option<Vec<String>>,
}
optional_fields!(WorkflowSchedule { cadence: SourceCadence, time: String, cron: String, timezone: String, weekdays: Vec<u8> });
optional_fields!(WorkflowFilters { role_types: Vec<RoleType>, include_keywords: Vec<String>, exclude_keywords: Vec<String>, locations: Vec<String>, query: String, policy: Value, compensation: CompensationFilter, matching: PostingRules, experience_min: u32, experience_max: u32 });
optional_fields!(WorkflowRetention {
    days: u32,
    policy: String
});
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WorkflowSpec {
    pub version: u32,
    pub metadata: WorkflowMetadata,
    pub extractor: WorkflowExtractor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub schedule: Option<WorkflowSchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub filters: Option<WorkflowFilters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub retention: Option<WorkflowRetention>,
}

pub fn validate(value: Value, default_timezone: &str) -> Result<WorkflowSpec> {
    let mut value = value;
    let version = value.get("version").and_then(Value::as_u64).unwrap_or(2);
    if version == 1 {
        // Legacy yamlJob shape is explicitly migrated; unknown legacy fields
        // remain errors, never silently dropped by the migration.
        let allowed = ["version", "company", "url", "cadence", "engine"];
        let object = value
            .as_object()
            .ok_or_else(|| AppError::Validation("workflow: expected mapping".into()))?;
        if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
            return Err(AppError::Validation(format!(
                "workflow.{key}: unknown legacy field"
            )));
        }
        let engine = value
            .get("engine")
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::Validation("engine: expected mapping".into()))?;
        if let Some(key) = engine
            .keys()
            .find(|key| !["kind", "selectors", "regex"].contains(&key.as_str()))
        {
            return Err(AppError::Validation(format!(
                "engine.{key}: unknown legacy field"
            )));
        }
        let mut extractor = serde_json::json!({"kind":engine.get("kind"),"url":value.get("url")});
        for (group, mappings) in [
            (
                "selectors",
                vec![
                    ("item", "job_container"),
                    ("title", "title_selector"),
                    ("link", "url_selector"),
                    ("location", "location_selector"),
                    ("salary", "salary_selector"),
                ],
            ),
            (
                "regex",
                vec![
                    ("title", "title_regex"),
                    ("link", "link_regex"),
                    ("location", "location_regex"),
                    ("salary", "salary_regex"),
                ],
            ),
        ] {
            if let Some(object) = engine.get(group).and_then(Value::as_object) {
                for (key, field) in object {
                    let target = mappings
                        .iter()
                        .find(|(legacy, _)| *legacy == key)
                        .ok_or_else(|| {
                            AppError::Validation(format!("engine.{group}.{key}: unknown field"))
                        })?
                        .1;
                    extractor[target] = field.clone();
                }
            }
        }
        value = serde_json::json!({"version":3,"metadata":{"name":value.get("company"),"company":value.get("company")},"extractor":extractor,"schedule":{"cadence":value.pointer("/cadence/type").cloned().unwrap_or(Value::String("global".into())),"cron":value.pointer("/cadence/cron")}});
    } else if matches!(version, 2 | 3) {
        value["version"] = Value::from(3);
    } else {
        return Err(AppError::Validation(format!(
            "version: unsupported workflow version {version}; supported versions are 1, 2, 3"
        )));
    }
    let text = serde_json::to_string(&value)?;
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    let mut spec: WorkflowSpec = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|e| AppError::Validation(format!("{}: {}", e.path(), e.inner())))?;
    spec.metadata.name = spec.metadata.name.trim().into();
    if spec.metadata.name.is_empty() {
        return Err(AppError::Validation(
            "metadata.name: must not be blank".into(),
        ));
    }
    let mut filters = spec.filters.clone().unwrap_or_default();
    if filters.experience_min.is_some() || filters.experience_max.is_some() {
        let matching = filters.matching.get_or_insert_with(PostingRules::default);
        if filters.experience_min.is_some() {
            matching.experience_min = filters.experience_min.take().map(i64::from);
        }
        if filters.experience_max.is_some() {
            matching.experience_max = filters.experience_max.take().map(i64::from);
        }
        spec.filters = Some(filters);
    }
    let input = source_input(&spec, default_timezone)?;
    crate::services::runner::resolve_worker_filters(
        input.filters.as_ref().unwrap_or(&Value::Null),
        WorkerFilters::default(),
    )?;
    Ok(spec)
}

pub fn source_input(spec: &WorkflowSpec, timezone: &str) -> Result<CreateSourceInput> {
    let extractor = &spec.extractor;
    let identifier = if extractor.kind.expects_url() {
        scrape_core::identity::normalized_detail_url(&extractor.url)
            .map_err(AppError::Validation)?;
        extractor.url.clone()
    } else if !extractor.url.contains("://") {
        extractor.url.trim().to_string()
    } else {
        let url = url::Url::parse(&extractor.url)
            .map_err(|e| AppError::Validation(format!("extractor.url: {e}")))?;
        let valid = match extractor.kind {
            SourceKind::Greenhouse => matches!(
                url.host_str(),
                Some("boards.greenhouse.io" | "job-boards.greenhouse.io")
            ),
            SourceKind::Lever => url.host_str() == Some("jobs.lever.co"),
            SourceKind::Ashby => url.host_str() == Some("jobs.ashbyhq.com"),
            _ => false,
        };
        if !valid {
            return Err(AppError::Validation(
                "extractor.url: use a supported board URL/identifier or run URL detection".into(),
            ));
        }
        url.path_segments()
            .and_then(|mut segments| segments.next())
            .unwrap_or("")
            .into()
    };
    if !extractor.kind.expects_url()
        && (identifier.is_empty()
            || !identifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)))
    {
        return Err(AppError::Validation(
            "extractor.url: invalid ATS account identifier".into(),
        ));
    }
    let mut config = extractor
        .source_config
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    if !config.is_object() {
        return Err(AppError::Validation(
            "extractor.source_config: must be a mapping".into(),
        ));
    }
    config["url"] = Value::String(extractor.url.clone());
    for (key, value) in [
        ("item", &extractor.job_container),
        ("title", &extractor.title_selector),
        ("link", &extractor.url_selector),
        ("location", &extractor.location_selector),
        ("salary", &extractor.salary_selector),
        ("description", &extractor.description_selector),
        ("date", &extractor.date_selector),
        ("title_regex", &extractor.title_regex),
        ("link_regex", &extractor.link_regex),
        ("location_regex", &extractor.location_regex),
        ("salary_regex", &extractor.salary_regex),
        ("url_pattern", &extractor.url_pattern),
    ] {
        if let Some(value) = value {
            config[key] = Value::String(value.clone());
        }
    }
    if let Some(patterns) = &extractor.url_patterns {
        config["url_patterns"] = serde_json::to_value(patterns)?;
    }
    if let Some(patterns) = &extractor.exclude_url_patterns {
        config["exclude_url_patterns"] = serde_json::to_value(patterns)?;
    }
    validate_config(extractor.kind, &config)?;
    let schedule = spec.schedule.clone().unwrap_or_default();
    let schedule_timezone = schedule.timezone.as_deref().unwrap_or(timezone);
    schedule_timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| AppError::Validation("schedule.timezone: expected an IANA timezone".into()))?;
    config["__cron_timezone"] = Value::String(schedule_timezone.into());
    let cadence = schedule.cadence.unwrap_or(SourceCadence::Global);
    if matches!(cadence, SourceCadence::Daily | SourceCadence::Weekly) {
        let calendar = super::calendar::CalendarSchedule {
            time: schedule.time.unwrap_or_else(|| "09:00".into()),
            weekdays: schedule.weekdays.unwrap_or_else(|| {
                if cadence == SourceCadence::Weekly {
                    vec![1]
                } else {
                    vec![]
                }
            }),
            timezone: schedule.timezone.unwrap_or_else(|| timezone.into()),
        };
        calendar.validate()?;
        config["__schedule"] = serde_json::to_value(calendar)?;
    } else if cadence == SourceCadence::Cron
        && super::scheduler::calculate_next_run(
            schedule.cron.as_deref().unwrap_or(""),
            schedule.timezone.as_deref().unwrap_or(timezone),
        )
        .is_none()
    {
        return Err(AppError::Validation(
            "schedule.cron: invalid cron/timezone".into(),
        ));
    }
    let retention = spec.retention.clone().unwrap_or_default();
    if retention
        .policy
        .as_deref()
        .is_some_and(|value| !matches!(value, "all" | "untracked_only"))
    {
        return Err(AppError::Validation(
            "retention.policy: unsupported policy".into(),
        ));
    }
    config["__workflow_spec"] = serde_json::to_value(spec)?;
    if let Some(company) = &spec.metadata.company {
        config["__company"] = Value::String(company.clone());
    }
    Ok(CreateSourceInput {
        kind: extractor.kind,
        name: spec.metadata.name.clone(),
        identifier,
        config: Some(config),
        enabled: Some(spec.metadata.enabled.unwrap_or(true)),
        cadence: Some(cadence),
        cadence_cron: schedule.cron,
        retention_days: Some(i64::from(retention.days.unwrap_or(30))),
        retention_policy: Some("untracked_only".into()),
        filters: Some(serde_json::to_value(
            spec.filters.clone().unwrap_or_default(),
        )?),
    })
}
pub fn validate_config(kind: SourceKind, config: &Value) -> Result<()> {
    let allowed = [
        "url",
        "item",
        "title",
        "link",
        "location",
        "salary",
        "description",
        "date",
        "follow_links",
        "max_pages",
        "title_regex",
        "link_regex",
        "location_regex",
        "salary_regex",
        "url_pattern",
        "url_patterns",
        "exclude_url_patterns",
    ];
    for key in config
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
    {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::Validation(format!(
                "extractor.source_config.{key}: unsupported field"
            )));
        }
    }
    if config.get("max_pages").is_some_and(|value| {
        value
            .as_u64()
            .is_none_or(|pages| !(1..=100).contains(&pages))
    }) {
        return Err(AppError::Validation(
            "extractor.source_config.max_pages: expected 1–100".into(),
        ));
    }
    for key in [
        "item",
        "title",
        "link",
        "location",
        "salary",
        "description",
        "date",
        "follow_links",
    ] {
        if let Some(value) = config.get(key) {
            let selector = value.as_str().ok_or_else(|| {
                AppError::Validation(format!("extractor.{key}: expected selector string"))
            })?;
            scraper::Selector::parse(selector).map_err(|e| {
                AppError::Validation(format!("extractor.{key}: invalid selector {e}"))
            })?;
        }
    }
    if kind == SourceKind::Selector
        && ["item", "title", "link"].iter().any(|key| {
            config
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(|value| value.trim().is_empty())
        })
    {
        return Err(AppError::Validation(
            "extractor: selector sources require item, title and link selectors".into(),
        ));
    }
    for key in [
        "title_regex",
        "link_regex",
        "location_regex",
        "salary_regex",
    ] {
        if let Some(value) = config.get(key) {
            regex::Regex::new(value.as_str().ok_or_else(|| {
                AppError::Validation(format!("extractor.{key}: expected regex string"))
            })?)
            .map_err(|e| AppError::Validation(format!("extractor.{key}: {e}")))?;
        }
    }
    for key in ["url_patterns", "exclude_url_patterns"] {
        if let Some(value) = config.get(key) {
            if value
                .as_array()
                .is_none_or(|values| values.iter().any(|v| !v.is_string()))
            {
                return Err(AppError::Validation(format!(
                    "extractor.{key}: expected array of patterns"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_unknown_paths_queries_and_empty_overrides_are_validated() {
        let base = serde_json::json!({"version":2,"metadata":{"name":"Acme"},"extractor":{"kind":"greenhouse","url":"https://boards.greenhouse.io/acme"},"filters":{"include_keywords":[]},"schedule":{"cadence":"daily","time":"09:00","timezone":"America/New_York","weekdays":[1,3,5]}});
        let spec = validate(base.clone(), "UTC").unwrap();
        assert_eq!(spec.version, 3);
        assert_eq!(
            spec.filters.as_ref().unwrap().include_keywords,
            Some(vec![])
        );
        let input = source_input(&spec, "UTC").unwrap();
        assert_eq!(input.identifier, "acme");
        assert_eq!(
            input.config.unwrap()["__schedule"]["timezone"],
            "America/New_York"
        );
        for (path, value, expected) in [
            ("/version", Value::from(99), "version"),
            (
                "/extractor/url",
                Value::String("javascript:alert(1)".into()),
                "extractor",
            ),
            (
                "/schedule/timezone",
                Value::String("Unknown/Zone".into()),
                "timezone",
            ),
        ] {
            let mut bad = base.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(validate(bad, "UTC")
                .unwrap_err()
                .to_string()
                .contains(expected));
        }
        let mut bad = base.clone();
        bad["filters"]["query"] = Value::String("title LIKE".into());
        assert!(validate(bad, "UTC").is_err());
        let mut bad = base;
        bad["extractor"]["ignored_setting"] = Value::Bool(true);
        assert!(validate(bad, "UTC")
            .unwrap_err()
            .to_string()
            .contains("extractor"));
    }
}
