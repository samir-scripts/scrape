use crate::client::{ClientError, HttpClient};
use scrape_core::{RoleType, WorkerEvent, WorkerSource};
use scraper::{Html, Selector};
use url::Url;

pub async fn scrape_selector(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let base_url = Url::parse(&source.identifier)?;
    let html_text = client.fetch_text(base_url.as_str(), true).await?;

    let events = extract_jobs_with_selectors(&html_text, &base_url, source)?;
    Ok(events)
}

pub fn extract_jobs_with_selectors(
    html_text: &str,
    base_url: &Url,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let item_sel_str = source
        .config
        .get("item")
        .and_then(|v| v.as_str())
        .unwrap_or(".job-item");

    let title_sel_str = source
        .config
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(".job-title");

    let link_sel_str = source
        .config
        .get("link")
        .and_then(|v| v.as_str())
        .unwrap_or("a");

    let loc_sel_str = source.config.get("location").and_then(|v| v.as_str());

    let item_sel = Selector::parse(item_sel_str)
        .map_err(|e| ClientError::HttpStatus(reqwest::StatusCode::BAD_REQUEST, e.to_string()))?;
    let title_sel = Selector::parse(title_sel_str)
        .map_err(|e| ClientError::HttpStatus(reqwest::StatusCode::BAD_REQUEST, e.to_string()))?;
    let link_sel = Selector::parse(link_sel_str)
        .map_err(|e| ClientError::HttpStatus(reqwest::StatusCode::BAD_REQUEST, e.to_string()))?;
    let loc_sel = loc_sel_str.and_then(|s| Selector::parse(s).ok());

    let document = Html::parse_document(html_text);
    let mut events = Vec::new();

    for item in document.select(&item_sel) {
        let title_elem = match item.select(&title_sel).next() {
            Some(t) => t,
            None => continue,
        };
        let title = title_elem
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if title.is_empty() {
            continue;
        }

        let link_elem = match item.select(&link_sel).next() {
            Some(l) => l,
            None => continue,
        };
        let href = match link_elem.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        let job_url = match base_url.join(href) {
            Ok(u) => u.to_string(),
            Err(_) => continue,
        };

        let location = if let Some(ref l_sel) = loc_sel {
            item.select(l_sel)
                .next()
                .map(|l| l.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let role_type = RoleType::infer(&title, None);

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: job_url.clone(),
            title,
            company: source.name.clone(),
            role_type,
            location,
            url: job_url,
            date_posted: None,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::SourceKind;

    #[test]
    fn test_extract_jobs_with_selectors() {
        let html = r#"
        <div class="careers-list">
            <div class="job-card">
                <h3 class="job-header"><a class="link" href="/jobs/eng-1">Staff Platform Engineer</a></h3>
                <span class="city">New York, NY</span>
            </div>
            <div class="job-card">
                <h3 class="job-header"><a class="link" href="/jobs/eng-2">Software Engineering Intern</a></h3>
                <span class="city">San Francisco, CA</span>
            </div>
        </div>
        "#;

        let config = serde_json::json!({
            "item": ".job-card",
            "title": ".job-header",
            "link": ".link",
            "location": ".city"
        });

        let source = WorkerSource {
            id: "src-selector".to_string(),
            kind: SourceKind::Selector,
            name: "Custom Corp".to_string(),
            identifier: "https://customcorp.com/careers".to_string(),
            config,
        };

        let base_url = Url::parse("https://customcorp.com/careers").unwrap();
        let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
        assert_eq!(events.len(), 2);

        if let WorkerEvent::Job {
            title,
            role_type,
            location,
            url,
            ..
        } = &events[1]
        {
            assert_eq!(title, "Software Engineering Intern");
            assert_eq!(*role_type, RoleType::Internship);
            assert_eq!(location, "San Francisco, CA");
            assert_eq!(url, "https://customcorp.com/jobs/eng-2");
        } else {
            panic!("Expected Job event");
        }
    }
}
