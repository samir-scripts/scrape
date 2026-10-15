use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, normalize_description_with_format,
    DescriptionFormat, Salary,
};
use scrape_core::{RoleType, TriState, WorkAuthRequirement, WorkerEvent, WorkerSource};
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub async fn scrape_jsonld(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    scrape_jsonld_with(client, source, &mut |event| {
        events.push(event);
        true
    })
    .await?;
    Ok(events)
}

pub async fn scrape_jsonld_with(
    client: &HttpClient,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    let base_url = Url::parse(&source.identifier)?;
    let html_text = client.fetch_text(base_url.as_str(), true).await?;
    if !extract_job_postings_with(&html_text, base_url.as_str(), source, sink) {
        return Ok(());
    }
    if let Some(selector) = source.config.get("follow_links").and_then(|v| v.as_str()) {
        let max_pages = source
            .config
            .get("max_pages")
            .and_then(|v| v.as_u64())
            .unwrap_or(25) as usize;
        Selector::parse(selector).map_err(|e| {
            ClientError::HttpStatus(
                reqwest::StatusCode::BAD_REQUEST,
                format!("Invalid follow_links selector: {e}"),
            )
        })?;
        let links = find_links(&html_text, &base_url, selector, max_pages.saturating_add(1));
        let truncated = links.len() > max_pages;
        for link in links.into_iter().take(max_pages) {
            match client.fetch_text(link.as_str(), true).await {
                Ok(page) => {
                    if !extract_job_postings_with(&page, link.as_str(), source, sink) {
                        return Ok(());
                    }
                }
                Err(error) => {
                    if !sink(WorkerEvent::Error {
                        source_id: source.id.clone(),
                        message: format!("Could not extract detail page {link}: {error}"),
                        retryable: true,
                        invalid_posting: Some(false),
                    }) {
                        return Ok(());
                    }
                }
            }
        }
        if truncated {
            sink(WorkerEvent::Error {
                source_id: source.id.clone(),
                message: format!(
                    "Detail-page limit of {max_pages} reached; results cover fetched pages only"
                ),
                retryable: false,
                invalid_posting: Some(false),
            });
        }
    }
    Ok(())
}

pub fn extract_job_postings_from_html(
    html_text: &str,
    page_url: &str,
    source: &WorkerSource,
) -> Vec<WorkerEvent> {
    let mut events = Vec::new();
    extract_job_postings_with(html_text, page_url, source, &mut |event| {
        events.push(event);
        true
    });
    events
}

fn extract_job_postings_with(
    html_text: &str,
    page_url: &str,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> bool {
    let document = Html::parse_document(html_text);
    let selector = Selector::parse(r#"script[type="application/ld+json"]"#).unwrap();
    for script in document.select(&selector) {
        if let Ok(value) = serde_json::from_str::<Value>(&script.text().collect::<String>()) {
            if !extract_job_postings_from_value(&value, page_url, source, sink) {
                return false;
            }
        }
    }
    true
}

fn extract_job_postings_from_value(
    value: &Value,
    page_url: &str,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> bool {
    match value {
        Value::Object(map) => {
            if is_job_posting(map.get("@type")) {
                if let Some(event) = parse_job_posting_map(map, page_url, source) {
                    if !sink(event) {
                        return false;
                    }
                }
            }
            if let Some(graph) = map.get("@graph") {
                return extract_job_postings_from_value(graph, page_url, source, sink);
            }
        }
        Value::Array(items) => {
            for item in items {
                if !extract_job_postings_from_value(item, page_url, source, sink) {
                    return false;
                }
            }
        }
        _ => {}
    }
    true
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

    let raw_url = map.get("url").and_then(|u| u.as_str());
    let job_url = raw_url
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| Url::parse(page_url).ok()?.join(s).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"));
    let Some(job_url) = job_url else {
        return Some(WorkerEvent::Error {
            invalid_posting: Some(true),
            source_id: source.id.clone(),
            message: format!("Posting '{title}' has no valid HTTP(S) detail URL on {page_url}"),
            retryable: false,
        });
    };
    let job_url = job_url.to_string();

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

    let description_source = map
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let description = description_source
        .as_deref()
        .map(|html| normalize_description_with_format(html, DescriptionFormat::Html, 32768));
    let desc_text = description.as_ref().map(|d| d.plain_text.clone());

    let salary = extract_salary(map, desc_text.as_deref());
    let mut locations = loc_enrich::parse(&location);
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

    let is_telecommute = map
        .get("jobLocationType")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("TELECOMMUTE"))
        .unwrap_or(false);

    let explicit = if is_telecommute { Some("Remote") } else { None };
    let workplace_type = classify_workplace(explicit, &location, desc_text.as_deref());

    let text_for_enrich = desc_text.as_deref().unwrap_or("");

    // Clearance: check explicit requirement first
    let clearance = if let Some(req) = map
        .get("securityClearanceRequirement")
        .and_then(|v| v.as_str())
    {
        let mut clr = classify_clearance(req);
        if clr.value == TriState::Unknown && !text_for_enrich.is_empty() {
            clr = classify_clearance(text_for_enrich);
        }
        Some(clr)
    } else if !text_for_enrich.is_empty() {
        Some(classify_clearance(text_for_enrich))
    } else {
        None
    };

    // Sponsorship: check explicit eligibility first
    let sponsorship = if let Some(req) = map
        .get("eligibilityToWorkRequirement")
        .and_then(|v| v.as_str())
    {
        let mut spon = classify_sponsorship(req);
        if spon.value == TriState::Unknown && !text_for_enrich.is_empty() {
            spon = classify_sponsorship(text_for_enrich);
        }
        Some(spon)
    } else if !text_for_enrich.is_empty() {
        Some(classify_sponsorship(text_for_enrich))
    } else {
        None
    };

    let experience = Some(extract_experience(&title, text_for_enrich));
    let is_new_grad = experience
        .as_ref()
        .map(|e| e.is_new_grad)
        .unwrap_or(TriState::Unknown);

    let work_auth = if let Some(req) = map
        .get("eligibilityToWorkRequirement")
        .and_then(|v| v.as_str())
    {
        let mut wa = classify_work_auth(req);
        if wa.value == WorkAuthRequirement::Unknown && !text_for_enrich.is_empty() {
            wa = classify_work_auth(text_for_enrich);
        }
        Some(wa)
    } else if !text_for_enrich.is_empty() {
        Some(classify_work_auth(text_for_enrich))
    } else {
        None
    };

    let logo_url = map
        .get("hiringOrganization")
        .and_then(|ho| ho.get("logo"))
        .and_then(|l| {
            if let Some(s) = l.as_str() {
                Some(s.to_string())
            } else {
                l.get("url").and_then(|u| u.as_str()).map(|s| s.to_string())
            }
        });
    let company_icon_url = logo_url.or_else(|| {
        Url::parse(page_url).ok().and_then(|u| {
            u.host_str()
                .map(|h| format!("https://www.google.com/s2/favicons?domain={h}&sz=64"))
        })
    });

    Some(WorkerEvent::Job {
        source_id: source.id.clone(),
        external_id,
        title,
        company,
        role_type,
        location,
        url: job_url,
        provider_updated_at: None,
        date_posted,
        salary,
        locations,
        location_data_complete,
        workplace_type,
        sponsorship,
        work_auth,
        clearance,
        experience,
        is_new_grad,
        description_text: desc_text,
        description,
        description_format: description_source.as_ref().map(|_| DescriptionFormat::Html),
        description_source,
        company_icon_url,
    })
}

fn extract_salary(map: &serde_json::Map<String, Value>, desc_text: Option<&str>) -> Option<Salary> {
    if let Some(base_salary) = map.get("baseSalary") {
        if let Some(s) = base_salary.as_str() {
            if let Some(sal) = Salary::from_text(s) {
                return Some(sal);
            }
        }
        if let Some(sal_obj) = base_salary.as_object() {
            let currency = sal_obj.get("currency").and_then(|v| v.as_str());
            let value = sal_obj.get("value");
            if let Some(val_num) = value.and_then(|v| v.as_f64()) {
                return Salary::from_structured(
                    Some(val_num),
                    Some(val_num),
                    currency,
                    Some("year"),
                    None,
                );
            }
            if let Some(val_obj) = value.and_then(|v| v.as_object()) {
                let min = val_obj.get("minValue").and_then(|v| v.as_f64());
                let max = val_obj.get("maxValue").and_then(|v| v.as_f64());
                let unit = val_obj.get("unitText").and_then(|v| v.as_str());
                return Salary::from_structured(min, max, currency, unit, None);
            }
        }
    }
    if let Some(text) = desc_text {
        return Salary::from_text(text);
    }
    None
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
            if !matches!(resolved.scheme(), "http" | "https") {
                continue;
            }
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
