use crate::enrich::{
    ClassifiedClearance, ClassifiedSponsorship, ClassifiedWorkAuth, ExperienceClassification,
    ParsedLocation, Salary,
};
use crate::{RoleType, SourceKind, TriState, WorkplaceType};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const PROTOCOL_VERSION: u32 = 2;

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
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkerSource {
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub config: serde_json::Value,
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
        use crate::filter::{FilterExpr, FilterField, FilterOp};
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
                and_clauses.push(FilterExpr::parse_query(trimmed)?);
            }
        }

        // 2. If explicit structured policy object is present, parse it
        if let Some(ref pol) = self.policy {
            if !pol.is_null() {
                and_clauses.push(FilterExpr::parse_structured_policy(pol)?);
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
            and_clauses.push(FilterExpr::Or(includes));
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
            and_clauses.push(FilterExpr::Not(Box::new(FilterExpr::Or(excludes))));
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
            and_clauses.push(FilterExpr::Or(locs));
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
            and_clauses.push(FilterExpr::FieldIn {
                field: FilterField::RoleType,
                values: roles,
                negated: false,
            });
        }

        if and_clauses.is_empty() {
            Ok(FilterExpr::AlwaysTrue)
        } else if and_clauses.len() == 1 {
            Ok(and_clauses.remove(0))
        } else {
            Ok(FilterExpr::And(and_clauses))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkerLimits {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_per_host_delay_ms")]
    pub per_host_delay_ms: u64,
    #[serde(default = "default_request_timeout_s")]
    pub request_timeout_s: u64,
}

impl Default for WorkerLimits {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            per_host_delay_ms: default_per_host_delay_ms(),
            request_timeout_s: default_request_timeout_s(),
        }
    }
}

/// NDJSON events streamed by scrape-worker over stdout.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
#[allow(clippy::large_enum_variant)]
pub enum WorkerEvent {
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
        salary: Option<Salary>,
        #[serde(default)]
        locations: Vec<ParsedLocation>,
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
        company_icon_url: Option<String>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkerStats {
    pub sources_ok: u64,
    pub sources_failed: u64,
    pub jobs_emitted: u64,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            date_posted: Some("2026-10-01T00:00:00Z".to_string()),
            salary: None,
            locations: Vec::new(),
            workplace_type: WorkplaceType::Remote,
            sponsorship: None,
            clearance: None,
            experience: None,
            is_new_grad: TriState::Unknown,
            description_text: None,
            company_icon_url: None,
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
            },
        };
        let done_str = serde_json::to_string(&done).unwrap();
        assert!(done_str.contains(r#""type":"done""#));
        let done_deserialized: WorkerEvent = serde_json::from_str(&done_str).unwrap();
        assert_eq!(done_deserialized, done);
    }
}
