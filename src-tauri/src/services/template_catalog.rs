//! Authoritative, versioned built-in templates. Generated frontend/YAML files
//! are projections of this validated catalog, never inputs to seeding.
use super::workflow::{self, WorkflowSpec};
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use ts_rs::TS;

pub const CATALOG_VERSION: u32 = 2;

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

fn extractors(spec: &WorkflowSpec) -> Result<Vec<TemplateExtractor>> {
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

/// Company choices reference one catalog source; execution configuration stays in its template.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct CompanyPreset {
    pub id: String,
    pub name: String,
    pub template_id: String,
    pub source_id: String,
    pub logo: Option<scrape_core::branding::CompanyLogo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct WorkflowTemplateCatalog {
    pub version: u32,
    pub templates: Vec<WorkflowTemplate>,
    pub companies: Vec<CompanyPreset>,
}

fn definitions() -> Vec<(&'static str, &'static str, &'static str, Value)> {
    vec![
        (
            "cloudflare-systems-network",
            "cloudflare",
            "Cloudflare — Engineering (title keywords)",
            json!({
                "version": 6,
                "metadata": {
                    "name": "Cloudflare — Software Engineering & Internships",
                    "company": "Cloudflare",
                    "enabled": true
                },
                "sources": [
                    {
                        "id": "cloudflare",
                        "label": "Cloudflare — Greenhouse",
                        "enabled": true,
                        "employer_hint": "Cloudflare",
                        "extractor": {
                            "kind": "greenhouse",
                            "url": "https://boards.greenhouse.io/cloudflare"
                        }
                    }
                ],
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "cron": "0 0 9 * * *",
                    "timezone": "UTC"
                },
                "filters": {
                    "matching": {
                        "role_types": [
                            "internship",
                            "full_time"
                        ],
                        "workplaces": [
                            "remote",
                            "hybrid",
                            "onsite"
                        ],
                        "countries": [
                            "US",
                            "GB",
                            "PT"
                        ],
                        "cities": [
                            "Austin",
                            "San Francisco",
                            "London",
                            "Lisbon"
                        ],
                        "keywords": {
                            "any": [
                                "Software",
                                "Engineer",
                                "Developer",
                                "Systems",
                                "Intern"
                            ],
                            "all": [],
                            "exclude": [
                                "Recruiter",
                                "Account Executive",
                                "Sales Representative"
                            ],
                            "fields": [
                                "title"
                            ]
                        }
                    }
                },
                "retention": {
                    "days": 45,
                    "policy": "untracked_only"
                }
            }),
        ),
        (
            "bitwarden-remote-security",
            "bitwarden",
            "Bitwarden — Engineering (title keywords)",
            json!({
                "version": 6,
                "metadata": {
                    "name": "Bitwarden — Remote Full-Stack & Security Engineering",
                    "company": "Bitwarden",
                    "enabled": true
                },
                "sources": [
                    {
                        "id": "bitwarden",
                        "label": "Bitwarden — Greenhouse",
                        "enabled": true,
                        "employer_hint": "Bitwarden",
                        "extractor": {
                            "kind": "greenhouse",
                            "url": "https://boards.greenhouse.io/bitwarden"
                        }
                    }
                ],
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "cron": "0 0 9 * * *",
                    "timezone": "UTC"
                },
                "filters": {
                    "matching": {
                        "role_types": [
                            "full_time"
                        ],
                        "workplaces": [
                            "remote"
                        ],
                        "countries": [
                            "US",
                            "CA",
                            "GB",
                            "IN"
                        ],
                        "cities": [],
                        "keywords": {
                            "any": [
                                "Engineer",
                                "Developer",
                                "Security",
                                "QA",
                                "Cloud"
                            ],
                            "all": [],
                            "exclude": [],
                            "fields": [
                                "title"
                            ]
                        }
                    }
                },
                "retention": {
                    "days": 30,
                    "policy": "untracked_only"
                }
            }),
        ),
        (
            "linear-product-engineering",
            "linear",
            "Linear — Product Engineering (title keywords)",
            json!({
                "version": 6,
                "metadata": {
                    "name": "Linear — Product, Frontend & Mobile Engineering",
                    "company": "Linear",
                    "enabled": true
                },
                "sources": [
                    {
                        "id": "linear",
                        "label": "Linear — Ashby",
                        "enabled": true,
                        "employer_hint": "Linear",
                        "extractor": {
                            "kind": "ashby",
                            "url": "https://jobs.ashbyhq.com/linear"
                        }
                    }
                ],
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "cron": "0 0 9 * * *",
                    "timezone": "UTC"
                },
                "filters": {
                    "matching": {
                        "role_types": [
                            "full_time"
                        ],
                        "workplaces": [
                            "remote"
                        ],
                        "countries": [
                            "US",
                            "CA",
                            "GB",
                            "DE",
                            "FR"
                        ],
                        "cities": [],
                        "keywords": {
                            "any": [
                                "Engineer",
                                "Product",
                                "Designer",
                                "Frontend",
                                "Mobile"
                            ],
                            "all": [],
                            "exclude": [],
                            "fields": [
                                "title"
                            ]
                        }
                    }
                },
                "retention": {
                    "days": 30,
                    "policy": "untracked_only"
                }
            }),
        ),
        (
            "sep-midwest-consulting",
            "sep",
            "SEP — Engineering (title keywords)",
            json!({
                "version": 6,
                "metadata": {
                    "name": "SEP — Software, AI & Embedded Engineering",
                    "company": "SEP",
                    "enabled": true
                },
                "sources": [
                    {
                        "id": "sep",
                        "label": "SEP — Lever",
                        "enabled": true,
                        "employer_hint": "SEP",
                        "extractor": {
                            "kind": "lever",
                            "url": "https://jobs.lever.co/sep"
                        }
                    }
                ],
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "cron": "0 0 9 * * *",
                    "timezone": "UTC"
                },
                "filters": {
                    "matching": {
                        "role_types": [
                            "full_time",
                            "internship"
                        ],
                        "workplaces": [
                            "hybrid",
                            "onsite"
                        ],
                        "countries": [
                            "US"
                        ],
                        "regions": [
                            "US-IN"
                        ],
                        "cities": [
                            "Indianapolis",
                            "Westfield"
                        ],
                        "keywords": {
                            "any": [
                                "Software",
                                "Engineer",
                                "AI",
                                "Producer",
                                "Embedded",
                                "Cloud"
                            ],
                            "all": [],
                            "exclude": [],
                            "fields": [
                                "title"
                            ]
                        }
                    }
                },
                "retention": {
                    "days": 30,
                    "policy": "untracked_only"
                }
            }),
        ),
        (
            "bigtech-static-swe",
            "google-apple-selector",
            "Google & Apple — Static extraction example (Needs review)",
            json!({
                "version": 6,
                "metadata": {
                    "name": "Google & Apple — Core Software & ML Engineering",
                    "company": "BigTech",
                    "enabled": true
                },
                "sources": [
                    {
                        "id": "google",
                        "label": "Google Careers",
                        "enabled": true,
                        "employer_hint": "Google",
                        "extractor": {
                            "kind": "selector",
                            "mode": "static",
                            "url": "https://www.google.com/about/careers/applications/jobs/results/",
                            "job_container": "li.lLd3Je",
                            "title_selector": "h3.QJPWVe",
                            "url_selector": "a[href*=\"jobs/results/\"]",
                            "location_selector": "span.r0wTof",
                            "description_selector": "div.Xsxa1e",
                            "next_page_selector": "a[aria-label=\"Go to next page\"]"
                        }
                    },
                    {
                        "id": "apple",
                        "label": "Apple Jobs",
                        "enabled": true,
                        "employer_hint": "Apple",
                        "extractor": {
                            "kind": "selector",
                            "mode": "static",
                            "url": "https://jobs.apple.com/en-us/search",
                            "job_container": "ul#search-job-list > li.rc-accordion-item",
                            "title_selector": "h3 a.link-inline",
                            "url_selector": "a.link-inline",
                            "location_selector": "div.job-title-location span:not(.a11y)",
                            "date_selector": "span.job-posted-date",
                            "description_selector": "p.text-align-start",
                            "next_page_selector": "link[rel=\"next\"]"
                        }
                    }
                ],
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "cron": "0 0 9 * * *",
                    "timezone": "UTC"
                },
                "filters": {
                    "matching": {
                        "role_types": [],
                        "workplaces": [],
                        "countries": [],
                        "regions": [],
                        "cities": [],
                        "keywords": {
                            "any": [
                                "Software",
                                "Machine Learning",
                                "Data",
                                "Systems",
                                "Platform"
                            ],
                            "all": [],
                            "exclude": [],
                            "fields": [
                                "title"
                            ]
                        }
                    }
                },
                "retention": {
                    "days": 30,
                    "policy": "untracked_only"
                }
            }),
        ),
        (
            "streaming-cloud-script",
            "streaming-cloud-script",
            "Netflix & Amazon — Script example (Setup required)",
            json!({
                "version": 6,
                "metadata": {
                    "name": "Netflix & Amazon — Distributed Systems & Platform",
                    "company": "Cloud",
                    "enabled": true
                },
                "sources": [
                    {
                        "id": "netflix",
                        "label": "Netflix",
                        "enabled": true,
                        "employer_hint": "Netflix",
                        "extractor": {
                            "kind": "selector",
                            "mode": "script",
                            "url": "https://explore.jobs.netflix.net/api/apply/v2/jobs?domain=netflix.com&start=0&num=10",
                            "script": {
                                "runtime": "js",
                                "code": include_str!("../../../crates/scrape-worker/examples/extractors/netflix_extractor.js")
                            }
                        }
                    },
                    {
                        "id": "amazon",
                        "label": "Amazon",
                        "enabled": true,
                        "employer_hint": "Amazon",
                        "extractor": {
                            "kind": "selector",
                            "mode": "script",
                            "url": "https://www.amazon.jobs/en/search.json?offset=0&result_limit=10",
                            "script": {
                                "runtime": "js",
                                "code": include_str!("../../../crates/scrape-worker/examples/extractors/amazon_extractor.js")
                            }
                        }
                    }
                ],
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "cron": "0 0 9 * * *",
                    "timezone": "UTC"
                },
                "filters": {
                    "matching": {
                        "role_types": [
                            "full_time"
                        ],
                        "workplaces": [
                            "remote",
                            "hybrid",
                            "onsite"
                        ],
                        "countries": [
                            "US"
                        ],
                        "regions": [
                            "US-CA",
                            "US-WA"
                        ],
                        "cities": [],
                        "keywords": {
                            "any": [
                                "Engineer",
                                "Developer",
                                "Distributed",
                                "Platform",
                                "Backend",
                                "Data",
                                "Infrastructure"
                            ],
                            "all": [],
                            "exclude": [],
                            "fields": [
                                "title"
                            ]
                        }
                    }
                },
                "retention": {
                    "days": 30,
                    "policy": "untracked_only"
                }
            }),
        ),
    ]
}

pub fn catalog() -> Result<WorkflowTemplateCatalog> {
    let mut templates = Vec::new();
    for (index, (id, alias, name, original)) in definitions().into_iter().enumerate() {
        // Retain strict examples explicitly; never run this operation on saved data.
        let mut broad = original.clone();
        broad["metadata"]["name"] = name.into();
        let keywords = original["filters"]["matching"]["keywords"].clone();
        broad["filters"] = json!({"matching":{"keywords":keywords}});
        let (readiness, description) = match index {
            0..=3 => (TemplateReadiness::Ready, "Title keywords only. Employment, workplace and geography are unrestricted, including unknown evidence. Preview before saving; live availability varies."),
            4 => (TemplateReadiness::NeedsReview, "Static selector example, disabled for review. Google URL resolution and Apple selectors/pagination still need verification; this is not verified full-site ingestion."),
            _ => (TemplateReadiness::SetupRequired, "Bundled script protocol 2 example, disabled. Select an absolute Node.js executable for each source, save, review and trust each script, then enable explicitly. Pagination coverage remains incomplete."),
        };
        if index >= 4 {
            broad["metadata"]["enabled"] = false.into();
            for source in broad["sources"].as_array_mut().unwrap() {
                source["enabled"] = false.into();
            }
        }
        let broad = workflow::validate(broad, "UTC")?;
        templates.push(WorkflowTemplate {
            extractors: extractors(&broad)?,
            id: id.into(),
            name: name.into(),
            description: description.into(),
            readiness,
            seed: true,
            legacy_identifier: alias.into(),
            spec: broad,
        });
        if index < 4 {
            let mut strict = original;
            let name = format!(
                "{} — Restrictive role/geography example",
                strict["metadata"]["company"].as_str().unwrap()
            );
            strict["metadata"]["name"] = name.clone().into();
            // Examples require an explicit choice before scheduling.
            strict["metadata"]["enabled"] = false.into();
            let strict = workflow::validate(strict, "UTC")?;
            templates.push(WorkflowTemplate {
                extractors: extractors(&strict)?,
                id: format!("{id}-restrictive"), name,
                description: "Restrictive example: selected employment, workplace and geography must all match. Missing evidence may produce zero matches; inspect Preview diagnostics. Not added by Load test workflows.".into(),
                readiness: TemplateReadiness::RestrictiveExample, seed: false,
                legacy_identifier: alias.into(), spec: strict,
            });
        }
    }
    let mut internship = templates[0].spec.clone();
    internship.metadata.name = "Cloudflare — Internship/location example (Restrictive)".into();
    internship.metadata.enabled = Some(false);
    internship.filters = Some(serde_json::from_value(json!({
        "role_types":["internship"], "include_keywords":["Software","Engineer","Intern"],
        "exclude_keywords":["MBA","Sales","Marketing"],
        "locations":["Remote","San Francisco, CA","Austin, TX"]
    }))?);
    let internship = workflow::validate(serde_json::to_value(internship)?, "UTC")?;
    templates.push(WorkflowTemplate {
        id: "software-engineering-internships".into(), name: internship.metadata.name.clone(),
        description: "Restrictive legacy example: internship AND keyword rules AND any listed location text. Zero matches may be valid. Not the default; not seeded automatically.".into(),
        readiness: TemplateReadiness::RestrictiveExample, seed: false, legacy_identifier: "cloudflare".into(),
        extractors: extractors(&internship)?, spec: internship,
    });
    let nvidia = json!({
        "version": 6,
        "metadata": {
            "name": "NVIDIA — Systems, CUDA & AI Engineering (Workday)",
            "company": "NVIDIA",
            "enabled": false
        },
        "sources": [
            {
                "id": "nvidia",
                "label": "NVIDIA — Workday",
                "enabled": true,
                "employer_hint": "NVIDIA",
                "extractor": {
                    "kind": "workday",
                    "url": "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite"
                }
            }
        ],
        "schedule": {
            "cadence": "daily",
            "time": "09:00",
            "cron": "0 0 9 * * *",
            "timezone": "UTC"
        },
        "filters": {
            "matching": {
                "keywords": {
                    "any": [
                        "Systems",
                        "CUDA",
                        "AI",
                        "Kernel",
                        "Compiler",
                        "Performance"
                    ],
                    "all": [],
                    "exclude": [],
                    "fields": [
                        "title"
                    ]
                }
            }
        },
        "retention": {
            "days": 30,
            "policy": "untracked_only"
        }
    });
    let nvidia_spec = workflow::validate(nvidia, "UTC")?;
    templates.push(WorkflowTemplate {
        id: "nvidia-workday-engineering".into(),
        name: "NVIDIA — Systems, CUDA & AI Engineering (Workday)".into(),
        description: "Workday ATS example for NVIDIA career site. Title keywords only. Preview before saving; live availability varies.".into(),
        readiness: TemplateReadiness::Ready,
        seed: false,
        legacy_identifier: "nvidia".into(),
        extractors: extractors(&nvidia_spec)?,
        spec: nvidia_spec,
    });
    let catalog = WorkflowTemplateCatalog {
        version: CATALOG_VERSION,
        templates,
        companies: company_presets()?,
    };
    validate_companies(&catalog)?;
    Ok(catalog)
}

fn company_presets() -> Result<Vec<CompanyPreset>> {
    [
        (
            "cloudflare",
            "Cloudflare",
            "cloudflare-systems-network",
            "cloudflare",
        ),
        (
            "bitwarden",
            "Bitwarden",
            "bitwarden-remote-security",
            "bitwarden",
        ),
        ("linear", "Linear", "linear-product-engineering", "linear"),
        ("sep", "SEP", "sep-midwest-consulting", "sep"),
        ("google", "Google", "bigtech-static-swe", "google"),
        ("apple", "Apple", "bigtech-static-swe", "apple"),
        ("netflix", "Netflix", "streaming-cloud-script", "netflix"),
        ("amazon", "Amazon", "streaming-cloud-script", "amazon"),
        ("nvidia", "NVIDIA", "nvidia-workday-engineering", "nvidia"),
    ]
    .into_iter()
    .map(|(id, name, template_id, source_id)| {
        Ok(CompanyPreset {
            id: id.into(),
            name: name.into(),
            template_id: template_id.into(),
            source_id: source_id.into(),
            logo: crate::db::branding::resolve_curated_logo(id),
        })
    })
    .collect()
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
        let template = catalog
            .templates
            .iter()
            .find(|t| t.id == company.template_id)
            .ok_or_else(|| {
                invalid(format!(
                    "company '{}' references missing template '{}'",
                    company.id, company.template_id
                ))
            })?;
        if template.readiness == TemplateReadiness::RestrictiveExample {
            return Err(invalid(format!(
                "company '{}' references restrictive template '{}'",
                company.id, company.template_id
            )));
        }
        let source_count = template
            .spec
            .sources
            .iter()
            .filter(|s| s.id == company.source_id)
            .count();
        if source_count != 1 {
            return Err(invalid(format!(
                "company '{}' references source '{}' in template '{}' ({source_count} matches)",
                company.id, company.source_id, company.template_id
            )));
        }
        if let Some(logo) = &company.logo {
            logo.validate()
                .map_err(|error| invalid(format!("company '{}' logo: {error}", company.id)))?;
            if logo.status != scrape_core::branding::LogoStatus::Verified
                || !logo
                    .asset
                    .as_deref()
                    .is_some_and(|a| a.starts_with("data:image/webp;base64,"))
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
        .filter(|t| t.seed)
        .map(|template| {
            let mut input = workflow::source_input(&template.spec, "UTC")?;
            // Preserve legacy seed aliases solely for idempotent recognition.
            input.identifier = template.legacy_identifier;
            Ok(input)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn companies_reference_existing_sources_without_granting_readiness() {
        let catalog = catalog().unwrap();
        assert_eq!(catalog.version, 2);
        assert_eq!(
            catalog
                .companies
                .iter()
                .map(|c| c.id.as_str())
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
        validate_companies(&catalog).unwrap();
        for company in &catalog.companies {
            let template = catalog
                .templates
                .iter()
                .find(|t| t.id == company.template_id)
                .unwrap();
            assert_ne!(template.readiness, TemplateReadiness::RestrictiveExample);
            if matches!(company.id.as_str(), "google" | "apple") {
                assert_eq!(template.readiness, TemplateReadiness::NeedsReview);
                assert_eq!(template.spec.metadata.enabled, Some(false));
            }
            if matches!(company.id.as_str(), "netflix" | "amazon") {
                assert_eq!(template.readiness, TemplateReadiness::SetupRequired);
                assert_eq!(template.spec.metadata.enabled, Some(false));
            }
            if company.id == "nvidia" {
                assert_eq!(template.spec.metadata.enabled, Some(false));
            }
            assert!(company.logo.is_some());
        }
    }

    #[test]
    fn company_validation_reports_ids_and_broken_references_deterministically() {
        let original = catalog().unwrap();
        let assert_invalid = |catalog: &WorkflowTemplateCatalog, expected: &str| {
            let first = validate_companies(catalog).unwrap_err().to_string();
            assert!(first.contains(expected), "{first}");
            assert_eq!(first, validate_companies(catalog).unwrap_err().to_string());
        };
        let mut broken = original.clone();
        broken.companies[0].id.clear();
        assert_invalid(&broken, "invalid company ID ''");
        let mut broken = original.clone();
        broken.companies.push(broken.companies[0].clone());
        assert_invalid(&broken, "duplicate company ID 'cloudflare'");
        let mut broken = original.clone();
        broken.companies[0].template_id = "missing-template".into();
        assert_invalid(
            &broken,
            "company 'cloudflare' references missing template 'missing-template'",
        );
        let mut broken = original.clone();
        broken.companies[0].source_id = "missing-source".into();
        assert_invalid(
            &broken,
            "company 'cloudflare' references source 'missing-source'",
        );
        let mut broken = original.clone();
        broken.companies[0].template_id = "software-engineering-internships".into();
        assert_invalid(
            &broken,
            "references restrictive template 'software-engineering-internships'",
        );
        let mut broken = original.clone();
        broken.templates.push(broken.templates[0].clone());
        assert_invalid(
            &broken,
            "duplicate template ID 'cloudflare-systems-network'",
        );
        let mut broken = original.clone();
        broken.templates[0].id.clear();
        assert_invalid(&broken, "missing template ID");
        let mut broken = original.clone();
        let duplicate = broken.templates[0].spec.sources[0].clone();
        broken.templates[0].spec.sources.push(duplicate);
        assert_invalid(
            &broken,
            "source 'cloudflare' in template 'cloudflare-systems-network' (2 matches)",
        );
        let mut broken = original;
        broken.companies[0].logo.as_mut().unwrap().evidence = None;
        assert_invalid(
            &broken,
            "company 'cloudflare' logo: Verified logo lacks source evidence",
        );
    }

    #[test]
    fn catalog_projection_and_execution_configs_are_canonical() {
        let catalog = catalog().unwrap();
        let generated: Value = serde_json::from_str(include_str!(
            "../../../src/generated/workflowTemplates.json"
        ))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&catalog).unwrap(),
            generated,
            "Run npm run gen:templates; do not edit generated catalogs"
        );
        assert_eq!(catalog.templates.iter().filter(|t| t.seed).count(), 6);
        assert_eq!(
            catalog
                .templates
                .iter()
                .filter(|t| t.readiness == TemplateReadiness::RestrictiveExample)
                .count(),
            5
        );
        for template in &catalog.templates {
            assert_eq!(
                workflow::validate(serde_json::to_value(&template.spec).unwrap(), "UTC").unwrap(),
                template.spec
            );
            assert_eq!(extractors(&template.spec).unwrap(), template.extractors);
            if template.seed {
                let filters = template.spec.filters.as_ref().unwrap();
                assert!(filters.role_types.as_ref().is_none_or(Vec::is_empty));
                let rules = filters.matching.as_ref().unwrap();
                assert!(rules.role_types.is_empty() && rules.workplaces.is_empty());
                assert!(
                    rules.countries.is_empty()
                        && rules.regions.is_empty()
                        && rules.cities.is_empty()
                );
            }
            if template.readiness == TemplateReadiness::SetupRequired {
                assert_eq!(template.spec.metadata.enabled, Some(false));
                for source in &template.spec.sources {
                    assert!(!source.enabled);
                    let script = source.extractor.script.as_ref().unwrap();
                    assert!(script.code.as_ref().unwrap().contains("version: 2"));
                    assert!(script.path.is_none() && script.interpreter.is_none());
                    let environment =
                        super::super::script_trust::check_script_environment(script).unwrap();
                    assert!(!environment.available);
                    assert!(environment.message.unwrap().contains("absolute path"));
                }
            }
        }
    }
}
