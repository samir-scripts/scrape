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
}
optional_fields!(WorkflowSchedule { cadence: SourceCadence, time: String, cron: String, timezone: String, weekdays: Vec<u8>, run_on_startup: bool });
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub max_jobs_per_run: Option<u32>,
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
    } else if matches!(version, 2..=4) {
        value["version"] = Value::from(4);
    } else {
        return Err(AppError::Validation(format!(
            "version: unsupported workflow version {version}; supported versions are 1, 2, 3, 4"
        )));
    }
    let text = serde_json::to_string(&value)?;
    let mut deserializer = serde_json::Deserializer::from_str(&text);
    let mut spec: WorkflowSpec = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|e| AppError::Validation(format!("{}: {}", e.path(), e.inner())))?;
    if submitted_version == 4
        && spec.schedule.as_ref().and_then(|schedule| schedule.cadence)
            == Some(SourceCadence::Global)
    {
        return Err(AppError::Validation(
            "schedule.cadence: version 4 must use an explicit workflow schedule".into(),
        ));
    }
    spec.max_jobs_per_run = Some(
        scrape_core::protocol::validate_job_limit(
            spec.max_jobs_per_run
                .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN),
        )
        .map_err(AppError::Validation)?,
    );
    // Version 4 is fully explicit. These defaults apply only to fields omitted
    // by the submitted document; stored/imported v1-v3 values are resolved by
    // the compatibility entry point before they reach this normalizer.
    if spec.version == 4 {
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
    spec.version = 4;
    spec.metadata.enabled.get_or_insert(true);
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

/// Resolve legacy inheritance once at the import boundary. Version 4 never
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
        let defaults = serde_json::to_value(global_filters)?;
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
                if let Some(value) = defaults.get(key) {
                    merged.insert(key.into(), value.clone());
                }
            }
        }
        *filters = serde_json::from_value(Value::Object(merged))
            .map_err(|e| AppError::Validation(format!("workflow.filters: {e}")))?;
    }
    spec.version = 4;
    let spec = validate(serde_json::to_value(spec)?, default_timezone)?;
    let input = source_input(&spec, default_timezone)?;
    crate::services::runner::resolve_worker_filters(
        input.filters.as_ref().unwrap_or(&Value::Null),
        WorkerFilters::default(),
    )?;
    Ok(spec)
}

pub fn source_input(spec: &WorkflowSpec, timezone: &str) -> Result<CreateSourceInput> {
    scrape_core::protocol::validate_job_limit(
        spec.max_jobs_per_run
            .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN),
    )
    .map_err(AppError::Validation)?;
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
    config["__run_on_startup"] = Value::Bool(schedule.run_on_startup.unwrap_or(false));
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
        "__invalid_original_config",
        "__invalid_original_filters",
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
    let allowed = [
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
        let implemented = match kind {
            SourceKind::Jsonld => {
                ["url", "follow_links", "max_pages", "employer_logo"].contains(&key.as_str())
            }
            SourceKind::Selector => {
                allowed.contains(&key.as_str())
                    && !["follow_links", "max_pages"].contains(&key.as_str())
            }
            _ => key == "url",
        };
        if !implemented {
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
        "employer_logo",
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
    fn versions_unknown_paths_queries_and_empty_overrides_are_validated() {
        let base = serde_json::json!({"version":2,"metadata":{"name":"Acme"},"extractor":{"kind":"greenhouse","url":"https://boards.greenhouse.io/acme"},"filters":{"include_keywords":[]},"schedule":{"cadence":"daily","time":"09:00","timezone":"America/New_York","weekdays":[1,3,5]}});
        let spec = validate(base.clone(), "UTC").unwrap();
        assert_eq!(spec.version, 4);
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
        assert_eq!(resolved.version, 4);
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
        assert_eq!(converted.version, 4);
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
        assert_eq!(spec.max_jobs_per_run, Some(20));
        assert_eq!(spec.metadata.enabled, Some(true));
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
}
