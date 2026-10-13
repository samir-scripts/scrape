use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

pub mod compensation;
pub mod jobs;
pub mod metrics;
pub mod notifications;
pub mod postings;
pub mod runs;
pub mod settings;
pub mod sources;

pub async fn init_db(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    // Inspect existing files read-only before setting WAL or any writable pragma.
    // Unknown schema/checksum states must remain byte-for-byte recoverable.
    if db_path.exists() && std::fs::metadata(db_path).is_ok_and(|m| m.len() > 0) {
        let inspection = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(db_path)
                    .read_only(true),
            )
            .await?;
        let history = has_pending_migrations(&inspection).await;
        inspection.close().await;
        history?;
    }
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    // VACUUM INTO takes a consistent SQLite snapshot, including committed WAL data.
    // A filesystem copy of the main database alone is not a safe backup.
    if has_pending_migrations(&pool).await? {
        let backup = db_path.with_file_name(format!(
            "{}.pre-migration-{}.db",
            db_path.file_name().unwrap_or_default().to_string_lossy(),
            uuid::Uuid::new_v4()
        ));
        sqlx::query("VACUUM INTO ?")
            .bind(backup.to_string_lossy().as_ref())
            .execute(&pool)
            .await?;
        tracing::info!(path = %backup.display(), "Created pre-migration database backup");
    }
    run_migrations(&pool).await?;

    Ok(pool)
}

/// Check applied metadata before any migration can write to the database.
/// Unknown/modified histories must be recovered explicitly, never relabeled.
async fn has_pending_migrations(pool: &SqlitePool) -> Result<bool> {
    let exists: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?;
    if exists == 0 {
        // A new, empty database needs no backup. An unversioned populated one
        // is not a supported upgrade source.
        let tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_one(pool)
        .await?;
        if tables > 0 {
            return Err(crate::error::AppError::Validation(
                "Unversioned database: refusing automatic migration. Preserve the database and recover from a compatible backup.".into(),
            ));
        }
        return Ok(false);
    }
    let applied: Vec<(i64, Vec<u8>, bool)> =
        sqlx::query_as("SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await?;
    let migrator = sqlx::migrate!("./migrations");
    for (version, checksum, success) in &applied {
        let expected = migrator.iter().find(|m| m.version == *version);
        if !success || expected.is_none_or(|m| m.checksum.as_ref() != checksum.as_slice()) {
            return Err(crate::error::AppError::Validation(format!(
                "Migration history mismatch at version {version}. No checksums were changed. Close the application, preserve the database and its WAL files, and restore a compatible pre-migration backup or use a reviewed forward repair. See plans/database-recovery.md."
            )));
        }
    }
    // Applied migrations must be a prefix, not a history with missing steps.
    if applied
        .iter()
        .map(|r| r.0)
        .ne(migrator.iter().take(applied.len()).map(|m| m.version))
    {
        return Err(crate::error::AppError::Validation(
            "Migration history has missing versions; refusing automatic repair. See plans/database-recovery.md.".into(),
        ));
    }
    // Metadata may have been relabeled by older releases. Compare the actual
    // schema with the exact supported migration prefix before allowing writes.
    let reference = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(SqliteConnectOptions::new().filename(":memory:"))
        .await?;
    let mut historical = sqlx::migrate!("./migrations");
    historical.migrations = historical
        .iter()
        .take(applied.len())
        .cloned()
        .collect::<Vec<_>>()
        .into();
    historical.run(&reference).await?;
    let schema_sql = "SELECT type, name, tbl_name, sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY type, name";
    type SchemaObject = (String, String, String, Option<String>);
    let expected: Vec<SchemaObject> = sqlx::query_as(schema_sql).fetch_all(&reference).await?;
    let actual: Vec<SchemaObject> = sqlx::query_as(schema_sql).fetch_all(pool).await?;
    reference.close().await;
    if actual != expected {
        return Err(crate::error::AppError::Validation(
            "Database schema does not match its migration history. No automatic reconciliation was performed. Preserve the database and use a compatible backup or reviewed forward repair; see plans/database-recovery.md.".into(),
        ));
    }
    Ok(applied.len() < migrator.iter().count())
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    has_pending_migrations(pool).await?;
    sqlx::migrate!("./migrations").run(pool).await?;
    postings::backfill(pool).await?;
    compensation::backfill(pool).await?;
    Ok(())
}

#[cfg(test)]
pub async fn init_memory_db() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[tokio::test]
    async fn populated_version_ten_upgrade_preserves_annotations_history_settings_and_blocks() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("historical.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let mut old = sqlx::migrate!("./migrations");
        old.migrations = old.iter().take(10).cloned().collect::<Vec<_>>().into();
        old.run(&pool).await.unwrap();
        sqlx::query("INSERT INTO sources (id, kind, name, identifier, config, created_at, filters, retention_policy) VALUES ('old-workflow', 'greenhouse', 'Preserved workflow', 'acme', ?, '2026-01-01T00:00:00Z', ?, 'all')")
            .bind(serde_json::json!({"url":"https://boards.greenhouse.io/acme"}).to_string())
            .bind(serde_json::json!({"include_keywords":[],"query":"title LIKE '%Engineer%'"}).to_string()).execute(&pool).await.unwrap();
        for (index, status) in ["applied", "interviewing", "offer", "rejected"]
            .iter()
            .enumerate()
        {
            sqlx::query("INSERT INTO jobs (id,dedupe_key,source_id,external_id,title,company,role_type,url,date_posted,date_added,last_seen_at,status,status_changed_at,notes,favorited,salary_min,salary_max,salary_currency,salary_interval,salary_source) VALUES (?,?,?,?, 'Engineer','Acme','full_time',?,'2026-01-02T00:00:00Z','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z',?,'2026-01-01T00:00:00Z','Protected note',1,104520,126360,'USD','hour','structured')")
                .bind(format!("saved-{index}")).bind(format!("old-workflow:{index}")).bind("old-workflow").bind(index.to_string()).bind(format!("https://careers.example.com/apply?gh_jid={index}")).bind(status).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO job_status_events (job_id,from_status,to_status,changed_at) VALUES (?, 'scraped', ?, '2026-01-01T00:00:00Z')").bind(format!("saved-{index}")).bind(status).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO deleted_jobs VALUES ('old-workflow:99', '2026-01-02T00:00:00Z')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO app_settings VALUES ('custom.preserved', ?, '2026-01-01T00:00:00Z')",
        )
        .bind(serde_json::json!({"value":42}).to_string())
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;
        let upgraded = init_db(&path).await.unwrap();
        let page = jobs::JobsRepo::list(&upgraded, jobs::JobFilter::default())
            .await
            .unwrap();
        assert_eq!(page.total, 4);
        for job in page.items {
            assert_eq!(job.notes.as_deref(), Some("Protected note"));
            assert!(job.favorited);
            assert!(job.date_posted.is_none());
            assert_eq!(
                job.provider_updated_at.as_deref(),
                Some("2026-01-02T00:00:00Z")
            );
            assert!(matches!(
                job.status,
                scrape_core::JobStatus::Applied
                    | scrape_core::JobStatus::Interviewing
                    | scrape_core::JobStatus::Offer
                    | scrape_core::JobStatus::Rejected
            ));
            assert_eq!(
                jobs::JobsRepo::get_status_events(&upgraded, &job.id)
                    .await
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(job.compensation.unwrap()[0].min_minor, Some(5025));
        }
        let source = sources::SourcesRepo::get(&upgraded, "old-workflow")
            .await
            .unwrap();
        assert_eq!(source.retention_policy, "untracked_only");
        assert_eq!(source.filters["include_keywords"], serde_json::json!([]));
        assert_eq!(source.filters["query"], "title LIKE '%Engineer%'");
        let blocked: i64 = sqlx::query_scalar("SELECT count(*) FROM blocked_postings")
            .fetch_one(&upgraded)
            .await
            .unwrap();
        assert_eq!(blocked, 1);
        let identity: String = sqlx::query_scalar("SELECT identity FROM blocked_postings")
            .fetch_one(&upgraded)
            .await
            .unwrap();
        assert!(identity.contains("99"));
        let custom: String =
            sqlx::query_scalar("SELECT value FROM app_settings WHERE key='custom.preserved'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&custom).unwrap(),
            serde_json::json!({"value":42})
        );
        assert_eq!(
            jobs::JobsRepo::prune_expired_by_source(&upgraded, "old-workflow", 1, "all")
                .await
                .unwrap(),
            0
        );
        assert!(std::fs::read_dir(directory.path())
            .unwrap()
            .filter_map(|file| file.ok())
            .any(|file| file.file_name().to_string_lossy().contains("pre-migration")));
        upgraded.close().await;
    }

    #[tokio::test]
    async fn test_all_migrations_apply_cleanly() {
        let pool = init_memory_db()
            .await
            .expect("Failed to initialize in-memory DB");

        let count_row =
            sqlx::query("SELECT COUNT(*) as cnt, MAX(version) as max_v FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .expect("Failed to query _sqlx_migrations");

        let cnt: i64 = count_row.get("cnt");
        let max_v: i64 = count_row.get("max_v");

        assert_eq!(cnt as usize, sqlx::migrate!("./migrations").iter().count());
        assert_eq!(
            max_v,
            sqlx::migrate!("./migrations")
                .iter()
                .last()
                .unwrap()
                .version,
            "Expected latest migration to be highest version"
        );

        // Verify that app_notifications allows 'error'
        let insert_res = sqlx::query(
            "INSERT INTO app_notifications (id, kind, title, message, read, created_at) VALUES ('test-err-id', 'error', 'Error Title', 'Error Message', 0, '2026-10-11T00:00:00Z')"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_res.is_ok(),
            "Inserting 'error' notification kind must succeed on migrated DB"
        );

        // Verify that company_icon_url column exists on jobs
        let col_res = sqlx::query("SELECT company_icon_url FROM jobs LIMIT 1")
            .fetch_optional(&pool)
            .await;
        assert!(
            col_res.is_ok(),
            "company_icon_url column must exist on jobs table"
        );

        // Verify that work_auth and work_auth_evidence columns exist on jobs
        let wa_res = sqlx::query("SELECT work_auth, work_auth_evidence FROM jobs LIMIT 1")
            .fetch_optional(&pool)
            .await;
        assert!(
            wa_res.is_ok(),
            "work_auth and work_auth_evidence columns must exist on jobs table"
        );

        // Verify that deleted_jobs table exists
        let dj_res = sqlx::query("SELECT dedupe_key, deleted_at FROM deleted_jobs LIMIT 1")
            .fetch_optional(&pool)
            .await;
        assert!(
            dj_res.is_ok(),
            "deleted_jobs table must exist on migrated DB"
        );
    }

    #[tokio::test]
    async fn checksum_mismatch_preserves_metadata_and_user_data() {
        let pool = init_memory_db().await.unwrap();
        sqlx::query(
            "INSERT INTO deleted_jobs (dedupe_key, deleted_at) VALUES ('protected', '2026-01-01')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let corrupt = vec![0xde_u8, 0xad, 0xbe, 0xef];
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 6")
            .bind(&corrupt)
            .execute(&pool)
            .await
            .unwrap();
        let err = run_migrations(&pool).await.unwrap_err();
        assert!(err.to_string().contains("No checksums were changed"));
        let after: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 6")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after, corrupt);
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM deleted_jobs WHERE dedupe_key = 'protected'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn matching_checksums_do_not_hide_an_unknown_schema() {
        let pool = init_memory_db().await.unwrap();
        let before: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        sqlx::query("ALTER TABLE jobs ADD COLUMN unexpected TEXT")
            .execute(&pool)
            .await
            .unwrap();
        let err = run_migrations(&pool).await.unwrap_err();
        assert!(err.to_string().contains("schema does not match"));
        let after: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version, checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(before, after);
        // Failure does not drop or rewrite the unfamiliar column.
        sqlx::query("SELECT unexpected FROM jobs")
            .fetch_all(&pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn populated_upgrade_creates_consistent_backup() {
        let dir = std::env::temp_dir().join(format!("scrape-upgrade-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("jobs.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal),
            )
            .await
            .unwrap();
        // Apply an actual historical prefix, not edited metadata.
        let mut historical = sqlx::migrate!("./migrations");
        historical.migrations = historical
            .iter()
            .take(9)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&pool).await.unwrap();
        sqlx::query("INSERT INTO jobs (id, dedupe_key, title, company, role_type, url, date_added, last_seen_at, status, status_changed_at, notes, favorited) VALUES ('saved', 'saved', 'Engineer', 'Acme', 'full_time', 'https://example.com/1', '2020-01-01', '2020-01-01', 'applied', '2020-01-01', 'Referral', 1)")
            .execute(&pool).await.unwrap();
        // Leave the original connection open to exercise committed WAL data.
        let upgraded = init_db(&path).await.unwrap();
        let mut entries = tokio::fs::read_dir(&dir).await.unwrap();
        let mut backup_path = None;
        while let Some(entry) = entries.next_entry().await.unwrap() {
            if entry
                .file_name()
                .to_string_lossy()
                .contains("pre-migration")
            {
                backup_path = Some(entry.path());
            }
        }
        let backup = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(backup_path.unwrap())
                    .read_only(true),
            )
            .await
            .unwrap();
        for db in [&backup, &upgraded] {
            let state: (String, bool, String) =
                sqlx::query_as("SELECT notes, favorited, status FROM jobs WHERE id = 'saved'")
                    .fetch_one(db)
                    .await
                    .unwrap();
            assert_eq!(state, ("Referral".into(), true, "applied".into()));
        }
        let version: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&backup)
            .await
            .unwrap();
        assert_eq!(version, 9);
        backup.close().await;
        upgraded.close().await;
        pool.close().await;
        tokio::fs::remove_dir_all(dir).await.unwrap();
    }
    #[tokio::test]
    async fn unknown_database_is_rejected_before_journal_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unknown.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        sqlx::query("CREATE TABLE unrelated (secret TEXT)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO unrelated VALUES ('preserve me')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let original = std::fs::read(&path).unwrap();
        assert!(init_db(&path).await.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
