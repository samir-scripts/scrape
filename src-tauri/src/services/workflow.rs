use crate::db::sources::CreateSourceInput;
use crate::error::{AppError, Result};
use scrape_core::enrich::CompensationFilter;
pub use scrape_core::extraction::{ExtractionMode, WorkflowScriptConfig};
use scrape_core::matching::PostingRules;
use scrape_core::{RoleType, SourceCadence, SourceKind, WorkerFilters};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
pub const WORKFLOW_SPEC_VERSION: u32 = 10;

macro_rules! optional_fields {
    ($name:ident { $($field:ident: $kind:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
        #[serde(deny_unknown_fields)]
        #[ts(export)]
        pub struct $name { $(#[serde(default, skip_serializing_if="Option::is_none")] #[ts(optional = nullable)] pub $field: Option<$kind>),* }
    };
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
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
    pub employer_logo_selector: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub mode: Option<ExtractionMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub id_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub id_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub date_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub description_regex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub title_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub url_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub location_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub salary_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub description_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub date_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub id_attribute: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub detail_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub next_page_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub load_more_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub wait_for_selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number")]
    pub navigation_timeout_s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub max_pages: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number")]
    pub max_source_duration_s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number")]
    pub max_workflow_duration_s: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub script: Option<WorkflowScriptConfig>,
}
optional_fields!(WorkflowSchedule { cadence: SourceCadence, time: String, cron: String, timezone: String, weekdays: Vec<u8>, run_on_startup: bool });
optional_fields!(WorkflowFilters { role_types: Vec<RoleType>, include_keywords: Vec<String>, exclude_keywords: Vec<String>, locations: Vec<String>, query: String, policy: Value, compensation: CompensationFilter, matching: PostingRules, experience_min: u32, experience_max: u32 });

optional_fields!(WorkflowRetention {
    days: u32,
    policy: String
});

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WorkflowSourceSpec {
    pub id: String,
    pub label: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub employer_hint: Option<String>,
    pub extractor: WorkflowExtractor,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WorkflowSpec {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub max_jobs_per_run: Option<u32>,
    pub metadata: WorkflowMetadata,
    pub sources: Vec<WorkflowSourceSpec>,
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

optional_fields!(LegacySelectors {
    item: String,
    title: String,
    link: String,
    location: String,
    salary: String
});
optional_fields!(LegacyRegex {
    title: String,
    link: String,
    location: String,
    salary: String
});
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LegacyCadence {
    pub r#type: SourceCadence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub cron: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LegacyEngine {
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub selectors: Option<LegacySelectors>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub regex: Option<LegacyRegex>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct LegacyWorkflowSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub version: Option<u32>,
    pub company: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub cadence: Option<LegacyCadence>,
    pub engine: LegacyEngine,
}

pub fn validate(value: Value, default_timezone: &str) -> Result<WorkflowSpec> {
    let mut value = value;
    if !value.is_object() {
        return Err(AppError::Validation("workflow: expected a mapping".into()));
    }
    let version = match value.get("version") {
        None => {
            if value.get("engine").is_some() {
                1
            } else {
                2
            }
        }
        Some(version) => version
            .as_u64()
            .ok_or_else(|| AppError::Validation("version: expected an integer".into()))?,
    };
    let submitted_version = version;
    if version == 1 {
        if let Some(cadence) = value.pointer_mut("/cadence/type") {
            match cadence.as_str() {
                Some("custom_cron") => *cadence = Value::String("cron".into()),
                Some("every_1h") => *cadence = Value::String("hourly".into()),
                _ => {}
            }
        }
        let encoded = serde_json::to_string(&value)?;
        let mut decoder = serde_json::Deserializer::from_str(&encoded);
        let _: LegacyWorkflowSpec = serde_path_to_error::deserialize(&mut decoder)
            .map_err(|e| AppError::Validation(format!("{}: {}", e.path(), e.inner())))?;
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
    } else if matches!(version, 2..=10) {
        // Legacy extractor roots become source collections before decoding.
    } else {
        return Err(AppError::Validation(format!(
            "version: unsupported workflow version {version}; supported versions are 1–10"
        )));
    }
    if submitted_version <= 4 && value.get("sources").is_none() {
        let extractor = value.get("extractor").cloned().unwrap_or(Value::Null);
        let name = value
            .pointer("/metadata/name")
            .and_then(Value::as_str)
            .unwrap_or("Default source")
            .to_string();
        let enabled = value
            .pointer("/metadata/enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let employer_hint = value
            .pointer("/metadata/company")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let source_id = "default".to_string();
        value["sources"] = serde_json::json!([{
            "id": source_id,
            "label": name,
            "enabled": enabled,
            "employer_hint": employer_hint,
            "extractor": extractor,
        }]);
        if let Some(obj) = value.as_object_mut() {
            obj.remove("extractor");
        }
    }
    // YAML aliases normalize to the existing selector kind; the database CHECK
    // and provider identity vocabulary remain unchanged.
    if let Some(sources) = value.get_mut("sources").and_then(Value::as_array_mut) {
        for source in sources {
            if let Some(extractor) = source.get_mut("extractor").and_then(Value::as_object_mut) {
                if let Some(alias) = extractor
                    .get("kind")
                    .and_then(Value::as_str)
                    .filter(|kind| matches!(*kind, "browser" | "script"))
                    .map(str::to_owned)
                {
                    if extractor
                        .get("mode")
                        .is_some_and(|mode| mode != &Value::String(alias.clone()))
                    {
                        return Err(AppError::Validation(
                            "extractor.mode: conflicts with extractor.kind alias".into(),
                        ));
                    }
                    extractor.insert("kind".into(), Value::String("selector".into()));
                    extractor.insert("mode".into(), Value::String(alias));
                }
            }
        }
    }
    if let Some(filters) = value.get_mut("filters").and_then(Value::as_object_mut) {
        if (filters.get("experience_min").is_some_and(|v| !v.is_null())
            || filters.get("experience_max").is_some_and(|v| !v.is_null()))
            && filters.get("matching").is_none_or(Value::is_null)
        {
            filters.insert("matching".into(), serde_json::json!({}));
        }
    }
    if let Some(matching) = value
        .pointer_mut("/filters/matching")
        .and_then(Value::as_object_mut)
    {
        if submitted_version <= 6 {
            matching.insert(
                "experience_match_mode".into(),
                Value::String("advertised_range_overlap".into()),
            );
        } else {
            matching
                .entry("experience_match_mode")
                .or_insert(Value::String("required_minimum".into()));
        }
    }
    value["version"] = Value::from(WORKFLOW_SPEC_VERSION);
    let text = serde_json::to_string(&value)?;
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    let mut spec: WorkflowSpec = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|e| AppError::Validation(format!("{}: {}", e.path(), e.inner())))?;
    if submitted_version >= 4
        && spec.schedule.as_ref().and_then(|schedule| schedule.cadence)
            == Some(SourceCadence::Global)
    {
        return Err(AppError::Validation(
            "schedule.cadence: version 4 and newer must use an explicit workflow schedule".into(),
        ));
    }
    spec.max_jobs_per_run = Some(
        scrape_core::protocol::validate_job_limit(
            spec.max_jobs_per_run
                .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN),
        )
        .map_err(AppError::Validation)?,
    );
    // Versions 5 and 6 are fully explicit. Defaults apply only to fields omitted
    // by the submitted document; stored/imported v1-v3 values are resolved by
    // the compatibility entry point before they reach this normalizer.
    if spec.version == WORKFLOW_SPEC_VERSION {
        let schedule = spec.schedule.get_or_insert_with(WorkflowSchedule::default);
        schedule.cadence.get_or_insert(SourceCadence::Daily);
        schedule.time.get_or_insert_with(|| "09:00".into());
        schedule
            .timezone
            .get_or_insert_with(|| default_timezone.into());
        schedule.run_on_startup.get_or_insert(false);
        let filters = spec.filters.get_or_insert_with(WorkflowFilters::default);
        filters.role_types.get_or_insert_with(Vec::new);
        filters.include_keywords.get_or_insert_with(Vec::new);
        filters.exclude_keywords.get_or_insert_with(Vec::new);
        filters.locations.get_or_insert_with(Vec::new);
        let retention = spec
            .retention
            .get_or_insert_with(WorkflowRetention::default);
        retention.days.get_or_insert(30);
        retention
            .policy
            .get_or_insert_with(|| "untracked_only".into());
    }
    spec.version = WORKFLOW_SPEC_VERSION;
    spec.metadata.enabled.get_or_insert(true);
    spec.metadata.name = spec.metadata.name.trim().into();
    if spec.metadata.name.is_empty() {
        return Err(AppError::Validation(
            "metadata.name: must not be blank".into(),
        ));
    }
    if spec.sources.is_empty() {
        return Err(AppError::Validation(
            "sources: must contain at least one source".into(),
        ));
    }
    if spec.metadata.enabled == Some(true) && !spec.sources.iter().any(|s| s.enabled) {
        return Err(AppError::Validation(
            "sources: at least one source must be enabled".into(),
        ));
    }
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_endpoints = std::collections::HashSet::new();
    for source in &spec.sources {
        let trimmed_id = source.id.trim();
        if trimmed_id.is_empty() {
            return Err(AppError::Validation("sources.id: must not be blank".into()));
        }
        if !seen_ids.insert(trimmed_id.to_string()) {
            return Err(AppError::Validation(format!(
                "sources: duplicate source ID {}",
                source.id
            )));
        }
        let endpoint = normalized_source_endpoint(&source.extractor)?;
        if !seen_endpoints.insert(endpoint.clone()) {
            return Err(AppError::Validation(format!(
                "sources: duplicate source endpoint {} within workflow",
                endpoint
            )));
        }
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
    for source in &spec.sources {
        let input = extract_source_input(&spec, source, default_timezone)?;
        crate::services::runner::resolve_worker_filters(
            input.filters.as_ref().unwrap_or(&Value::Null),
        )?;
    }
    Ok(spec)
}

pub fn normalized_source_endpoint(extractor: &WorkflowExtractor) -> Result<String> {
    if extractor.kind == SourceKind::Search {
        let provider =
            scrape_core::search_source::identifier(&extractor.url).map_err(AppError::Validation)?;
        let config = extractor.source_config.as_ref().unwrap_or(&Value::Null);
        scrape_core::search_source::validate_config(config).map_err(AppError::Validation)?;
        scrape_core::search_source::endpoint(provider, config).map_err(AppError::Validation)?;
        return Ok(format!("Search:{provider}"));
    }
    if extractor.kind.expects_url() {
        if extractor
            .source_config
            .as_ref()
            .is_some_and(|config| config.get("crawl").is_some())
            && !scrape_core::branding::http_url(&extractor.url)
        {
            return Err(AppError::Validation(
                "extractor.url: website crawl requires an HTTP(S) URL without credentials".into(),
            ));
        }
        let normalized = scrape_core::identity::normalized_detail_url(&extractor.url)
            .map_err(AppError::Validation)?;
        if extractor.kind == SourceKind::Workday {
            let parsed = url::Url::parse(&normalized)
                .map_err(|e| AppError::Validation(format!("extractor.url: {e}")))?;
            let valid = parsed
                .host_str()
                .map(|h| h.ends_with(".myworkdayjobs.com"))
                .unwrap_or(false);
            if !valid {
                return Err(AppError::Validation(
                    "extractor.url: Workday URL must end with .myworkdayjobs.com".into(),
                ));
            }
        }
        Ok(format!("{:?}:{}", extractor.kind, normalized))
    } else if !extractor.url.contains("://") {
        let slug = extractor.url.trim().to_lowercase();
        if slug.is_empty()
            || !slug
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        {
            return Err(AppError::Validation(
                "extractor.url: invalid ATS account identifier".into(),
            ));
        }
        Ok(format!("{:?}:{}", extractor.kind, slug))
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
        let slug = url
            .path_segments()
            .and_then(|mut segments| segments.next())
            .unwrap_or("")
            .to_lowercase();
        if slug.is_empty()
            || !slug
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        {
            return Err(AppError::Validation(
                "extractor.url: invalid ATS account identifier".into(),
            ));
        }
        Ok(format!("{:?}:{}", extractor.kind, slug))
    }
}

/// Resolve legacy inheritance once at the import boundary. Version 5 never
/// consults application matching or calendar settings after this conversion.
pub fn validate_legacy_import(
    value: Value,
    default_timezone: &str,
    global_cron: &str,
    global_filters: &WorkerFilters,
    global_run_on_startup: bool,
) -> Result<WorkflowSpec> {
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| if value.get("engine").is_some() { 1 } else { 2 });
    let legacy_filter_values = value
        .get("filters")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let legacy_cadence = if version == 1 {
        value.pointer("/cadence/type")
    } else {
        value.pointer("/schedule/cadence")
    };
    let inherited_schedule =
        legacy_cadence.is_none_or(|cadence| cadence.is_null() || cadence == "global");
    let mut spec = validate(value, default_timezone)?;
    if version < 4 {
        let schedule = spec.schedule.get_or_insert_with(WorkflowSchedule::default);
        if inherited_schedule {
            schedule.cadence = Some(SourceCadence::Cron);
            schedule.cron = Some(global_cron.to_owned());
            schedule.timezone = Some(default_timezone.to_owned());
            schedule.run_on_startup = Some(global_run_on_startup);
        }
        let filters = spec.filters.get_or_insert_with(WorkflowFilters::default);
        let mut merged = serde_json::to_value(&*filters)?
            .as_object()
            .cloned()
            .ok_or_else(|| AppError::Validation("workflow.filters: expected mapping".into()))?;
        for key in [
            "include_keywords",
            "exclude_keywords",
            "locations",
            "role_types",
        ] {
            if legacy_filter_values
                .get(key)
                .is_none_or(|value| value.is_null())
            {
                merged.remove(key);
            }
        }
        // Legacy application defaults consisted only of these four fields.
        // The shared core resolver owns overlay/validation; modern workflows
        // never enter this branch or consult application matching settings.
        let inherited = WorkerFilters {
            include_keywords: global_filters.include_keywords.clone(),
            exclude_keywords: global_filters.exclude_keywords.clone(),
            locations: global_filters.locations.clone(),
            role_types: global_filters.role_types.clone(),
            ..Default::default()
        };
        let resolved =
            WorkerFilters::resolve_compatibility(&Value::Object(merged), Some(&inherited))
                .map_err(AppError::Validation)?;
        *filters = serde_json::from_value(serde_json::to_value(resolved)?)?;
    }
    spec.version = WORKFLOW_SPEC_VERSION;
    let spec = validate(serde_json::to_value(spec)?, default_timezone)?;
    let input = source_input(&spec, default_timezone)?;
    crate::services::runner::resolve_worker_filters(
        input.filters.as_ref().unwrap_or(&Value::Null),
    )?;
    Ok(spec)
}

pub fn source_input(spec: &WorkflowSpec, timezone: &str) -> Result<CreateSourceInput> {
    let source = spec
        .sources
        .iter()
        .find(|s| s.enabled)
        .or_else(|| spec.sources.first())
        .ok_or_else(|| AppError::Validation("sources: must contain at least one source".into()))?;
    extract_source_input(spec, source, timezone)
}

pub fn workflow_source_inputs(
    spec: &WorkflowSpec,
    timezone: &str,
) -> Result<Vec<CreateSourceInput>> {
    spec.sources
        .iter()
        .map(|s| extract_source_input(spec, s, timezone))
        .collect()
}

pub fn extract_source_input(
    spec: &WorkflowSpec,
    source: &WorkflowSourceSpec,
    timezone: &str,
) -> Result<CreateSourceInput> {
    scrape_core::protocol::validate_job_limit(
        spec.max_jobs_per_run
            .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN),
    )
    .map_err(AppError::Validation)?;
    let extractor = &source.extractor;
    let identifier = if extractor.kind == SourceKind::Search {
        scrape_core::search_source::identifier(&extractor.url)
            .map_err(AppError::Validation)?
            .to_string()
    } else if extractor.kind.expects_url() {
        scrape_core::identity::normalized_detail_url(&extractor.url)
            .map_err(AppError::Validation)?;
        if extractor.kind == SourceKind::Workday {
            let parsed = url::Url::parse(&extractor.url)
                .map_err(|e| AppError::Validation(format!("extractor.url: {e}")))?;
            let valid = parsed
                .host_str()
                .map(|h| h.ends_with(".myworkdayjobs.com"))
                .unwrap_or(false);
            if !valid {
                return Err(AppError::Validation(
                    "extractor.url: Workday URL must end with .myworkdayjobs.com".into(),
                ));
            }
        }
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
        && extractor.kind != SourceKind::Search
        && (identifier.is_empty()
            || !identifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)))
    {
        return Err(AppError::Validation(
            "extractor.url: invalid ATS account identifier".into(),
        ));
    }
    let mut config =
        normalize_source_config(extractor.source_config.as_ref().unwrap_or(&Value::Null))?;
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
        ("employer_logo", &extractor.employer_logo_selector),
        ("title_regex", &extractor.title_regex),
        ("link_regex", &extractor.link_regex),
        ("location_regex", &extractor.location_regex),
        ("salary_regex", &extractor.salary_regex),
        ("url_pattern", &extractor.url_pattern),
        ("id", &extractor.id_selector),
        ("id_regex", &extractor.id_regex),
        ("date_regex", &extractor.date_regex),
        ("description_regex", &extractor.description_regex),
        ("title_attribute", &extractor.title_attribute),
        ("link_attribute", &extractor.url_attribute),
        ("location_attribute", &extractor.location_attribute),
        ("salary_attribute", &extractor.salary_attribute),
        ("description_attribute", &extractor.description_attribute),
        ("date_attribute", &extractor.date_attribute),
        ("id_attribute", &extractor.id_attribute),
        ("detail", &extractor.detail_selector),
        ("next_page", &extractor.next_page_selector),
        ("load_more", &extractor.load_more_selector),
        ("wait_for", &extractor.wait_for_selector),
        ("session_id", &extractor.session_id),
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
    for (key, value) in [
        ("mode", serde_json::to_value(extractor.mode)?),
        ("script", serde_json::to_value(&extractor.script)?),
        (
            "navigation_timeout_s",
            serde_json::to_value(extractor.navigation_timeout_s)?,
        ),
        ("max_pages", serde_json::to_value(extractor.max_pages)?),
        (
            "max_source_duration_s",
            serde_json::to_value(extractor.max_source_duration_s)?,
        ),
        (
            "max_workflow_duration_s",
            serde_json::to_value(extractor.max_workflow_duration_s)?,
        ),
    ] {
        if !value.is_null() {
            config[key] = value;
        }
    }
    validate_config(extractor.kind, &config)?;
    let schedule = spec.schedule.clone().unwrap_or_default();
    let schedule_timezone = schedule.timezone.as_deref().unwrap_or(timezone);
    schedule_timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| AppError::Validation("schedule.timezone: expected an IANA timezone".into()))?;
    config["__cron_timezone"] = Value::String(schedule_timezone.into());
    let cadence = schedule.cadence.unwrap_or(SourceCadence::Daily);
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
    config["__source_id"] = Value::String(source.id.clone());
    config["__run_on_startup"] = Value::Bool(schedule.run_on_startup.unwrap_or(false));
    if let Some(company) = source
        .employer_hint
        .as_ref()
        .or(spec.metadata.company.as_ref())
    {
        config["__company"] = Value::String(company.clone());
    }
    Ok(CreateSourceInput {
        kind: extractor.kind,
        name: source.label.clone(),
        identifier,
        config: Some(config),
        enabled: Some(source.enabled && spec.metadata.enabled.unwrap_or(true)),
        cadence: Some(cadence),
        cadence_cron: schedule.cron,
        retention_days: Some(i64::from(retention.days.unwrap_or(30))),
        retention_policy: Some(
            retention
                .policy
                .clone()
                .unwrap_or_else(|| "untracked_only".into()),
        ),
        filters: Some(serde_json::to_value(
            spec.filters.clone().unwrap_or_default(),
        )?),
    })
}
/// Save through the same persistence path used by IPC, with import trust explicit.
pub async fn persist(
    pool: &sqlx::SqlitePool,
    spec: WorkflowSpec,
    id: Option<String>,
    imported: bool,
    timezone: &str,
) -> Result<crate::db::sources::Source> {
    super::workflow_store::save(pool, spec, id, imported, timezone, false).await
}

pub fn normalize_source_config(config: &Value) -> Result<Value> {
    let mut config = if config.is_null() {
        serde_json::json!({})
    } else {
        config.clone()
    };
    let object = config
        .as_object_mut()
        .ok_or_else(|| AppError::Validation("extractor.source_config: expected mapping".into()))?;
    if let Some(legacy) = object.remove("selectors") {
        let selectors: LegacySelectors = serde_json::from_value(legacy).map_err(|error| {
            AppError::Validation(format!("extractor.source_config.selectors: {error}"))
        })?;
        for (key, value) in serde_json::to_value(selectors)?.as_object().unwrap() {
            object.entry(key.clone()).or_insert(value.clone());
        }
    }
    Ok(config)
}
pub fn validated_stored_config(kind: SourceKind, config: &Value) -> Result<Value> {
    let mut normalized = normalize_source_config(config)?;
    let mut public = normalized.clone();
    for key in [
        "__workflow_spec",
        "__schedule",
        "__cron_timezone",
        "__company",
        "__run_on_startup",
        "__source_id",
        "__starting_source_index",
        "__invalid_original_config",
        "__invalid_original_filters",
        "__builtin_workflow",
    ] {
        public.as_object_mut().unwrap().remove(key);
    }
    validate_config(kind, &public)?;
    if let Some(company) = normalized.get("__company") {
        if !company.is_string() {
            return Err(AppError::Validation(
                "metadata.company: expected string".into(),
            ));
        }
    }
    if let Some(spec) = normalized.get("__workflow_spec") {
        let timezone = normalized
            .pointer("/__schedule/timezone")
            .and_then(Value::as_str)
            .or_else(|| normalized.get("__cron_timezone").and_then(Value::as_str))
            .unwrap_or("UTC");
        let spec = validate(spec.clone(), timezone)?;
        normalized["__workflow_spec"] = serde_json::to_value(spec)?;
    }
    Ok(normalized)
}

pub fn validate_config(kind: SourceKind, config: &Value) -> Result<()> {
    if !config.is_object() {
        return Err(AppError::Validation(
            "extractor.source_config: expected mapping".into(),
        ));
    }
    if kind == SourceKind::Search {
        return scrape_core::search_source::validate_config(config).map_err(AppError::Validation);
    }
    let crawl =
        scrape_core::static_source::CrawlConfig::parse(config).map_err(AppError::Validation)?;
    if crawl.is_some() && kind != SourceKind::Selector {
        return Err(AppError::Validation(
            "extractor.crawl: requires kind: selector".into(),
        ));
    }
    let mode = scrape_core::extraction::mode(config).map_err(AppError::Validation)?;
    if kind != SourceKind::Selector && mode != ExtractionMode::Static {
        return Err(AppError::Validation(
            "extractor.mode: browser and script modes require kind: selector".into(),
        ));
    }
    let allowed = [
        "mode",
        "script",
        "session_id",
        "navigation_timeout_s",
        "max_workflow_duration_s",
        "load_more",
        "url",
        "item",
        "title",
        "link",
        "location",
        "salary",
        "description",
        "date",
        "employer_logo",
        "follow_links",
        "max_pages",
        "max_source_duration_s",
        "crawl",
        "next_page",
        "next_page_attribute",
        "next_page_regex",
        "follow_detail",
        "detail",
        "detail_attribute",
        "detail_regex",
        "detail_fields",
        "detail_container",
        "wait_for",
        "title_regex",
        "link_regex",
        "location_regex",
        "salary_regex",
        "date_regex",
        "description_regex",
        "id_regex",
        "id",
        "title_attribute",
        "link_attribute",
        "location_attribute",
        "salary_attribute",
        "description_attribute",
        "date_attribute",
        "id_attribute",
        "hydration_script",
        "hydration_pointer",
        "hydration_fields",
        "url_pattern",
        "url_patterns",
        "exclude_url_patterns",
        "per_host_delay_ms",
        "jitter_ms",
        "ignore_robots",
        "public_html",
    ];
    for key in config
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
    {
        let implemented = match kind {
            SourceKind::Jsonld => {
                ["url", "follow_links", "max_pages", "employer_logo"].contains(&key.as_str())
            }
            SourceKind::Selector => allowed.contains(&key.as_str()) && key != "follow_links",
            _ => key == "url",
        };
        if !implemented {
            return Err(AppError::Validation(format!(
                "extractor.source_config.{key}: unsupported field"
            )));
        }
    }
    if kind == SourceKind::Selector {
        scrape_core::extraction::limits(config).map_err(AppError::Validation)?;
        if let Some(session) = config.get("session_id") {
            if mode == ExtractionMode::Static
                || session.as_str().is_none_or(|id| {
                    id.is_empty()
                        || id.len() > 128
                        || !id
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
                })
            {
                return Err(AppError::Validation("extractor.session_id: use a saved session reference in browser or script mode; credentials and file paths are not accepted".into()));
            }
        }
        if mode == ExtractionMode::Script {
            scrape_core::extraction::script(config).map_err(AppError::Validation)?;
            for key in [
                "item",
                "title",
                "link",
                "location",
                "salary",
                "description",
                "date",
                "id",
                "hydration_script",
                "hydration_pointer",
                "hydration_fields",
                "detail",
                "next_page",
                "load_more",
                "wait_for",
            ] {
                if config.get(key).is_some() {
                    return Err(AppError::Validation(format!(
                        "extractor.{key}: DOM navigation/extraction is owned by the custom script"
                    )));
                }
            }
        }
        if let Some(value) = config.get("load_more") {
            if mode != ExtractionMode::Browser {
                return Err(AppError::Validation(
                    "extractor.load_more: requires browser mode".into(),
                ));
            }
            let query = value.as_str().ok_or_else(|| {
                AppError::Validation("extractor.load_more: expected locator string".into())
            })?;
            scrape_core::locator::QueryEngine::parse_elements(query)
                .map_err(|error| AppError::Validation(format!("extractor.load_more: {error}")))?;
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
    if kind == SourceKind::Selector && mode != ExtractionMode::Script {
        let mut static_config = config.clone();
        static_config.as_object_mut().unwrap().remove("load_more");
        scrape_core::static_source::StaticSourceConfig::parse(
            &static_config,
            &scrape_core::WorkerLimits::default(),
        )
        .map_err(AppError::Validation)?;
    }
    let hydration = if kind == SourceKind::Selector {
        scrape_core::hydration::HydrationConfig::parse(config).map_err(AppError::Validation)?
    } else {
        None
    };
    if kind == SourceKind::Selector && hydration.is_none() {
        if let Some(value) = config.get("item") {
            let query = value.as_str().ok_or_else(|| {
                AppError::Validation("extractor.item: expected locator string".into())
            })?;
            scrape_core::locator::QueryEngine::parse_elements(query)
                .map_err(|error| AppError::Validation(format!("extractor.item: {error}")))?;
        }
        for key in [
            "title",
            "link",
            "location",
            "salary",
            "description",
            "date",
            "id",
        ] {
            scrape_core::locator::FieldLocator::from_config(config, key, None)
                .map_err(|error| AppError::Validation(format!("extractor.{error}")))?;
        }
    }
    for key in ["employer_logo", "follow_links"] {
        if let Some(value) = config.get(key) {
            let selector = value.as_str().ok_or_else(|| {
                AppError::Validation(format!("extractor.{key}: expected CSS selector string"))
            })?;
            scraper::Selector::parse(selector).map_err(|error| {
                AppError::Validation(format!("extractor.{key}: invalid CSS selector {error}"))
            })?;
        }
    }
    if kind == SourceKind::Selector
        && mode != ExtractionMode::Script
        && hydration.is_none()
        && !crawl.as_ref().is_some_and(|crawl| {
            matches!(
                crawl.extraction,
                scrape_core::static_source::CrawlExtraction::Jsonld
                    | scrape_core::static_source::CrawlExtraction::PublicHtml
            )
        })
        && !config
            .get("public_html")
            .and_then(Value::as_bool)
            .unwrap_or(false)
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
    fn website_crawl_requires_explicit_extraction_and_preserves_selector_validation() {
        use serde_json::json;
        assert!(validate_config(
            SourceKind::Selector,
            &json!({"crawl":{"extraction":"jsonld"}})
        )
        .is_ok());
        assert!(validate_config(
            SourceKind::Selector,
            &json!({"crawl":{"extraction":"selectors"},"item":"article","title":"h2","link":"a"})
        )
        .is_ok());
        for config in [
            json!({}),
            json!({"crawl":{"extraction":"selectors"}}),
            json!({"crawl":{"extraction":"jsonld","unknown":1}}),
            json!({"crawl":{"extraction":"jsonld","max_pages":0}}),
            json!({"crawl":{"extraction":"jsonld"},"next_page":"a"}),
            json!({"crawl":{"extraction":"jsonld"},"mode":"browser"}),
            json!({"crawl":{"extraction":"jsonld"},"mode":"script","script":{"runtime":"js","code":"ignored"}}),
        ] {
            assert!(
                validate_config(SourceKind::Selector, &config).is_err(),
                "{config}"
            );
        }
        assert!(validate_config(
            SourceKind::Jsonld,
            &json!({"crawl":{"extraction":"jsonld"}})
        )
        .is_err());
    }

    #[test]
    fn compensation_modes_upgrade_without_changing_saved_overlap_rules() {
        use scrape_core::enrich::{CompensationMatchMode, Salary};

        assert_eq!(WORKFLOW_SPEC_VERSION, 10);
        assert_eq!(scrape_core::PROTOCOL_VERSION, 13);
        let v1 = validate(
            serde_json::json!({
                "version": 1, "company": "Legacy", "url": "https://boards.greenhouse.io/fixture",
                "engine": {"kind": "greenhouse"}
            }),
            "UTC",
        )
        .unwrap();
        assert_eq!(v1.version, 10);
        assert!(v1
            .filters
            .as_ref()
            .is_none_or(|filters| filters.compensation.is_none()));
        let lower =
            Salary::from_structured(Some(50000.0), None, Some("USD"), Some("year"), None).unwrap();
        for version in 2..=10 {
            let mut value = serde_json::json!({
                "version": version, "metadata": {"name": "Saved native pay"},
                "sources": [{"id": "fixture", "label": "Fixture",
                    "extractor": {"kind": "greenhouse", "url": "https://boards.greenhouse.io/fixture"}}],
                "filters": {
                    "compensation": {"min_minor": 6000001, "currency": "USD", "interval": "year"},
                    "matching": {"compensation": {"min_minor": 6000001, "currency": "USD", "interval": "year"}}
                }
            });
            for explicit_overlap in [false, true] {
                if explicit_overlap {
                    value["filters"]["compensation"]["match_mode"] =
                        Value::String("range_overlap".into());
                    value["filters"]["matching"]["compensation"]["match_mode"] =
                        Value::String("range_overlap".into());
                }
                let spec = validate(value.clone(), "UTC").unwrap();
                assert_eq!(spec.version, 10);
                let reopened = validate(serde_json::to_value(&spec).unwrap(), "UTC").unwrap();
                assert_eq!(spec.filters, reopened.filters);
                let filters = reopened.filters.unwrap();
                for pay in [
                    filters.compensation.unwrap(),
                    filters.matching.unwrap().compensation.unwrap(),
                ] {
                    assert_eq!(
                        pay.match_mode,
                        CompensationMatchMode::RangeOverlap,
                        "v{version}"
                    );
                    assert!(pay.matches(&lower.ranges), "v{version}");
                }
            }
        }
        let spec = validate(serde_json::json!({
            "version": 10, "metadata": {"name": "Advertised pay"},
            "sources": [{"id": "fixture", "label": "Fixture",
                "extractor": {"kind": "greenhouse", "url": "https://boards.greenhouse.io/fixture"}}],
            "filters": {"matching": {"compensation": {
                "min_minor": 6000001, "currency": "USD", "interval": "year",
                "match_mode": "advertised_amount"
            }}}
        }), "UTC")
        .unwrap();
        let reopened = validate(serde_json::to_value(&spec).unwrap(), "UTC").unwrap();
        let pay = reopened
            .filters
            .unwrap()
            .matching
            .unwrap()
            .compensation
            .unwrap();
        assert_eq!(pay.match_mode, CompensationMatchMode::AdvertisedAmount);
        assert!(!pay.matches(&lower.ranges));
    }

    #[test]
    fn role_first_search_validates_relative_dates_provider_identity_and_old_filter_preservation() {
        let value = serde_json::json!({
            "version":8,"metadata":{"name":"Software Engineer at Google & Amazon"},
            "sources":[{"id":"search","label":"Cross-Company Search","extractor":{"kind":"search","url":""}}],
            "filters":{"matching":{"target_roles":["Software Engineer"],"companies":["Google","Amazon"],
                "posting_date":{"preset":"today","timezone":"America/New_York"}}}
        });
        let spec = validate(value.clone(), "UTC").unwrap();
        let input = source_input(&spec, "UTC").unwrap();
        assert_eq!(input.identifier, "themuse");
        let reopened = validate(serde_json::to_value(&spec).unwrap(), "UTC").unwrap();
        assert_eq!(reopened.filters, spec.filters);

        let mut duplicate = value.clone();
        let mut second = duplicate["sources"][0].clone();
        second["id"] = Value::String("another".into());
        second["extractor"]["url"] = Value::String("themuse".into());
        duplicate["sources"].as_array_mut().unwrap().push(second);
        assert!(validate(duplicate, "UTC")
            .unwrap_err()
            .to_string()
            .contains("duplicate source endpoint"));
        for invalid in [
            serde_json::json!({"preset":"today","last_hours":12,"timezone":"UTC"}),
            serde_json::json!({"preset":"invalid","timezone":"UTC"}),
        ] {
            let mut invalid_spec = value.clone();
            invalid_spec["filters"]["matching"]["posting_date"] = invalid;
            assert!(validate(invalid_spec, "UTC").is_err());
        }
        let mut invalid_spec = value;
        invalid_spec["sources"][0]["extractor"]["url"] = Value::String("unknown-provider".into());
        assert!(validate(invalid_spec, "UTC")
            .unwrap_err()
            .to_string()
            .contains("search provider"));

        for version in 2..=7 {
            let legacy = serde_json::json!({
                "version":version,"metadata":{"name":"Saved board"},
                "sources":[{"id":"saved","label":"Saved","extractor":{"kind":"greenhouse","url":"fixture"}}],
                "filters":{"include_keywords":["Engineer"],"matching":{"keywords":{"all":["Rust"]},
                    "posting_date":{"last_hours":36,"timezone":"UTC","include_unknown":true}}}
            });
            let old = validate(legacy, "UTC").unwrap();
            let resaved = validate(serde_json::to_value(&old).unwrap(), "UTC").unwrap();
            assert_eq!(old.filters, resaved.filters);
            let matching = resaved.filters.unwrap().matching.unwrap();
            assert!(matching.target_roles.is_empty());
            assert!(matching.companies.is_empty());
            assert_eq!(matching.posting_date.unwrap().last_hours, Some(36));
        }
    }
    #[test]
    fn experience_modes_preserve_legacy_rules_and_explicit_v7_edits() {
        for version in 2..=WORKFLOW_SPEC_VERSION {
            let value = serde_json::json!({"version":version,"metadata":{"name":"Experience compatibility"},"sources":[{"id":"fixture","label":"Fixture","enabled":true,"extractor":{"kind":"greenhouse","url":"https://boards.greenhouse.io/fixture"}}],"filters":{"matching":{"experience_min":3,"experience_max":5}}});
            let mut spec = validate(value, "UTC").unwrap();
            let expected = if version <= 6 {
                scrape_core::matching::ExperienceMatchMode::AdvertisedRangeOverlap
            } else {
                scrape_core::matching::ExperienceMatchMode::RequiredMinimum
            };
            assert_eq!(
                spec.filters
                    .as_ref()
                    .unwrap()
                    .matching
                    .as_ref()
                    .unwrap()
                    .experience_match_mode,
                expected
            );
            spec.filters
                .as_mut()
                .unwrap()
                .matching
                .as_mut()
                .unwrap()
                .experience_max = Some(6);
            let reopened = validate(serde_json::to_value(spec).unwrap(), "UTC").unwrap();
            assert_eq!(
                reopened
                    .filters
                    .unwrap()
                    .matching
                    .unwrap()
                    .experience_match_mode,
                expected
            );
        }
        let legacy = serde_json::json!({"version":6,"metadata":{"name":"Legacy bounds"},"sources":[{"id":"fixture","label":"Fixture","enabled":true,"extractor":{"kind":"greenhouse","url":"https://boards.greenhouse.io/fixture"}}],"filters":{"experience_min":3}});
        assert_eq!(
            validate(legacy, "UTC")
                .unwrap()
                .filters
                .unwrap()
                .matching
                .unwrap()
                .experience_match_mode,
            scrape_core::matching::ExperienceMatchMode::AdvertisedRangeOverlap
        );
    }
    #[test]
    fn extraction_modes_are_versioned_lossless_and_secrets_are_rejected() {
        let value = serde_json::json!({"version":6,"metadata":{"name":"Browser fixture"},"sources":[{"id":"fixture","label":"Fixture","enabled":true,"extractor":{"kind":"browser","url":"https://fixture.test/jobs","job_container":"//article","title_selector":".title","url_selector":"a","url_attribute":"href","id_selector":".","id_attribute":"data-id","date_regex":"(?P<value>.+)","date_selector":"time","next_page_selector":"a.next@href","load_more_selector":"button.more","wait_for_selector":"main","navigation_timeout_s":30,"max_pages":2,"max_source_duration_s":60,"max_workflow_duration_s":120,"session_id":"fixture-login"}}]});
        let spec = validate(value.clone(), "UTC").unwrap();
        assert_eq!(spec.version, WORKFLOW_SPEC_VERSION);
        assert_eq!(spec.sources[0].extractor.kind, SourceKind::Selector);
        let config = source_input(&spec, "UTC").unwrap().config.unwrap();
        assert_eq!(config["mode"], "browser");
        assert_eq!(config["link_attribute"], "href");
        assert_eq!(config["session_id"], "fixture-login");
        assert_eq!(
            serde_json::to_value(validate(serde_json::to_value(&spec).unwrap(), "UTC").unwrap())
                .unwrap(),
            serde_json::to_value(spec).unwrap()
        );
        for field in [
            "cookies",
            "headers",
            "authorization",
            "storage_state",
            "token",
        ] {
            for nested in [false, true] {
                let mut invalid = value.clone();
                if nested {
                    invalid["sources"][0]["extractor"]["source_config"] =
                        serde_json::json!({field:"secret"});
                } else {
                    invalid["sources"][0]["extractor"][field] = Value::String("secret".into());
                }
                assert!(validate(invalid, "UTC")
                    .unwrap_err()
                    .to_string()
                    .contains(field));
            }
        }
        for (field, invalid_value) in [
            ("max_pages", 101),
            ("navigation_timeout_s", 4),
            ("max_source_duration_s", 601),
            ("max_workflow_duration_s", 1801),
        ] {
            let mut invalid = value.clone();
            invalid["sources"][0]["extractor"][field] = invalid_value.into();
            assert!(validate(invalid, "UTC")
                .unwrap_err()
                .to_string()
                .contains(field));
        }
    }

    #[test]
    fn script_mode_accepts_no_dom_fields_and_rejects_invalid_execution_config() {
        let base = serde_json::json!({"version":6,"metadata":{"name":"Script fixture"},"sources":[{"id":"script","label":"Script","enabled":true,"extractor":{"kind":"script","url":"https://fixture.test/jobs","script":{"runtime":"python","interpreter":"/usr/bin/python3","code":"print('fixture')","args":["a b","$(literal)"]}}}]});
        let spec = validate(base.clone(), "UTC").unwrap();
        let config = source_input(&spec, "UTC").unwrap().config.unwrap();
        assert_eq!(config["mode"], "script");
        for patch in [
            serde_json::json!({"runtime":"python","code":"x"}),
            serde_json::json!({"runtime":"js","code":"x","path":"x.js"}),
            serde_json::json!({"runtime":"js","code":"x","timeout_s":601}),
            serde_json::json!({"runtime":"bash","code":"x"}),
        ] {
            let mut invalid = base.clone();
            invalid["sources"][0]["extractor"]["script"] = patch;
            assert!(validate(invalid, "UTC")
                .unwrap_err()
                .to_string()
                .contains("script"));
        }
        let mut invalid = base;
        invalid["sources"][0]["extractor"]["mode"] = "static".into();
        assert!(validate(invalid, "UTC").is_err());
    }
    #[test]
    fn job_limit_defaults_validates_and_survives_stored_configuration() {
        let base = serde_json::json!({"version":3,"metadata":{"name":"Careers"},"extractor":{"kind":"lever","url":"acme"}});
        assert_eq!(
            validate(base.clone(), "UTC").unwrap().max_jobs_per_run,
            Some(20)
        );
        for limit in [1, 7, 20, 10000] {
            let mut value = base.clone();
            value["max_jobs_per_run"] = Value::from(limit);
            let spec = validate(value, "UTC").unwrap();
            let input = source_input(&spec, "UTC").unwrap();
            let worker_source = scrape_core::WorkerSource {
                id: "test".into(),
                kind: input.kind,
                name: input.name,
                identifier: input.identifier,
                config: input.config.unwrap(),
            };
            assert_eq!(worker_source.max_jobs_per_run().unwrap(), limit);
            let reopened =
                validate(worker_source.config["__workflow_spec"].clone(), "UTC").unwrap();
            assert_eq!(reopened.max_jobs_per_run, Some(limit));
        }
        for invalid in [
            serde_json::json!(0),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!(10001),
            serde_json::json!("20"),
            serde_json::json!(true),
        ] {
            let mut value = base.clone();
            value["max_jobs_per_run"] = invalid;
            assert!(validate(value, "UTC")
                .unwrap_err()
                .to_string()
                .contains("max_jobs_per_run"));
        }
    }
    #[test]
    fn malformed_root_and_version_values_return_errors_without_mutation_or_panics() {
        for value in [
            Value::Null,
            serde_json::json!([]),
            Value::String("workflow".into()),
            Value::Bool(true),
        ] {
            assert!(validate(value, "UTC").is_err());
        }
        for version in [
            Value::String("3".into()),
            Value::from(3.5),
            Value::Null,
            Value::Bool(true),
        ] {
            assert!(validate(serde_json::json!({"version":version,"metadata":{"name":"X"},"extractor":{"kind":"greenhouse","url":"acme"}}), "UTC").unwrap_err().to_string().contains("version"));
        }
    }

    #[test]
    fn advanced_specs_are_lossless_and_irrelevant_or_malformed_selectors_fail() {
        let value = serde_json::json!({"version":3,"metadata":{"name":"Careers","company":"Publisher"},"extractor":{"kind":"selector","url":"https://example.com/careers","job_container":".job","title_selector":"h2","url_selector":"a","date_selector":"time","employer_logo_selector":".employer-logo","salary_regex":"[0-9]+","url_patterns":["*/jobs/*"]},"filters":{"include_keywords":[],"matching":{"keywords":{"any":["Engineer"],"all":["Rust"],"exclude":["Senior"],"fields":["title","description"]},"compensation":{"min_minor":5025,"max_minor":6075,"currency":"USD","interval":"hour"}}},"schedule":{"cadence":"weekly","time":"02:30","weekdays":[1,5],"timezone":"America/New_York"},"retention":{"days":90}});
        let spec = validate(value.clone(), "UTC").unwrap();
        let encoded = serde_json::to_value(&spec).unwrap();
        assert_eq!(
            serde_json::to_value(validate(encoded.clone(), "UTC").unwrap()).unwrap(),
            encoded
        );
        let input = source_input(&spec, "UTC").unwrap();
        assert_eq!(input.config.as_ref().unwrap()["date"], "time");
        assert_eq!(
            input.config.as_ref().unwrap()["employer_logo"],
            ".employer-logo"
        );
        assert_eq!(input.config.as_ref().unwrap()["__workflow_spec"], encoded);
        for (field, invalid) in [
            ("salary_regex", "["),
            ("date_selector", "["),
            ("employer_logo_selector", "["),
            ("kind", "greenhouse"),
        ] {
            let mut bad = value.clone();
            bad["extractor"][field] = Value::String(invalid.into());
            assert!(validate(bad, "UTC").is_err());
        }
        let mut bad = value;
        bad["extractor"]["source_config"] = serde_json::json!({"ignored_option":true});
        assert!(validate(bad, "UTC")
            .unwrap_err()
            .to_string()
            .contains("ignored_option"));
    }

    #[test]
    fn custom_locators_validate_and_survive_workflow_round_trips() {
        let config = serde_json::json!({"item":"//article[@data-id]", "title":"h2@data-title", "link":"xpath:.//a[1]/@href", "id":".", "id_attribute":"data-id", "date":"time", "date_attribute":"datetime", "date_regex":"(?P<value>[0-9-]+)", "description":"p", "description_attribute":"text", "description_regex":"(?P<value>.+)"});
        let spec = validate(serde_json::json!({"version":5,"metadata":{"name":"Locator fixture"},"sources":[{"id":"fixture","label":"Fixture","extractor":{"kind":"selector","url":"https://fixture.test/careers","source_config":config}}],"schedule":{"cadence":"manual"}}), "UTC").unwrap();
        let encoded = serde_json::to_string(&spec).unwrap();
        let decoded: Value = serde_json::from_str(&encoded).unwrap();
        let reopened = validate(decoded, "UTC").unwrap();
        assert_eq!(spec, reopened);
        let input = source_input(&reopened, "UTC").unwrap();
        for (key, value) in config.as_object().unwrap() {
            assert_eq!(&input.config.as_ref().unwrap()[key], value);
        }
        for (key, bad_value) in [
            ("item", "//article/text()"),
            ("item", "//article/@id"),
            ("title", "xpath:.//h2[unsupported()]"),
            ("title", "a@"),
            ("description_regex", "["),
            ("date_attribute", "bad attribute"),
        ] {
            let mut invalid = config.clone();
            invalid[key] = bad_value.into();
            assert!(
                validate_config(SourceKind::Selector, &invalid).is_err(),
                "accepted {key}: {bad_value}"
            );
        }
        assert!(validate_config(SourceKind::Greenhouse, &config).is_err());
    }

    #[test]
    fn hydration_mapping_validation_is_shared_and_does_not_require_dom_fields() {
        let config = serde_json::json!({"hydration_pointer":"/props/jobs","hydration_fields":{"title":"/name","link":"/url","id":"/id"},"title_regex":"(?P<value>.+)"});
        validate_config(SourceKind::Selector, &config).unwrap();
        for (key, value) in [
            ("hydration_pointer", serde_json::json!("missing/slash")),
            ("hydration_fields", serde_json::json!({"title":"/name"})),
            (
                "hydration_fields",
                serde_json::json!({"title":"/name","link":"/url","made_up":"/value"}),
            ),
            ("title_regex", serde_json::json!("[")),
            ("title", serde_json::json!("h2")),
        ] {
            let mut invalid = config.clone();
            invalid[key] = value;
            assert!(
                validate_config(SourceKind::Selector, &invalid).is_err(),
                "accepted {key}"
            );
        }
        assert!(validate_config(SourceKind::Greenhouse, &config).is_err());
    }

    #[test]
    fn static_navigation_and_detail_settings_validate_without_weakening_identity() {
        let config = serde_json::json!({"item":".job","title":"h2","link":"a","next_page":"xpath://a[@rel='next']/@href","max_pages":3,"max_source_duration_s":10,"detail":"a.detail@href","detail_container":"main","detail_fields":{"date":"time","salary":".pay","salary_regex":"(?P<value>.+)"},"wait_for":"body"});
        validate_config(SourceKind::Selector, &config).unwrap();
        for (key, value) in [
            ("max_pages", serde_json::json!(0)),
            ("max_source_duration_s", serde_json::json!(601)),
            ("detail_fields", serde_json::json!({"title":"h1"})),
            ("detail_fields", serde_json::json!({"salary":"["})),
            ("wait_for", serde_json::json!("xpath://a/text()")),
        ] {
            let mut invalid = config.clone();
            invalid[key] = value;
            assert!(
                validate_config(SourceKind::Selector, &invalid).is_err(),
                "accepted {key}"
            );
        }
    }

    #[test]
    fn versions_unknown_paths_queries_and_empty_overrides_are_validated() {
        let base = serde_json::json!({"version":2,"metadata":{"name":"Acme"},"extractor":{"kind":"greenhouse","url":"https://boards.greenhouse.io/acme"},"filters":{"include_keywords":[]},"schedule":{"cadence":"daily","time":"09:00","timezone":"America/New_York","weekdays":[1,3,5]}});
        let spec = validate(base.clone(), "UTC").unwrap();
        assert_eq!(spec.version, WORKFLOW_SPEC_VERSION);
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

    #[test]
    fn legacy_inheritance_is_resolved_once_and_v4_rejects_global_cadence() {
        let legacy = serde_json::json!({
            "version": 3,
            "metadata": {"name":"Legacy"},
            "extractor": {"kind":"greenhouse", "url":"acme"},
            "filters": {"include_keywords": []}
        });
        let globals = WorkerFilters {
            include_keywords: vec!["ignored by explicit empty".into()],
            exclude_keywords: vec!["sales".into()],
            ..WorkerFilters::default()
        };
        let resolved =
            validate_legacy_import(legacy, "America/New_York", "0 0 6 * * *", &globals, true)
                .unwrap();
        assert_eq!(resolved.version, WORKFLOW_SPEC_VERSION);
        let schedule = resolved.schedule.unwrap();
        assert_eq!(schedule.cadence, Some(SourceCadence::Cron));
        assert_eq!(schedule.cron.as_deref(), Some("0 0 6 * * *"));
        assert_eq!(schedule.timezone.as_deref(), Some("America/New_York"));
        assert_eq!(schedule.run_on_startup, Some(true));
        let filters = resolved.filters.unwrap();
        assert_eq!(filters.include_keywords, Some(vec![]));
        assert_eq!(filters.exclude_keywords, Some(vec!["sales".into()]));

        let invalid_v4 = serde_json::json!({
            "version":4,
            "metadata":{"name":"No inheritance"},
            "extractor":{"kind":"greenhouse", "url":"acme"},
            "schedule":{"cadence":"global"}
        });
        assert!(validate(invalid_v4, "UTC")
            .unwrap_err()
            .to_string()
            .contains("explicit workflow schedule"));
    }

    #[test]
    fn version_one_cadence_and_global_matching_are_converted_without_overriding_explicit_schedule()
    {
        let legacy_v1 = serde_json::json!({
            "version":1,
            "company":"Legacy ATS",
            "url":"acme",
            "cadence":{"type":"every_1h"},
            "engine":{"kind":"greenhouse"}
        });
        let globals = WorkerFilters {
            include_keywords: vec!["engineer".into()],
            ..WorkerFilters::default()
        };
        let converted =
            validate_legacy_import(legacy_v1, "UTC", "0 0 9 * * *", &globals, true).unwrap();
        assert_eq!(converted.version, WORKFLOW_SPEC_VERSION);
        assert_eq!(
            converted.schedule.as_ref().unwrap().cadence,
            Some(SourceCadence::Hourly)
        );
        assert_eq!(
            converted.filters.as_ref().unwrap().include_keywords,
            Some(vec!["engineer".into()])
        );
        assert_eq!(
            converted.schedule.as_ref().unwrap().run_on_startup,
            Some(false)
        );
    }

    #[test]
    fn version_four_uses_explicit_independent_defaults() {
        let value = serde_json::json!({
            "version":4,
            "metadata":{"name":"New workflow"},
            "extractor":{"kind":"greenhouse", "url":"acme"}
        });
        let spec = validate(value, "America/New_York").unwrap();
        assert_eq!(spec.version, WORKFLOW_SPEC_VERSION);
        assert_eq!(spec.max_jobs_per_run, Some(20));
        assert_eq!(spec.metadata.enabled, Some(true));
        assert_eq!(spec.sources.len(), 1);
        let schedule = spec.schedule.unwrap();
        assert_eq!(schedule.cadence, Some(SourceCadence::Daily));
        assert_eq!(schedule.time.as_deref(), Some("09:00"));
        assert_eq!(schedule.timezone.as_deref(), Some("America/New_York"));
        assert_eq!(schedule.run_on_startup, Some(false));
        let filters = spec.filters.unwrap();
        assert_eq!(filters.include_keywords, Some(vec![]));
        assert_eq!(filters.exclude_keywords, Some(vec![]));
        assert_eq!(filters.locations, Some(vec![]));
        assert_eq!(filters.role_types, Some(vec![]));
        let retention = spec.retention.unwrap();
        assert_eq!(retention.days, Some(30));
        assert_eq!(retention.policy.as_deref(), Some("untracked_only"));
    }

    #[test]
    fn multi_source_workflow_validates_and_rejects_duplicates() {
        let valid_v5 = serde_json::json!({
            "version": 5,
            "metadata": {"name": "Multi-Source Tech"},
            "sources": [
                {
                    "id": "src-1",
                    "label": "Cloudflare",
                    "enabled": true,
                    "employer_hint": "Cloudflare",
                    "extractor": {"kind": "greenhouse", "url": "cloudflare"}
                },
                {
                    "id": "src-2",
                    "label": "Linear",
                    "enabled": true,
                    "employer_hint": "Linear",
                    "extractor": {"kind": "ashby", "url": "linear"}
                }
            ]
        });
        let spec = validate(valid_v5, "UTC").unwrap();
        assert_eq!(spec.version, WORKFLOW_SPEC_VERSION);
        assert_eq!(spec.sources.len(), 2);
        assert_eq!(spec.sources[0].label, "Cloudflare");
        assert_eq!(spec.sources[1].label, "Linear");

        // Duplicate normalized endpoint (slug vs URL)
        let dup_endpoint = serde_json::json!({
            "version": 5,
            "metadata": {"name": "Dup"},
            "sources": [
                {
                    "id": "src-1",
                    "label": "Cloudflare Board",
                    "enabled": true,
                    "extractor": {"kind": "greenhouse", "url": "https://boards.greenhouse.io/cloudflare"}
                },
                {
                    "id": "src-2",
                    "label": "Cloudflare Slug",
                    "enabled": true,
                    "extractor": {"kind": "greenhouse", "url": "cloudflare"}
                }
            ]
        });
        let err = validate(dup_endpoint, "UTC").unwrap_err().to_string();
        assert!(err.contains("duplicate source endpoint"), "{err}");

        // Duplicate source ID
        let dup_id = serde_json::json!({
            "version": 5,
            "metadata": {"name": "Dup"},
            "sources": [
                {
                    "id": "same-id",
                    "label": "Cloudflare",
                    "enabled": true,
                    "extractor": {"kind": "greenhouse", "url": "cloudflare"}
                },
                {
                    "id": "same-id",
                    "label": "Linear",
                    "enabled": true,
                    "extractor": {"kind": "ashby", "url": "linear"}
                }
            ]
        });
        let err = validate(dup_id, "UTC").unwrap_err().to_string();
        assert!(err.contains("duplicate source ID"), "{err}");

        // Empty sources
        let empty_sources = serde_json::json!({
            "version": 5,
            "metadata": {"name": "Empty"},
            "sources": []
        });
        let err = validate(empty_sources, "UTC").unwrap_err().to_string();
        assert!(err.contains("must contain at least one source"), "{err}");

        // All sources disabled
        let all_disabled = serde_json::json!({
            "version": 5,
            "metadata": {"name": "Disabled"},
            "sources": [
                {
                    "id": "src-1",
                    "label": "Cloudflare",
                    "enabled": false,
                    "extractor": {"kind": "greenhouse", "url": "cloudflare"}
                }
            ]
        });
        let err = validate(all_disabled, "UTC").unwrap_err().to_string();
        assert!(err.contains("at least one source must be enabled"), "{err}");
    }
}
