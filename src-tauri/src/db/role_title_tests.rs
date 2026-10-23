use super::*;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::time::Instant;

async fn pool_with_schema(path: &std::path::Path) -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn role_title_seed_search_is_indexed_idempotent_and_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let pool = pool_with_schema(&directory.path().join("roles.db")).await;

    let started = Instant::now();
    role_titles::seed_role_titles(&pool).await.unwrap();
    let cold_seed = started.elapsed();
    let seeded: i64 = sqlx::query_scalar("SELECT COUNT(1) FROM role_titles")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        seeded > 45_000,
        "expected the complete O*NET title set, got {seeded}"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT title FROM role_titles WHERE title = 'Software Developers' COLLATE NOCASE",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "Software Developers"
    );

    let matches = role_titles::search_role_titles(&pool, "software dev")
        .await
        .unwrap();
    assert!(matches
        .iter()
        .any(|role| role.title == "Software Developers"));
    assert!(matches.len() <= 50);
    let engineer_matches = role_titles::search_role_titles(&pool, "software engineer")
        .await
        .unwrap();
    assert!(
        engineer_matches
            .iter()
            .any(|role| role.title.to_lowercase().contains("software engineer")),
        "full title-term lookup must suggest software engineer variants"
    );
    assert_eq!(
        role_titles::search_role_titles(&pool, "")
            .await
            .unwrap()
            .len(),
        50
    );
    let exact = role_titles::search_role_titles(&pool, "Software Developers")
        .await
        .unwrap();
    assert_eq!(exact[0].title, "Software Developers");
    assert_eq!(exact[0].soc_code, "15-1252");
    assert_eq!(exact[0].priority_tier, 1);
    assert!(role_titles::search_role_titles(&pool, &"x".repeat(257))
        .await
        .is_err());
    assert!(
        role_titles::search_role_titles(&pool, "a b c d e f g h i j k l m n o p q")
            .await
            .is_err()
    );
    let query_plan: Vec<(i64, i64, i64, String)> = sqlx::query_as(
        "EXPLAIN QUERY PLAN SELECT role_title_id FROM role_title_terms WHERE term GLOB 'soft*'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        query_plan
            .iter()
            .any(|line| line.3.contains("INDEX sqlite_autoindex_role_title_terms")),
        "prefix lookup must use its token index: {query_plan:?}",
    );
    for query in ["Software Developers", "software dev", "engineer", "ware"] {
        let started = Instant::now();
        let suggestions = role_titles::search_role_titles(&pool, query).await.unwrap();
        println!(
            "role-title query {query:?}: {} us",
            started.elapsed().as_micros()
        );
        if query == "software dev" {
            let first_nonprefix = suggestions
                .iter()
                .position(|role| !role.title.to_lowercase().starts_with(query))
                .unwrap_or(suggestions.len());
            assert!(suggestions[first_nonprefix..]
                .iter()
                .all(|role| !role.title.to_lowercase().starts_with(query)));
        }
        if query == "ware" {
            assert!(
                !suggestions
                    .iter()
                    .any(|role| role.title == "Software Developers"),
                "infix-only matches are not token-prefix suggestions"
            );
        }
    }

    sqlx::query("UPDATE role_titles SET priority_tier = 3 WHERE title = 'Software Developers'")
        .execute(&pool)
        .await
        .unwrap();
    let started = Instant::now();
    role_titles::seed_role_titles(&pool).await.unwrap();
    let warm_skip = started.elapsed();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT priority_tier FROM role_titles WHERE title = 'Software Developers'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        3,
        "non-empty table must skip seeding without overwriting user data"
    );
    println!(
        "role-title cold seed: {} ms; populated-table skip: {} ms; rows: {seeded}",
        cold_seed.as_millis(),
        warm_skip.as_millis()
    );
    pool.close().await;

    let rollback_pool = pool_with_schema(&directory.path().join("rollback.db")).await;
    sqlx::query(
        "CREATE TRIGGER inject_role_seed_failure BEFORE INSERT ON role_title_terms BEGIN SELECT RAISE(ABORT, 'injected seed failure'); END",
    )
    .execute(&rollback_pool)
    .await
    .unwrap();
    assert!(role_titles::seed_role_titles(&rollback_pool).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM role_titles")
            .fetch_one(&rollback_pool)
            .await
            .unwrap(),
        0,
        "role rows must roll back with failed term indexing"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM role_title_terms")
            .fetch_one(&rollback_pool)
            .await
            .unwrap(),
        0
    );
    sqlx::query("DROP TRIGGER inject_role_seed_failure")
        .execute(&rollback_pool)
        .await
        .unwrap();
    role_titles::seed_role_titles(&rollback_pool).await.unwrap();
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM role_titles")
            .fetch_one(&rollback_pool)
            .await
            .unwrap()
            > 45_000,
        "failed first launch must be retryable"
    );
    rollback_pool.close().await;
}

#[tokio::test]
async fn migration_30_preserves_v29_user_data_and_recovers_after_failure() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("upgrade.db");
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .foreign_keys(true),
        )
        .await
        .unwrap();
    let mut historical = sqlx::migrate!("./migrations");
    historical.migrations = historical
        .iter()
        .filter(|migration| migration.version <= 29)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    historical.run(&pool).await.unwrap();
    upgrade_workflow_settings_v4(&pool).await.unwrap();
    upgrade_workflow_settings_v5(&pool).await.unwrap();
    postings::backfill(&pool).await.unwrap();
    compensation::backfill(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO app_settings(key, value, updated_at) VALUES ('role.seed.fixture', 'kept', '2026-01-01')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO jobs(id, dedupe_key, title, company, url, date_added, last_seen_at, status, status_changed_at, notes, favorited) VALUES ('protected-job', 'protected-identity', 'Software Developer', 'Fixture Employer', 'https://fixture.invalid/job', '2026-01-01', '2026-01-02', 'applied', '2026-01-02', 'keep this note', 1)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO job_status_events(job_id, to_status, changed_at) VALUES ('protected-job', 'applied', '2026-01-02')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut failing = sqlx::migrate!("./migrations");
    failing.migrations = failing
        .iter()
        .map(|migration| {
            if migration.version == 30 {
                sqlx::migrate::Migration::new(
                    30,
                    migration.description.clone(),
                    migration.migration_type,
                    format!(
                        "{}\nSELECT * FROM injected_missing_role_migration_table;",
                        migration.sql
                    )
                    .into(),
                    false,
                )
            } else {
                migration.clone()
            }
        })
        .collect::<Vec<_>>()
        .into();
    assert!(failing.run(&pool).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('role_titles', 'role_title_terms')",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 30",)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    pool.close().await;

    let upgraded = init_db(&path).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM app_settings WHERE key = 'role.seed.fixture'",
        )
        .fetch_one(&upgraded)
        .await
        .unwrap(),
        "kept"
    );
    let job: (String, String, i64) =
        sqlx::query_as("SELECT status, notes, favorited FROM jobs WHERE id = 'protected-job'")
            .fetch_one(&upgraded)
            .await
            .unwrap();
    assert_eq!(job, ("applied".into(), "keep this note".into(), 1));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM job_status_events WHERE job_id = 'protected-job'",
        )
        .fetch_one(&upgraded)
        .await
        .unwrap(),
        1
    );
    assert!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM role_titles")
            .fetch_one(&upgraded)
            .await
            .unwrap()
            > 45_000
    );
    let backups: Vec<_> = std::fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|entry| {
            entry
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("pre-migration")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&backups[0])
                .read_only(true),
        )
        .await
        .unwrap();
    let protected_backup: (String, String, i64) =
        sqlx::query_as("SELECT status, notes, favorited FROM jobs WHERE id = 'protected-job'")
            .fetch_one(&backup)
            .await
            .unwrap();
    assert_eq!(
        protected_backup,
        ("applied".into(), "keep this note".into(), 1)
    );
    backup.close().await;
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&upgraded)
        .await
        .unwrap()
        .is_empty());
    upgraded.close().await;

    let restarted = init_db(&path).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(1) FROM role_titles")
            .fetch_one(&restarted)
            .await
            .unwrap(),
        45_716
    );
    assert_eq!(
        std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|entry| entry
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("pre-migration"))
            .collect::<Vec<_>>(),
        backups
    );
    restarted.close().await;
}

#[tokio::test]
async fn migration_31_repairs_known_role_history_without_relabeling_or_data_loss() {
    for historical_sql in [
        None,
        Some(include_str!(
            "../../historical-migrations/0030_role_titles.sql"
        )),
        Some(include_str!(
            "../../historical-migrations/0030_role_titles_role_first.sql"
        )),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("known30.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let mut prefix = sqlx::migrate!("./migrations");
        prefix.migrations = prefix
            .iter()
            .filter(|m| m.version <= 30)
            .map(|m| {
                if let Some(sql) = historical_sql.filter(|_| m.version == 30) {
                    sqlx::migrate::Migration::new(
                        30,
                        m.description.clone(),
                        m.migration_type,
                        sql.into(),
                        false,
                    )
                } else {
                    m.clone()
                }
            })
            .collect::<Vec<_>>()
            .into();
        prefix.run(&pool).await.unwrap();
        role_titles::seed_role_titles(&pool).await.unwrap();
        sqlx::query("INSERT INTO jobs(id, dedupe_key, title, company, url, date_added, last_seen_at, status, status_changed_at, notes, favorited) VALUES ('kept', 'kept', 'Engineer', 'Fixture', 'https://fixture.invalid/kept', '2026-01-01', '2026-01-02', 'applied', '2026-01-02', 'protected note', 1)")
            .execute(&pool).await.unwrap();
        let history: Vec<history::Applied> = sqlx::query_as(
            "SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let terms: Vec<(String, i64)> = sqlx::query_as(
            "SELECT term, role_title_id FROM role_title_terms ORDER BY term, role_title_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let mut failing = history::manifest(&history);
        failing.migrations = failing
            .iter()
            .map(|m| {
                if m.version == 31 {
                    sqlx::migrate::Migration::new(
                        31,
                        m.description.clone(),
                        m.migration_type,
                        format!("{}\nSELECT * FROM injected_missing_table;", m.sql).into(),
                        false,
                    )
                } else {
                    m.clone()
                }
            })
            .collect::<Vec<_>>()
            .into();
        assert!(failing.run(&pool).await.is_err());
        let after_failure: Vec<history::Applied> = sqlx::query_as(
            "SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(after_failure, history);
        assert_eq!(
            sqlx::query_as::<_, (String, i64)>(
                "SELECT term, role_title_id FROM role_title_terms ORDER BY term, role_title_id"
            )
            .fetch_all(&pool)
            .await
            .unwrap(),
            terms
        );
        pool.close().await;
        let repaired = init_db(&path).await.unwrap();
        let after: Vec<history::Applied> = sqlx::query_as("SELECT version, checksum, success FROM _sqlx_migrations WHERE version <= 30 ORDER BY version").fetch_all(&repaired).await.unwrap();
        assert_eq!(
            after, history,
            "all historical checksums must remain immutable"
        );
        assert_eq!(
            sqlx::query_as::<_, (String, i64)>(
                "SELECT term, role_title_id FROM role_title_terms ORDER BY term, role_title_id"
            )
            .fetch_all(&repaired)
            .await
            .unwrap(),
            terms
        );
        assert_eq!(
            sqlx::query_as::<_, (String, String, i64)>(
                "SELECT status, notes, favorited FROM jobs WHERE id = 'kept'"
            )
            .fetch_one(&repaired)
            .await
            .unwrap(),
            ("applied".into(), "protected note".into(), 1)
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
                .fetch_one(&repaired)
                .await
                .unwrap(),
            "ok"
        );
        assert!(sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&repaired)
            .await
            .unwrap()
            .is_empty());
        repaired.close().await;
        let backups = || {
            std::fs::read_dir(directory.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| path.to_string_lossy().contains("pre-migration"))
                .collect::<Vec<_>>()
        };
        assert_eq!(backups().len(), 1);
        let backup = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&backups()[0])
                    .read_only(true),
            )
            .await
            .unwrap();
        let backup_history: Vec<history::Applied> = sqlx::query_as(
            "SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&backup)
        .await
        .unwrap();
        assert_eq!(backup_history, history);
        backup.close().await;
        init_db(&path).await.unwrap().close().await;
        assert_eq!(
            backups().len(),
            1,
            "restart must not repeat the repair/backup"
        );
    }
}

#[tokio::test]
async fn migration_30_unknown_history_is_rejected_without_mutating_database() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unknown.db");
    let pool = pool_with_schema(&path).await;
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'12345678' WHERE version = 30")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let before = std::fs::read(&path).unwrap();
    let error = init_db(&path).await.unwrap_err().to_string();
    assert!(error.contains("mismatch at version 30"), "{error}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!std::fs::read_dir(directory.path())
        .unwrap()
        .any(|entry| entry
            .unwrap()
            .path()
            .to_string_lossy()
            .contains("pre-migration")));
}
