//! Authoritative, versioned built-in templates. Generated frontend/YAML files
//! are projections of this validated catalog, never inputs to seeding.
use super::workflow::{self, WorkflowSourceSpec, WorkflowSpec};
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use ts_rs::TS;

pub const CATALOG_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TemplateReadiness {
    Ready,
    NeedsReview,
    SetupRequired,
    RestrictiveExample,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub readiness: TemplateReadiness,
    pub seed: bool,
    pub legacy_identifier: String,
    pub spec: WorkflowSpec,
    pub extractors: Vec<TemplateExtractor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct TemplateExtractor {
    pub id: String,
    pub kind: scrape_core::SourceKind,
    pub identifier: String,
    pub config: Value,
}

pub(crate) fn extractors(spec: &WorkflowSpec) -> Result<Vec<TemplateExtractor>> {
    spec.sources
        .iter()
        .map(|source| {
            let input = workflow::extract_source_input(spec, source, "UTC")?;
            let mut config = input.config.unwrap();
            // Execution-only projection; filters belong to the workflow, not an adapter.
            config.as_object_mut().unwrap().remove("__workflow_spec");
            Ok(TemplateExtractor {
                id: source.id.clone(),
                kind: input.kind,
                identifier: input.identifier,
                config,
            })
        })
        .collect()
}

/// Company choices own their execution source independently of workflow templates.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct CompanyPreset {
    pub id: String,
    pub name: String,
    pub source: WorkflowSourceSpec,
    pub readiness: TemplateReadiness,
    pub description: String,
    pub logo: Option<scrape_core::branding::CompanyLogo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkflowTemplateCatalog {
    pub version: u32,
    pub templates: Vec<WorkflowTemplate>,
    pub companies: Vec<CompanyPreset>,
}

const ENGINEERING_ROLES: &[&str] = &[
    "Software Engineer",
    "Software Developer",
    "Software Development Engineer",
    "Backend Engineer",
    "Backend Developer",
    "Back-end Engineer",
    "Back-end Developer",
    "Frontend Engineer",
    "Frontend Developer",
    "Front-end Engineer",
    "Front-end Developer",
    "Full Stack",
    "Full-Stack",
    "Platform Engineer",
    "Infrastructure Engineer",
    "DevOps Engineer",
    "Site Reliability Engineer",
    "Data Engineer",
    "Machine Learning Engineer",
    "ML Engineer",
    "AI Engineer",
    "QA Engineer",
    "Quality Assurance Engineer",
    "Test Automation Engineer",
    "Embedded Software",
    "Firmware Engineer",
    "Mobile Engineer",
    "iOS Engineer",
    "Android Engineer",
    "Web Developer",
];
const FAST_FOOD_ROLES: &[&str] = &[
    "Fast Food",
    "Fast-Food",
    "Crew Member",
    "Crew Team Member",
    "Team Member",
    "Restaurant Crew",
    "Cashier",
    "Cook",
    "Shift Manager",
    "Shift Leader",
    "Restaurant Manager",
    "Food Service Worker",
];
const FAST_FOOD_EVIDENCE: &[&str] = &[
    "fast food",
    "fast-food",
    "quick service",
    "quick-service",
    "McDonald's",
    "McDonalds",
    "Burger King",
    "Wendy's",
    "Taco Bell",
    "Chick-fil-A",
    "KFC",
    "Popeyes",
    "Chipotle",
    "Five Guys",
    "Shake Shack",
    "Sonic Drive-In",
    "Arby's",
    "Dairy Queen",
    "White Castle",
    "Checkers",
    "Rally's",
    "Jollibee",
    "Culver's",
    "Whataburger",
];
const COVERAGE: &str = "Adzuna not enabled — setup required: configure credentials in Settings > System > Job providers, then explicitly enable the source. Loading or saving never starts scraping. Sources are bounded, not exhaustive; local matching remains authoritative.";
const BROAD_COVERAGE: &str = "The Muse and Germany-focused Arbeitnow provide bounded listings; Arbeitnow does not establish original publication dates. Jobicy covers seven days with a three-hour delay and hourly response caching. Direct boards: Cloudflare/Bitwarden Greenhouse, Linear Ashby, SEP Lever and NVIDIA Workday. Google/Apple (Needs review) and Netflix/Amazon (Setup required) remain company-picker choices, excluded here without granting script trust or enabling them.";

fn search_source(id: &str, label: &str, enabled: bool, config: Value) -> Value {
    json!({"id":id,"label":label,"enabled":enabled,"extractor":{"kind":"search","url":id,"source_config":config}})
}

fn sources(
    companies: &[CompanyPreset],
    fast_food: bool,
    query: &str,
    days: Option<u32>,
) -> Result<Vec<Value>> {
    let mut adzuna = json!({"what_or":query});
    if fast_food {
        adzuna["where"] = "Cherry Hill, New Jersey".into();
    }
    if let Some(days) = days {
        adzuna["max_days_old"] = days.into();
    }
    let muse = search_source(
        "themuse",
        "The Muse — bounded cross-company listings",
        true,
        json!({}),
    );
    let adzuna = search_source(
        "adzuna",
        "Adzuna not enabled — setup required",
        false,
        adzuna,
    );
    if fast_food {
        return Ok(vec![adzuna, muse]);
    }
    let mut sources = vec![
        muse,
        search_source(
            "arbeitnow",
            "Arbeitnow — Germany-focused; original date unknown",
            true,
            json!({}),
        ),
        search_source(
            "jobicy",
            "Jobicy — remote; seven-day feed, three-hour delay",
            true,
            json!({}),
        ),
        adzuna,
    ];
    for id in ["cloudflare", "bitwarden", "linear", "sep", "nvidia"] {
        let company = companies
            .iter()
            .find(|company| company.id == id)
            .ok_or_else(|| AppError::Internal(format!("Missing built-in company source {id}")))?;
        sources.push(serde_json::to_value(&company.source)?);
    }
    Ok(sources)
}

pub fn catalog() -> Result<WorkflowTemplateCatalog> {
    let companies = super::provider_presets::companies()?;
    let date =
        |preset| json!({"preset":preset,"timezone":"America/New_York","include_unknown":false});
    let definitions = [
        (
            "software-engineering-last-12-hours",
            "Software Engineering & Related — Past 12 Hours",
            format!("Literal engineering title roles; original publication in the past 12 hours, including the exact boundary. Unknown dates excluded (direct employer boards such as Cloudflare, Bitwarden, Linear, SEP, and NVIDIA match 0 candidates unless a role was originally published within the past 12 hours); employment, geography, workplace and pay unrestricted. {BROAD_COVERAGE} {COVERAGE}"),
            sources(&companies, false, "software developer engineer devops firmware", Some(1))?,
            json!({"target_roles":ENGINEERING_ROLES,"posting_date":date("past_12h")}),
        ),
        (
            "cherry-hill-fast-food-last-month",
            "Fast Food — Cherry Hill, NJ — Past 30 Days",
            format!("Front-line role title AND literal fast-food evidence in title/company/description; Cherry Hill + US-NJ + US in one location tuple; original publication within rolling 720 hours. Unknown dates/geography excluded. Adzuna discovery is disabled until setup. Remaining The Muse coverage is limited, not verified Cherry Hill availability. {COVERAGE}"),
            sources(&companies, true, "restaurant crew cashier cook fast food", Some(30))?,
            json!({"target_roles":FAST_FOOD_ROLES,"keywords":{"any":FAST_FOOD_EVIDENCE,"all":[],"exclude":[],"fields":["title","company","description"]},
                "posting_date":date("past_month"),"countries":["US"],"regions":["US-NJ"],"cities":["Cherry Hill"],"location_include_unknown":false}),
        ),
        (
            "remote-usd-over-60000",
            "Remote — Advertised Annual USD Pay Over $60,000",
            format!("Remote-only with evidenced advertised annual USD pay over $60,000: a range whose upper endpoint exceeds the threshold qualifies, not a guaranteed minimum. Unknown, hourly/monthly and other currencies excluded; no annualization, currency conversion, date or geography restriction. Adzuna predicted pay and bare native amounts without annual evidence do not qualify. {BROAD_COVERAGE} {COVERAGE}"),
            sources(&companies, false, "remote", None)?,
            json!({"workplaces":["remote"],"compensation":{"min_minor":6000001,"max_minor":null,"currency":"USD","interval":"year","include_unknown":false,"match_mode":"advertised_amount"}}),
        ),
    ];
    let mut templates = Vec::new();
    for (id, name, description, sources, matching) in definitions {
        let spec = workflow::validate(
            json!({
                "version":workflow::WORKFLOW_SPEC_VERSION,"max_jobs_per_run":20,
                "metadata":{"name":name,"enabled":true},"sources":sources,
                "schedule":{"cadence":"manual","timezone":"America/New_York","run_on_startup":false},
                "filters":{"matching":matching},"retention":{"days":30,"policy":"untracked_only"}
            }),
            "America/New_York",
        )?;
        templates.push(WorkflowTemplate {
            id: id.into(),
            name: name.into(),
            description,
            readiness: TemplateReadiness::Ready,
            seed: true,
            legacy_identifier: id.into(),
            extractors: extractors(&spec)?,
            spec,
        });
    }
    let catalog = WorkflowTemplateCatalog {
        version: CATALOG_VERSION,
        templates,
        companies,
    };
    validate_companies(&catalog)?;
    Ok(catalog)
}

fn validate_companies(catalog: &WorkflowTemplateCatalog) -> Result<()> {
    let invalid = |message| AppError::Validation(format!("Company catalog: {message}"));
    let mut template_ids = std::collections::HashSet::new();
    for template in &catalog.templates {
        if template.id.trim().is_empty() {
            return Err(invalid("missing template ID".into()));
        }
        if !template_ids.insert(&template.id) {
            return Err(invalid(format!("duplicate template ID '{}'", template.id)));
        }
    }
    let mut ids = std::collections::HashSet::new();
    for company in &catalog.companies {
        if company.id.is_empty()
            || company.id.starts_with('-')
            || company.id.ends_with('-')
            || company.id.contains("--")
            || !company
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(invalid(format!("invalid company ID '{}'", company.id)));
        }
        if !ids.insert(&company.id) {
            return Err(invalid(format!("duplicate company ID '{}'", company.id)));
        }
        if company.source.id != company.id || company.source.extractor.url.trim().is_empty() {
            return Err(invalid(format!(
                "company '{}' has an invalid owned source",
                company.id
            )));
        }
        if company.readiness != TemplateReadiness::Ready && company.source.enabled {
            return Err(invalid(format!(
                "company '{}' requires explicit setup/review",
                company.id
            )));
        }
        // Validate the standalone source through the same resolver as editor saves.
        workflow::validate(
            json!({"version":workflow::WORKFLOW_SPEC_VERSION,
            "metadata":{"name":company.name,"enabled":company.source.enabled},
            "sources":[company.source],"schedule":{"cadence":"manual"}}),
            "UTC",
        )?;
        if let Some(logo) = &company.logo {
            logo.validate()
                .map_err(|error| invalid(format!("company '{}' logo: {error}", company.id)))?;
            if logo.status != scrape_core::branding::LogoStatus::Verified
                || !logo
                    .asset
                    .as_deref()
                    .is_some_and(|asset| asset.starts_with("data:image/webp;base64,"))
            {
                return Err(invalid(format!(
                    "company '{}' logo must be verified inline WebP",
                    company.id
                )));
            }
        }
    }
    Ok(())
}

pub fn seed_inputs() -> Result<Vec<crate::db::sources::CreateSourceInput>> {
    catalog()?
        .templates
        .into_iter()
        .map(|template| workflow::source_input(&template.spec, "America/New_York"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_exactly_three_targeted_manual_profiles() {
        let catalog = catalog().unwrap();
        assert_eq!(catalog.version, 3);
        assert_eq!(
            catalog
                .templates
                .iter()
                .map(|template| template.id.as_str())
                .collect::<Vec<_>>(),
            [
                "software-engineering-last-12-hours",
                "cherry-hill-fast-food-last-month",
                "remote-usd-over-60000"
            ]
        );
        for template in &catalog.templates {
            assert!(template.seed);
            assert_eq!(template.spec.version, 10);
            assert_eq!(template.spec.max_jobs_per_run, Some(20));
            assert_eq!(
                template.spec.schedule.as_ref().unwrap().cadence,
                Some(scrape_core::SourceCadence::Manual)
            );
            assert_eq!(template.spec.retention.as_ref().unwrap().days, Some(30));
            assert_eq!(
                template.spec.retention.as_ref().unwrap().policy.as_deref(),
                Some("untracked_only")
            );
            assert_eq!(
                workflow::validate(serde_json::to_value(&template.spec).unwrap(), "UTC").unwrap(),
                template.spec
            );
            assert_eq!(extractors(&template.spec).unwrap(), template.extractors);
            let adzuna = template
                .spec
                .sources
                .iter()
                .find(|source| source.id == "adzuna")
                .unwrap();
            assert!(!adzuna.enabled);
            assert!(template
                .description
                .contains("Adzuna not enabled — setup required"));
            assert!(!adzuna
                .extractor
                .source_config
                .as_ref()
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("salary_min"));
            assert!(template
                .spec
                .sources
                .iter()
                .all(|source| source.extractor.script.is_none()));
        }
        let engineering = serde_json::to_value(&catalog.templates[0].spec).unwrap();
        assert_eq!(
            engineering["filters"]["matching"]["target_roles"],
            json!(ENGINEERING_ROLES)
        );
        assert_eq!(
            engineering["filters"]["matching"]["posting_date"]["preset"],
            "past_12h"
        );
        let fast_food = serde_json::to_value(&catalog.templates[1].spec).unwrap();
        assert_eq!(
            fast_food["filters"]["matching"]["target_roles"],
            json!(FAST_FOOD_ROLES)
        );
        assert_eq!(
            fast_food["filters"]["matching"]["keywords"]["any"],
            json!(FAST_FOOD_EVIDENCE)
        );
        assert_eq!(
            fast_food["filters"]["matching"]["posting_date"]["preset"],
            "past_month"
        );
        assert_eq!(
            fast_food["filters"]["matching"]["cities"],
            json!(["Cherry Hill"])
        );
        assert_eq!(
            fast_food["filters"]["matching"]["regions"],
            json!(["US-NJ"])
        );
        assert_eq!(fast_food["filters"]["matching"]["countries"], json!(["US"]));
        assert_eq!(
            catalog.templates[1]
                .spec
                .sources
                .iter()
                .map(|source| source.id.as_str())
                .collect::<Vec<_>>(),
            ["adzuna", "themuse"]
        );
        for index in [0, 2] {
            assert_eq!(
                catalog.templates[index]
                    .spec
                    .sources
                    .iter()
                    .map(|source| source.id.as_str())
                    .collect::<Vec<_>>(),
                [
                    "themuse",
                    "arbeitnow",
                    "jobicy",
                    "adzuna",
                    "cloudflare",
                    "bitwarden",
                    "linear",
                    "sep",
                    "nvidia"
                ]
            );
        }
        let remote = serde_json::to_value(&catalog.templates[2].spec).unwrap();
        assert_eq!(
            remote["filters"]["matching"]["workplaces"],
            json!(["remote"])
        );
        assert_eq!(
            remote["filters"]["matching"]["compensation"]["match_mode"],
            "advertised_amount"
        );
        assert_eq!(
            remote["filters"]["matching"]["compensation"]["min_minor"],
            6000001
        );
        assert_eq!(
            remote["filters"]["matching"]["compensation"]["currency"],
            "USD"
        );
        assert_eq!(
            remote["filters"]["matching"]["compensation"]["interval"],
            "year"
        );
        assert_eq!(
            remote["filters"]["matching"]["compensation"]["include_unknown"],
            false
        );
    }

    #[test]
    fn independent_company_sources_keep_review_and_script_gates() {
        let catalog = catalog().unwrap();
        assert_eq!(
            catalog
                .companies
                .iter()
                .map(|company| company.id.as_str())
                .collect::<Vec<_>>(),
            [
                "cloudflare",
                "bitwarden",
                "linear",
                "sep",
                "google",
                "apple",
                "netflix",
                "amazon",
                "nvidia"
            ]
        );
        for company in &catalog.companies {
            assert!(company.logo.is_some());
            if matches!(company.id.as_str(), "google" | "apple") {
                assert_eq!(company.readiness, TemplateReadiness::NeedsReview);
                assert!(!company.source.enabled);
            } else if matches!(company.id.as_str(), "netflix" | "amazon") {
                assert_eq!(company.readiness, TemplateReadiness::SetupRequired);
                assert!(!company.source.enabled);
                let script = company.source.extractor.script.as_ref().unwrap();
                assert!(script.code.as_ref().unwrap().contains("version: 2"));
                assert!(script.path.is_none() && script.interpreter.is_none());
            } else {
                assert_eq!(company.readiness, TemplateReadiness::Ready);
                assert!(company.source.enabled);
            }
        }
        let mut invalid = catalog;
        invalid.companies[0].source.id = "wrong-owner".into();
        assert!(validate_companies(&invalid)
            .unwrap_err()
            .to_string()
            .contains("invalid owned source"));
    }

    #[test]
    fn catalog_projection_is_generated_from_canonical_specs() {
        let generated: Value = serde_json::from_str(include_str!(
            "../../../src/generated/workflowTemplates.json"
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(catalog().unwrap()).unwrap(),
            generated,
            "Run npm run gen:templates; do not edit generated catalogs"
        );
    }

    fn matching_ids(template: usize, candidates: Vec<(&str, Value)>) -> Vec<String> {
        let catalog = catalog().unwrap();
        let rules = catalog.templates[template]
            .spec
            .filters
            .as_ref()
            .unwrap()
            .matching
            .as_ref()
            .unwrap();
        let started = chrono::DateTime::parse_from_rfc3339("2026-10-23T16:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        candidates.into_iter().filter_map(|(id, fields)| {
            let mut event = json!({
                "type":"job","source_id":"fixture","external_id":id,"title":"Software Engineer","company":"Fixture",
                "location":"","role_type":"full_time","url":format!("https://fixture.invalid/{id}"),
                "date_posted":"2026-10-23T16:00:00Z"
            });
            event.as_object_mut().unwrap().extend(fields.as_object().unwrap().clone());
            let event = serde_json::from_value::<scrape_core::WorkerEvent>(event).unwrap();
            let assessment = rules.assess(&event, started, &scrape_core::matching::MatchingEvidence::complete());
            (assessment.rejected.is_empty() && assessment.unresolved.is_empty()).then(|| id.into())
        }).collect()
    }

    #[test]
    fn engineering_profile_enforces_exact_title_and_original_twelve_hour_window() {
        assert_eq!(
            matching_ids(
                0,
                vec![
                    (
                        "software-boundary",
                        json!({"title":"Software Engineer","date_posted":"2026-10-23T04:00:00Z"})
                    ),
                    (
                        "backend",
                        json!({"title":"Backend Developer","date_posted":"2026-10-23T12:00:00Z"})
                    ),
                    ("data", json!({"title":"Data Engineer"})),
                    ("firmware", json!({"title":"Firmware Engineer"})),
                    ("qa", json!({"title":"QA Engineer"})),
                    ("too-old", json!({"date_posted":"2026-10-23T03:59:59.999Z"})),
                    ("future", json!({"date_posted":"2026-10-23T16:00:00.001Z"})),
                    ("unknown-date", json!({"date_posted":null})),
                    ("invalid-date", json!({"date_posted":"not-a-date"})),
                    ("mechanical", json!({"title":"Mechanical Engineer"})),
                    ("sales", json!({"title":"Sales Intern"})),
                ]
            ),
            ["software-boundary", "backend", "data", "firmware", "qa"]
        );
    }

    #[test]
    fn fast_food_profile_requires_industry_evidence_and_same_tuple_geography() {
        let cherry_hill = json!([{"raw":"Cherry Hill, New Jersey, US","city":"Cherry Hill","region_code":"US-NJ","country_code":"US","is_remote":false}]);
        let candidate = |fields: Value| {
            let mut value = json!({"title":"Crew Member","company":"Chipotle","description_text":"Restaurant role",
                "locations":cherry_hill,"location_data_complete":true});
            value
                .as_object_mut()
                .unwrap()
                .extend(fields.as_object().unwrap().clone());
            value
        };
        assert_eq!(
            matching_ids(
                1,
                vec![
                    ("chipotle", candidate(json!({}))),
                    (
                        "boundary",
                        candidate(json!({"date_posted":"2026-09-23T16:00:00Z"}))
                    ),
                    (
                        "shift",
                        candidate(
                            json!({"title":"Shift Leader","company":"Fixture","description_text":"A fast-food restaurant"})
                        )
                    ),
                    (
                        "too-old",
                        candidate(json!({"date_posted":"2026-09-23T15:59:59.999Z"}))
                    ),
                    ("unknown-date", candidate(json!({"date_posted":null}))),
                    (
                        "future",
                        candidate(json!({"date_posted":"2026-10-23T16:00:00.001Z"}))
                    ),
                    (
                        "retail",
                        candidate(json!({"title":"Cashier","company":"Generic Retail"}))
                    ),
                    (
                        "fine-dining",
                        candidate(json!({"title":"Cook","company":"Fine Dining"}))
                    ),
                    ("corporate", candidate(json!({"title":"Software Engineer"}))),
                    (
                        "other-city",
                        candidate(
                            json!({"locations":[{"raw":"Newark, NJ","city":"Newark","region_code":"US-NJ","country_code":"US","is_remote":false}]})
                        )
                    ),
                    (
                        "other-state",
                        candidate(
                            json!({"locations":[{"raw":"Cherry Hill, VA","city":"Cherry Hill","region_code":"US-VA","country_code":"US","is_remote":false}]})
                        )
                    ),
                    (
                        "split-tuple",
                        candidate(json!({"locations":[
                {"raw":"Cherry Hill, VA","city":"Cherry Hill","region_code":"US-VA","country_code":"US","is_remote":false},
                {"raw":"Newark, NJ","city":"Newark","region_code":"US-NJ","country_code":"US","is_remote":false}]}))
                    ),
                    ("unknown-geography", candidate(json!({"locations":[]}))),
                ]
            ),
            ["chipotle", "boundary", "shift"]
        );
    }
}
