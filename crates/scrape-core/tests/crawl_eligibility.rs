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

fn assess(filters: Value, event: &WorkerEvent) -> MatchingAssessment {
    let filters: WorkerFilters = serde_json::from_value(filters).unwrap();
    let evidence = MatchingEvidence::from_event(event, true);
    MatchingEvaluator::new(&filters).unwrap().assess(
        event,
        "2026-10-16T00:00:00Z".parse::<DateTime<Utc>>().unwrap(),
        &evidence,
    )
}

#[test]
fn original_date_and_geography_are_mandatory() {
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
        let result = assess(rules(), &event);
        assert!(
            result.rejected.contains(&reason) || result.unresolved.contains(&reason),
            "mandatory {reason:?} waived: date={date:?}, country={country:?}, {result:?}"
        );
    }
}

#[test]
fn recent_us_and_exact_lower_boundary_are_eligible() {
    for date in ["2026-10-15T00:00:00Z", "2026-10-15T23:59:59Z"] {
        let result = assess(rules(), &candidate(Some(date), Some("US")));
        assert_eq!(result, MatchingAssessment::default());
    }
}

#[test]
fn explicit_unknown_inclusion_is_still_honored() {
    let mut filters = rules();
    filters["matching"]["posting_date"]["include_unknown"] = json!(true);
    filters["matching"]["location_include_unknown"] = json!(true);
    assert_eq!(
        assess(filters, &candidate(None, None)),
        MatchingAssessment::default()
    );
}

#[test]
fn crawl_requires_every_active_requirement_without_date_or_geography_rules() {
    let mut filters = rules();
    filters["matching"]
        .as_object_mut()
        .unwrap()
        .remove("posting_date");
    filters["matching"]["countries"] = json!([]);
    filters["matching"]["workplaces"] = json!(["onsite"]);
    let event = candidate(None, None);
    let strict = assess(filters, &event);
    assert_eq!(strict.unresolved, vec![RejectionReason::Workplace]);
    assert!(strict.rejected.is_empty());
}

#[test]
fn every_active_group_remains_strict() {
    for (field, value, reason) in [
        (
            "keywords",
            json!({"all":["Rust","missing"],"fields":["description"]}),
            RejectionReason::KeywordAll,
        ),
        ("workplaces", json!(["onsite"]), RejectionReason::Workplace),
        ("experience_min", json!(5), RejectionReason::Experience),
        (
            "compensation",
            json!({"min_minor":20000000,"currency":"USD"}),
            RejectionReason::Compensation,
        ),
    ] {
        let mut filters = rules();
        filters["matching"][field] = value;
        let result = assess(
            filters,
            &candidate(Some("2026-10-15T12:00:00Z"), Some("US")),
        );
        assert!(
            result.rejected.contains(&reason) || result.unresolved.contains(&reason),
            "{reason:?}: {result:?}"
        );
    }
}

#[test]
fn priority_places_geography_before_date_and_date_before_relevance() {
    let filters: WorkerFilters = serde_json::from_value(rules()).unwrap();
    let evaluator = MatchingEvaluator::new(&filters).unwrap();
    let started = "2026-10-16T00:00:00Z".parse().unwrap();
    let priority = |event: &WorkerEvent| {
        evaluator.discovery_priority(event, started, &MatchingEvidence::from_event(event, false))
    };
    assert!(
        priority(&candidate(None, Some("US")))
            > priority(&candidate(Some("2026-10-15T12:00:00Z"), None))
    );
    let older = candidate(Some("2026-10-15T01:00:00Z"), Some("US"));
    let mut newer =
        serde_json::to_value(candidate(Some("2026-10-15T12:00:00Z"), Some("US"))).unwrap();
    newer["title"] = json!("Other occupation");
    let newer: WorkerEvent = serde_json::from_value(newer).unwrap();
    assert!(priority(&newer) > priority(&older));
    let wrong_country = candidate(Some("2026-10-15T23:59:00Z"), Some("CA"));
    assert!(priority(&older) > priority(&wrong_country));
}

#[test]
fn priority_counts_satisfied_categories_and_ignores_dates_without_date_rules() {
    let filters: WorkerFilters = serde_json::from_value(
        json!({"matching":{"keywords":{"any":["Rust","Python"],"fields":["description"]}}}),
    )
    .unwrap();
    let evaluator = MatchingEvaluator::new(&filters).unwrap();
    let started = "2026-10-16T00:00:00Z".parse().unwrap();
    let relevant = candidate(None, None);
    let mut irrelevant =
        serde_json::to_value(candidate(Some("2026-10-15T12:00:00Z"), None)).unwrap();
    irrelevant["description_text"] = json!("Unrelated work");
    let irrelevant: WorkerEvent = serde_json::from_value(irrelevant).unwrap();
    let key = |event: &WorkerEvent| {
        evaluator.discovery_priority(event, started, &MatchingEvidence::from_event(event, false))
    };
    assert!(key(&relevant) > key(&irrelevant));
    assert_eq!(key(&relevant).2, key(&irrelevant).2);
    assert!(!evaluator
        .assess(
            &irrelevant,
            started,
            &MatchingEvidence::from_event(&irrelevant, true)
        )
        .rejected
        .is_empty());
}
