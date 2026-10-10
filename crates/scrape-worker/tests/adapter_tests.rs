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

    let events = extract_job_postings_from_html(html, "https://defense.example.com/careers", &source);
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

        let clr = clearance.as_ref().expect("Clearance requirement should be extracted");
        assert_eq!(clr.value, TriState::Yes);

        let spon = sponsorship.as_ref().expect("Sponsorship requirement should be extracted");
        assert_eq!(spon.value, TriState::No);

        let exp = experience.as_ref().expect("Experience should be extracted");
        assert_eq!(exp.min_years, Some(8));

        assert_eq!(is_new_grad, &TriState::No);
    } else {
        panic!("Expected WorkerEvent::Job");
    }
}

