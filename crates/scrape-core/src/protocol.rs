use crate::enrich::{
    ClassifiedClearance, ClassifiedSponsorship, ClassifiedWorkAuth, ExperienceClassification,
    ParsedLocation, Salary,
};
use crate::{RoleType, SourceKind, TriState, WorkplaceType};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROTOCOL_VERSION: u32 = 7;
pub const DEFAULT_NAVIGATION_TIMEOUT_S: u64 = 30;
pub const MAX_PAGES_PER_SOURCE: u32 = 100;
pub const DEFAULT_SOURCE_DURATION_S: u64 = 180;
pub const DEFAULT_WORKFLOW_DURATION_S: u64 = 600;
pub const DEFAULT_MAX_JOBS_PER_RUN: u32 = 20;
pub const PREVIEW_MAX_JOBS: u32 = 10;
pub const MAX_JOB_LIMIT: u32 = 10_000;

pub fn validate_job_limit(value: u32) -> Result<u32, String> {
    if (1..=MAX_JOB_LIMIT).contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "max_jobs_per_run: expected a whole number from 1 to {MAX_JOB_LIMIT}"
        ))
    }
}

fn default_protocol_version() -> u32 {
    PROTOCOL_VERSION
}

fn default_concurrency() -> usize {
    1
}

fn default_per_host_delay_ms() -> u64 {
    750
}

fn default_request_timeout_s() -> u64 {
    20
}

/// Configuration payload passed from Tauri host to scrape-worker via stdin JSON.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    #[serde(default = "default_protocol_version")]
    pub version: u32,
    pub sources: Vec<WorkerSource>,
    #[serde(default)]
    pub filters: WorkerFilters,
    #[serde(default)]
    pub filters_by_source: std::collections::HashMap<String, WorkerFilters>,
    #[serde(default)]
    pub run_started_at: Option<String>,
    #[serde(default)]
    pub limits: WorkerLimits,
    // Canonical identities and unresolved legacy suppression keys at run start.
    #[serde(default)]
    pub blocked_postings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub max_jobs_per_run: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub starting_source_index: Option<usize>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            sources: vec![],
            filters: WorkerFilters::default(),
            filters_by_source: std::collections::HashMap::default(),
            run_started_at: None,
            limits: WorkerLimits::default(),
            blocked_postings: vec![],
            max_jobs_per_run: None,
            starting_source_index: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct WorkerSource {
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

impl WorkerSource {
    pub fn max_jobs_per_run(&self) -> Result<u32, String> {
        let value = match self.config.pointer("/__workflow_spec/max_jobs_per_run") {
            None | Some(serde_json::Value::Null) => DEFAULT_MAX_JOBS_PER_RUN,
            Some(value) => value
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .ok_or("max_jobs_per_run: expected a positive whole number")?,
        };
        validate_job_limit(value)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct WorkerFilters {
    #[serde(default)]
    pub include_keywords: Vec<String>,
    #[serde(default)]
    pub exclude_keywords: Vec<String>,
    #[serde(default)]
    pub locations: Vec<String>,
    #[serde(default)]
    pub role_types: Vec<RoleType>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub policy: Option<serde_json::Value>,
    #[serde(default)]
    pub compensation: Option<crate::enrich::CompensationFilter>,
    #[serde(default)]
    pub matching: Option<crate::matching::PostingRules>,
}

impl WorkerFilters {
    pub fn to_expr(&self) -> Result<crate::filter::FilterExpr, String> {
        Ok(crate::FilterExpr::And(
            self.categorized_predicates()?
                .into_iter()
                .map(|(_, expr)| expr)
                .collect(),
        ))
    }
    pub fn categorized_predicates(
        &self,
    ) -> Result<Vec<(crate::matching::RejectionReason, crate::FilterExpr)>, String> {
        use crate::filter::{FilterExpr, FilterField, FilterOp};
        use crate::matching::RejectionReason as Reason;
        if let Some(matching) = &self.matching {
            matching.validate()?;
        }
        if let Some(pay) = &self.compensation {
            pay.validate()?;
        }
        let mut and_clauses = Vec::new();

        // 1. If explicit SQL-like query string is present, parse it
        if let Some(ref q) = self.query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                and_clauses.push((Reason::Query, FilterExpr::parse_query(trimmed)?));
            }
        }

        // 2. If explicit structured policy object is present, parse it
        if let Some(ref pol) = self.policy {
            if !pol.is_null() {
                and_clauses.push((Reason::Policy, FilterExpr::parse_structured_policy(pol)?));
            }
        }

        // 3. Legacy keyword / location / role_type compilation
        if !self.include_keywords.is_empty() {
            let includes: Vec<FilterExpr> = self
                .include_keywords
                .iter()
                .map(|k| {
                    FilterExpr::Or(vec![
                        FilterExpr::FieldMatch {
                            field: FilterField::Title,
                            op: FilterOp::Like,
                            value: format!("%{}%", k),
                        },
                        FilterExpr::FieldMatch {
                            field: FilterField::Location,
                            op: FilterOp::Like,
                            value: format!("%{}%", k),
                        },
                    ])
                })
                .collect();
            and_clauses.push((Reason::LegacyKeywords, FilterExpr::Or(includes)));
        }

        if !self.exclude_keywords.is_empty() {
            let excludes: Vec<FilterExpr> = self
                .exclude_keywords
                .iter()
                .map(|k| {
                    FilterExpr::Or(vec![
                        FilterExpr::FieldMatch {
                            field: FilterField::Title,
                            op: FilterOp::Like,
                            value: format!("%{}%", k),
                        },
                        FilterExpr::FieldMatch {
                            field: FilterField::Location,
                            op: FilterOp::Like,
                            value: format!("%{}%", k),
                        },
                    ])
                })
                .collect();
            and_clauses.push((
                Reason::LegacyExclusion,
                FilterExpr::Not(Box::new(FilterExpr::Or(excludes))),
            ));
        }

        if !self.locations.is_empty() {
            let locs: Vec<FilterExpr> = self
                .locations
                .iter()
                .map(|loc| FilterExpr::FieldMatch {
                    field: FilterField::Location,
                    op: FilterOp::Like,
                    value: format!("%{}%", loc),
                })
                .collect();
            and_clauses.push((Reason::LegacyLocation, FilterExpr::Or(locs)));
        }

        if !self.role_types.is_empty() {
            let mut roles = Vec::new();
            for rt in &self.role_types {
                match rt {
                    RoleType::FullTime => {
                        roles.push("full_time".to_string());
                        roles.push("fulltime".to_string());
                    }
                    RoleType::PartTime => {
                        roles.push("part_time".to_string());
                        roles.push("parttime".to_string());
                    }
                    RoleType::Internship => {
                        roles.push("internship".to_string());
                        roles.push("intern".to_string());
                    }
                    RoleType::Contract => roles.push("contract".to_string()),
                    RoleType::Seasonal => roles.push("seasonal".to_string()),
                    RoleType::Unknown => roles.push("unknown".to_string()),
                }
            }
            and_clauses.push((
                Reason::LegacyRoleType,
                FilterExpr::FieldIn {
                    field: FilterField::RoleType,
                    values: roles,
                    negated: false,
                },
            ));
        }

        Ok(and_clauses)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(default, deny_unknown_fields)]
pub struct WorkerLimits {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub enrichment: Option<EnrichmentLimits>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_per_host_delay_ms")]
    pub per_host_delay_ms: u64,
    #[serde(default = "default_request_timeout_s")]
    pub request_timeout_s: u64,
    pub max_browser_processes: u32,
    pub max_browser_contexts: u32,
    #[ts(type = "number")]
    pub navigation_timeout_s: u64,
    pub max_pages_per_source: u32,
    #[ts(type = "number")]
    pub max_source_duration_s: u64,
    #[ts(type = "number")]
    pub max_workflow_duration_s: u64,
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            max_browser_processes: 1,
            max_browser_contexts: 2,
            navigation_timeout_s: DEFAULT_NAVIGATION_TIMEOUT_S,
            max_pages_per_source: MAX_PAGES_PER_SOURCE,
            max_source_duration_s: DEFAULT_SOURCE_DURATION_S,
            max_workflow_duration_s: DEFAULT_WORKFLOW_DURATION_S,
            enrichment: None,
            concurrency: default_concurrency(),
            per_host_delay_ms: default_per_host_delay_ms(),
            request_timeout_s: default_request_timeout_s(),
        }
    }
}

impl WorkerLimits {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_browser_processes != 1 || !(1..=2).contains(&self.max_browser_contexts) {
            return Err(
                "limits: max_browser_processes must be 1 and max_browser_contexts must be 1–2"
                    .into(),
            );
        }
        for (name, valid) in [
            (
                "navigation_timeout_s",
                (5..=120).contains(&self.navigation_timeout_s),
            ),
            (
                "max_pages_per_source",
                (1..=MAX_PAGES_PER_SOURCE).contains(&self.max_pages_per_source),
            ),
            (
                "max_source_duration_s",
                (10..=600).contains(&self.max_source_duration_s),
            ),
            (
                "max_workflow_duration_s",
                (30..=1800).contains(&self.max_workflow_duration_s),
            ),
        ] {
            if !valid {
                return Err(format!(
                    "limits.{name}: outside the supported extraction bound"
                ));
            }
        }
        if let Some(enrichment) = &self.enrichment {
            enrichment.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(default, deny_unknown_fields)]
#[ts(export)]
pub struct EnrichmentLimits {
    pub max_requests: u32,
    #[ts(type = "number")]
    pub timeout_ms: u64,
}
impl Default for EnrichmentLimits {
    fn default() -> Self {
        Self {
            max_requests: 100,
            timeout_ms: 45_000,
        }
    }
}
impl EnrichmentLimits {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_requests > 100 || !(1..=45_000).contains(&self.timeout_ms) {
            return Err(
                "limits.enrichment: max_requests must be 0–100 and timeout_ms must be 1–45000"
                    .into(),
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ScanOutcome {
    Success,
    SuccessWithWarnings,
    Partial,
    Failed,
    Cancelled,
}

/// NDJSON events streamed by scrape-worker over stdout.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
#[allow(clippy::large_enum_variant)]
pub enum WorkerEvent {
    Branding {
        source_id: String,
        company: String,
        logo: crate::branding::CompanyLogo,
    },
    Warning {
        source_id: String,
        message: String,
        eligibility_incomplete: bool,
        termination: Option<CoverageTermination>,
    },
    SourceInfo {
        source_id: String,
        #[ts(type = "number | null")]
        provider_total: Option<u64>,
    },
    Job {
        source_id: String,
        external_id: String,
        title: String,
        company: String,
        role_type: RoleType,
        #[serde(default)]
        location: String,
        url: String,
        #[serde(default)]
        date_posted: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        provider_updated_at: Option<String>,
        #[serde(default)]
        salary: Option<Salary>,
        #[serde(default)]
        locations: Vec<ParsedLocation>,
        #[serde(default)]
        location_data_complete: bool,
        #[serde(default)]
        workplace_type: WorkplaceType,
        #[serde(default)]
        sponsorship: Option<ClassifiedSponsorship>,
        #[serde(default)]
        clearance: Option<ClassifiedClearance>,
        #[serde(default)]
        experience: Option<ExperienceClassification>,
        #[serde(default)]
        is_new_grad: TriState,
        #[serde(default)]
        description_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        description: Option<crate::enrich::JobDescription>,
        #[serde(default)]
        #[ts(optional = nullable)]
        description_source: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        description_format: Option<crate::enrich::DescriptionFormat>,
        #[serde(default)]
        company_icon_url: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        company_logo: Option<crate::branding::LogoEvidence>,
        #[serde(default)]
        work_auth: Option<ClassifiedWorkAuth>,
    },
    Progress {
        source_id: String,
        fetched: u64,
        matched: u64,
    },
    Error {
        #[serde(default)]
        #[ts(optional = nullable)]
        invalid_posting: Option<bool>,
        source_id: String,
        message: String,
        retryable: bool,
    },
    Done {
        stats: WorkerStats,
    },
}

impl WorkerEvent {
    pub fn source_id(&self) -> Option<&str> {
        match self {
            Self::Branding { source_id, .. }
            | Self::Job { source_id, .. }
            | Self::Warning { source_id, .. }
            | Self::SourceInfo { source_id, .. }
            | Self::Progress { source_id, .. }
            | Self::Error { source_id, .. } => Some(source_id),
            Self::Done { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkerStats {
    pub sources_ok: u64,
    pub sources_failed: u64,
    pub jobs_emitted: u64,
    pub duration_ms: u64,
    #[serde(default)]
    pub sources: Vec<SourceScanStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct SourceScanStats {
    pub source_id: String,
    pub limit: u32,
    #[ts(type = "number")]
    pub examined: u64,
    #[ts(type = "number")]
    pub matched: u64,
    #[ts(type = "number")]
    pub blocked: u64,
    #[ts(type = "number")]
    pub duplicates: u64,
    #[ts(type = "number")]
    pub invalid: u64,
    pub limit_reached: bool,
    #[serde(default)]
    #[ts(type = "number")]
    pub accepted: u64,
    #[serde(default)]
    #[ts(type = "number")]
    pub rejected: u64,
    #[serde(default)]
    pub rejections: Vec<RejectionCount>,
    #[serde(default)]
    pub rejection_examples: Vec<RejectionExample>,
    #[serde(default)]
    pub termination: CoverageTermination,
    // Unknown unless the provider explicitly supplies a total.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub provider_total: Option<u64>,
    #[serde(default)]
    #[ts(type = "number")]
    pub unresolved: u64,
    #[serde(default)]
    #[ts(type = "number")]
    pub enrichment_warnings: u64,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub outcome: Option<ScanOutcome>,
}

impl SourceScanStats {
    pub fn validate(&self) -> Result<(), String> {
        let examined = u128::from(self.accepted)
            + u128::from(self.blocked)
            + u128::from(self.rejected)
            + u128::from(self.duplicates)
            + u128::from(self.invalid)
            + u128::from(self.unresolved);
        let primary_count: u128 = self.rejections.iter().map(|r| u128::from(r.count)).sum();
        let categories: std::collections::BTreeSet<_> =
            self.rejections.iter().map(|r| r.reason).collect();
        let max_safe_integer = (1u64 << 53) - 1;
        if [
            self.examined,
            self.matched,
            self.accepted,
            self.blocked,
            self.rejected,
            self.duplicates,
            self.invalid,
            self.unresolved,
            self.enrichment_warnings,
        ]
        .iter()
        .any(|count| *count > max_safe_integer)
            || self
                .provider_total
                .is_some_and(|total| total > max_safe_integer)
            || examined != u128::from(self.examined)
            || u128::from(self.matched) != u128::from(self.accepted) + u128::from(self.blocked)
            || primary_count != u128::from(self.rejected)
            || categories.len() != self.rejections.len()
            || self.accepted > u64::from(self.limit)
            || self.limit_reached != (self.accepted == u64::from(self.limit))
            || self.rejection_examples.len() > 5
            || self.rejection_examples.iter().any(|e| {
                e.overlapping.first() != Some(&e.primary) || !categories.contains(&e.primary)
            })
        {
            return Err(format!("Worker scan summary for {} has inconsistent candidate counts or rejection diagnostics", self.source_id));
        }
        validate_job_limit(self.limit)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct RejectionCount {
    pub reason: crate::matching::RejectionReason,
    #[ts(type = "number")]
    pub count: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct RejectionExample {
    pub title: String,
    pub primary: crate::matching::RejectionReason,
    // Includes primary; these conditions overlap and must not be added together.
    pub overlapping: Vec<crate::matching::RejectionReason>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CoverageTermination {
    #[default]
    Exhausted,
    JobLimit,
    RequestLimit,
    PageLimit,
    TimeLimit,
    NeedsLogin,
    Cancelled,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_bounds_accept_valid_overrides_and_supply_authoritative_defaults() {
        let limits: WorkerLimits = serde_json::from_value(
            serde_json::json!({"max_pages_per_source":2,"navigation_timeout_s":5}),
        )
        .unwrap();
        let encoded = serde_json::to_value(limits).unwrap();
        assert_eq!(encoded["max_pages_per_source"], 2);
        assert_eq!(encoded["max_source_duration_s"], 180);
        assert_eq!(encoded["max_workflow_duration_s"], 600);
        assert_eq!(encoded["max_browser_processes"], 1);
        assert_eq!(encoded["max_browser_contexts"], 2);
    }

    #[test]
    fn export_bindings_ingestion_limits() {
        let values = serde_json::json!({"default": DEFAULT_MAX_JOBS_PER_RUN, "preview": PREVIEW_MAX_JOBS, "maximum": MAX_JOB_LIMIT});
        let output = format!("// Generated by scrape-core; do not edit.\nexport const INGESTION_LIMITS = {} as const;\n", serde_json::to_string_pretty(&values).unwrap());
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../src/bindings/ingestionLimits.ts");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output).unwrap();
    }

    #[test]
    fn scan_summaries_reject_inconsistent_or_overlapping_primary_counts() {
        let valid = SourceScanStats {
            source_id: "fixture".into(),
            limit: 20,
            examined: 4,
            matched: 2,
            accepted: 1,
            blocked: 1,
            invalid: 1,
            rejected: 1,
            rejections: vec![RejectionCount {
                reason: crate::matching::RejectionReason::Geography,
                count: 1,
            }],
            ..Default::default()
        };
        valid.validate().unwrap();
        let mut invalid = valid.clone();
        invalid.rejected = 2;
        assert!(invalid
            .validate()
            .unwrap_err()
            .contains("inconsistent candidate counts"));
        let mut invalid = valid.clone();
        invalid.rejections.push(valid.rejections[0].clone());
        assert!(invalid.validate().is_err());
        let mut invalid = valid.clone();
        invalid.accepted = 21;
        assert!(invalid.validate().is_err());
        let mut invalid = valid.clone();
        invalid.provider_total = Some(1u64 << 53);
        assert!(invalid.validate().is_err());
        let mut invalid = valid.clone();
        invalid.enrichment_warnings = u64::MAX;
        assert!(invalid.validate().is_err());
        let mut invalid = valid;
        invalid.limit_reached = true;
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn unknown_worker_configuration_fields_are_rejected_at_each_boundary() {
        for value in [
            serde_json::json!({"sources": [], "unexpected": true}),
            serde_json::json!({"sources": [], "filters": {"salary_guess": 50}}),
            serde_json::json!({"sources": [], "limits": {"unsupported": 2}}),
            serde_json::json!({"sources": [{"id":"one", "kind":"greenhouse", "name":"Acme", "identifier":"acme", "unexpected":true}]}),
        ] {
            assert!(serde_json::from_value::<WorkerConfig>(value).is_err());
        }
    }

    #[test]
    fn test_worker_config_deserialization() {
        let json = r#"{
            "version": 1,
            "sources": [
                {
                    "id": "src-1",
                    "kind": "greenhouse",
                    "name": "Stripe",
                    "identifier": "stripe",
                    "config": {}
                }
            ],
            "filters": {
                "include_keywords": ["engineer"],
                "exclude_keywords": ["manager"],
                "locations": ["remote"],
                "role_types": ["full_time"]
            },
            "limits": {
                "concurrency": 2,
                "per_host_delay_ms": 500,
                "request_timeout_s": 15
            }
        }"#;

        let config: WorkerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].name, "Stripe");
        assert_eq!(config.filters.include_keywords, vec!["engineer"]);
        assert_eq!(config.filters.role_types, vec![RoleType::FullTime]);
        assert_eq!(config.limits.concurrency, 2);
    }

    #[test]
    fn test_worker_events_ndjson_roundtrip() {
        let job = WorkerEvent::Job {
            source_id: "src-1".to_string(),
            external_id: "5012345".to_string(),
            title: "Software Engineer, Data".to_string(),
            company: "Stripe".to_string(),
            role_type: RoleType::FullTime,
            location: "Remote".to_string(),
            url: "https://stripe.com/jobs/5012345".to_string(),
            provider_updated_at: None,
            date_posted: Some("2026-10-01T00:00:00Z".to_string()),
            salary: None,
            locations: Vec::new(),
            location_data_complete: false,
            workplace_type: WorkplaceType::Remote,
            sponsorship: None,
            clearance: None,
            experience: None,
            is_new_grad: TriState::Unknown,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            work_auth: None,
        };

        let serialized = serde_json::to_string(&job).unwrap();
        assert!(serialized.contains(r#""type":"job""#));
        assert!(serialized.contains(r#""role_type":"full_time""#));

        let deserialized: WorkerEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, job);

        let done = WorkerEvent::Done {
            stats: WorkerStats {
                sources_ok: 5,
                sources_failed: 1,
                jobs_emitted: 63,
                duration_ms: 8123,
                sources: vec![],
            },
        };
        let done_str = serde_json::to_string(&done).unwrap();
        assert!(done_str.contains(r#""type":"done""#));
        let done_deserialized: WorkerEvent = serde_json::from_str(&done_str).unwrap();
        assert_eq!(done_deserialized, done);
    }
}
