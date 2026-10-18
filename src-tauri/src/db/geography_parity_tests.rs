//! Independent geographic expectations, shared worker predicate vs actual SQLite.
use crate::db::{
    init_memory_db,
    jobs::{JobFilter, JobsRepo},
};
use scrape_core::{enrich::ParsedLocation, matching::PostingRules};
fn loc(
    city: Option<&str>,
    country: Option<&str>,
    region: Option<&str>,
    raw: &str,
) -> ParsedLocation {
    ParsedLocation {
        raw: raw.into(),
        city: city.map(str::to_owned),
        country_code: country.map(str::to_owned),
        region_code: region.map(str::to_owned),
        is_remote: false,
    }
}
#[tokio::test]
async fn geography_parity_uses_one_tuple_and_explicit_city_unknown_semantics() {
    let pool = init_memory_db().await.unwrap();
    let rows = vec![
        (
            "split",
            vec![
                loc(Some("Toronto"), Some("CA"), Some("CA-ON"), "Toronto"),
                loc(Some("Cupertino"), Some("US"), Some("US-CA"), "Cupertino"),
            ],
        ),
        ("raw", vec![loc(None, None, None, "Unlistedville")]),
        (
            "unknown_city",
            vec![loc(None, Some("US"), None, "United States")],
        ),
        (
            "near",
            vec![loc(
                Some("Cupertino Heights"),
                Some("US"),
                None,
                "California",
            )],
        ),
        ("literal", vec![loc(Some("A_B%"), Some("US"), None, "A_B%")]),
        (
            "unicode",
            vec![loc(Some("München"), Some("DE"), None, "München")],
        ),
        (
            "blank",
            vec![loc(Some("\t\u{a0}"), Some("US"), None, "Remote")],
        ),
    ];
    for (id, locations) in &rows {
        sqlx::query("INSERT INTO jobs (id,dedupe_key,title,company,url,location,date_added,last_seen_at,status_changed_at) VALUES (?1,?1,'Engineer','Authored fixture','https://fixture.test/job',?2,'2026-10-18','2026-10-18','2026-10-18')").bind(id).bind(locations.iter().map(|l|l.raw.as_str()).collect::<Vec<_>>().join("; ")).execute(&pool).await.unwrap();
        for l in locations {
            sqlx::query("INSERT INTO job_locations (job_id,raw,city,region_code,country_code,is_remote) VALUES (?,?,?,?,?,0)").bind(id).bind(&l.raw).bind(&l.city).bind(&l.region_code).bind(&l.country_code).execute(&pool).await.unwrap();
        }
    }
    for (countries, cities, unknown, expected) in [
        (vec!["CA"], vec!["Cupertino"], false, vec![]),
        (vec!["US"], vec!["Cupertino"], false, vec!["split"]),
        (vec![], vec!["Unlistedville"], false, vec!["raw"]),
        (
            vec!["US"],
            vec!["Missingtown"],
            true,
            vec!["blank", "raw", "unknown_city"],
        ),
        (vec![], vec!["Cupertino"], false, vec!["split"]),
        (vec![], vec!["A_B%"], false, vec!["literal"]),
        (vec![], vec![" münchen "], false, vec!["unicode"]),
        (vec!["US"], vec!["Missingtown"], false, vec![]),
    ] {
        let rules = PostingRules {
            countries: countries.into_iter().map(str::to_owned).collect(),
            cities: cities.into_iter().map(str::to_owned).collect(),
            location_include_unknown: unknown,
            ..Default::default()
        };
        let mut actual:Vec<_>=rows.iter().filter(|(_,locs)|rules.matches_event(&serde_json::from_value(serde_json::json!({"type":"job","source_id":"fixture","external_id":"1","title":"Engineer","company":"Fixture","url":"https://fixture.test/job","location":"","role_type":"unknown","locations":locs})).unwrap())).map(|(id,_)|*id).collect();
        actual.sort();
        assert_eq!(actual, expected, "worker: {rules:?}");
        let saved = JobsRepo::list(
            &pool,
            JobFilter {
                matching: Some(rules.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let mut actual: Vec<_> = saved.items.iter().map(|j| j.id.as_str()).collect();
        actual.sort();
        assert_eq!(actual, expected, "SQLite: {rules:?}");
    }
    pool.close().await;
}
