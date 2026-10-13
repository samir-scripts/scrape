use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, to_plain_text, Salary,
};
use scrape_core::{RoleType, TriState, WorkerEvent, WorkerSource};
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
    offices: Option<Vec<GreenhouseOffice>>,
    first_published: Option<String>,
    content: Option<String>,
    pay_input_ranges: Option<Vec<GreenhousePayRange>>,
}

#[derive(Debug, Deserialize)]
struct GreenhouseOffice {
    name: Option<String>,
    location: Option<String>,
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
        let raw_location_name = job.location.and_then(|l| l.name).unwrap_or_default();
        let offices_str = job
            .offices
            .as_ref()
            .map(|offs| {
                offs.iter()
                    .filter_map(|o| o.name.as_deref().or(o.location.as_deref()))
                    .filter(|s| !s.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();

        let is_arrangement_keyword = raw_location_name.eq_ignore_ascii_case("In-Office")
            || raw_location_name.eq_ignore_ascii_case("In Office")
            || raw_location_name.eq_ignore_ascii_case("On-Site")
            || raw_location_name.eq_ignore_ascii_case("OnSite")
            || raw_location_name.eq_ignore_ascii_case("Hybrid")
            || raw_location_name.eq_ignore_ascii_case("Distributed")
            || raw_location_name.eq_ignore_ascii_case("Remote");

        let effective_location_str = if is_arrangement_keyword && !offices_str.is_empty() {
            offices_str
        } else if !raw_location_name.is_empty() {
            raw_location_name.clone()
        } else {
            offices_str
        };

        let locations = loc_enrich::parse(&effective_location_str);

        let desc_text = job
            .content
            .as_deref()
            .map(|html| to_plain_text(html, 32768));

        let workplace_type = classify_workplace(
            Some(&raw_location_name),
            &effective_location_str,
            desc_text.as_deref(),
        );

        let mut salary = None;
        if let Some(ranges) = &job.pay_input_ranges {
            let extracted = ranges
                .iter()
                .flat_map(|r| {
                    let evidence = format!(
                        "{} {}",
                        r.title.as_deref().unwrap_or(""),
                        r.blurb.as_deref().unwrap_or("")
                    );
                    Salary::from_structured(
                        r.min_cents.map(|c| c as f64 / 100.0),
                        r.max_cents.map(|c| c as f64 / 100.0),
                        r.currency_type.as_deref(),
                        Some(&evidence),
                        Some(&evidence),
                    )
                    .map(|s| {
                        s.ranges
                            .into_iter()
                            .map(|mut range| {
                                range.location = r.title.clone();
                                range
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
                })
                .collect();
            salary = Salary::from_ranges(extracted);
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
        let work_auth = if !text_for_enrich.is_empty() {
            Some(classify_work_auth(text_for_enrich))
        } else {
            None
        };
        let experience = Some(extract_experience(&job.title, text_for_enrich));
        let is_new_grad = experience
            .as_ref()
            .map(|e| e.is_new_grad)
            .unwrap_or(TriState::Unknown);

        let company_icon_url = None;
        let url = job.absolute_url;

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: job.id.to_string(),
            title: job.title,
            company: source.name.clone(),
            role_type,
            location: effective_location_str,
            url,
            date_posted: job.first_published,
            salary,
            locations,
            workplace_type,
            sponsorship,
            clearance,
            work_auth,
            experience,
            is_new_grad,
            description_text: desc_text,
            company_icon_url,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::WorkplaceType;

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
    fn custom_destination_and_publication_are_preserved() {
        let source = WorkerSource {
            id: "workflow".into(),
            kind: scrape_core::SourceKind::Greenhouse,
            name: "Acme".into(),
            identifier: "acme".into(),
            config: serde_json::Value::Null,
        };
        for published in [None, Some("2026-09-01T00:00:00Z")] {
            let payload = serde_json::json!({"jobs": [{
                "id": 42, "title": "Engineer", "absolute_url": "https://careers.acme.test/detail?gh_jid=42",
                "updated_at": "2026-10-01T00:00:00Z", "first_published": published,
                "location": {"name": "Remote"}
            }]}).to_string();
            let events = parse_greenhouse_payload(&payload, &source).unwrap();
            match &events[0] {
                WorkerEvent::Job {
                    external_id,
                    title,
                    url,
                    date_posted,
                    company_icon_url,
                    ..
                } => {
                    assert_eq!(external_id, "42");
                    assert_eq!(title, "Engineer");
                    assert_eq!(url, "https://careers.acme.test/detail?gh_jid=42");
                    assert_eq!(date_posted.as_deref(), published);
                    assert!(company_icon_url.is_none());
                }
                _ => panic!("Expected posting"),
            }
        }
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
                            "currency_type": "USD", "title": "Annual base salary"
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
            url,
            salary,
            sponsorship,
            clearance,
            experience,
            is_new_grad,
            workplace_type,
            ..
        } = &events[0]
        {
            assert_eq!(url, "https://boards.greenhouse.io/acme/jobs/999123");
            let sal = salary
                .as_ref()
                .expect("Salary should be extracted from pay_input_ranges");
            assert_eq!(sal.min, Some(160000));
            assert_eq!(sal.max, Some(210000));
            assert_eq!(sal.currency, "USD");

            let spon = sponsorship
                .as_ref()
                .expect("Sponsorship should be extracted");
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
