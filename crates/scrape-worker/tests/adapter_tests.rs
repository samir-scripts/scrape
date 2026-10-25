use scrape_core::{RoleType, SourceKind, TriState, WorkerEvent, WorkerSource, WorkplaceType};
use scrape_worker::adapters::jsonld::extract_job_postings_from_html;
use scrape_worker::adapters::selector::extract_jobs_with_selectors;
use std::fs;
use std::path::PathBuf;
use url::Url;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_greenhouse_fixture() {
    let path = fixtures_dir().join("greenhouse.json");
    let json_str = fs::read_to_string(path).expect("Failed to read greenhouse fixture");

    let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let jobs = val.get("jobs").and_then(|j| j.as_array()).unwrap();
    assert_eq!(jobs.len(), 3);

    // Verify role heuristics on titles
    let title0 = jobs[0].get("title").and_then(|t| t.as_str()).unwrap();
    let title1 = jobs[1].get("title").and_then(|t| t.as_str()).unwrap();
    let title2 = jobs[2].get("title").and_then(|t| t.as_str()).unwrap();

    assert_eq!(RoleType::infer(title0, None), RoleType::Unknown);
    assert_eq!(RoleType::infer(title1, None), RoleType::Internship);
    assert_eq!(RoleType::infer(title2, None), RoleType::Contract);
}

#[test]
fn test_lever_fixture() {
    let path = fixtures_dir().join("lever.json");
    let json_str = fs::read_to_string(path).expect("Failed to read lever fixture");

    let postings: Vec<serde_json::Value> = serde_json::from_str(&json_str).unwrap();
    assert_eq!(postings.len(), 2);

    let p0 = &postings[0];
    let title0 = p0.get("text").and_then(|t| t.as_str()).unwrap();
    let commit0 = p0
        .get("categories")
        .and_then(|c| c.get("commitment"))
        .and_then(|c| c.as_str());
    assert_eq!(RoleType::infer(title0, commit0), RoleType::FullTime);

    let p1 = &postings[1];
    let title1 = p1.get("text").and_then(|t| t.as_str()).unwrap();
    let commit1 = p1
        .get("categories")
        .and_then(|c| c.get("commitment"))
        .and_then(|c| c.as_str());
    assert_eq!(RoleType::infer(title1, commit1), RoleType::Internship);
}

#[test]
fn test_ashby_fixture() {
    let path = fixtures_dir().join("ashby.json");
    let json_str = fs::read_to_string(path).expect("Failed to read ashby fixture");

    let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let jobs = val.get("jobs").and_then(|j| j.as_array()).unwrap();
    assert_eq!(jobs.len(), 2);

    let emp0 = jobs[0].get("employmentType").and_then(|e| e.as_str());
    let emp1 = jobs[1].get("employmentType").and_then(|e| e.as_str());

    assert_eq!(RoleType::infer("ML Engineer", emp0), RoleType::FullTime);
    assert_eq!(
        RoleType::infer("Security Analyst", emp1),
        RoleType::Contract
    );
}

#[test]
fn test_workday_fixture() {
    let path = fixtures_dir().join("workday_jobs.json");
    let json_str = fs::read_to_string(path).expect("Failed to read workday fixture");

    let resp: scrape_worker::adapters::workday::WorkdayListResponse =
        serde_json::from_str(&json_str).unwrap();
    assert_eq!(resp.total, Some(3));
    assert_eq!(resp.job_postings.len(), 3);

    let p0 = &resp.job_postings[0];
    let p1 = &resp.job_postings[1];
    let p2 = &resp.job_postings[2];

    assert_eq!(
        p0.title.as_deref(),
        Some("Senior Distributed Systems Engineer")
    );
    assert_eq!(p0.bullet_fields.first().map(String::as_str), Some("JR1001"));
    assert_eq!(
        RoleType::infer(p0.title.as_deref().unwrap(), Some("Full time")),
        RoleType::FullTime
    );
    assert_eq!(
        RoleType::infer(p0.title.as_deref().unwrap(), None),
        RoleType::Unknown
    );
    assert_eq!(
        scrape_worker::adapters::workday::workplace_from_locations(&[p0
            .locations_text
            .as_deref()
            .unwrap()]),
        WorkplaceType::Onsite
    );

    assert_eq!(
        scrape_worker::adapters::workday::workplace_from_locations(&[p1
            .locations_text
            .as_deref()
            .unwrap()]),
        WorkplaceType::Remote
    );

    assert_eq!(
        RoleType::infer(p2.title.as_deref().unwrap(), None),
        RoleType::Internship
    );
}

#[test]
fn test_jsonld_single_fixture() {
    let path = fixtures_dir().join("jsonld_single.html");
    let html = fs::read_to_string(path).expect("Failed to read jsonld_single fixture");

    let source = WorkerSource {
        id: "src-acme".to_string(),
        kind: SourceKind::Jsonld,
        name: "Acme Corp".to_string(),
        identifier: "https://acme.example.com/careers".to_string(),
        config: serde_json::Value::Null,
    };

    let events = extract_job_postings_from_html(&html, "https://acme.example.com/careers", &source);
    assert_eq!(events.len(), 1);

    if let WorkerEvent::Job {
        title,
        company,
        role_type,
        location,
        url,
        date_posted,
        external_id,
        ..
    } = &events[0]
    {
        assert_eq!(title, "Site Reliability Engineer");
        assert_eq!(company, "Acme Global");
        assert_eq!(*role_type, RoleType::FullTime);
        assert_eq!(location, "Austin, TX, US");
        assert_eq!(url, "https://acme.example.com/careers/sre");
        assert_eq!(external_id, "acme-sre-99");
        assert_eq!(date_posted.as_deref(), Some("2026-10-03"));
    } else {
        panic!("Expected Job event");
    }
}

#[test]
fn test_jsonld_graph_fixture() {
    let path = fixtures_dir().join("jsonld_graph.html");
    let html = fs::read_to_string(path).expect("Failed to read jsonld_graph fixture");

    let source = WorkerSource {
        id: "src-initech".to_string(),
        kind: SourceKind::Jsonld,
        name: "Initech".to_string(),
        identifier: "https://initech.example.com/careers".to_string(),
        config: serde_json::Value::Null,
    };

    let events =
        extract_job_postings_from_html(&html, "https://initech.example.com/careers", &source);
    assert_eq!(events.len(), 2);

    if let WorkerEvent::Job {
        title,
        company,
        role_type,
        external_id,
        ..
    } = &events[0]
    {
        assert_eq!(title, "Database Administrator");
        assert_eq!(company, "Initech");
        assert_eq!(*role_type, RoleType::FullTime);
        assert_eq!(external_id, "initech-dba-1");
    } else {
        panic!("Expected Job event");
    }

    if let WorkerEvent::Job {
        title,
        role_type,
        external_id,
        ..
    } = &events[1]
    {
        assert_eq!(title, "QA Automation Contractor");
        assert_eq!(*role_type, RoleType::Contract);
        assert_eq!(external_id, "initech-qa-2");
    } else {
        panic!("Expected Job event");
    }
}

#[test]
fn test_selector_fixture() {
    let path = fixtures_dir().join("selector.html");
    let html = fs::read_to_string(path).expect("Failed to read selector fixture");

    let config = serde_json::json!({
        "item": ".job-row",
        "title": ".title",
        "link": ".job-link",
        "location": ".location"
    });

    let source = WorkerSource {
        id: "src-cyberdyne".to_string(),
        kind: SourceKind::Selector,
        name: "Cyberdyne".to_string(),
        identifier: "https://cyberdyne.example.com/careers".to_string(),
        config,
    };

    let base_url = Url::parse("https://cyberdyne.example.com/careers").unwrap();
    let events = extract_jobs_with_selectors(&html, &base_url, &source).unwrap();
    assert_eq!(events.len(), 2);

    if let WorkerEvent::Job {
        title,
        role_type,
        location,
        url,
        ..
    } = &events[0]
    {
        assert_eq!(title, "AI Research Scientist");
        assert_eq!(*role_type, RoleType::Unknown);
        assert_eq!(location, "Sunnyvale, CA");
        assert_eq!(url, "https://cyberdyne.example.com/careers/ai-researcher");
    } else {
        panic!("Expected Job event");
    }

    if let WorkerEvent::Job {
        title,
        role_type,
        location,
        url,
        ..
    } = &events[1]
    {
        assert_eq!(title, "Robotics Intern");
        assert_eq!(*role_type, RoleType::Internship);
        assert_eq!(location, "Remote");
        assert_eq!(url, "https://cyberdyne.example.com/careers/robotics-intern");
    } else {
        panic!("Expected Job event");
    }
}

#[test]
fn test_heuristic_public_html_extraction() {
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Acme Careers</title></head>
<body>
  <header><nav><a href="/">Home</a><a href="/about">About</a></nav></header>
  <main>
    <h1>Open Roles at Acme</h1>
    <div class="openings">
      <div class="career-card">
        <a href="/careers/senior-systems-engineer">Senior Systems Engineer</a>
        <span class="loc">Remote</span>
      </div>
      <div class="career-card">
        <a href="/jobs/platform-architect?dept=infra">Staff Platform Architect</a>
        <span class="loc">New York, NY</span>
      </div>
      <div class="nav-links">
        <a href="/careers/apply">Apply</a>
        <a href="/privacy">Privacy Policy</a>
      </div>
    </div>
  </main>
</body>
</html>"#;

    let source = WorkerSource {
        id: "src-acme-heuristic".to_string(),
        kind: SourceKind::Selector,
        name: "Acme Careers".to_string(),
        identifier: "https://acme.example.com/careers".to_string(),
        config: serde_json::json!({}),
    };

    let base_url = Url::parse("https://acme.example.com/careers").unwrap();
    let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
    assert_eq!(events.len(), 2);

    match &events[0] {
        WorkerEvent::Job { title, url, .. } => {
            assert_eq!(title, "Senior Systems Engineer");
            assert_eq!(url, "https://acme.example.com/careers/senior-systems-engineer");
        }
        other => panic!("Expected Job event, got: {other:?}"),
    }

    match &events[1] {
        WorkerEvent::Job { title, url, .. } => {
            assert_eq!(title, "Staff Platform Architect");
            assert_eq!(url, "https://acme.example.com/jobs/platform-architect?dept=infra");
        }
        other => panic!("Expected Job event, got: {other:?}"),
    }
}

#[test]
fn test_detect_fixtures() {
    let gh_path = fixtures_dir().join("detect_greenhouse.html");
    let gh_html = fs::read_to_string(gh_path).expect("Failed to read detect_greenhouse fixture");
    let gh_url = Url::parse("https://stripe.com/careers").unwrap();
    let res = scrape_worker::detect::detect_from_html(&gh_html, &gh_url);
    assert_eq!(res.kind, SourceKind::Greenhouse);
    assert_eq!(res.identifier, "stripe");
    assert_eq!(res.name, "Stripe Careers");

    let lever_path = fixtures_dir().join("detect_lever.html");
    let lever_html = fs::read_to_string(lever_path).expect("Failed to read detect_lever fixture");
    let lever_url = Url::parse("https://figma.com/careers").unwrap();
    let res = scrape_worker::detect::detect_from_html(&lever_html, &lever_url);
    assert_eq!(res.kind, SourceKind::Lever);
    assert_eq!(res.identifier, "figma");
    assert_eq!(res.name, "Figma Careers");
}

#[test]
fn test_jsonld_enrichment() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
<script type="application/ld+json">
{
  "@context": "https://schema.org",
  "@type": "JobPosting",
  "title": "Principal Cyber Operations Engineer",
  "url": "https://careers.example.test/jobs/req-99001",
  "description": "Looking for senior talent with 8+ years of engineering experience.",
  "identifier": {
    "@type": "PropertyValue",
    "name": "Lockheed",
    "value": "req-99001"
  },
  "datePosted": "2026-10-05",
  "employmentType": "FULL_TIME",
  "hiringOrganization": {
    "@type": "Organization",
    "name": "Lockheed Martin"
  },
  "jobLocation": {
    "@type": "Place",
    "address": {
      "@type": "PostalAddress",
      "addressLocality": "Washington",
      "addressRegion": "DC",
      "addressCountry": "US"
    }
  },
  "jobLocationType": "TELECOMMUTE",
  "baseSalary": {
    "@type": "MonetaryAmount",
    "currency": "USD",
    "value": {
      "@type": "QuantitativeValue",
      "minValue": 185000,
      "maxValue": 245000,
      "unitText": "YEAR"
    }
  },
  "securityClearanceRequirement": "Active Top Secret clearance required",
  "eligibilityToWorkRequirement": "Must be a US Citizen. No visa sponsorship provided."
}
</script>
</head>
<body><h1>Job Details</h1></body>
</html>"#;

    let source = WorkerSource {
        id: "src-defense".to_string(),
        kind: SourceKind::Jsonld,
        name: "Defense Corp".to_string(),
        identifier: "https://defense.example.com/careers".to_string(),
        config: serde_json::Value::Null,
    };

    let events =
        extract_job_postings_from_html(html, "https://defense.example.com/careers", &source);
    assert_eq!(events.len(), 1);

    if let WorkerEvent::Job {
        title,
        company,
        salary,
        locations,
        workplace_type,
        sponsorship,
        clearance,
        experience,
        is_new_grad,
        ..
    } = &events[0]
    {
        assert_eq!(title, "Principal Cyber Operations Engineer");
        assert_eq!(company, "Lockheed Martin");

        let sal = salary.as_ref().expect("Salary should be extracted");
        assert_eq!(sal.min, Some(185000));
        assert_eq!(sal.max, Some(245000));
        assert_eq!(sal.currency, "USD");

        assert_eq!(workplace_type, &WorkplaceType::Remote);

        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].city.as_deref(), Some("Washington"));
        assert_eq!(locations[0].region_code.as_deref(), Some("US-DC"));

        let clr = clearance
            .as_ref()
            .expect("Clearance requirement should be extracted");
        assert_eq!(clr.value, TriState::Yes);

        let spon = sponsorship
            .as_ref()
            .expect("Sponsorship requirement should be extracted");
        assert_eq!(spon.value, TriState::No);

        let exp = experience.as_ref().expect("Experience should be extracted");
        assert_eq!(exp.min_years, Some(8));

        assert_eq!(is_new_grad, &TriState::No);
    } else {
        panic!("Expected WorkerEvent::Job");
    }
}

#[test]
fn test_google_document_base_resolution_and_safeguards() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let google_path = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/providers/selector/google.html");
    let html = fs::read_to_string(&google_path).expect("Failed to read google fixture");

    let config = serde_json::json!({
        "item": "li.lLd3Je",
        "title": "h3.QJPWVe",
        "link": "a[href*=\"jobs/results/\"]",
        "location": "span.r0wTof",
        "description": "div.Xsxa1e",
        "next_page": "a[aria-label=\"Go to next page\"]"
    });
    let source = WorkerSource {
        id: "google-test".to_string(),
        kind: SourceKind::Selector,
        name: "Google Careers".to_string(),
        identifier: "https://www.google.com/about/careers/applications/jobs/results/".to_string(),
        config,
    };

    let response_url =
        Url::parse("https://www.google.com/about/careers/applications/jobs/results/").unwrap();
    let events = extract_jobs_with_selectors(&html, &response_url, &source).unwrap();
    assert!(!events.is_empty(), "Expected extracted Google events");

    let first_job = events
        .iter()
        .find_map(|e| match e {
            WorkerEvent::Job { title, url, .. } => Some((title, url)),
            _ => None,
        })
        .expect("Expected at least one valid Job event");

    assert_eq!(first_job.0, "Senior Software Engineer, Ask Google Pay");
    // Crucial: Destination URL must NOT contain duplicate `jobs/results/jobs/results/`!
    assert_eq!(
        first_job.1,
        "https://www.google.com/about/careers/applications/jobs/results/131138433534829254-senior-software-engineer-ask-google-pay"
    );
    assert!(!first_job.1.contains("jobs/results/jobs/results/"));
    let desc = events
        .iter()
        .find_map(|e| match e {
            WorkerEvent::Job {
                description_text, ..
            } => description_text.as_deref(),
            _ => None,
        })
        .expect("Expected extracted Google description");
    assert!(desc.contains("Minimum qualifications"));
    assert!(desc.contains("Bachelor's degree"));
}

#[test]
fn test_document_base_relative_multiple_and_unsafe_tags() {
    let response_url = Url::parse("https://example.com/sub/dir/page.html").unwrap();
    let source = WorkerSource {
        id: "base-test".to_string(),
        kind: SourceKind::Selector,
        name: "Base Test".to_string(),
        identifier: response_url.to_string(),
        config: serde_json::json!({
            "item": ".item",
            "title": "h2",
            "link": "a"
        }),
    };

    // 1. Relative base tag
    let rel_html = r#"
        <html>
        <head><base href="/app/v1/"></head>
        <body><div class="item"><h2>Job 1</h2><a href="roles/1">Apply</a></div></body>
        </html>
    "#;
    let events = extract_jobs_with_selectors(rel_html, &response_url, &source).unwrap();
    if let WorkerEvent::Job { url, .. } = &events[0] {
        assert_eq!(url, "https://example.com/app/v1/roles/1");
    } else {
        panic!("Expected Job event");
    }

    // 2. Multiple base tags (first wins)
    let multi_html = r#"
        <html>
        <head>
            <base href="https://example.com/first/">
            <base href="https://example.com/second/">
        </head>
        <body><div class="item"><h2>Job 2</h2><a href="roles/2">Apply</a></div></body>
        </html>
    "#;
    let events = extract_jobs_with_selectors(multi_html, &response_url, &source).unwrap();
    if let WorkerEvent::Job { url, .. } = &events[0] {
        assert_eq!(url, "https://example.com/first/roles/2");
    } else {
        panic!("Expected Job event");
    }

    // 3. Unsafe base tag: different foreign host must be ignored
    let foreign_html = r#"
        <html>
        <head><base href="https://attacker.evil.com/jobs/"></head>
        <body><div class="item"><h2>Job 3</h2><a href="roles/3">Apply</a></div></body>
        </html>
    "#;
    let events = extract_jobs_with_selectors(foreign_html, &response_url, &source).unwrap();
    if let WorkerEvent::Job { url, .. } = &events[0] {
        assert_eq!(url, "https://example.com/sub/dir/roles/3");
    } else {
        panic!("Expected Job event");
    }

    // 4. Unsafe base tag: javascript: scheme must be ignored
    let js_html = r#"
        <html>
        <head><base href="javascript:alert(1)"></head>
        <body><div class="item"><h2>Job 4</h2><a href="roles/4">Apply</a></div></body>
        </html>
    "#;
    let events = extract_jobs_with_selectors(js_html, &response_url, &source).unwrap();
    if let WorkerEvent::Job { url, .. } = &events[0] {
        assert_eq!(url, "https://example.com/sub/dir/roles/4");
    } else {
        panic!("Expected Job event");
    }

    // 5. Unsafe base tag: credentials must be ignored
    let cred_html = r#"
        <html>
        <head><base href="https://user:pass@example.com/safe/"></head>
        <body><div class="item"><h2>Job 5</h2><a href="roles/5">Apply</a></div></body>
        </html>
    "#;
    let events = extract_jobs_with_selectors(cred_html, &response_url, &source).unwrap();
    if let WorkerEvent::Job { url, .. } = &events[0] {
        assert_eq!(url, "https://example.com/sub/dir/roles/5");
    } else {
        panic!("Expected Job event");
    }
}

#[test]
fn test_apple_selectors_and_pagination() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let apple_path = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("fixtures/providers/selector/apple.html");
    let html = fs::read_to_string(&apple_path).expect("Failed to read apple fixture");

    let config = serde_json::json!({
        "item": "ul#search-job-list > li.rc-accordion-item",
        "title": "h3 a.link-inline",
        "link": "a.link-inline",
        "location": "div.job-title-location span:not(.a11y)",
        "date": "span.job-posted-date",
        "description": "p.text-align-start",
        "next_page": "link[rel=\"next\"]"
    });
    let source = WorkerSource {
        id: "apple-test".to_string(),
        kind: SourceKind::Selector,
        name: "Apple Jobs".to_string(),
        identifier: "https://jobs.apple.com/en-us/search".to_string(),
        config,
    };

    let response_url = Url::parse("https://jobs.apple.com/en-us/search").unwrap();
    let static_config = scrape_core::static_source::StaticSourceConfig::parse(
        &source.config,
        &scrape_core::WorkerLimits::default(),
    )
    .unwrap();

    let page = scrape_worker::adapters::selector::extract_page(
        &html,
        &response_url,
        &source,
        &static_config,
    )
    .unwrap();

    assert_eq!(page.candidates.len(), 20, "Expected 20 Apple jobs");
    for cand in &page.candidates {
        let event = cand.clone().event(&source);
        if let WorkerEvent::Job {
            location,
            title,
            description_text,
            ..
        } = event
        {
            assert!(
                !location.trim().is_empty(),
                "Posting '{title}' should have extracted location"
            );
            assert!(
                !description_text
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty(),
                "Posting '{title}' should have extracted description"
            );
        } else {
            panic!("Expected Job event");
        }
    }

    assert_eq!(
        page.next_page.as_ref().map(|u| u.as_str()),
        Some("https://jobs.apple.com/en-us/searchlocation=united-states-USA&page=2")
    );
}
