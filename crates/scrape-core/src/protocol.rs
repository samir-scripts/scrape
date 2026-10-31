use crate::enrich::{
    ClassifiedClearance, ClassifiedSponsorship, ClassifiedWorkAuth, ExperienceClassification,
    ParsedLocation, Salary,
};
use crate::{RoleType, SourceKind, TriState, WorkplaceType};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROTOCOL_VERSION: u32 = 15;
pub const DEFAULT_NAVIGATION_TIMEOUT_S: u64 = 30;
pub const MAX_PAGES_PER_SOURCE: u32 = 100;
pub const DEFAULT_SOURCE_DURATION_S: u64 = 180;
pub const DEFAULT_WORKFLOW_DURATION_S: u64 = 600;
pub const DEFAULT_MAX_JOBS_PER_RUN: u32 = 20;
pub const PREVIEW_MAX_JOBS: u32 = 10;
pub const MAX_JOB_LIMIT: u32 = 10_000;

/// Runtime discovery only. No installation or executable launch is implied.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct BrowserRuntimeStatus {
    pub available: bool,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum BrowserSessionStatus {
    #[default]
    Ready,
    NeedsLogin,
    Expired,
}

impl BrowserSessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NeedsLogin => "needs_login",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct BrowserSessionSummary {
    pub id: String,
    pub label: String,
    pub domain: String,
    pub status: BrowserSessionStatus,
    pub created_at: String,
    pub updated_at: String,
    pub last_verified_at: Option<String>,
}

/// Raw extraction evidence; a sample is not an accepted or persisted posting.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct ExtractorSampleCandidate {
    pub evidence_url: String,
    pub external_id: Option<String>,
    pub title: String,
    pub url: String,
    pub company: Option<String>,
    pub location: Option<String>,
    pub salary: Option<String>,
    pub description: Option<String>,
    pub date_posted: Option<String>,
    pub employer_logo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct ExtractorTestResult {
    pub samples: Vec<ExtractorSampleCandidate>,
    pub field_match_counts: std::collections::BTreeMap<String, u32>,
    pub warnings: Vec<String>,
    pub termination: CoverageTermination,
}

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

pub fn default_per_host_delay_ms() -> u64 {
    750
}

fn default_request_timeout_s() -> u64 {
    20
}

/// Execution-only network configuration. Never store in workflow or run snapshots.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HttpProxyConfig {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl std::fmt::Debug for HttpProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpProxyConfig")
            .field("server", &"[redacted]")
            .field("authenticated", &self.username.is_some())
            .finish()
    }
}

impl HttpProxyConfig {
    pub fn validate(mut self) -> Result<Self, String> {
        let invalid = || {
            "Proxy server must be an HTTP(S) scheme, host and port only; enter credentials separately."
                .to_string()
        };
        let mut url = url::Url::parse(self.server.trim()).map_err(|_| invalid())?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.port_or_known_default().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err(invalid());
        }
        let port = url.port_or_known_default().ok_or_else(invalid)?;
        url.set_port(Some(port)).map_err(|_| invalid())?;
        self.server = url.as_str().trim_end_matches('/').to_owned();
        self.username = self.username.map(|value| value.trim().to_owned());
        match (&self.username, &self.password) {
            (None, None) => {}
            (Some(username), Some(password)) if !username.is_empty() && !password.is_empty() => {}
            _ => {
                return Err(
                    "Proxy username and password must both be provided or both omitted.".into(),
                )
            }
        }
        Ok(self)
    }

    pub fn validate_with_socks(mut self) -> Result<Self, String> {
        let invalid = || {
            "Proxy server must be an HTTP(S) or SOCKS5 scheme, host and port only; enter credentials separately."
                .to_string()
        };
        let mut url = url::Url::parse(self.server.trim()).map_err(|_| invalid())?;
        let port = url
            .port_or_known_default()
            .or_else(|| matches!(url.scheme(), "socks5" | "socks5h").then_some(1080));
        if !matches!(url.scheme(), "http" | "https" | "socks5" | "socks5h")
            || url.host_str().is_none()
            || port.is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !matches!(url.path(), "" | "/")
        {
            return Err(invalid());
        }
        url.set_port(port).map_err(|_| invalid())?;
        self.server = url.as_str().trim_end_matches('/').to_owned();
        self.username = self.username.map(|value| value.trim().to_owned());
        match (&self.username, &self.password) {
            (None, None) => {}
            (Some(username), Some(password)) if !username.is_empty() && !password.is_empty() => {}
            _ => {
                return Err(
                    "Proxy username and password must both be provided or both omitted.".into(),
                )
            }
        }
        Ok(self)
    }
}

/// Configuration payload passed from Tauri host to scrape-worker via stdin JSON.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    #[serde(default = "default_protocol_version")]
    pub version: u32,
    // Persisted runs wait for an ordered host commit acknowledgement for each job.
    #[serde(default)]
    pub commit_feedback: bool,
    #[serde(default)]
    pub candidate_diagnostics: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(skip)]
    pub http_proxy: Option<HttpProxyConfig>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            commit_feedback: false,
            candidate_diagnostics: false,
            sources: vec![],
            filters: WorkerFilters::default(),
            filters_by_source: std::collections::HashMap::default(),
            run_started_at: None,
            limits: WorkerLimits::default(),
            blocked_postings: vec![],
            max_jobs_per_run: None,
            starting_source_index: None,
            http_proxy: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerCommitAck {
    pub source_id: String,
    pub external_id: String,
    pub outcome: WorkerCommitOutcome,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerCommitOutcome {
    Inserted,
    Updated,
    Blocked,
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

/// Visible restrictions retained verbatim during compatibility resolution.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct FilterCompatibilityRestriction {
    pub field: String,
    pub explanation: String,
}

impl WorkerFilters {
    /// Resolve compatibility without rewriting independently ANDed predicates.
    /// Inheritance is explicit and reserved for historical imports/upgrades.
    pub fn resolve_compatibility(
        value: &serde_json::Value,
        inherited: Option<&Self>,
    ) -> Result<Self, String> {
        let mut effective = serde_json::to_value(inherited.cloned().unwrap_or_default())
            .map_err(|e| e.to_string())?;
        if !value.is_null() {
            let overrides = value
                .as_object()
                .ok_or("Workflow filters must be an object")?;
            for (key, value) in overrides {
                effective[key] = value.clone();
            }
        }
        let filters: Self =
            serde_json::from_value(effective).map_err(|e| format!("workflow.filters: {e}"))?;
        filters.to_expr()?;
        Ok(filters)
    }

    /// Descriptions come from the same domain owner as predicate compilation.
    /// No automatic structured replacement is offered: these restrictions have
    /// different fields, OR groups, LIKE semantics or independent intersections.
    pub fn compatibility_restrictions(&self) -> Vec<FilterCompatibilityRestriction> {
        let mut restrictions = Vec::new();
        for (active, field, explanation) in [
            (!self.include_keywords.is_empty(), "include_keywords", "Any legacy include term must match the title OR location, in addition to the structured keyword groups. SQL LIKE wildcards remain active."),
            (!self.exclude_keywords.is_empty(), "exclude_keywords", "No legacy exclude term may match the title OR location. SQL LIKE wildcards remain active."),
            (!self.locations.is_empty(), "locations", "Any legacy location term may match the original location text (OR). These terms are not inferred as workplace, country or city constraints."),
            (!self.role_types.is_empty(), "role_types", "A legacy role type AND a structured role type must both match when both groups are present. The groups are not combined."),
            (self.query.as_ref().is_some_and(|q| !q.trim().is_empty()), "query", "This expert query applies with AND, including queries from old examples. It is never removed by recognizing its text."),
            (self.policy.as_ref().is_some_and(|p| !p.is_null()), "policy", "This expert policy applies with AND alongside every other restriction."),
        ] {
            if active {
                restrictions.push(FilterCompatibilityRestriction { field: field.into(), explanation: explanation.into() });
            }
        }
        if let Some(pay) = &self.compensation {
            restrictions.push(FilterCompatibilityRestriction {
                field: "compensation".into(),
                explanation: format!(
                    "Legacy compensation and structured compensation must both match when both are present. {}",
                    pay.explanation()
                ),
            });
        }
        if let Some(pay) = self
            .matching
            .as_ref()
            .and_then(|rules| rules.compensation.as_ref())
        {
            restrictions.push(FilterCompatibilityRestriction {
                field: "matching.compensation".into(),
                explanation: pay.explanation().into(),
            });
        }
        restrictions
    }

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number")]
    pub jitter_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub ignore_robots: Option<bool>,
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
            jitter_ms: None,
            ignore_robots: None,
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
        if self.jitter_ms.is_some_and(|jitter| jitter > 10_000) {
            return Err("limits.jitter_ms: outside the supported extraction bound".into());
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CandidateDisposition {
    Eligible,
    Rejected,
    Unresolved,
    Duplicate,
    Blocked,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct CandidateFacts {
    pub external_id: String,
    pub title: String,
    pub company: String,
    pub url: String,
    pub location: String,
    pub date_posted: Option<String>,
    pub provider_updated_at: Option<String>,
    pub description_excerpt: Option<String>,
    pub description_truncated: bool,
    pub role_type: RoleType,
    pub locations: Vec<ParsedLocation>,
    pub location_data_complete: bool,
    pub workplace_type: WorkplaceType,
    pub salary: Option<Salary>,
    pub experience: Option<ExperienceClassification>,
    pub is_new_grad: TriState,
    pub sponsorship: Option<ClassifiedSponsorship>,
    pub work_auth: Option<ClassifiedWorkAuth>,
    pub clearance: Option<ClassifiedClearance>,
}

impl CandidateFacts {
    pub fn from_event(event: &WorkerEvent) -> Option<Self> {
        let WorkerEvent::Job {
            external_id,
            title,
            company,
            url,
            location,
            date_posted,
            provider_updated_at,
            description_text,
            role_type,
            locations,
            location_data_complete,
            workplace_type,
            salary,
            experience,
            is_new_grad,
            sponsorship,
            work_auth,
            clearance,
            ..
        } = event
        else {
            return None;
        };
        Some(Self {
            external_id: external_id.clone(),
            title: title.clone(),
            company: company.clone(),
            url: url.clone(),
            location: location.clone(),
            date_posted: date_posted.clone(),
            provider_updated_at: provider_updated_at.clone(),
            description_excerpt: description_text
                .as_ref()
                .map(|text| text.chars().take(2048).collect()),
            description_truncated: description_text
                .as_ref()
                .is_some_and(|text| text.chars().count() > 2048),
            role_type: *role_type,
            locations: locations.clone(),
            location_data_complete: *location_data_complete,
            workplace_type: *workplace_type,
            salary: salary.clone(),
            experience: experience.clone(),
            is_new_grad: *is_new_grad,
            sponsorship: sponsorship.clone(),
            work_auth: work_auth.clone(),
            clearance: clearance.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct CandidateEvaluation {
    #[ts(type = "number")]
    pub sequence: u64,
    pub disposition: CandidateDisposition,
    pub candidate: Option<CandidateFacts>,
    pub rejected: Vec<crate::matching::RejectionReason>,
    pub unresolved: Vec<crate::matching::RejectionReason>,
    pub diagnostic: Option<String>,
}

impl CandidateEvaluation {
    pub fn validate(&self) -> Result<(), String> {
        use CandidateDisposition::*;
        let unique = |reasons: &[crate::matching::RejectionReason]| {
            reasons
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == reasons.len()
        };
        let shape = match self.disposition {
            Rejected => !self.rejected.is_empty(),
            Unresolved => self.rejected.is_empty() && !self.unresolved.is_empty(),
            _ => self.rejected.is_empty() && self.unresolved.is_empty(),
        };
        if self.sequence == 0
            || self.sequence > (1u64 << 53) - 1
            || !shape
            || !unique(&self.rejected)
            || !unique(&self.unresolved)
            || (self.candidate.is_none() && self.disposition != Invalid)
            || self
                .diagnostic
                .as_ref()
                .is_some_and(|text| text.chars().count() > 512)
        {
            return Err("Invalid candidate evaluation sequence, reasons or shape".into());
        }
        if let Some(facts) = &self.candidate {
            if facts.locations.len() > 64
                || facts
                    .description_excerpt
                    .as_ref()
                    .is_some_and(|text| text.chars().count() > 2048)
            {
                return Err("Candidate evidence exceeds bounds".into());
            }
            if [
                &facts.external_id,
                &facts.title,
                &facts.company,
                &facts.url,
                &facts.location,
            ]
            .iter()
            .any(|text| text.len() > 65_536)
                || facts
                    .date_posted
                    .as_ref()
                    .is_some_and(|text| text.len() > 1024)
                || facts
                    .provider_updated_at
                    .as_ref()
                    .is_some_and(|text| text.len() > 1024)
                || facts
                    .salary
                    .as_ref()
                    .is_some_and(|salary| salary.ranges.len() > 64)
            {
                return Err(
                    "Candidate text or compensation evidence exceeds protocol bounds".into(),
                );
            }
            // The complete evidence payload is bounded even for nested provider strings.
            if serde_json::to_vec(facts)
                .map_err(|error| error.to_string())?
                .len()
                > 1_048_576
            {
                return Err("Candidate evidence exceeds protocol frame bounds".into());
            }
        }
        Ok(())
    }
}

/// Evidence retained for an independently discovered posting.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct DiscoveryProvenance {
    pub seed_url: String,
    pub referrer_url: Option<String>,
    pub final_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub seed_manifest: Option<crate::web_discovery::SeedManifest>,
    pub date_evidence: Option<DiscoveryDateEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct DiscoveryDateEvidence {
    pub kind: String,
    pub raw_value: String,
    pub page_url: String,
    pub locator: String,
    pub observed_at: String,
    pub precision: String,
}

impl DiscoveryProvenance {
    pub fn validate(&self) -> Result<(), String> {
        fn url(value: &str) -> Result<(), String> {
            if value.len() > 4096 {
                return Err("Discovery URL exceeds 4096 bytes".into());
            }
            let parsed = url::Url::parse(value).map_err(|_| "Invalid discovery URL")?;
            if !matches!(parsed.scheme(), "http" | "https")
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
            {
                return Err("Discovery URL must be an HTTP(S) URL without credentials".into());
            }
            Ok(())
        }
        fn text(value: &str, max: usize) -> Result<(), String> {
            if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
                return Err("Invalid or oversized discovery date evidence".into());
            }
            Ok(())
        }
        url(&self.seed_url)?;
        url(&self.final_url)?;
        if let Some(referrer) = &self.referrer_url {
            url(referrer)?;
        }
        if let Some(manifest) = &self.seed_manifest {
            manifest.validate()?;
        }
        if let Some(date) = &self.date_evidence {
            url(&date.page_url)?;
            text(&date.kind, 128)?;
            text(&date.raw_value, 1024)?;
            text(&date.locator, 1024)?;
            text(&date.precision, 128)?;
            text(&date.observed_at, 64)?;
            chrono::DateTime::parse_from_rfc3339(&date.observed_at)
                .map_err(|_| "Invalid discovery evidence observation time")?;
            if date.precision.starts_with("relative:") {
                let (age, unit) = crate::posting_date::relative_age_seconds(&date.raw_value)
                    .ok_or("Invalid relative publication evidence")?;
                let observed = chrono::DateTime::parse_from_rfc3339(&date.observed_at)
                    .map_err(|_| "Invalid relative observation time")?;
                if observed.offset().local_minus_utc() != 0 {
                    return Err("Relative observation time must use UTC".into());
                }
                if date.precision != format!("relative:{age}:{unit}") {
                    return Err("Relative publication precision does not match evidence".into());
                }
            }
        }
        Ok(())
    }
}

/// NDJSON events streamed by scrape-worker over stdout.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
#[allow(clippy::large_enum_variant)]
pub enum WorkerEvent {
    CandidateEvaluated {
        source_id: String,
        evaluation: CandidateEvaluation,
    },
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
        #[serde(default)]
        #[ts(optional = nullable)]
        discovery_provenance: Option<DiscoveryProvenance>,
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
            | Self::CandidateEvaluated { source_id, .. }
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
    #[serde(default)]
    pub discovery_pages: u32,
    #[serde(default)]
    pub discovery_origins: u32,
    #[serde(default)]
    pub commit_feedback: bool,
    #[serde(default)]
    #[ts(type = "number")]
    pub newly_committed: u64,
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
            self.newly_committed,
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
            || self.newly_committed > self.accepted
            || (!self.commit_feedback
                && (self.newly_committed != 0
                    || self.accepted > u64::from(self.limit)
                    || self.limit_reached != (self.accepted == u64::from(self.limit))))
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
    fn discovery_fields_default_for_older_event_and_stats_frames() {
        let event: WorkerEvent = serde_json::from_value(serde_json::json!({
            "type":"job","source_id":"s","external_id":"1","title":"Engineer",
            "company":"Employer","role_type":"unknown","url":"https://example.test/1"
        }))
        .unwrap();
        assert!(matches!(
            event,
            WorkerEvent::Job {
                discovery_provenance: None,
                ..
            }
        ));
        let stats: SourceScanStats = serde_json::from_value(serde_json::json!({
            "source_id":"s","limit":1,"examined":0,"matched":0,"blocked":0,
            "duplicates":0,"invalid":0,"limit_reached":false
        }))
        .unwrap();
        assert_eq!((stats.discovery_pages, stats.discovery_origins), (0, 0));
        stats.validate().unwrap();
    }

    #[test]
    fn feedback_summaries_allow_refreshes_but_not_unconfirmed_insertions() {
        let mut stats = SourceScanStats {
            source_id: "fixture".into(),
            limit: 2,
            commit_feedback: true,
            examined: 4,
            matched: 4,
            accepted: 4,
            newly_committed: 2,
            limit_reached: true,
            ..Default::default()
        };
        stats.validate().unwrap();
        stats.newly_committed = 5;
        assert!(stats.validate().is_err());
        stats.newly_committed = 0;
        stats.commit_feedback = false;
        assert!(stats.validate().is_err());
    }

    #[test]
    fn commit_acknowledgement_requires_exact_known_fields_and_outcome() {
        let valid = serde_json::json!({"source_id":"s","external_id":"e","outcome":"inserted"});
        let ack: WorkerCommitAck = serde_json::from_value(valid.clone()).unwrap();
        assert_eq!(ack.outcome, WorkerCommitOutcome::Inserted);
        let mut extra = valid.clone();
        extra["unexpected"] = true.into();
        assert!(serde_json::from_value::<WorkerCommitAck>(extra).is_err());
        let mut unknown = valid;
        unknown["outcome"] = "ignored".into();
        assert!(serde_json::from_value::<WorkerCommitAck>(unknown).is_err());
    }

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
            discovery_provenance: None,
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
    #[test]
    fn proxy_validation_normalizes_endpoint_and_never_formats_credentials() {
        let config = HttpProxyConfig {
            server: " https://proxy.example ".into(),
            username: Some(" fixture-user ".into()),
            password: Some("fixture-password".into()),
        }
        .validate()
        .unwrap();
        assert_eq!(config.server, "https://proxy.example");
        assert_eq!(config.username.as_deref(), Some("fixture-user"));
        assert_eq!(config.password.as_deref(), Some("fixture-password"));
        let debug = format!("{config:?}");
        assert!(!debug.contains("fixture-user"));
        assert!(!debug.contains("fixture-password"));
        for (server, username, password) in [
            ("socks5://proxy.example:8080", None, None),
            ("http://user:pass@proxy.example:8080", None, None),
            ("http://proxy.example:8080/path", None, None),
            ("http://proxy.example:8080?key=x", None, None),
            ("http://proxy.example:8080", Some("user"), None),
            ("http://proxy.example:8080", None, Some("password")),
            ("http://proxy.example:8080", Some(" "), Some("password")),
        ] {
            assert!(HttpProxyConfig {
                server: server.into(),
                username: username.map(str::to_owned),
                password: password.map(str::to_owned),
            }
            .validate()
            .is_err());
        }
    }

    #[test]
    fn socks5_proxy_and_jitter_limits_validate_cleanly() {
        let socks = HttpProxyConfig {
            server: " socks5://127.0.0.1:1080 ".into(),
            username: None,
            password: None,
        }
        .validate_with_socks()
        .unwrap();
        assert_eq!(socks.server, "socks5://127.0.0.1:1080");
        let socks5h = HttpProxyConfig {
            server: "socks5h://proxy.example:1080".into(),
            username: Some("user".into()),
            password: Some("pass".into()),
        }
        .validate_with_socks()
        .unwrap();
        assert_eq!(socks5h.server, "socks5h://proxy.example:1080");
        assert!(HttpProxyConfig {
            server: "ftp://proxy.example:21".into(),
            username: None,
            password: None,
        }
        .validate_with_socks()
        .is_err());
        let mut limits = WorkerLimits {
            jitter_ms: Some(500),
            ignore_robots: Some(true),
            ..Default::default()
        };
        assert!(limits.validate().is_ok());
        limits.jitter_ms = Some(10_001);
        assert!(limits.validate().is_err());
    }
}

#[cfg(test)]
mod candidate_diagnostic_tests {
    use super::*;

    #[test]
    fn facts_preserve_identity_and_bound_unicode_description_only() {
        let identity = "x".repeat(1024);
        let event: WorkerEvent = serde_json::from_value(serde_json::json!({
            "type":"job","source_id":"fixture","external_id":identity,
            "title":"Engineer","company":"Fixture","role_type":"unknown",
            "url":"https://fixture.test/jobs/1","description_text":"é".repeat(2049)
        }))
        .unwrap();
        let facts = CandidateFacts::from_event(&event).unwrap();
        assert_eq!(facts.external_id.len(), 1024);
        assert_eq!(
            facts.description_excerpt.as_ref().unwrap().chars().count(),
            2048
        );
        assert!(facts.description_truncated);
        let mut evaluation = CandidateEvaluation {
            sequence: 1,
            disposition: CandidateDisposition::Eligible,
            candidate: Some(facts),
            rejected: vec![],
            unresolved: vec![],
            diagnostic: None,
        };
        evaluation.validate().unwrap();
        evaluation.sequence = 0;
        assert!(evaluation.validate().is_err());
        evaluation.sequence = 1;
        evaluation.candidate = None;
        assert!(evaluation.validate().is_err());
        evaluation.disposition = CandidateDisposition::Invalid;
        evaluation.validate().unwrap();
    }

    #[test]
    fn evaluation_shape_preserves_overlapping_categories_and_rejects_duplicates() {
        use crate::matching::RejectionReason;
        let event: WorkerEvent = serde_json::from_value(serde_json::json!({
            "type":"job","source_id":"fixture","external_id":"1","title":"Engineer",
            "company":"Fixture","role_type":"unknown","url":"https://fixture.test/jobs/1"
        }))
        .unwrap();
        let reason = RejectionReason::InvalidCandidate;
        let mut evaluation = CandidateEvaluation {
            sequence: 1,
            disposition: CandidateDisposition::Rejected,
            candidate: CandidateFacts::from_event(&event),
            rejected: vec![reason],
            unresolved: vec![reason],
            diagnostic: None,
        };
        evaluation.validate().unwrap();
        evaluation.rejected.push(reason);
        assert!(evaluation.validate().is_err());
        evaluation.rejected.clear();
        assert!(evaluation.validate().is_err());
        evaluation.disposition = CandidateDisposition::Unresolved;
        evaluation.validate().unwrap();
        evaluation.sequence = 1u64 << 53;
        assert!(evaluation.validate().is_err());
    }
}
