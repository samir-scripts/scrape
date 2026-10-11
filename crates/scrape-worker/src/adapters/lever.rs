use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, SecondsFormat};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, extract_experience, location as loc_enrich, Salary,
};
use scrape_core::{RoleType, TriState, WorkerEvent, WorkerSource, WorkplaceType};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LeverPosting {
    id: String,
    text: String,
    #[serde(rename = "hostedUrl")]
    hosted_url: String,
    #[serde(rename = "createdAt")]
    created_at: Option<i64>,
    categories: Option<LeverCategories>,
    #[serde(rename = "salaryRange")]
    salary_range: Option<LeverSalaryRange>,
    #[serde(rename = "salaryDescriptionPlain")]
    salary_description_plain: Option<String>,
    #[serde(rename = "workplaceType")]
    workplace_type: Option<String>,
    #[serde(rename = "descriptionPlain")]
    description_plain: Option<String>,
    #[serde(rename = "additionalPlain")]
    additional_plain: Option<String>,
    lists: Option<Vec<LeverList>>,
}

#[derive(Debug, Deserialize)]
struct LeverList {
    text: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LeverSalaryRange {
    min: Option<f64>,
    max: Option<f64>,
    currency: Option<String>,
    interval: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LeverCategories {
    location: Option<String>,
    commitment: Option<String>,
    #[serde(rename = "allLocations")]
    all_locations: Option<Vec<String>>,
}

pub async fn scrape_lever(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let url = format!(
        "https://api.lever.co/v0/postings/{}?mode=json",
        source.identifier.trim()
    );

    let postings: Vec<LeverPosting> = client.fetch_json(&url).await?;
    parse_lever_postings(postings, source)
}

pub fn parse_lever_payload(
    json: &str,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let postings: Vec<LeverPosting> = serde_json::from_str(json)?;
    parse_lever_postings(postings, source)
}

fn parse_lever_postings(
    postings: Vec<LeverPosting>,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::with_capacity(postings.len());

    for posting in postings {
        let commitment = posting
            .categories
            .as_ref()
            .and_then(|c| c.commitment.as_deref());
        let role_type = RoleType::infer(&posting.text, commitment);

        // Build combined location string
        let mut loc_strings = Vec::new();
        if let Some(cat) = &posting.categories {
            if let Some(loc) = &cat.location {
                if !loc.trim().is_empty() {
                    loc_strings.push(loc.clone());
                }
            }
            if let Some(all) = &cat.all_locations {
                for l in all {
                    if !loc_strings.contains(l) && !l.trim().is_empty() {
                        loc_strings.push(l.clone());
                    }
                }
            }
        }
        let location_str = if loc_strings.is_empty() {
            String::new()
        } else {
            loc_strings.join("; ")
        };
        let locations = loc_enrich::parse(&location_str);

        // Workplace type
        let workplace_type = match posting.workplace_type.as_deref() {
            Some("remote") => WorkplaceType::Remote,
            Some("hybrid") => WorkplaceType::Hybrid,
            Some("on-site") => WorkplaceType::Onsite,
            _ => {
                if locations.iter().any(|l| l.is_remote)
                    || location_str.to_lowercase().contains("remote")
                {
                    WorkplaceType::Remote
                } else if location_str.to_lowercase().contains("hybrid") {
                    WorkplaceType::Hybrid
                } else if !location_str.is_empty() {
                    WorkplaceType::Onsite
                } else {
                    WorkplaceType::Unknown
                }
            }
        };

        // Combine description texts
        let mut desc_parts = Vec::new();
        if let Some(dp) = &posting.description_plain {
            if !dp.trim().is_empty() {
                desc_parts.push(dp.clone());
            }
        }
        if let Some(lists) = &posting.lists {
            for l in lists {
                if let Some(t) = &l.text {
                    desc_parts.push(t.clone());
                }
                if let Some(c) = &l.content {
                    desc_parts.push(c.clone());
                }
            }
        }
        if let Some(ap) = &posting.additional_plain {
            if !ap.trim().is_empty() {
                desc_parts.push(ap.clone());
            }
        }
        let full_description = desc_parts.join("\n\n");
        let desc_text = if full_description.is_empty() {
            None
        } else {
            Some(full_description.chars().take(32768).collect::<String>())
        };

        // Salary
        let mut salary = None;
        if let Some(sr) = &posting.salary_range {
            salary = Salary::from_structured(
                sr.min,
                sr.max,
                sr.currency.as_deref(),
                sr.interval.as_deref(),
                posting.salary_description_plain.as_deref(),
            );
        }
        if salary.is_none() {
            if let Some(desc) = &posting.salary_description_plain {
                salary = Salary::from_text(desc);
            }
        }
        if salary.is_none() {
            if let Some(text) = &desc_text {
                salary = Salary::from_text(text);
            }
        }

        // Eligibility and experience
        let text_for_enrich = desc_text.as_deref().unwrap_or("");
        let sponsorship = if !text_for_enrich.is_empty() {
            Some(classify_sponsorship(text_for_enrich))
        } else {
            None
        };
        let clearance = if !text_for_enrich.is_empty() {
            Some(classify_clearance(text_for_enrich))
        } else {
            None
        };
        let experience = Some(extract_experience(&posting.text, text_for_enrich));
        let is_new_grad = experience
            .as_ref()
            .map(|e| e.is_new_grad)
            .unwrap_or(TriState::Unknown);

        let date_posted = posting.created_at.and_then(|ms| {
            DateTime::from_timestamp_millis(ms)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
        });

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: posting.id,
            title: posting.text,
            company: source.name.clone(),
            role_type,
            location: location_str,
            url: posting.hosted_url,
            date_posted,
            salary,
            locations,
            workplace_type,
            sponsorship,
            clearance,
            experience,
            is_new_grad,
            description_text: desc_text,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lever_json() {
        let json = r#"[
            {
                "id": "lever-123",
                "text": "Frontend Engineer",
                "hostedUrl": "https://jobs.lever.co/figma/lever-123",
                "createdAt": 1727740800000,
                "categories": {
                    "location": "San Francisco",
                    "commitment": "Full Time"
                }
            }
        ]"#;

        let postings: Vec<LeverPosting> = serde_json::from_str(json).unwrap();
        assert_eq!(postings.len(), 1);
        let p = &postings[0];
        assert_eq!(p.id, "lever-123");
        assert_eq!(p.text, "Frontend Engineer");
        let dt = p.created_at.and_then(|ms| {
            DateTime::from_timestamp_millis(ms)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
        });
        assert!(dt.is_some());
    }

    #[test]
    fn test_parse_lever_enrichment() {
        let json = r#"[
            {
                "id": "lever-456",
                "text": "Data Platform Engineer",
                "hostedUrl": "https://jobs.lever.co/stripe/lever-456",
                "createdAt": 1727740800000,
                "workplaceType": "hybrid",
                "categories": {
                    "location": "New York, NY",
                    "commitment": "Full Time",
                    "allLocations": ["New York, NY", "Toronto, ON"]
                },
                "salaryRange": {
                    "min": 175000,
                    "max": 225000,
                    "currency": "USD",
                    "interval": "per-year-salary"
                },
                "descriptionPlain": "We sponsor H1B visas. Requires 3-5 years experience with distributed systems."
            }
        ]"#;

        let source = WorkerSource {
            id: "src-lever".to_string(),
            kind: scrape_core::SourceKind::Lever,
            name: "Stripe".to_string(),
            identifier: "stripe".to_string(),
            config: serde_json::Value::Null,
        };

        let events = parse_lever_payload(json, &source).unwrap();
        assert_eq!(events.len(), 1);

        if let WorkerEvent::Job {
            salary,
            locations,
            sponsorship,
            experience,
            workplace_type,
            ..
        } = &events[0]
        {
            let sal = salary.as_ref().expect("Salary should be extracted");
            assert_eq!(sal.min, Some(175000));
            assert_eq!(sal.max, Some(225000));
            assert_eq!(sal.currency, "USD");

            assert_eq!(locations.len(), 2);
            assert_eq!(locations[0].city.as_deref(), Some("New York"));
            assert_eq!(locations[1].city.as_deref(), Some("Toronto"));

            let spon = sponsorship
                .as_ref()
                .expect("Sponsorship should be extracted");
            assert_eq!(spon.value, TriState::Yes);

            let exp = experience.as_ref().expect("Experience should be extracted");
            assert_eq!(exp.min_years, Some(3));
            assert_eq!(exp.max_years, Some(5));

            assert_eq!(workplace_type, &WorkplaceType::Hybrid);
        } else {
            panic!("Expected WorkerEvent::Job");
        }
    }
}
