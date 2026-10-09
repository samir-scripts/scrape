use crate::client::{ClientError, HttpClient};
use scrape_core::{RoleType, WorkerEvent, WorkerSource};
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub async fn scrape_jsonld(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let base_url = Url::parse(&source.identifier)?;
    let html_text = client.fetch_text(base_url.as_str(), true).await?;

    let mut all_events = extract_job_postings_from_html(&html_text, base_url.as_str(), source);

    // Check if follow_links is configured
    let follow_links_selector = source.config.get("follow_links").and_then(|v| v.as_str());

    if let Some(sel_str) = follow_links_selector {
        let max_pages = source
            .config
            .get("max_pages")
            .and_then(|v| v.as_u64())
            .unwrap_or(25) as usize;

        let links = find_links(&html_text, &base_url, sel_str, max_pages);
        for link in links {
            if let Ok(page_html) = client.fetch_text(link.as_str(), true).await {
                let sub_events = extract_job_postings_from_html(&page_html, link.as_str(), source);
                all_events.extend(sub_events);
            }
        }
    }

    Ok(all_events)
}

pub fn extract_job_postings_from_html(
    html_text: &str,
    page_url: &str,
    source: &WorkerSource,
) -> Vec<WorkerEvent> {
    let document = Html::parse_document(html_text);
    let script_selector = Selector::parse(r#"script[type="application/ld+json"]"#).unwrap();

    let mut events = Vec::new();

    for script in document.select(&script_selector) {
        let text = script.text().collect::<String>();
        if let Ok(json_val) = serde_json::from_str::<Value>(&text) {
            extract_job_postings_from_value(&json_val, page_url, source, &mut events);
        }
    }

    events
}

fn extract_job_postings_from_value(
    value: &Value,
    page_url: &str,
    source: &WorkerSource,
    out: &mut Vec<WorkerEvent>,
) {
    match value {
        Value::Object(map) => {
            if is_job_posting(map.get("@type")) {
                if let Some(event) = parse_job_posting_map(map, page_url, source) {
                    out.push(event);
                }
            }
            if let Some(graph) = map.get("@graph") {
                extract_job_postings_from_value(graph, page_url, source, out);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                extract_job_postings_from_value(item, page_url, source, out);
            }
        }
        _ => {}
    }
}

fn is_job_posting(type_val: Option<&Value>) -> bool {
    match type_val {
        Some(Value::String(s)) => s.eq_ignore_ascii_case("JobPosting"),
        Some(Value::Array(arr)) => arr.iter().any(|v| {
            v.as_str()
                .map(|s| s.eq_ignore_ascii_case("JobPosting"))
                .unwrap_or(false)
        }),
        _ => false,
    }
}

fn parse_job_posting_map(
    map: &serde_json::Map<String, Value>,
    page_url: &str,
    source: &WorkerSource,
) -> Option<WorkerEvent> {
    let title = map.get("title").and_then(|v| v.as_str())?.to_string();

    let company = map
        .get("hiringOrganization")
        .and_then(|ho| ho.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or(&source.name)
        .to_string();

    let emp_type = map.get("employmentType").and_then(|et| match et {
        Value::String(s) => Some(s.as_str()),
        Value::Array(arr) => arr.first().and_then(|v| v.as_str()),
        _ => None,
    });
    let role_type = RoleType::infer(&title, emp_type);

    let location = extract_location(map.get("jobLocation"));

    let job_url = map
        .get("url")
        .and_then(|u| u.as_str())
        .unwrap_or(page_url)
        .to_string();

    let external_id = map
        .get("identifier")
        .and_then(|id| {
            if let Some(s) = id.as_str() {
                Some(s.to_string())
            } else {
                id.get("value")
                    .and_then(|val| val.as_str())
                    .map(|v| v.to_string())
            }
        })
        .unwrap_or_else(|| job_url.clone());

    let date_posted = map
        .get("datePosted")
        .and_then(|dp| dp.as_str())
        .map(|s| s.to_string());

    Some(WorkerEvent::Job {
        source_id: source.id.clone(),
        external_id,
        title,
        company,
        role_type,
        location,
        url: job_url,
        date_posted,
    })
}

fn extract_location(loc_val: Option<&Value>) -> String {
    let loc_val = match loc_val {
        Some(v) => v,
        None => return String::new(),
    };

    if let Some(s) = loc_val.as_str() {
        return s.to_string();
    }

    if let Some(arr) = loc_val.as_array() {
        if let Some(first) = arr.first() {
            return extract_location(Some(first));
        }
    }

    if let Some(map) = loc_val.as_object() {
        if let Some(addr) = map.get("address") {
            if let Some(s) = addr.as_str() {
                return s.to_string();
            }
            if let Some(addr_map) = addr.as_object() {
                let locality = addr_map
                    .get("addressLocality")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let region = addr_map
                    .get("addressRegion")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let country = addr_map
                    .get("addressCountry")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let mut parts = Vec::new();
                if !locality.is_empty() {
                    parts.push(locality);
                }
                if !region.is_empty() {
                    parts.push(region);
                }
                if !country.is_empty() {
                    parts.push(country);
                }
                return parts.join(", ");
            }
        }
        if let Some(name) = map.get("name").and_then(|v| v.as_str()) {
            return name.to_string();
        }
    }

    String::new()
}

fn find_links(html_text: &str, base_url: &Url, selector_str: &str, max_links: usize) -> Vec<Url> {
    let document = Html::parse_document(html_text);
    let selector = match Selector::parse(selector_str) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut links = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(base_url.as_str().to_string());

    for elem in document.select(&selector) {
        if links.len() >= max_pages_cap(max_links) {
            break;
        }

        let href = match elem.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        if let Ok(resolved) = base_url.join(href) {
            let s = resolved.as_str().to_string();
            if !visited.contains(&s) {
                visited.insert(s);
                links.push(resolved);
            }
        }
    }

    links
}

fn max_pages_cap(max_links: usize) -> usize {
    if max_links == 0 {
        25
    } else {
        max_links
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::SourceKind;

    #[test]
    fn test_extract_jsonld() {
        let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <script type="application/ld+json">
            {
                "@context": "https://schema.org/",
                "@type": "JobPosting",
                "title": "Data Infrastructure Engineer",
                "hiringOrganization": {
                    "@type": "Organization",
                    "name": "Acme Corp"
                },
                "employmentType": "FULL_TIME",
                "datePosted": "2026-10-05",
                "jobLocation": {
                    "@type": "Place",
                    "address": {
                        "addressLocality": "Austin",
                        "addressRegion": "TX"
                    }
                },
                "url": "https://acme.com/jobs/123"
            }
            </script>
        </head>
        <body></body>
        </html>
        "#;

        let source = WorkerSource {
            id: "src-jsonld".to_string(),
            kind: SourceKind::Jsonld,
            name: "Acme".to_string(),
            identifier: "https://acme.com/careers".to_string(),
            config: serde_json::Value::Null,
        };

        let events = extract_job_postings_from_html(html, "https://acme.com/careers", &source);
        assert_eq!(events.len(), 1);
        if let WorkerEvent::Job {
            title,
            company,
            role_type,
            location,
            url,
            ..
        } = &events[0]
        {
            assert_eq!(title, "Data Infrastructure Engineer");
            assert_eq!(company, "Acme Corp");
            assert_eq!(*role_type, RoleType::FullTime);
            assert_eq!(location, "Austin, TX");
            assert_eq!(url, "https://acme.com/jobs/123");
        } else {
            panic!("Expected Job event");
        }
    }
}
