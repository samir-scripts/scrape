//! Frozen shipped catalog-v2 definitions, used only for recognition and recorded verification.
//! Never project these descriptors into the production picker or workflow YAML.
use super::template_catalog::{extractors, TemplateReadiness, WorkflowTemplate};
use super::workflow::{self, WorkflowSpec};
use crate::error::Result;
use serde_json::{json, Value};

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

pub fn templates() -> Result<Vec<WorkflowTemplate>> {
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
    Ok(templates)
}

/// Canonical shipped profiles, including the two catalog-v1 NVIDIA projections
/// that remained on disk and the original pre-broadening sample definitions.
/// The replacement service normalizes IDs/timezones and hashes the complete spec.
pub fn recognition_specs() -> Result<Vec<WorkflowSpec>> {
    let mut specs: Vec<_> = templates()?
        .into_iter()
        .map(|template| template.spec)
        .collect();
    for (_, _, _, value) in definitions() {
        specs.push(workflow::validate(value, "UTC")?);
    }
    let mut nvidia = specs
        .iter()
        .find(|spec| spec.metadata.company.as_deref() == Some("NVIDIA"))
        .unwrap()
        .clone();
    nvidia.metadata.enabled = Some(true);
    specs.push(nvidia.clone());
    nvidia.metadata.name = "NVIDIA — Restrictive role/geography example".into();
    nvidia.metadata.enabled = Some(false);
    let matching = nvidia.filters.as_mut().unwrap().matching.as_mut().unwrap();
    matching.role_types = serde_json::from_value(json!(["full_time"]))?;
    matching.workplaces = serde_json::from_value(json!(["hybrid", "onsite", "remote"]))?;
    matching.countries = vec!["US".into()];
    matching.regions = vec!["US-CA".into(), "US-WA".into(), "US-TX".into()];
    matching.cities = vec!["Santa Clara".into(), "Austin".into(), "Redmond".into()];
    specs.push(workflow::validate(serde_json::to_value(nvidia)?, "UTC")?);
    Ok(specs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognition_retains_shipped_examples_and_actual_stale_nvidia_variants() {
        let descriptors = templates().unwrap();
        assert_eq!(descriptors.len(), 12);
        assert_eq!(
            descriptors.iter().filter(|template| template.seed).count(),
            6
        );
        assert_eq!(
            descriptors
                .iter()
                .filter(|template| template.id.ends_with("-restrictive"))
                .count(),
            4
        );
        assert!(descriptors
            .iter()
            .any(|template| template.id == "software-engineering-internships"));
        assert!(descriptors
            .iter()
            .any(|template| template.id == "nvidia-workday-engineering"));
        let recognized = recognition_specs().unwrap();
        assert_eq!(recognized.len(), 20);
        let nvidia: Vec<_> = recognized
            .iter()
            .filter(|spec| spec.metadata.company.as_deref() == Some("NVIDIA"))
            .collect();
        assert_eq!(nvidia.len(), 3);
        assert!(nvidia
            .iter()
            .any(|spec| spec.metadata.enabled == Some(true)));
        let restrictive = nvidia
            .iter()
            .find(|spec| spec.metadata.name == "NVIDIA — Restrictive role/geography example")
            .unwrap();
        let matching = restrictive
            .filters
            .as_ref()
            .unwrap()
            .matching
            .as_ref()
            .unwrap();
        assert_eq!(matching.cities, ["Santa Clara", "Austin", "Redmond"]);
        assert_eq!(matching.regions, ["US-CA", "US-WA", "US-TX"]);
        assert!(recognized
            .iter()
            .any(|spec| spec.metadata.name == "Cloudflare — Software Engineering & Internships"));
        for spec in recognized {
            assert_eq!(
                workflow::validate(serde_json::to_value(&spec).unwrap(), "UTC").unwrap(),
                spec
            );
        }
    }
}
