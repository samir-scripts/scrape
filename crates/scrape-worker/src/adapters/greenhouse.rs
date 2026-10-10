use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, extract_experience, location as loc_enrich,
    to_plain_text, Salary,
};
use scrape_core::{RoleType, TriState, WorkerEvent, WorkerSource, WorkplaceType};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GreenhouseResponse {
    #[serde(default)]
    jobs: Vec<GreenhouseJob>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseJob {
    id: i64,
    title: String,
    absolute_url: String,
    location: Option<GreenhouseLocation>,
    updated_at: Option<String>,
    content: Option<String>,
    pay_input_ranges: Option<Vec<GreenhousePayRange>>,
}

#[derive(Debug, Deserialize)]
struct GreenhousePayRange {
    min_cents: Option<i64>,
    max_cents: Option<i64>,
    currency_type: Option<String>,
    title: Option<String>,
    blurb: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseLocation {
    name: Option<String>,
}

pub async fn scrape_greenhouse(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let url = format!(
        "https://boards-api.greenhouse.io/v1/boards/{}/jobs?content=true",
        source.identifier.trim()
    );

    let resp: GreenhouseResponse = client.fetch_json(&url).await?;
    parse_greenhouse_jobs(resp.jobs, source)
}

pub fn parse_greenhouse_payload(
    json: &str,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let resp: GreenhouseResponse = serde_json::from_str(json)?;
    parse_greenhouse_jobs(resp.jobs, source)
}

fn parse_greenhouse_jobs(
    jobs: Vec<GreenhouseJob>,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::with_capacity(jobs.len());

    for job in jobs {
        let role_type = RoleType::infer(&job.title, None);
        let location_str = job.location.and_then(|l| l.name).unwrap_or_default();
        let locations = loc_enrich::parse(&location_str);

        let workplace_type = if locations.iter().any(|l| l.is_remote)
            || location_str.to_lowercase().contains("remote")
        {
            WorkplaceType::Remote
        } else if location_str.to_lowercase().contains("hybrid") {
            WorkplaceType::Hybrid
        } else if !location_str.is_empty() {
            WorkplaceType::Onsite
        } else {
            WorkplaceType::Unknown
        };

        let desc_text = job.content.as_deref().map(|html| to_plain_text(html, 32768));

        let mut salary = None;
        if let Some(ranges) = &job.pay_input_ranges {
            if let Some(r) = ranges.first() {
                let min = r.min_cents.map(|c| (c as f64) / 100.0);
                let max = r.max_cents.map(|c| (c as f64) / 100.0);
                let curr = r.currency_type.as_deref();
                let interval_str = r.title.as_deref().or(r.blurb.as_deref());
                salary = Salary::from_structured(min, max, curr, interval_str, None);
            }
        }
        if salary.is_none() {
            if let Some(text) = &desc_text {
                salary = Salary::from_text(text);
            }
        }

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
            external_id: job.id.to_string(),
            title: job.title,
            company: source.name.clone(),
            role_type,
            location: location_str,
            url: job.absolute_url,
            date_posted: job.updated_at,
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
    fn test_parse_greenhouse_json() {
        let json = r#"{
            "jobs": [
                {
                    "id": 5012345,
                    "title": "Software Engineer, Data Intern",
                    "absolute_url": "https://boards.greenhouse.io/stripe/jobs/5012345",
                    "location": { "name": "Remote" },
                    "updated_at": "2026-10-01T12:00:00Z"
                }
            ]
        }"#;

        let resp: GreenhouseResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jobs.len(), 1);
        let job = &resp.jobs[0];
        assert_eq!(job.id, 5012345);
        assert_eq!(job.title, "Software Engineer, Data Intern");
        let role_type = RoleType::infer(&job.title, None);
        assert_eq!(role_type, RoleType::Internship);
    }

    #[test]
    fn test_parse_greenhouse_enrichment() {
        let json = r#"{
            "jobs": [
                {
                    "id": 999123,
                    "title": "Lead Security Engineer",
                    "absolute_url": "https://boards.greenhouse.io/acme/jobs/999123",
                    "location": { "name": "San Francisco, CA (Hybrid)" },
                    "updated_at": "2026-10-01T12:00:00Z",
                    "content": "Must have an active Secret clearance. We do not provide visa sponsorship. Requires 6+ years experience in Rust.",
                    "pay_input_ranges": [
                        {
                            "min_cents": 16000000,
                            "max_cents": 21000000,
                            "currency": "USD"
                        }
                    ]
                }
            ]
        }"#;

        let source = WorkerSource {
            id: "src-gh".to_string(),
            kind: scrape_core::SourceKind::Greenhouse,
            name: "Acme".to_string(),
            identifier: "acme".to_string(),
            config: serde_json::Value::Null,
        };

        let events = parse_greenhouse_payload(json, &source).unwrap();
        assert_eq!(events.len(), 1);

        if let WorkerEvent::Job {
            salary,
            sponsorship,
            clearance,
            experience,
            is_new_grad,
            workplace_type,
            ..
        } = &events[0]
        {
            let sal = salary.as_ref().expect("Salary should be extracted from pay_input_ranges");
            assert_eq!(sal.min, Some(160000));
            assert_eq!(sal.max, Some(210000));
            assert_eq!(sal.currency, "USD");

            let spon = sponsorship.as_ref().expect("Sponsorship should be extracted");
            assert_eq!(spon.value, TriState::No);

            let clr = clearance.as_ref().expect("Clearance should be extracted");
            assert_eq!(clr.value, TriState::Yes);

            let exp = experience.as_ref().expect("Experience should be extracted");
            assert_eq!(exp.min_years, Some(6));

            assert_eq!(is_new_grad, &TriState::No);
            assert_eq!(workplace_type, &WorkplaceType::Hybrid);
        } else {
            panic!("Expected WorkerEvent::Job");
        }
    }
}
