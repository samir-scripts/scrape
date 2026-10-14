use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, normalize_description_with_format,
    DescriptionFormat, Salary,
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
    updated_at: Option<String>,
    content: Option<String>,
    company_name: Option<String>,
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
    scrape_greenhouse_at(client, source, "https://boards-api.greenhouse.io").await
}

async fn scrape_greenhouse_at(
    client: &HttpClient,
    source: &WorkerSource,
    base: &str,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let board_url = format!("{base}/v1/boards/{}/jobs", source.identifier.trim());
    let mut resp: GreenhouseResponse = client
        .fetch_json(&format!("{board_url}?content=true"))
        .await?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
    let mut diagnostics = vec![];
    for (index, job) in resp.jobs.iter_mut().enumerate() {
        if index >= 100 || tokio::time::Instant::now() >= deadline {
            diagnostics.push(WorkerEvent::Error { source_id: source.id.clone(), message: "Greenhouse detail enrichment reached its bound; remaining posting dates and structured pay are unknown".into(), retryable: true, invalid_posting: Some(false) });
            break;
        }
        let url = format!("{board_url}/{}?pay_transparency=true", job.id);
        match tokio::time::timeout_at(deadline, client.fetch_json_cached::<GreenhouseJob>(&url))
            .await
        {
            Ok(Ok(detail)) if detail.id == job.id => {
                // Use provider fields without constructing a destination or publication date.
                job.title = detail.title;
                job.absolute_url = detail.absolute_url;
                if detail.location.is_some() {
                    job.location = detail.location;
                }
                if detail.offices.is_some() {
                    job.offices = detail.offices;
                }
                job.first_published = detail.first_published;
                job.updated_at = detail.updated_at;
                job.pay_input_ranges = detail.pay_input_ranges;
                job.company_name = detail.company_name;
                if detail.content.is_some() {
                    job.content = detail.content;
                }
            }
            result => diagnostics.push(WorkerEvent::Error {
                source_id: source.id.clone(),
                message: format!(
                    "Greenhouse detail {} unavailable or has a mismatched ID: {result:?}",
                    job.id
                ),
                retryable: true,
                invalid_posting: Some(false),
            }),
        }
    }
    let mut events = parse_greenhouse_jobs(resp.jobs, source)?;
    events.extend(diagnostics);
    Ok(events)
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
                    .filter_map(|o| o.location.as_deref().or(o.name.as_deref()))
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

        let effective_location_str = if is_arrangement_keyword
            && !raw_location_name.eq_ignore_ascii_case("Remote")
            && !offices_str.is_empty()
        {
            offices_str
        } else if !raw_location_name.is_empty() {
            raw_location_name.clone()
        } else {
            offices_str
        };

        let mut locations = loc_enrich::parse(&effective_location_str);

        let description_source = job.content.clone();
        let description = description_source
            .as_deref()
            .map(|html| normalize_description_with_format(html, DescriptionFormat::Html, 32768));
        let desc_text = description.as_ref().map(|d| d.plain_text.clone());
        let section_locations = description
            .as_ref()
            .map(loc_enrich::from_description_sections);
        let location_data_complete = loc_enrich::has_geographic_value(&locations)
            || section_locations
                .as_ref()
                .is_some_and(|evidence| evidence.section_found);
        if let Some(evidence) = section_locations {
            locations.extend(evidence.locations);
        }

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
            company: job.company_name.unwrap_or_else(|| source.name.clone()),
            role_type,
            location: effective_location_str,
            url,
            provider_updated_at: job.updated_at,
            date_posted: job.first_published,
            salary,
            locations,
            location_data_complete,
            workplace_type,
            sponsorship,
            clearance,
            work_auth,
            experience,
            is_new_grad,
            description_text: desc_text,
            description,
            description_format: description_source.as_ref().map(|_| DescriptionFormat::Html),
            description_source,
            company_icon_url,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::WorkplaceType;

    #[tokio::test]
    async fn detail_fields_and_cache_preserve_provider_provenance() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let mut requests = 0;
            while requests < 3 && std::time::Instant::now() < deadline {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = [0; 4096];
                    let count = stream.read(&mut buffer).unwrap();
                    let request = String::from_utf8_lossy(&buffer[..count]);
                    let body = if request
                        .starts_with("GET /v1/boards/acme/jobs/42?pay_transparency=true ")
                    {
                        r#"{"id":42,"title":"Actual Engineer","company_name":"Actual Company","absolute_url":"https://careers.example.com/apply?gh_jid=42","first_published":"2026-01-02T09:00:00Z","updated_at":"2026-10-12T15:00:00Z","pay_input_ranges":[{"min_cents":5025,"max_cents":6075,"currency_type":"USD","title":"Hourly compensation"},{"min_cents":6500,"currency_type":"CAD","title":"Hourly compensation Canada"}]}"#
                    } else {
                        assert!(request.starts_with("GET /v1/boards/acme/jobs?content=true "));
                        r#"{"jobs":[{"id":42,"title":"Older title","absolute_url":"https://boards.greenhouse.io/acme/jobs/42","updated_at":"2026-10-12T15:00:00Z"}]}"#
                    };
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                    requests += 1;
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            assert_eq!(requests, 3, "the second detail request must use the cache");
        });
        let source = WorkerSource {
            id: "workflow".into(),
            name: "Workflow label".into(),
            kind: scrape_core::SourceKind::Greenhouse,
            identifier: "acme".into(),
            config: serde_json::json!({}),
        };
        let client = HttpClient::new(3, 0);
        for _ in 0..2 {
            let events = scrape_greenhouse_at(&client, &source, &format!("http://{address}"))
                .await
                .unwrap();
            match &events[0] {
                WorkerEvent::Job {
                    external_id,
                    title,
                    company,
                    url,
                    date_posted,
                    provider_updated_at,
                    salary,
                    ..
                } => {
                    assert_eq!(external_id, "42");
                    assert_eq!(title, "Actual Engineer");
                    assert_eq!(company, "Actual Company");
                    assert_eq!(url, "https://careers.example.com/apply?gh_jid=42");
                    assert_eq!(date_posted.as_deref(), Some("2026-01-02T09:00:00Z"));
                    assert_eq!(provider_updated_at.as_deref(), Some("2026-10-12T15:00:00Z"));
                    let ranges = &salary.as_ref().unwrap().ranges;
                    assert_eq!(ranges.len(), 2);
                    assert_eq!(ranges[0].min_minor, Some(5025));
                    assert_eq!(ranges[0].interval, Some(scrape_core::SalaryInterval::Hour));
                    assert_eq!(ranges[1].currency.as_deref(), Some("CAD"));
                }
                _ => panic!("Expected actual extracted posting"),
            }
        }
        server.join().unwrap();
    }

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
    fn remote_job_uses_only_labeled_eligible_locations_from_its_description() {
        let source = WorkerSource {
            id: "workflow".into(),
            kind: scrape_core::SourceKind::Greenhouse,
            name: "Cloudflare".into(),
            identifier: "cloudflare".into(),
            config: serde_json::Value::Null,
        };
        // Authored regression payload based on the reported behavior. The raw
        // API response for the expired posting is unavailable; this is not a
        // captured Cloudflare response.
        let payload = serde_json::json!({"jobs": [
            {
                "id": 8024849,
                "title": "Senior Principal, Sales Compensation Design & Strategy",
                "company_name": "Cloudflare",
                "absolute_url": "https://careers.example.test/jobs/8024849",
                "location": {"name": "Remote"},
                "content": "<h3>Available Locations</h3><ul><li>US</li></ul><h3>About Cloudflare</h3><p>We operate across the United States.</p>"
            },
            {
                "id": 8024850,
                "title": "Remote Research Engineer",
                "company_name": "Cloudflare",
                "absolute_url": "https://careers.example.test/jobs/8024850",
                "location": {"name": "Remote"},
                "content": "<h3>About Cloudflare</h3><p>We operate across the United States.</p>"
            }
        ]}).to_string();

        let events = parse_greenhouse_payload(&payload, &source).unwrap();
        let WorkerEvent::Job {
            workplace_type,
            locations,
            ..
        } = &events[0]
        else {
            panic!("Expected first posting");
        };
        assert_eq!(*workplace_type, WorkplaceType::Remote);
        assert!(locations
            .iter()
            .any(|location| location.country_code.as_deref() == Some("US")));

        let WorkerEvent::Job { locations, .. } = &events[1] else {
            panic!("Expected second posting");
        };
        assert!(!locations
            .iter()
            .any(|location| location.country_code.as_deref() == Some("US")));
    }

    #[test]
    fn remote_jobs_do_not_inherit_office_address_as_eligibility() {
        let source = WorkerSource {
            id: "workflow".into(),
            kind: scrape_core::SourceKind::Greenhouse,
            name: "Fixture Co".into(),
            identifier: "fixture".into(),
            config: serde_json::Value::Null,
        };
        let payload = serde_json::json!({"jobs": [
            {
                "id": 77,
                "title": "Remote Engineer",
                "absolute_url": "https://jobs.example.test/77",
                "location": {"name": "Remote"},
                "offices": [{"name": "United States HQ", "location": "Austin, TX"}]
            },
            {
                "id": 78,
                "title": "Hybrid Engineer",
                "absolute_url": "https://jobs.example.test/78",
                "location": {"name": "Hybrid"},
                "offices": [{"name": "Bay Area HQ", "location": "San Francisco, CA"}]
            }
        ]})
        .to_string();
        let events = parse_greenhouse_payload(&payload, &source).unwrap();
        let WorkerEvent::Job {
            location,
            locations,
            location_data_complete,
            workplace_type,
            ..
        } = &events[0]
        else {
            panic!("Expected posting");
        };
        assert_eq!(location, "Remote");
        assert_eq!(*workplace_type, WorkplaceType::Remote);
        assert!(!location_data_complete);
        assert!(locations
            .iter()
            .all(|location| location.country_code.is_none()));

        let WorkerEvent::Job {
            location,
            locations,
            location_data_complete,
            workplace_type,
            ..
        } = &events[1]
        else {
            panic!("Expected posting");
        };
        assert_eq!(location, "San Francisco, CA");
        assert_eq!(*workplace_type, WorkplaceType::Hybrid);
        assert!(*location_data_complete);
        assert_eq!(locations[0].city.as_deref(), Some("San Francisco"));
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
