//! Independent acceptance expectations for compatibility; no conversion-derived oracle.
use super::{runner, workflow};
use scrape_core::{matching::MatchingEvaluator, WorkerEvent, WorkerFilters};
use serde_json::{json, Value};

fn spec(filters: Value) -> Value {
    json!({"version":6,"metadata":{"name":"Compatibility fixture"},"sources":[{"id":"fixture","label":"Fixture","extractor":{"kind":"greenhouse","url":"compatibility-fixture"}}],"schedule":{"cadence":"manual"},"filters":filters})
}

#[test]
fn matching_compatibility_preserves_independent_constraints_and_sample_queries() {
    let cases = [
        (
            json!({"locations":["Remote","Austin"],"matching":{}}),
            "Designer",
            "Austin",
            "full_time",
            true,
        ),
        (
            json!({"locations":["Remote","Austin"],"matching":{}}),
            "Designer",
            "Remote - London",
            "full_time",
            true,
        ),
        (
            json!({"locations":["Remote","Austin"],"matching":{}}),
            "Designer",
            "London",
            "full_time",
            false,
        ),
        (
            json!({"include_keywords":["Software"],"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
            "Software Designer",
            "Austin",
            "full_time",
            false,
        ),
        (
            json!({"include_keywords":["Software"],"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
            "Software Engineer",
            "Austin",
            "full_time",
            true,
        ),
        (
            json!({"query":"title LIKE '%Engineer%' OR title LIKE '%Developer%' OR title LIKE '%Intern%'","matching":{}}),
            "Designer",
            "Austin",
            "full_time",
            false,
        ),
        (
            json!({"query":"title LIKE '%Engineer%' OR title LIKE '%Intern%'","matching":{}}),
            "Designer",
            "Austin",
            "full_time",
            false,
        ),
        (
            json!({"role_types":["internship"],"matching":{"role_types":["full_time"]}}),
            "Engineer",
            "Austin",
            "full_time",
            false,
        ),
        (
            json!({"role_types":["internship"],"matching":{"role_types":["full_time"]}}),
            "Engineer",
            "Austin",
            "internship",
            false,
        ),
        (
            json!({"include_keywords":["Austin"],"matching":{"keywords":{"any":["Engineer"],"all":["Software"],"exclude":["Senior"],"fields":["title"]}}}),
            "Software Engineer",
            "Austin",
            "full_time",
            true,
        ),
        (
            json!({"exclude_keywords":["Austin"],"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
            "Engineer",
            "Austin",
            "full_time",
            false,
        ),
        (
            json!({"query":"company = 'Fixture'","policy":{"title":"%Engineer%"},"matching":{"keywords":{"all":["Systems"],"fields":["description"]}}}),
            "Engineer",
            "Austin",
            "full_time",
            true,
        ),
        (
            json!({"compensation":{"min_minor":5000,"currency":"USD","interval":"hour"},"matching":{"compensation":{"max_minor":4000,"currency":"USD","interval":"hour"}}}),
            "Engineer",
            "Austin",
            "full_time",
            false,
        ),
    ];
    let started = chrono::DateTime::parse_from_rfc3339("2026-10-18T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    for (index, (raw, title, location, role, expected)) in cases.into_iter().enumerate() {
        let event: WorkerEvent = serde_json::from_value(json!({"type":"job","source_id":"fixture","external_id":"1","title":title,"company":"Fixture","url":"https://fixture.test/jobs/1","location":location,"role_type":role,"description_text":"Systems work"})).unwrap();
        let original: WorkerFilters = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            MatchingEvaluator::new(&original)
                .unwrap()
                .matches_at(&event, started),
            expected,
            "original case {index}"
        );
        let validated = workflow::validate(spec(raw.clone()), "UTC").unwrap();
        let input = workflow::source_input(&validated, "UTC").unwrap();
        let resolved = runner::resolve_worker_filters(input.filters.as_ref().unwrap()).unwrap();
        assert_eq!(
            MatchingEvaluator::new(&resolved)
                .unwrap()
                .matches_at(&event, started),
            expected,
            "resolved case {index}: {raw}"
        );
        assert_eq!(
            resolved, original,
            "compatibility must preserve original fields, case {index}"
        );
        let reopened =
            workflow::validate(serde_json::to_value(&validated).unwrap(), "UTC").unwrap();
        assert_eq!(reopened, validated, "validation must be idempotent");
    }
}

#[tokio::test]
async fn matching_compatibility_save_import_reopen_and_global_isolation() {
    use super::execution_diagnostics;
    use crate::db::{settings::SettingsRepo, sources::SourcesRepo};
    let pool = crate::db::init_memory_db().await.unwrap();
    SettingsRepo::set_value(&pool, "filters.include_keywords", json!(["hidden-global"]))
        .await
        .unwrap();
    SettingsRepo::set_value(&pool, "filters.locations", json!(["hidden-location"]))
        .await
        .unwrap();
    let globals = runner::load_global_filters(&pool).await.unwrap();
    let filters = json!({
        "include_keywords":["Software"],"exclude_keywords":["Staff"],"locations":["Remote","Austin"],"role_types":["full_time","internship"],
        "query":"title LIKE '%Engineer%' OR title LIKE '%Intern%'", "policy":{"title":"%Software%"},
        "compensation":{"min_minor":5000,"currency":"USD","interval":"hour","include_unknown":true},
        "matching":{"keywords":{"any":["Engineer","Developer"],"all":["Software"],"exclude":["Senior"],"fields":["title","description"]},"role_types":["internship"]}
    });
    let expected: WorkerFilters = serde_json::from_value(filters.clone()).unwrap();
    for version in [4, 5, 6] {
        let mut document = spec(filters.clone());
        document["version"] = json!(version);
        let imported =
            workflow::validate_legacy_import(document, "UTC", "0 0 9 * * *", &globals, false)
                .unwrap();
        let saved = workflow::persist(&pool, imported, None, true, "UTC")
            .await
            .unwrap();
        let reopened = SourcesRepo::get(&pool, &saved.id).await.unwrap();
        let prepared =
            execution_diagnostics::prepare(&reopened, "UTC", "2026-10-18T00:00:00Z").unwrap();
        assert_eq!(prepared.filters, expected);
        let reviewed =
            workflow::validate(reopened.config["__workflow_spec"].clone(), "UTC").unwrap();
        let saved_again = workflow::persist(&pool, reviewed, Some(saved.id.clone()), false, "UTC")
            .await
            .unwrap();
        assert_eq!(saved_again.filters, saved.filters);
    }
    let modern = workflow::validate_legacy_import(
        spec(json!({"matching":{"keywords":{"any":["Engineer"]}}})),
        "UTC",
        "0 0 9 * * *",
        &globals,
        true,
    )
    .unwrap();
    let input = workflow::source_input(&modern, "UTC").unwrap();
    let independent = runner::resolve_worker_filters(input.filters.as_ref().unwrap()).unwrap();
    assert!(independent.include_keywords.is_empty());
    assert!(independent.locations.is_empty());
    // Legacy omission inherits once; explicit [] overrides, and then execution is independent.
    let mut legacy =
        spec(json!({"include_keywords":[],"matching":{"keywords":{"any":["Engineer"]}}}));
    legacy["version"] = json!(3);
    legacy["extractor"] = legacy["sources"][0]["extractor"].clone();
    legacy.as_object_mut().unwrap().remove("sources");
    let imported =
        workflow::validate_legacy_import(legacy, "UTC", "0 0 9 * * *", &globals, false).unwrap();
    let input = workflow::source_input(&imported, "UTC").unwrap();
    let resolved = runner::resolve_worker_filters(input.filters.as_ref().unwrap()).unwrap();
    assert!(resolved.include_keywords.is_empty());
    assert_eq!(resolved.locations, ["hidden-location"]);
    pool.close().await;
}

#[test]
fn matching_compatibility_descriptions_preserve_raw_evidence_and_fail_invalid_filters() {
    let raw = json!({"include_keywords":["S_ft%"],"exclude_keywords":["Remote"],"locations":["Remote","Austin"],"role_types":["internship"],"query":"title LIKE '%Intern%'","policy":{"title":"%Engineer%"},"compensation":{"include_unknown":true},"matching":{}});
    let before: WorkerFilters = serde_json::from_value(raw.clone()).unwrap();
    let resolved = runner::resolve_worker_filters(&raw).unwrap();
    assert_eq!(resolved, before);
    let descriptions = crate::commands::scrape::describe_workflow_filters(raw).unwrap();
    assert_eq!(
        descriptions
            .iter()
            .map(|d| d.field.as_str())
            .collect::<Vec<_>>(),
        [
            "include_keywords",
            "exclude_keywords",
            "locations",
            "role_types",
            "query",
            "policy",
            "compensation"
        ]
    );
    for invalid in [
        json!({"query":"title LIKE"}),
        json!({"policy":{"unsupported_field":"invalid"}}),
        json!({"include_keywords":"Rust"}),
        json!({"compensation":{"min_minor":-1}}),
    ] {
        assert!(
            crate::commands::scrape::describe_workflow_filters(invalid.clone()).is_err(),
            "{invalid}"
        );
    }
}
