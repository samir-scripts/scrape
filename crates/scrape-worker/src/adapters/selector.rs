use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, to_plain_text, Salary,
};
use scrape_core::{RoleType, TriState, WorkerEvent, WorkerSource};
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

fn apply_regex_capture(re: &regex::Regex, text: &str) -> Option<String> {
    re.captures(text).map(|caps| {
        if let Some(m) = caps
            .name("value")
            .or_else(|| caps.name("title"))
            .or_else(|| caps.name("link"))
            .or_else(|| caps.name("location"))
            .or_else(|| caps.name("salary"))
        {
            m.as_str().trim().to_string()
        } else if let Some(m) = caps.get(1) {
            m.as_str().trim().to_string()
        } else if let Some(m) = caps.get(0) {
            m.as_str().trim().to_string()
        } else {
            text.trim().to_string()
        }
    })
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
    let sal_sel_str = source.config.get("salary").and_then(|v| v.as_str());
    let desc_sel_str = source.config.get("description").and_then(|v| v.as_str());

    let parse_re = |key: &str| -> Result<Option<regex::Regex>, ClientError> {
        match source.config.get(key).and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => {
                let re = regex::Regex::new(s).map_err(|e| {
                    ClientError::HttpStatus(
                        reqwest::StatusCode::BAD_REQUEST,
                        format!("Invalid regex for '{key}': {e}"),
                    )
                })?;
                Ok(Some(re))
            }
            _ => Ok(None),
        }
    };

    let title_re = parse_re("title_regex")?;
    let link_re = parse_re("link_regex")?;
    let loc_re = parse_re("location_regex")?;
    let sal_re = parse_re("salary_regex")?;

    let mut include_patterns = Vec::new();
    if let Some(pat) = source.config.get("url_pattern").and_then(|v| v.as_str()) {
        if !pat.trim().is_empty() {
            if let Ok(re) = glob_to_regex(pat) {
                include_patterns.push(re);
            }
        }
    }
    if let Some(pats) = source.config.get("url_patterns").and_then(|v| v.as_array()) {
        for p in pats {
            if let Some(s) = p.as_str() {
                if !s.trim().is_empty() {
                    if let Ok(re) = glob_to_regex(s) {
                        include_patterns.push(re);
                    }
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
                    if let Ok(re) = glob_to_regex(s) {
                        exclude_patterns.push(re);
                    }
                }
            }
        }
    }

    let item_sel = Selector::parse(item_sel_str)
        .map_err(|e| ClientError::HttpStatus(reqwest::StatusCode::BAD_REQUEST, e.to_string()))?;
    let title_sel = Selector::parse(title_sel_str)
        .map_err(|e| ClientError::HttpStatus(reqwest::StatusCode::BAD_REQUEST, e.to_string()))?;
    let link_sel = Selector::parse(link_sel_str)
        .map_err(|e| ClientError::HttpStatus(reqwest::StatusCode::BAD_REQUEST, e.to_string()))?;
    let loc_sel = loc_sel_str.and_then(|s| Selector::parse(s).ok());
    let sal_sel = sal_sel_str.and_then(|s| Selector::parse(s).ok());
    let desc_sel = desc_sel_str.and_then(|s| Selector::parse(s).ok());

    let document = Html::parse_document(html_text);
    let mut events = Vec::new();

    for item in document.select(&item_sel) {
        let item_full_text = item.text().collect::<Vec<_>>().join(" ");

        let raw_title = match item.select(&title_sel).next() {
            Some(t) => t.text().collect::<Vec<_>>().join(" ").trim().to_string(),
            None => {
                if title_re.is_some() {
                    item_full_text.trim().to_string()
                } else {
                    continue;
                }
            }
        };

        let title = if let Some(ref re) = title_re {
            match apply_regex_capture(re, &raw_title) {
                Some(val) if !val.is_empty() => val,
                _ => raw_title,
            }
        } else {
            raw_title
        };

        if title.is_empty() {
            continue;
        }

        let href = match item
            .select(&link_sel)
            .next()
            .and_then(|l| l.value().attr("href"))
            .or_else(|| {
                if item.value().name() == "a" {
                    item.value().attr("href")
                } else {
                    None
                }
            }) {
            Some(h) => h,
            None => continue,
        };

        let clean_href = if let Some(ref re) = link_re {
            match apply_regex_capture(re, href) {
                Some(val) if !val.is_empty() => val,
                _ => href.to_string(),
            }
        } else {
            href.to_string()
        };

        let job_url = match base_url.join(&clean_href) {
            Ok(u) => u.to_string(),
            Err(_) => continue,
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

        let raw_location = if let Some(ref l_sel) = loc_sel {
            item.select(l_sel)
                .next()
                .map(|l| l.text().collect::<Vec<_>>().join(" ").trim().to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let location = if let Some(ref re) = loc_re {
            let target_text = if !raw_location.is_empty() {
                &raw_location
            } else {
                &item_full_text
            };
            apply_regex_capture(re, target_text).unwrap_or(raw_location)
        } else {
            raw_location
        };

        let role_type = RoleType::infer(&title, None);

        let raw_salary_text = sal_sel.as_ref().and_then(|s| {
            item.select(s)
                .next()
                .map(|el| el.text().collect::<Vec<_>>().join(" ").trim().to_string())
        });

        let salary_text = if let Some(ref re) = sal_re {
            let target_text = raw_salary_text.as_deref().unwrap_or(&item_full_text);
            apply_regex_capture(re, target_text).or(raw_salary_text)
        } else {
            raw_salary_text
        };

        let desc_text = desc_sel.as_ref().and_then(|s| {
            item.select(s)
                .next()
                .map(|el| to_plain_text(&el.html(), 32768))
        });

        let mut salary = salary_text.as_deref().and_then(|t| {
            Salary::from_text(t).or_else(|| Salary::from_text(&format!("salary: {t}")))
        });
        if salary.is_none() {
            if let Some(text) = &desc_text {
                salary = Salary::from_text(text);
            }
        }

        let locations = loc_enrich::parse(&location);
        let workplace_type = classify_workplace(
            None,
            &location,
            desc_text.as_deref(),
        );

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

        let company_icon_url = base_url.host_str().map(|h| {
            format!("https://www.google.com/s2/favicons?domain={h}&sz=64")
        });

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: job_url.clone(),
            title,
            company: source.name.clone(),
            role_type,
            location,
            url: job_url,
            date_posted: None,
            salary,
            locations,
            workplace_type,
            sponsorship,
            work_auth,
            clearance,
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
    use scrape_core::{SourceKind, WorkplaceType};

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
