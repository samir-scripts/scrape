use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, normalize_description_with_format,
    DescriptionFormat, Salary,
};
use scrape_core::locator::{FieldLocator, QueryEngine};
use scrape_core::{RoleType, TriState, WorkerEvent, WorkerSource};
use scraper::Html;
use url::Url;

pub async fn scrape_selector(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    scrape_selector_with(client, source, &mut |event| {
        events.push(event);
        true
    })
    .await?;
    Ok(events)
}
pub async fn scrape_selector_with(
    client: &HttpClient,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    let base_url = Url::parse(&source.identifier)?;
    let (html_text, actual_page) = client.fetch_text_at(base_url.as_str(), true).await?;
    let base_url = Url::parse(&actual_page)?;

    extract_jobs_with_selectors_with(&html_text, &base_url, source, sink)
}

pub fn glob_to_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let mut regex_str = String::from("^");
    let trimmed = pattern.trim();
    let has_prefix = trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with('/')
        || trimmed.starts_with('*');
    if !has_prefix {
        regex_str.push_str(".*");
    }

    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                }
                regex_str.push_str(".*");
            }
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(c);
            }
            _ => regex_str.push(c),
        }
    }
    regex_str.push('$');
    regex::Regex::new(&regex_str)
}

pub fn extract_jobs_with_selectors(
    html_text: &str,
    base_url: &Url,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    extract_jobs_with_selectors_with(html_text, base_url, source, &mut |event| {
        events.push(event);
        true
    })?;
    Ok(events)
}

pub fn extract_jobs_with_selectors_with(
    html_text: &str,
    base_url: &Url,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    let config_string = |key: &str| -> Result<Option<&str>, ClientError> {
        source
            .config
            .get(key)
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| ClientError::Configuration(format!("{key} must be a string")))
            })
            .transpose()
    };
    let item_sel = QueryEngine::parse_elements(config_string("item")?.unwrap_or(".job-item"))
        .map_err(ClientError::Configuration)?;
    let field = |key: &str, default: Option<&str>| {
        FieldLocator::from_config(&source.config, key, default).map_err(ClientError::Configuration)
    };
    let title_sel = field("title", Some(".job-title"))?.unwrap();
    let link_sel = field("link", Some("a"))?.unwrap();
    let loc_sel = field("location", None)?;
    let sal_sel = field("salary", None)?;
    let desc_sel = field("description", None)?;
    let date_sel = field("date", None)?;
    let id_sel = field("id", None)?;

    let mut include_patterns = Vec::new();
    if let Some(pat) = source.config.get("url_pattern").and_then(|v| v.as_str()) {
        if !pat.trim().is_empty() {
            include_patterns.push(glob_to_regex(pat).map_err(|error| {
                ClientError::HttpStatus(
                    reqwest::StatusCode::BAD_REQUEST,
                    format!("Invalid URL pattern: {error}"),
                )
            })?);
        }
    }
    if let Some(pats) = source.config.get("url_patterns").and_then(|v| v.as_array()) {
        for p in pats {
            if let Some(s) = p.as_str() {
                if !s.trim().is_empty() {
                    include_patterns.push(glob_to_regex(s).map_err(|error| {
                        ClientError::HttpStatus(
                            reqwest::StatusCode::BAD_REQUEST,
                            format!("Invalid URL pattern: {error}"),
                        )
                    })?);
                }
            }
        }
    }

    let mut exclude_patterns = Vec::new();
    if let Some(pats) = source
        .config
        .get("exclude_url_patterns")
        .and_then(|v| v.as_array())
    {
        for p in pats {
            if let Some(s) = p.as_str() {
                if !s.trim().is_empty() {
                    exclude_patterns.push(glob_to_regex(s).map_err(|error| {
                        ClientError::HttpStatus(
                            reqwest::StatusCode::BAD_REQUEST,
                            format!("Invalid excluded URL pattern: {error}"),
                        )
                    })?);
                }
            }
        }
    }

    let document = Html::parse_document(html_text);
    let company_logo = super::branding::resolve_document(
        &document,
        base_url.as_str(),
        &super::branding::employer_name(source),
        source.config.get("employer_logo").and_then(|v| v.as_str()),
    );
    let company_icon_url = company_logo.as_ref().map(|e| e.url.clone());

    for item in item_sel.elements(&document, None) {
        let Some(title) = title_sel.extract(&document, Some(item), "text") else {
            if !sink(invalid_candidate(
                source,
                "Required title locator or regex did not produce a value",
            )) {
                break;
            }
            continue;
        };
        let Some(clean_href) = link_sel.extract(&document, Some(item), "href").or_else(|| {
            // Legacy CSS cards can themselves be the destination anchor.
            if source
                .config
                .get("link")
                .and_then(|v| v.as_str())
                .unwrap_or("a")
                == "a"
                && item.value().name() == "a"
                && link_sel.regex.is_none()
                && link_sel.attribute.is_none()
            {
                item.value().attr("href").map(str::to_string)
            } else {
                None
            }
        }) else {
            if !sink(invalid_candidate(
                source,
                &format!("Posting '{title}' has no destination from its link locator or regex"),
            )) {
                break;
            }
            continue;
        };
        let job_url = match base_url.join(&clean_href) {
            Ok(u) if matches!(u.scheme(), "http" | "https") => u.to_string(),
            _ => {
                if !sink(WorkerEvent::Error {
                    invalid_posting: Some(true),
                    source_id: source.id.clone(),
                    message: format!("Posting '{title}' has an invalid HTTP(S) destination"),
                    retryable: false,
                }) {
                    break;
                }
                continue;
            }
        };

        if !include_patterns.is_empty() {
            let matched = include_patterns.iter().any(|re| re.is_match(&job_url));
            if !matched {
                continue;
            }
        }

        if exclude_patterns.iter().any(|re| re.is_match(&job_url)) {
            continue;
        }

        let location = loc_sel
            .as_ref()
            .and_then(|field| field.extract(&document, Some(item), "text"))
            .unwrap_or_default();
        let role_type = RoleType::infer(&title, None);
        let salary_text = sal_sel
            .as_ref()
            .and_then(|field| field.extract(&document, Some(item), "text"));
        let description_source = desc_sel
            .as_ref()
            .and_then(|field| field.extract(&document, Some(item), "html"));
        let description_format = desc_sel.as_ref().map(|field| {
            if field.extracts_html() {
                DescriptionFormat::Html
            } else {
                DescriptionFormat::PlainText
            }
        });
        let description = description_source.as_deref().map(|html| {
            normalize_description_with_format(
                html,
                description_format.unwrap_or(DescriptionFormat::Html),
                32768,
            )
        });
        let desc_text = description.as_ref().map(|d| d.plain_text.clone());

        let mut salary = salary_text.as_deref().and_then(|t| {
            Salary::from_text(t).or_else(|| Salary::from_text(&format!("salary: {t}")))
        });
        if salary.is_none() {
            if let Some(text) = &desc_text {
                salary = Salary::from_text(text);
            }
        }

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
        let workplace_type = classify_workplace(None, &location, desc_text.as_deref());

        let text_for_enrich = desc_text.as_deref().unwrap_or("");
        let sponsorship = if !text_for_enrich.is_empty() {
            Some(classify_sponsorship(text_for_enrich))
        } else {
            None
        };
        let work_auth = if !text_for_enrich.is_empty() {
            Some(classify_work_auth(text_for_enrich))
        } else {
            None
        };
        let clearance = if !text_for_enrich.is_empty() {
            Some(classify_clearance(text_for_enrich))
        } else {
            None
        };
        let experience = Some(extract_experience(&title, text_for_enrich));
        let is_new_grad = experience
            .as_ref()
            .map(|e| e.is_new_grad)
            .unwrap_or(TriState::Unknown);

        let date_posted = date_sel
            .as_ref()
            .and_then(|field| field.extract(&document, Some(item), "date"))
            .filter(|raw| scrape_core::posting_date::posting_instant(raw, "UTC").is_some());
        let external_id = match &id_sel {
            Some(field) => match field.extract(&document, Some(item), "text") {
                Some(id) => id,
                None => {
                    if !sink(invalid_candidate(
                        source,
                        &format!("Posting '{title}' has no configured original ID"),
                    )) {
                        break;
                    }
                    continue;
                }
            },
            None => job_url.clone(),
        };

        if !sink(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id,
            title,
            company: super::branding::employer_name(source),
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
            description_format: description_source.as_ref().and(description_format),
            description_source,
            company_icon_url: company_icon_url.clone(),
            company_logo: company_logo.clone(),
        }) {
            break;
        }
    }

    Ok(())
}

fn invalid_candidate(source: &WorkerSource, message: &str) -> WorkerEvent {
    WorkerEvent::Error {
        invalid_posting: Some(true),
        source_id: source.id.clone(),
        message: message.into(),
        retryable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::{SourceKind, WorkplaceType};

    #[test]
    fn mixed_locators_extract_original_id_attributes_and_regex() {
        let source = WorkerSource {
            id: "mixed".into(),
            kind: SourceKind::Selector,
            name: "Fixture".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({
                "item": ".job-card", "title": "xpath:.//h2/text()",
                "title_regex": "Role: (?P<value>.+)", "link": "xpath:.//a[1]/@href",
                "id": ".", "id_attribute": "data-job-id", "id_regex": "req-(?P<value>[0-9]+)",
                "location": "xpath:.//h2/following-sibling::span[1]",
                "date": "time", "date_attribute": "datetime",
            }),
        };
        let html = "<article class='job-card' data-job-id='req-42'><h2>Role: Rust Engineer</h2><span>Remote - US</span><a href='/jobs/42'>Apply</a><time datetime='2026-10-15'></time></article>";
        let events =
            extract_jobs_with_selectors(html, &Url::parse(&source.identifier).unwrap(), &source)
                .unwrap();
        match &events[0] {
            WorkerEvent::Job {
                title,
                external_id,
                location,
                url,
                date_posted,
                ..
            } => {
                assert_eq!(title, "Rust Engineer");
                assert_eq!(external_id, "42");
                assert_eq!(location, "Remote - US");
                assert_eq!(url, "https://fixture.test/jobs/42");
                assert_eq!(date_posted.as_deref(), Some("2026-10-15"));
            }
            other => panic!("Expected original extracted candidate, got {other:?}"),
        }
    }

    #[test]
    fn xpath_containers_mix_css_fields_and_keep_optional_unknowns() {
        let source = WorkerSource {
            id: "mixed".into(),
            kind: SourceKind::Selector,
            name: "Fixture".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({
                "item": "//article[contains(@class,'job-card')]", "title": "h2@data-title",
                "link": "a@data-url", "salary": "span", "salary_regex": "Pay: (?P<value>.+)",
                "description": "p", "description_attribute": "text", "description_regex": "Description: (?P<value>.+)",
                "date": "time", "date_attribute": "data-published", "date_regex": "Published: (?P<value>.+)",
            }),
        };
        let html = "<article class='job-card'><h2 data-title='Rust Engineer'></h2><a data-url='/jobs/42'></a><span>Unknown pay</span><p>Description: Literal &lt;b&gt; text</p><time data-published='Published: 2026-10-15'></time></article>";
        let events =
            extract_jobs_with_selectors(html, &Url::parse(&source.identifier).unwrap(), &source)
                .unwrap();
        match &events[0] {
            WorkerEvent::Job {
                salary,
                description_text,
                description_format,
                date_posted,
                ..
            } => {
                assert!(salary.is_none());
                assert_eq!(description_text.as_deref(), Some("Literal <b> text"));
                assert_eq!(*description_format, Some(DescriptionFormat::PlainText));
                assert_eq!(date_posted.as_deref(), Some("2026-10-15"));
            }
            other => panic!("Expected Job, got {other:?}"),
        }
    }

    #[test]
    fn failed_required_field_transforms_are_invalid_candidates() {
        for (field, regex) in [
            ("title", "Missing (.+)"),
            ("link", "missing/(.+)"),
            ("id", "missing-(.+)"),
        ] {
            let mut source = WorkerSource {
                id: "required".into(),
                kind: SourceKind::Selector,
                name: "Fixture".into(),
                identifier: "https://fixture.test/careers".into(),
                config: serde_json::json!({"item":"article", "title":"h2", "link":"a", "id":"article@data-id"}),
            };
            source.config["id"] = serde_json::json!(".@data-id");
            source.config[format!("{field}_regex")] = regex.into();
            let events = extract_jobs_with_selectors(
                "<article data-id='req-42'><h2>Engineer</h2><a href='/jobs/42'>Apply</a></article>",
                &Url::parse(&source.identifier).unwrap(),
                &source,
            )
            .unwrap();
            assert_eq!(events.len(), 1);
            assert!(
                matches!(
                    &events[0],
                    WorkerEvent::Error {
                        invalid_posting: Some(true),
                        ..
                    }
                ),
                "{field}: {:?}",
                events[0]
            );
        }
    }

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

    #[test]
    fn test_extract_jobs_with_custom_regex() {
        let html = r#"
        <div class="career-feed">
            <div class="listing">
                <h3 class="raw-header">Role: Senior Rust Architect [Ref #42]</h3>
                <a class="apply-btn" href="/postings/req-999?source=web&tracker=1">Apply</a>
                <div class="meta-row">Location: Remote - US | Salary: $180,000 - $220,000 / yr</div>
            </div>
        </div>
        "#;

        let config = serde_json::json!({
            "item": ".listing",
            "title": ".raw-header",
            "title_regex": r"Role:\s*(.*?)\s*\[Ref",
            "link": ".apply-btn",
            "link_regex": r"(/postings/req-\d+)",
            "location": ".meta-row",
            "location_regex": r"Location:\s*([^|]+)",
            "salary": ".meta-row",
            "salary_regex": r"Salary:\s*(\$\d+[\d,]*\s*-\s*\$\d+[\d,]*\s*/\s*yr)"
        });

        let source = WorkerSource {
            id: "src-regex".to_string(),
            kind: SourceKind::Selector,
            name: "Regex Corp".to_string(),
            identifier: "https://regexcorp.com/careers".to_string(),
            config,
        };

        let base_url = Url::parse("https://regexcorp.com/careers").unwrap();
        let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
        assert_eq!(events.len(), 1);

        if let WorkerEvent::Job {
            title,
            location,
            url,
            salary,
            workplace_type,
            ..
        } = &events[0]
        {
            assert_eq!(title, "Senior Rust Architect");
            assert_eq!(location, "Remote - US");
            assert_eq!(url, "https://regexcorp.com/postings/req-999");
            assert_eq!(*workplace_type, WorkplaceType::Remote);

            let sal = salary.as_ref().expect("Salary should be parsed");
            assert_eq!(sal.min, Some(180000));
            assert_eq!(sal.max, Some(220000));
        } else {
            panic!("Expected Job event");
        }
    }

    #[test]
    fn test_glob_to_regex() {
        let r1 = glob_to_regex("https://company.com/careers/*").unwrap();
        assert!(r1.is_match("https://company.com/careers/123"));
        assert!(!r1.is_match("https://other.com/careers/123"));

        let r2 = glob_to_regex("example/link/*").unwrap();
        assert!(r2.is_match("https://myco.com/example/link/engineer-1"));
        assert!(!r2.is_match("https://myco.com/about/team"));

        let r3 = glob_to_regex("*/privacy").unwrap();
        assert!(r3.is_match("https://myco.com/privacy"));
        assert!(!r3.is_match("https://myco.com/privacy/more"));
    }

    #[test]
    fn test_extract_jobs_with_url_pattern_glob() {
        let html = r#"
        <div class="jobs-list">
            <div class="item">
                <span class="title">Staff Engineer</span>
                <a href="/careers/jobs/101">Apply</a>
            </div>
            <div class="item">
                <span class="title">Privacy Policy</span>
                <a href="/legal/privacy">Read Policy</a>
            </div>
            <div class="item">
                <span class="title">Product Manager</span>
                <a href="/careers/jobs/102">Apply</a>
            </div>
            <div class="item">
                <span class="title">Company Blog</span>
                <a href="/blog/hello-world">Read Blog</a>
            </div>
        </div>
        "#;

        let config = serde_json::json!({
            "item": ".item",
            "title": ".title",
            "link": "a",
            "url_pattern": "*/careers/jobs/*",
            "exclude_url_patterns": ["*/privacy", "*/legal/*"]
        });

        let source = WorkerSource {
            id: "src-glob".to_string(),
            kind: SourceKind::Selector,
            name: "Glob Corp".to_string(),
            identifier: "https://globcorp.com".to_string(),
            config,
        };

        let base_url = Url::parse("https://globcorp.com").unwrap();
        let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
        assert_eq!(events.len(), 2);

        if let WorkerEvent::Job { title, url, .. } = &events[0] {
            assert_eq!(title, "Staff Engineer");
            assert_eq!(url, "https://globcorp.com/careers/jobs/101");
        } else {
            panic!("Expected Job event");
        }

        if let WorkerEvent::Job { title, url, .. } = &events[1] {
            assert_eq!(title, "Product Manager");
            assert_eq!(url, "https://globcorp.com/careers/jobs/102");
        } else {
            panic!("Expected Job event");
        }
    }
}
