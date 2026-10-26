use chrono::{DateTime, Utc};
use scrape_core::matching::{
    MatchingAssessment, MatchingEvaluator, MatchingEvidence, RejectionReason,
};
use scrape_core::{WorkerEvent, WorkerFilters};
use serde_json::{json, Value};

fn candidate(date: Option<&str>, country: Option<&str>) -> WorkerEvent {
    serde_json::from_value(json!({
        "type": "job", "source_id": "fixture", "external_id": "engineer",
        "title": "Software Engineer", "company": "Fictional Crawl Co",
        "url": "https://fixture.test/jobs/engineer", "role_type": "full_time",
        "description_text": "Rust distributed systems platform APIs",
        "date_posted": date, "location": country.unwrap_or("Remote"),
        "location_data_complete": country.is_some(),
        "locations": country.map(|code| vec![json!({"raw": code, "country_code": code, "is_remote": false})]).unwrap_or_default()
    })).unwrap()
}

fn rules() -> Value {
    json!({"matching": {
        "target_roles": ["Software Engineer"],
        "keywords": {"all": ["Rust", "distributed", "systems", "platform", "APIs"], "fields": ["description"]},
        "posting_date": {"preset": "past_24h", "timezone": "America/New_York", "include_unknown": false},
        "countries": ["US"], "location_include_unknown": false
    }})
}

fn assess(filters: Value, event: &WorkerEvent, heuristic: bool) -> MatchingAssessment {
    let filters: WorkerFilters = serde_json::from_value(filters).unwrap();
    let mut evidence = MatchingEvidence::from_event(event, true);
    evidence.heuristic_crawl = heuristic;
    MatchingEvaluator::new(&filters).unwrap().assess(
        event,
        "2026-10-16T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        &evidence,
    )
}

#[test]
fn majority_cannot_waive_mandatory_date_or_geography() {
    for (date, country, reason) in [
        (
            Some("2026-10-14T23:59:59.999Z"),
            Some("US"),
            RejectionReason::PostingDate,
        ),
        (
            Some("2026-10-16T00:00:00.001Z"),
            Some("US"),
            RejectionReason::PostingDate,
        ),
        (None, Some("US"), RejectionReason::PostingDate),
        (Some("invalid"), Some("US"), RejectionReason::PostingDate),
        (
            Some("2026-10-15T12:00:00Z"),
            Some("CA"),
            RejectionReason::Geography,
        ),
        (
            Some("2026-10-15T12:00:00Z"),
            None,
            RejectionReason::Geography,
        ),
    ] {
        let event = candidate(date, country);
        for heuristic in [false, true] {
            let result = assess(rules(), &event, heuristic);
            assert!(result.rejected.contains(&reason) || result.unresolved.contains(&reason),
                "mandatory {reason:?} waived: date={date:?}, country={country:?}, heuristic={heuristic}, {result:?}");
        }
    }
}

#[test]
fn recent_us_and_exact_lower_boundary_are_eligible() {
    for date in ["2026-10-15T00:00:00Z", "2026-10-15T23:59:59Z"] {
        let result = assess(rules(), &candidate(Some(date), Some("US")), true);
        assert_eq!(result, MatchingAssessment::default());
    }
}

#[test]
fn explicit_unknown_inclusion_is_still_honored() {
    let mut filters = rules();
    filters["matching"]["posting_date"]["include_unknown"] = json!(true);
    filters["matching"]["location_include_unknown"] = json!(true);
    assert_eq!(
        assess(filters, &candidate(None, None), true),
        MatchingAssessment::default()
    );
}

#[test]
fn majority_still_relaxes_nonmandatory_requirements_without_date_or_geography_rules() {
    let mut filters = rules();
    filters["matching"]
        .as_object_mut()
        .unwrap()
        .remove("posting_date");
    filters["matching"]["countries"] = json!([]);
    filters["matching"]["workplaces"] = json!(["onsite"]);
    let event = candidate(None, None);
    let strict = assess(filters.clone(), &event, false);
    assert!(strict.unresolved.contains(&RejectionReason::Workplace));
    assert_eq!(assess(filters, &event, true), MatchingAssessment::default());
}
