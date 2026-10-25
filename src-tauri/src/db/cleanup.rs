//! Explicit, confirmed data maintenance. No ingestion or retention calls this service.
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Connection, Row, SqliteConnection, SqlitePool};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CleanupMode {
    AllJobs,
    ExceptFavorites,
    ExceptTracked,
    ResetAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CleanupPreview {
    pub mode: CleanupMode,
    pub revision: String,
    #[ts(type = "number")]
    pub deleted_records: i64,
    #[ts(type = "number")]
    pub deleted_postings: i64,
    #[ts(type = "number")]
    pub preserved_records: i64,
    #[ts(type = "number")]
    pub preserved_postings: i64,
    #[ts(type = "number")]
    pub deleted_notes: i64,
    #[ts(type = "number")]
    pub deleted_history: i64,
    #[ts(type = "number")]
    pub workflows: i64,
    #[ts(type = "number")]
    pub blocks: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CleanupRequest {
    pub preview: CleanupPreview,
    pub pause_after: bool,
    pub confirmation: String,
}

impl CleanupRequest {
    pub fn validate(&self) -> Result<()> {
        if self.preview.mode == CleanupMode::ResetAll && self.confirmation != "RESET" {
            return Err(AppError::Validation(
                "Type RESET to confirm resetting all application data.".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CleanupResult {
    pub counts: CleanupPreview,
    pub backup_path: String,
    pub scheduling_paused: bool,
    pub warnings: Vec<String>,
}

// Legacy unresolved records stay separate; never group unrelated NULL identities.
const IDENTITY: &str = "COALESCE(NULLIF(posting_identity,''), 'legacy:' || dedupe_key)";
fn selection(mode: CleanupMode) -> String {
    let preserve = match mode {
        CleanupMode::ExceptFavorites => "favorited = 1",
        CleanupMode::ExceptTracked => "status IN ('applied','interviewing','offer','rejected')",
        _ => "0",
    };
    format!("WITH preserved AS (SELECT DISTINCT {IDENTITY} AS identity FROM jobs WHERE {preserve}), targets AS (SELECT id FROM jobs WHERE {IDENTITY} NOT IN (SELECT identity FROM preserved)) ")
}

async fn snapshot(conn: &mut SqliteConnection, mode: CleanupMode) -> Result<CleanupPreview> {
    let cte = selection(mode);
    let row = sqlx::query(&format!("{cte} SELECT COUNT(*) AS records, COUNT(DISTINCT {IDENTITY}) AS postings, COUNT(CASE WHEN id IN (SELECT id FROM targets) THEN 1 END) AS deleted, COUNT(DISTINCT CASE WHEN id IN (SELECT id FROM targets) THEN {IDENTITY} END) AS deleted_postings, COUNT(CASE WHEN id IN (SELECT id FROM targets) AND TRIM(COALESCE(notes,'')) != '' THEN 1 END) AS notes FROM jobs"))
        .fetch_one(&mut *conn).await?;
    let history: i64 = sqlx::query_scalar(&format!(
        "{cte} SELECT COUNT(*) FROM job_status_events WHERE job_id IN (SELECT id FROM targets)"
    ))
    .fetch_one(&mut *conn)
    .await?;
    let workflows = sqlx::query_scalar("SELECT COUNT(*) FROM workflows")
        .fetch_one(&mut *conn)
        .await?;
    let blocks = sqlx::query_scalar("SELECT COUNT(*) FROM (SELECT identity FROM blocked_postings UNION SELECT identity FROM posting_legacy_blocks UNION SELECT 'legacy:' || dedupe_key FROM deleted_jobs)").fetch_one(&mut *conn).await?;
    // Detect edits between confirmation and execution, even when counts don't change.
    #[derive(sqlx::FromRow, Hash)]
    struct RevisionRow {
        id: String,
        identity: String,
        status: String,
        favorited: i64,
        notes: Option<String>,
        removed_at: Option<String>,
    }
    let jobs: Vec<RevisionRow> = sqlx::query_as(&format!("SELECT id, {IDENTITY} AS identity, status, favorited, notes, removed_at FROM jobs ORDER BY id"))
        .fetch_all(&mut *conn).await?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    jobs.hash(&mut hasher);
    let deleted: i64 = row.get("deleted");
    let deleted_postings: i64 = row.get("deleted_postings");
    Ok(CleanupPreview {
        mode,
        revision: format!("{:016x}", hasher.finish()),
        deleted_records: deleted,
        deleted_postings,
        preserved_records: row.get::<i64, _>("records") - deleted,
        preserved_postings: row.get::<i64, _>("postings") - deleted_postings,
        deleted_notes: row.get("notes"),
        deleted_history: history,
        workflows,
        blocks,
    })
}

pub async fn preview(pool: &SqlitePool, mode: CleanupMode) -> Result<CleanupPreview> {
    let mut conn = pool.acquire().await?;
    let mut tx = conn.begin().await?;
    let result = snapshot(&mut tx, mode).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn backup_path(pool: &SqlitePool) -> Result<PathBuf> {
    let rows = sqlx::query("PRAGMA database_list").fetch_all(pool).await?;
    let file = rows
        .iter()
        .find(|row| row.get::<String, _>("name") == "main")
        .map(|row| row.get::<String, _>("file"))
        .filter(|file| !file.is_empty())
        .ok_or_else(|| {
            AppError::Validation("Cleanup requires a file-backed desktop database.".into())
        })?;
    let parent = Path::new(&file)
        .parent()
        .ok_or_else(|| AppError::Validation("Database directory is unavailable.".into()))?
        .join("backups");
    tokio::fs::create_dir_all(&parent).await?;
    Ok(parent.join(format!("pre-cleanup-{}.db", uuid::Uuid::new_v4())))
}

/// Caller holds exclusive maintenance access through backup, transaction, and
/// scheduler synchronization. Failure never changes migration metadata.
pub async fn execute(
    pool: &SqlitePool,
    request: &CleanupRequest,
    backup: &Path,
) -> Result<CleanupResult> {
    let reset = request.preview.mode == CleanupMode::ResetAll;
    request.validate()?;
    crate::db::backup_to(pool, backup).await.map_err(|error| {
        AppError::Validation(format!(
            "Recovery backup failed. No data was deleted: {error}"
        ))
    })?;
    let mut conn = pool.acquire().await?;
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
    let counts = snapshot(&mut tx, request.preview.mode).await?;
    if counts != request.preview {
        return Err(AppError::Validation("Saved data changed since confirmation. Review the updated cleanup counts and confirm again. No data was deleted.".into()));
    }
    let deleted = sqlx::query(&format!(
        "{} DELETE FROM jobs WHERE id IN (SELECT id FROM targets)",
        selection(counts.mode)
    ))
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if deleted != counts.deleted_records as u64 {
        return Err(AppError::Internal(
            "Cleanup selection changed; transaction rolled back.".into(),
        ));
    }
    if reset {
        // Explicit inventory: FK dependents first; migrations and sqlite_sequence
        // are deliberately untouched. Jobs' dependents cascade on the same tx.
        for table in [
            "employer_branding",
            "browser_sessions",
            "scrape_logs",
            "scrape_runs",
            "sources",
            "workflows",
            "app_notifications",
            "posting_legacy_blocks",
            "blocked_postings",
            "deleted_jobs",
            "posting_date_cache",
            "app_settings",
        ] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query(
            "INSERT INTO app_settings(key,value,updated_at) VALUES ('app.data_reset','true',?)",
        )
        .bind(scrape_core::now_iso())
        .execute(&mut *tx)
        .await?;
    }
    let paused = reset || request.pause_after;
    if paused {
        sqlx::query("INSERT INTO app_settings(key,value,updated_at) VALUES ('scraper.enabled','false',?) ON CONFLICT(key) DO UPDATE SET value='false', updated_at=excluded.updated_at")
            .bind(scrape_core::now_iso()).execute(&mut *tx).await?;
    }
    let enabled: Option<String> =
        sqlx::query_scalar("SELECT value FROM app_settings WHERE key='scraper.enabled'")
            .fetch_optional(&mut *tx)
            .await?;
    tx.commit().await?;
    if reset {
        if let Some(app_dir) = backup.parent().and_then(|p| p.parent()) {
            let session_dir = app_dir.join("browser_sessions");
            if session_dir.exists() {
                let _ = tokio::fs::remove_dir_all(&session_dir).await;
            }
        }
    }
    Ok(CleanupResult {
        counts,
        backup_path: backup.to_string_lossy().into_owned(),
        scheduling_paused: enabled.as_deref() == Some("false"),
        warnings: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn fixture() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::init_db(&dir.path().join("test.db"))
            .await
            .unwrap();
        crate::db::settings::SettingsRepo::ensure_defaults(&pool, &Default::default())
            .await
            .unwrap();
        crate::db::sources::SourcesRepo::seed_default_sample_if_empty(&pool)
            .await
            .unwrap();
        for (id, identity, favorite, status, removed) in [
            ("fav", "p1", 1, "scraped", None),
            ("copy", "p1", 0, "viewed", Some("2000")),
            ("applied", "p2", 0, "applied", None),
            ("interviewing", "p3", 0, "interviewing", Some("2000")),
            ("offer", "p4", 0, "offer", None),
            ("rejected", "p5", 0, "rejected", None),
            ("ordinary", "p6", 0, "ignored", None),
        ] {
            sqlx::query("INSERT INTO jobs(id,dedupe_key,posting_identity,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,removed_at) VALUES(?1,?1,?2,'Engineer','Acme','https://example.com/job','2000','2000',?3,'2000','Keep this note',?4,?5)")
                .bind(id).bind(identity).bind(status).bind(favorite).bind(removed).execute(&pool).await.unwrap();
            sqlx::query(
                "INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES(?,?,'2000')",
            )
            .bind(id)
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query("INSERT INTO job_locations(job_id,raw,country_code) VALUES(?,'US','US')")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
        let logo = scrape_core::branding::CompanyLogo {
            evidence: None,
            checked_at: "2026-10-16T00:00:00Z".into(),
            status: scrape_core::branding::LogoStatus::MissingMetadata,
            asset: None,
            cached_asset_reference: None,
            diagnostic: None,
        };
        crate::db::branding::record(&pool, "fixture", "Acme", &logo)
            .await
            .unwrap();
        for query in [
            "INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES('block','Title','Acme','2000')",
            "INSERT INTO posting_legacy_blocks(legacy_key,identity) VALUES('old','block')",
            "INSERT INTO deleted_jobs(dedupe_key,deleted_at) VALUES('old','2000')",
            "INSERT INTO browser_sessions(id,label,domain,storage_relpath,created_at,updated_at) VALUES('session','Fixture','fixture.test','sessions/fixture.json','2000','2000')",
            "INSERT INTO posting_date_cache VALUES('2000','UTC','2000')",
            "INSERT INTO scrape_runs(id,trigger,status,started_at) VALUES('run','manual','success','2000')",
            "INSERT INTO scrape_logs(id,run_id,timestamp,level,message) VALUES('log','run','2000','info','Saved')",
            "INSERT INTO app_notifications(id,kind,title,message,created_at) VALUES('n','system','Hi','Hi','2000')",
        ] { sqlx::query(query).execute(&pool).await.unwrap(); }
        (dir, pool)
    }
    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .unwrap()
    }
    async fn request(pool: &SqlitePool, mode: CleanupMode) -> CleanupRequest {
        CleanupRequest {
            preview: preview(pool, mode).await.unwrap(),
            pause_after: true,
            confirmation: "RESET".into(),
        }
    }
    #[tokio::test]
    async fn modes_preserve_whole_identities_and_state_and_backup() {
        for (mode, kept) in [
            (CleanupMode::AllJobs, 0),
            (CleanupMode::ExceptFavorites, 2),
            (CleanupMode::ExceptTracked, 4),
            (CleanupMode::ResetAll, 0),
        ] {
            let (dir, pool) = fixture().await;
            let migrations: Vec<(i64, Vec<u8>)> =
                sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            let req = request(&pool, mode).await;
            let saved_sources = count(&pool, "sources").await;
            let saved_workflows = count(&pool, "workflows").await;
            assert_eq!(req.preview.preserved_records, kept);
            let backup = dir.path().join("backup.db");
            let result = execute(&pool, &req, &backup).await.unwrap();
            assert_eq!(result.counts.deleted_records, 7 - kept);
            for table in ["jobs", "job_status_events", "job_locations"] {
                assert_eq!(count(&pool, table).await, kept, "{mode:?} {table}");
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM jobs WHERE notes='Keep this note'"
                )
                .fetch_one(&pool)
                .await
                .unwrap(),
                kept
            );
            for table in [
                "sources",
                "workflows",
                "scrape_runs",
                "employer_branding",
                "browser_sessions",
                "scrape_logs",
                "app_notifications",
                "blocked_postings",
                "posting_legacy_blocks",
                "deleted_jobs",
                "posting_date_cache",
            ] {
                let expected = if mode == CleanupMode::ResetAll {
                    0
                } else {
                    match table {
                        "sources" => saved_sources,
                        "workflows" => saved_workflows,
                        _ => 1,
                    }
                };
                assert_eq!(count(&pool, table).await, expected, "{mode:?} {table}");
            }
            let after: Vec<(i64, Vec<u8>)> =
                sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version")
                    .fetch_all(&pool)
                    .await
                    .unwrap();
            assert_eq!(migrations, after);
            assert!(result.scheduling_paused);
            assert!(
                !crate::db::settings::SettingsRepo::get_all(&pool)
                    .await
                    .unwrap()
                    .scraper_enabled
            );
            assert!(sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&pool)
                .await
                .unwrap()
                .is_empty());
            let restored = sqlx::SqlitePool::connect(&format!("sqlite:{}", backup.display()))
                .await
                .unwrap();
            assert_eq!(count(&restored, "jobs").await, 7);
            assert_eq!(count(&restored, "job_status_events").await, 7);
            restored.close().await;
            if mode == CleanupMode::ResetAll {
                pool.close().await;
                let (reopened, _) =
                    crate::bootstrap_database_and_services(&dir.path().join("test.db"))
                        .await
                        .unwrap();
                assert_eq!(count(&reopened, "sources").await, 0);
                assert!(
                    !crate::db::settings::SettingsRepo::get_all(&reopened)
                        .await
                        .unwrap()
                        .scraper_enabled
                );
                reopened.close().await;
            } else {
                pool.close().await;
            }
        }
    }
    #[tokio::test]
    async fn errors_and_changed_confirmation_leave_data_intact() {
        let (dir, pool) = fixture().await;
        let mut req = request(&pool, CleanupMode::AllJobs).await;
        // Existing destination makes consistent backup fail before any DELETE.
        let existing = dir.path().join("exists.db");
        std::fs::write(&existing, b"keep").unwrap();
        assert!(execute(&pool, &req, &existing)
            .await
            .unwrap_err()
            .to_string()
            .contains("backup failed"));
        assert_eq!(count(&pool, "jobs").await, 7);
        sqlx::query("UPDATE jobs SET favorited=1 WHERE id='ordinary'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(execute(&pool, &req, &dir.path().join("stale.db"))
            .await
            .unwrap_err()
            .to_string()
            .contains("changed since confirmation"));
        assert_eq!(count(&pool, "jobs").await, 7);
        req = request(&pool, CleanupMode::ResetAll).await;
        req.confirmation.clear();
        assert!(execute(&pool, &req, &dir.path().join("unconfirmed.db"))
            .await
            .is_err());
        assert!(!dir.path().join("unconfirmed.db").exists());
        // Fail after DELETE to prove dependents and settings roll back too.
        sqlx::query("CREATE TRIGGER forbid_cleanup BEFORE DELETE ON sources BEGIN SELECT RAISE(ABORT,'fixture failure'); END").execute(&pool).await.unwrap();
        req.confirmation = "RESET".into();
        assert!(execute(&pool, &req, &dir.path().join("rollback.db"))
            .await
            .is_err());
        assert_eq!(count(&pool, "jobs").await, 7);
        assert_eq!(count(&pool, "job_locations").await, 7);
        assert_eq!(count(&pool, "scrape_logs").await, 1);
        assert!(
            crate::db::settings::SettingsRepo::get_all(&pool)
                .await
                .unwrap()
                .scraper_enabled
        );
    }
    #[tokio::test]
    async fn unchecked_pause_preserves_existing_schedule_state() {
        let (dir, pool) = fixture().await;
        let mut req = request(&pool, CleanupMode::AllJobs).await;
        req.pause_after = false;
        assert!(
            !execute(&pool, &req, &dir.path().join("backup.db"))
                .await
                .unwrap()
                .scheduling_paused
        );
    }
    #[tokio::test]
    async fn maintenance_cancels_waits_and_rejects_new_work() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let runner = crate::services::runner::ScrapeRunner::new(pool);
        let run = runner.try_acquire_lock().unwrap();
        let command = runner.write_permit().unwrap();
        let maintenance = runner.begin_maintenance().unwrap();
        assert!(runner
            .cancellation_flag()
            .load(std::sync::atomic::Ordering::SeqCst));
        assert!(runner.try_acquire_lock().is_err());
        assert!(runner.write_permit().is_err());
        assert!(runner.begin_maintenance().is_err());
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(10),
            maintenance.exclusive()
        )
        .await
        .is_err());
        drop(run);
        drop(command);
        let exclusive = maintenance.exclusive().await.unwrap();
        assert!(runner.write_permit().is_err());
        drop(exclusive);
        drop(maintenance);
        assert!(runner.try_acquire_lock().is_ok());
    }
}
