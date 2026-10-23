use super::*;

#[tokio::test]
async fn company_search_merges_verified_templates_and_local_names_with_bounds() {
    let pool = crate::db::init_memory_db().await.unwrap();
    for (id, company) in [
        ("dollar-1", "Dollar General"),
        ("dollar-2", "dollar general"),
        ("dollar-3", "Dollar General Logistics"),
        ("delta-1", "Delta_Analytics"),
        ("delta-2", "DeltaXAnalytics"),
    ] {
        sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_added,last_seen_at,status,status_changed_at) VALUES(?,?,'Fixture engineer',?, 'https://fixture.invalid/job','2026-01-01','2026-01-01','scraped','2026-01-01')")
            .bind(id).bind(id).bind(company).execute(&pool).await.unwrap();
    }
    for index in 0..260 {
        let name = format!("LocalCo {index:03}");
        sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_added,last_seen_at,status,status_changed_at) VALUES(?,?, 'Fixture engineer', ?, 'https://fixture.invalid/job','2026-01-01','2026-01-01','scraped','2026-01-01')")
            .bind(format!("local-{index}"))
            .bind(format!("local-{index}"))
            .bind(name)
            .execute(&pool)
            .await
            .unwrap();
    }

    let exact = companies::search_companies(&pool, "Dollar General")
        .await
        .unwrap();
    assert_eq!(exact[0].name, "Dollar General");
    assert_eq!(exact[0].catalog_company_id, None);
    assert_eq!(
        exact[0].jobs_seen, 2,
        "case variants deduplicate as one employer"
    );
    assert!(exact
        .iter()
        .any(|item| item.name == "Dollar General Logistics"));

    let indexed = companies::search_companies(&pool, "Cloudflare")
        .await
        .unwrap();
    assert_eq!(indexed[0].name, "Cloudflare");
    assert!(indexed[0].catalog_company_id.is_some());
    assert_eq!(indexed[0].jobs_seen, 0);

    let escaped = companies::search_companies(&pool, "Delta_").await.unwrap();
    assert_eq!(escaped.len(), 1);
    assert_eq!(escaped[0].name, "Delta_Analytics");

    let bounded = companies::search_companies(&pool, "LocalCo").await.unwrap();
    assert_eq!(bounded.len(), 250);
    assert!(bounded.iter().all(|item| item.name.starts_with("LocalCo ")));

    let empty = companies::search_companies(&pool, "").await.unwrap();
    assert!(!empty.is_empty() && empty.len() <= 250);
    assert!(empty.iter().all(|item| item.jobs_seen == 0));
    assert!(companies::search_companies(&pool, &"x".repeat(129))
        .await
        .is_err());
    assert!(companies::search_companies(&pool, "a b c d e f g h i")
        .await
        .is_err());

    let plan: Vec<(i64, i64, i64, String)> = sqlx::query_as(
        "EXPLAIN QUERY PLAN SELECT MIN(company), COUNT(*) FROM jobs WHERE company <> '' AND company LIKE 'dollar%' ESCAPE '\\' COLLATE NOCASE GROUP BY company COLLATE NOCASE LIMIT 50",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        plan.iter()
            .any(|row| row.3.contains("idx_jobs_company_nocase")),
        "local employer prefix lookup must use the company index: {plan:?}"
    );
    pool.close().await;
}

#[tokio::test]
async fn company_search_index_migration_preserves_populated_v31_jobs() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(":memory:"))
        .await
        .unwrap();
    let mut prior = sqlx::migrate!("./migrations");
    prior.migrations = prior
        .iter()
        .filter(|migration| migration.version <= 31)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    prior.run(&pool).await.unwrap();
    sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited) VALUES ('kept','kept','Fixture engineer','Dollar General','https://fixture.invalid/job','2026-01-01','2026-01-02','applied','2026-01-02','keep note',1)")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let saved: (String, String, String, i64) =
        sqlx::query_as("SELECT company, status, notes, favorited FROM jobs WHERE id='kept'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        saved,
        (
            "Dollar General".into(),
            "applied".into(),
            "keep note".into(),
            1
        )
    );
    let indexed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_jobs_company_nocase'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(indexed, 1);
    pool.close().await;
}
