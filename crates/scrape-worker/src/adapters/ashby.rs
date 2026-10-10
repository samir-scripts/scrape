use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, extract_experience, location as loc_enrich,
    to_plain_text, Salary,
};
use scrape_core::{RoleType, TriState, WorkerEvent, WorkerSource, WorkplaceType};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AshbyResponse {
    #[serde(default)]
    jobs: Vec<AshbyJob>,
}

#[derive(Debug, Deserialize)]
struct AshbyJob {
    id: String,
    title: String,
    #[serde(rename = "jobUrl")]
    job_url: String,
    location: Option<String>,
    #[serde(rename = "secondaryLocations")]
    secondary_locations: Option<Vec<AshbySecondaryLocation>>,
    #[serde(rename = "employmentType")]
    employment_type: Option<String>,
    #[serde(rename = "publishedAt")]
    published_at: Option<String>,
    #[serde(rename = "isRemote")]
    is_remote: Option<bool>,
    #[serde(rename = "workplaceType")]
    workplace_type: Option<String>,
    compensation: Option<AshbyCompensation>,
    #[serde(rename = "descriptionPlain")]
    description_plain: Option<String>,
    #[serde(rename = "descriptionHtml")]
    description_html: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AshbySecondaryLocation {
    location: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AshbyCompensation {
    #[serde(rename = "summaryComponents")]
    summary_components: Option<Vec<AshbyCompensationComponent>>,
}

#[derive(Debug, Deserialize)]
struct AshbyCompensationComponent {
    #[serde(rename = "compensationType")]
    compensation_type: Option<String>,
    #[serde(rename = "minValue")]
    min_value: Option<f64>,
    #[serde(rename = "maxValue")]
    max_value: Option<f64>,
    #[serde(rename = "currencyCode")]
    currency_code: Option<String>,
    interval: Option<String>,
}

pub async fn scrape_ashby(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let url = format!(
        "https://api.ashbyhq.com/posting-api/job-board/{}?includeCompensation=true",
        source.identifier.trim()
    );

    let resp: AshbyResponse = client.fetch_json(&url).await?;
    parse_ashby_jobs(resp.jobs, source)
}

pub fn parse_ashby_payload(
    json: &str,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let resp: AshbyResponse = serde_json::from_str(json)?;
    parse_ashby_jobs(resp.jobs, source)
}

fn parse_ashby_jobs(
    jobs: Vec<AshbyJob>,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::with_capacity(jobs.len());

    for job in jobs {
        let role_type = RoleType::infer(&job.title, job.employment_type.as_deref());

        // Location parsing
        let mut loc_strings = Vec::new();
        if let Some(loc) = &job.location {
            if !loc.trim().is_empty() {
                loc_strings.push(loc.clone());
            }
        }
        if let Some(sec) = &job.secondary_locations {
            for s in sec {
                if let Some(l) = &s.location {
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
        let workplace_type = if job.is_remote == Some(true)
            || job.workplace_type.as_deref() == Some("Remote")
            || locations.iter().any(|l| l.is_remote)
        {
            WorkplaceType::Remote
        } else if job.workplace_type.as_deref() == Some("Hybrid")
            || location_str.to_lowercase().contains("hybrid")
        {
            WorkplaceType::Hybrid
        } else if job.workplace_type.as_deref() == Some("OnSite") || !location_str.is_empty() {
            WorkplaceType::Onsite
        } else {
            WorkplaceType::Unknown
        };

        // Description
        let desc_text = if let Some(dp) = &job.description_plain {
            if !dp.trim().is_empty() {
                Some(dp.chars().take(32768).collect::<String>())
            } else {
                None
            }
        } else {
            job.description_html.as_ref().map(|dh| to_plain_text(dh, 32768))
        };

        // Compensation
        let mut salary = None;
        if let Some(comp) = &job.compensation {
            if let Some(components) = &comp.summary_components {
                let salary_comp = components
                    .iter()
                    .find(|c| c.compensation_type.as_deref() == Some("Salary"))
                    .or_else(|| components.first());

                if let Some(sc) = salary_comp {
                    salary = Salary::from_structured(
                        sc.min_value,
                        sc.max_value,
                        sc.currency_code.as_deref(),
                        sc.interval.as_deref(),
                        None,
                    );
                }
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
        let experience = Some(extract_experience(&job.title, text_for_enrich));
        let is_new_grad = experience
            .as_ref()
            .map(|e| e.is_new_grad)
            .unwrap_or(TriState::Unknown);

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: job.id,
            title: job.title,
            company: source.name.clone(),
            role_type,
            location: location_str,
            url: job.job_url,
            date_posted: job.published_at,
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
    fn test_parse_ashby_json() {
        let json = r#"{
            "jobs": [
                {
                    "id": "ashby-99",
                    "title": "Systems Architect",
                    "jobUrl": "https://jobs.ashbyhq.com/example/ashby-99",
                    "location": "New York, NY",
                    "employmentType": "Contract",
                    "publishedAt": "2026-10-02T10:00:00Z"
                }
            ]
        }"#;

        let resp: AshbyResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jobs.len(), 1);
        let job = &resp.jobs[0];
        assert_eq!(job.id, "ashby-99");
        let role = RoleType::infer(&job.title, job.employment_type.as_deref());
        assert_eq!(role, RoleType::Contract);
    }

    #[test]
    fn test_parse_ashby_enrichment() {
        let json = r#"{
            "jobs": [
                {
                    "id": "ashby-101",
                    "title": "Staff Backend Engineer",
                    "jobUrl": "https://jobs.ashbyhq.com/example/ashby-101",
                    "location": "San Francisco, CA",
                    "secondaryLocations": [
                        { "location": "Seattle, WA" }
                    ],
                    "employmentType": "FullTime",
                    "publishedAt": "2026-10-02T10:00:00Z",
                    "isRemote": false,
                    "workplaceType": "Hybrid",
                    "compensation": {
                        "summaryComponents": [
                            {
                                "compensationType": "Salary",
                                "minValue": 190000.0,
                                "maxValue": 240000.0,
                                "currencyCode": "USD",
                                "interval": "Year"
                            }
                        ]
                    },
                    "descriptionPlain": "US Citizenship and active Top Secret clearance required. No visa sponsorship provided. Requires 7+ years of software engineering experience."
                }
            ]
        }"#;

        let source = WorkerSource {
            id: "src-ashby".to_string(),
            kind: scrape_core::SourceKind::Ashby,
            name: "Anthropic".to_string(),
            identifier: "anthropic".to_string(),
            config: serde_json::Value::Null,
        };

        let events = parse_ashby_payload(json, &source).unwrap();
        assert_eq!(events.len(), 1);

        if let WorkerEvent::Job {
            salary,
            locations,
            sponsorship,
            clearance,
            experience,
            is_new_grad,
            workplace_type,
            ..
        } = &events[0]
        {
            let sal = salary.as_ref().expect("Salary should be extracted");
            assert_eq!(sal.min, Some(190000));
            assert_eq!(sal.max, Some(240000));
            assert_eq!(sal.currency, "USD");

            assert_eq!(locations.len(), 2);
            assert_eq!(locations[0].city.as_deref(), Some("San Francisco"));
            assert_eq!(locations[1].city.as_deref(), Some("Seattle"));

            let spon = sponsorship.as_ref().expect("Sponsorship should be extracted");
            assert_eq!(spon.value, TriState::No);

            let clr = clearance.as_ref().expect("Clearance should be extracted");
            assert_eq!(clr.value, TriState::Yes);

            let exp = experience.as_ref().expect("Experience should be extracted");
            assert_eq!(exp.min_years, Some(7));

            assert_eq!(is_new_grad, &TriState::No);
            assert_eq!(workplace_type, &WorkplaceType::Hybrid);
        } else {
            panic!("Expected WorkerEvent::Job");
        }
    }
}
