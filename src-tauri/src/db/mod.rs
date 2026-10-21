use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

pub mod branding;
#[cfg(test)]
mod browser_upgrade_tests;
pub mod cleanup;
pub mod compensation;
mod history;
pub mod jobs;
pub mod metrics;
pub mod notifications;
#[cfg(test)]
mod ownership_upgrade_tests;
pub mod postings;
pub mod runs;
#[cfg(test)]
mod search_upgrade_tests;
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
        let history = checked_manifest(&inspection).await;
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
    if checked_manifest(&pool).await?.1 {
        let backup = db_path.with_file_name(format!(
            "{}.pre-migration-{}.db",
            db_path.file_name().unwrap_or_default().to_string_lossy(),
            uuid::Uuid::new_v4()
        ));
        backup_to(&pool, &backup).await?;
        tracing::info!(path = %backup.display(), "Created pre-migration database backup");
    }
    run_migrations(&pool).await?;

    Ok(pool)
}

pub(crate) async fn backup_to(pool: &SqlitePool, path: &Path) -> Result<()> {
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_string_lossy().as_ref())
        .execute(pool)
        .await?;
    Ok(())
}

/// Check applied metadata before any migration can write to the database.
/// Unknown/modified histories must be recovered explicitly, never relabeled.
async fn checked_manifest(pool: &SqlitePool) -> Result<(sqlx::migrate::Migrator, bool)> {
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
        return Ok((history::manifest(&[]), false));
    }
    let applied: Vec<(i64, Vec<u8>, bool)> =
        sqlx::query_as("SELECT version, checksum, success FROM _sqlx_migrations ORDER BY version")
            .fetch_all(pool)
            .await?;
    let migrator = history::manifest(&applied);
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
    let mut historical = history::manifest(&applied);
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
    let pending = applied.len() < migrator.iter().count();
    Ok((migrator, pending))
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let (manifest, _) = checked_manifest(pool).await?;
    manifest.run(pool).await?;
    upgrade_workflow_settings_v4(pool).await?;
    upgrade_workflow_settings_v5(pool).await?;
    postings::backfill(pool).await?;
    compensation::backfill(pool).await?;
    Ok(())
}

type WorkflowUpgradeRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    i64,
    String,
    bool,
);
type WorkflowUpgradeResult = (String, String, Option<String>, String, Option<String>);

async fn upgrade_workflow_settings_v4(pool: &SqlitePool) -> Result<()> {
    use scrape_core::{RoleType, WorkerFilters};
    use serde_json::Value;

    let done: Option<bool> =
        crate::db::settings::SettingsRepo::get_value(pool, "migration.workflow_settings_v4")
            .await?;
    if done.is_some() {
        return Ok(());
    }
    let cron: String = crate::db::settings::SettingsRepo::get_value(pool, "scraper.cron")
        .await?
        .unwrap_or_else(|| "0 0 */6 * * *".into());
    let timezone: String = crate::db::settings::SettingsRepo::get_value(pool, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let startup: bool =
        crate::db::settings::SettingsRepo::get_value(pool, "scraper.run_on_startup")
            .await?
            .unwrap_or(false);
    let saved_global_next: Option<String> =
        crate::db::settings::SettingsRepo::get_value(pool, "scraper.next_run_at")
            .await?
            .flatten();
    let migration_now = chrono::Utc::now();
    let global_next = saved_global_next
        .filter(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok())
        .or_else(|| {
            crate::services::scheduler::calculate_next_run_at(&cron, &timezone, migration_now)
        });
    let global = WorkerFilters {
        include_keywords: crate::db::settings::SettingsRepo::get_value(
            pool,
            "filters.include_keywords",
        )
        .await?
        .unwrap_or_default(),
        exclude_keywords: crate::db::settings::SettingsRepo::get_value(
            pool,
            "filters.exclude_keywords",
        )
        .await?
        .unwrap_or_default(),
        locations: crate::db::settings::SettingsRepo::get_value(pool, "filters.locations")
            .await?
            .unwrap_or_default(),
        role_types: crate::db::settings::SettingsRepo::get_value::<Vec<RoleType>>(
            pool,
            "filters.role_types",
        )
        .await?
        .unwrap_or_default(),
        ..WorkerFilters::default()
    };
    let mut tx = pool.begin().await?;
    let rows: Vec<WorkflowUpgradeRow> = sqlx::query_as(
        "SELECT id, kind, config, filters, cadence_cron, cadence, retention_days, retention_policy, enabled FROM sources ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    for (
        id,
        kind,
        config_text,
        filters_text,
        cadence_cron,
        cadence,
        retention_days,
        retention_policy,
        enabled,
    ) in rows
    {
        let converted = (|| -> Result<WorkflowUpgradeResult> {
            let mut config: Value = serde_json::from_str(&config_text)?;
            let source_filters: Value = serde_json::from_str(&filters_text)?;
            let resolved = crate::services::runner::resolve_legacy_worker_filters(
                &source_filters,
                global.clone(),
            )?;
            let filters = serde_json::to_string(&resolved)?;
            let mut cron_value = cadence_cron;
            let mut effective_cadence = cadence.clone();
            let mut next_run_at = None;
            if cadence == "global" {
                cron_value = Some(cron.clone());
                effective_cadence = "cron".into();
                next_run_at = if enabled { global_next.clone() } else { None };
                config["__cron_timezone"] = Value::String(timezone.clone());
                config["__run_on_startup"] = Value::Bool(startup);
            }
            if let Some(spec) = config.get_mut("__workflow_spec") {
                let spec_version = spec.get("version").and_then(Value::as_u64).unwrap_or(3);
                if spec_version < 4 {
                    spec["version"] = Value::from(4);
                    if !spec.get("schedule").is_some_and(Value::is_object) {
                        spec["schedule"] = serde_json::json!({});
                    }
                    if let Some(schedule) = spec["schedule"].as_object_mut() {
                        schedule.insert("cadence".into(), Value::String(effective_cadence.clone()));
                        if cadence == "global" {
                            schedule.insert("cron".into(), Value::String(cron.clone()));
                            schedule.insert("timezone".into(), Value::String(timezone.clone()));
                            schedule.insert("run_on_startup".into(), Value::Bool(startup));
                        } else if let Some(saved) = cron_value.as_ref() {
                            schedule.insert("cron".into(), Value::String(saved.clone()));
                        }
                    }
                    spec["filters"] = serde_json::to_value(&resolved)?;
                    spec["retention"] = serde_json::json!({
                        "days": retention_days.max(0),
                        "policy": retention_policy,
                    });
                }
            }
            crate::services::workflow::validated_stored_config(
                serde_json::from_value(Value::String(kind.clone()))?,
                &config,
            )?;
            Ok((
                serde_json::to_string(&config)?,
                filters,
                cron_value,
                effective_cadence,
                next_run_at,
            ))
        })();
        match converted {
            Ok((config, filters, cron, cadence, next_run_at)) => {
                let _ = sqlx::query("UPDATE workflows SET config=?, filters=?, cadence=?, cadence_cron=?, next_run_at=?, needs_review=0 WHERE id=?")
                    .bind(&config).bind(&filters).bind(&cadence).bind(&cron).bind(&next_run_at).bind(&id).execute(&mut *tx).await;
                sqlx::query("UPDATE sources SET config=?, filters=?, cadence=?, cadence_cron=?, next_run_at=?, needs_review=0 WHERE id=?")
                    .bind(config).bind(filters).bind(cadence).bind(cron).bind(next_run_at).bind(id).execute(&mut *tx).await?;
            }
            Err(error) => {
                tracing::warn!(source_id=%id, %error, "Workflow requires review after settings upgrade");
                let _ = sqlx::query("UPDATE workflows SET needs_review=1 WHERE id=?")
                    .bind(&id)
                    .execute(&mut *tx)
                    .await;
                sqlx::query("UPDATE sources SET needs_review=1 WHERE id=?")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    sqlx::query("INSERT INTO app_settings(key,value,updated_at) VALUES('migration.workflow_settings_v4','true',datetime('now')) ON CONFLICT(key) DO UPDATE SET value='true', updated_at=datetime('now')")
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn upgrade_workflow_settings_v5(pool: &SqlitePool) -> Result<()> {
    use serde_json::Value;

    let done: Option<bool> =
        crate::db::settings::SettingsRepo::get_value(pool, "migration.workflow_settings_v5")
            .await?;
    if done.is_some() {
        return Ok(());
    }

    let timezone: String = crate::db::settings::SettingsRepo::get_value(pool, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());

    let mut tx = pool.begin().await?;
    let rows: Vec<(String, String, i64, String)> = sqlx::query_as(
        "SELECT w.id, COALESCE(s.config, w.config), w.retention_days, w.retention_policy FROM workflows w LEFT JOIN sources s ON s.id = w.id ORDER BY w.id",
    )
    .fetch_all(&mut *tx)
    .await?;

    for (id, config_text, retention_days, retention_policy) in rows {
        let converted = (|| -> Result<(String, u32)> {
            let mut config: Value = serde_json::from_str(&config_text)?;
            if let Some(mut spec_val) = config.get("__workflow_spec").cloned() {
                if let Some(spec_obj) = spec_val.as_object_mut() {
                    if spec_obj.get("retention").is_none() {
                        spec_obj.insert(
                            "retention".into(),
                            serde_json::json!({
                                "days": retention_days.max(0),
                                "policy": retention_policy,
                            }),
                        );
                    }
                }
                let mut validated_spec = crate::services::workflow::validate(spec_val, &timezone)?;
                if validated_spec.sources.len() == 1 && validated_spec.sources[0].id == "default" {
                    validated_spec.sources[0].id = id.clone();
                }
                // Preserve the historical v5 storage backfill. New reads/saves
                // normalize through the current resolver without rewriting old rows.
                validated_spec.version = 5;
                let max_jobs = validated_spec.max_jobs_per_run.unwrap_or(20);
                config["__workflow_spec"] = serde_json::to_value(&validated_spec)?;
                Ok((serde_json::to_string(&config)?, max_jobs))
            } else {
                Ok((config_text.clone(), 20))
            }
        })();

        match converted {
            Ok((new_config, max_jobs)) => {
                let _ = sqlx::query(
                    "UPDATE workflows SET config = ?, max_jobs_per_run = ?, needs_review = 0 WHERE id = ?",
                )
                .bind(&new_config)
                .bind(max_jobs as i64)
                .bind(&id)
                .execute(&mut *tx)
                .await;

                let _ = sqlx::query(
                    "UPDATE sources SET config = ?, needs_review = 0 WHERE workflow_id = ? OR id = ?",
                )
                .bind(&new_config)
                .bind(&id)
                .bind(&id)
                .execute(&mut *tx)
                .await;
            }
            Err(err) => {
                tracing::warn!(workflow_id=%id, %err, "Workflow requires review after v5 upgrade");
                let _ = sqlx::query("UPDATE workflows SET needs_review = 1 WHERE id = ?")
                    .bind(&id)
                    .execute(&mut *tx)
                    .await;
                let _ = sqlx::query(
                    "UPDATE sources SET needs_review = 1 WHERE workflow_id = ? OR id = ?",
                )
                .bind(&id)
                .bind(&id)
                .execute(&mut *tx)
                .await;
            }
        }
    }

    sqlx::query(
        "INSERT INTO app_settings(key,value,updated_at) VALUES('migration.workflow_settings_v5','true',datetime('now')) ON CONFLICT(key) DO UPDATE SET value='true', updated_at=datetime('now')",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
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
    use serde_json::Value;
    use sqlx::Row;

    #[tokio::test]
    async fn workflow_settings_upgrade_materializes_inheritance_and_isolates_invalid_rows() {
        let pool = init_memory_db().await.unwrap();
        sqlx::query("DELETE FROM app_settings WHERE key='migration.workflow_settings_v4'")
            .execute(&pool)
            .await
            .unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "scraper.cron",
            serde_json::json!("0 15 9 * * *"),
        )
        .await
        .unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "system.timezone",
            serde_json::json!("America/New_York"),
        )
        .await
        .unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "scraper.run_on_startup",
            serde_json::json!(true),
        )
        .await
        .unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "scraper.next_run_at",
            serde_json::json!("2026-10-14T08:00:00Z"),
        )
        .await
        .unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "filters.include_keywords",
            serde_json::json!(["engineer"]),
        )
        .await
        .unwrap();
        let legacy = serde_json::json!({"__workflow_spec": {
            "version": 3,
            "metadata": {"name":"Saved workflow"},
            "extractor": {"kind":"greenhouse", "url":"https://boards.greenhouse.io/acme"},
            "schedule": {"cadence":"global"}
        }, "url":"https://boards.greenhouse.io/acme"});
        sqlx::query("INSERT INTO sources (id,kind,name,identifier,config,enabled,created_at,cadence,cadence_cron,next_run_at,retention_days,retention_policy,filters,needs_review) VALUES ('legacy','greenhouse','Legacy','acme',?,1,'2026-01-01T00:00:00Z','global',NULL,NULL,45,'untracked_only','{\"include_keywords\":[]}',0)")
            .bind(legacy.to_string()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO sources (id,kind,name,identifier,config,enabled,created_at,cadence,cadence_cron,next_run_at,retention_days,retention_policy,filters,needs_review) VALUES ('invalid','greenhouse','Invalid','bad',?,1,'2026-01-01T00:00:00Z','global',NULL,NULL,30,'untracked_only','not-json',0)")
            .bind(serde_json::json!({"url":"https://boards.greenhouse.io/bad"}).to_string()).execute(&pool).await.unwrap();

        upgrade_workflow_settings_v4(&pool).await.unwrap();
        let converted: (String, Option<String>, String, String, i64, Option<String>) = sqlx::query_as("SELECT cadence,cadence_cron,config,filters,needs_review,next_run_at FROM sources WHERE id='legacy'").fetch_one(&pool).await.unwrap();
        assert_eq!(converted.0, "cron");
        assert_eq!(converted.1.as_deref(), Some("0 15 9 * * *"));
        assert_eq!(converted.4, 0);
        assert_eq!(converted.5.as_deref(), Some("2026-10-14T08:00:00Z"));
        let filters: Value = serde_json::from_str(&converted.3).unwrap();
        assert_eq!(filters["include_keywords"], serde_json::json!([]));
        assert_eq!(filters["exclude_keywords"], serde_json::json!([]));
        let config: Value = serde_json::from_str(&converted.2).unwrap();
        assert_eq!(config["__run_on_startup"], true);
        assert_eq!(config["__cron_timezone"], "America/New_York");
        let spec = &config["__workflow_spec"];
        assert_eq!(spec["version"], 4);
        assert_eq!(spec["retention"]["days"], 45);
        let invalid: i64 =
            sqlx::query_scalar("SELECT needs_review FROM sources WHERE id='invalid'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(invalid, 1);
        let visible = crate::db::sources::SourcesRepo::list(&pool).await.unwrap();
        assert!(
            visible
                .iter()
                .find(|source| source.id == "invalid")
                .unwrap()
                .needs_review
        );
    }

    #[tokio::test]
    async fn version_twenty_upgrade_runs_v21_conversion_with_backup_and_restart_idempotence() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow-upgrade.db");
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
        let mut historical = history::manifest(&[]);
        historical.migrations = historical
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&pool).await.unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "scraper.cron",
            serde_json::json!("0 15 9 * * *"),
        )
        .await
        .unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "system.timezone",
            serde_json::json!("America/New_York"),
        )
        .await
        .unwrap();
        let legacy_config = serde_json::json!({
            "url":"https://boards.greenhouse.io/acme",
            "__workflow_spec": {
                "version":3,
                "metadata":{"name":"Historical workflow"},
                "extractor":{"kind":"greenhouse", "url":"https://boards.greenhouse.io/acme"},
                "schedule":{"cadence":"global"}
            }
        });
        sqlx::query("INSERT INTO sources (id,kind,name,identifier,config,enabled,created_at,cadence,retention_days,retention_policy,filters) VALUES ('historical','greenhouse','Historical','acme',?,1,'2026-01-01T00:00:00Z','global',60,'untracked_only','{}')")
            .bind(legacy_config.to_string())
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let upgraded = init_db(&path).await.unwrap();
        let (cadence, config, needs_review): (String, String, i64) =
            sqlx::query_as("SELECT cadence,config,needs_review FROM sources WHERE id='historical'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(cadence, "cron");
        assert_eq!(needs_review, 0);
        let config: Value = serde_json::from_str(&config).unwrap();
        assert_eq!(config["__workflow_spec"]["version"], 5);
        assert_eq!(config["__workflow_spec"]["retention"]["days"], 60);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            sqlx::migrate!("./migrations")
                .iter()
                .last()
                .unwrap()
                .version
        );
        let backup_count = std::fs::read_dir(directory.path())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")
            })
            .count();
        assert_eq!(backup_count, 1);
        upgraded.close().await;
        let restarted = init_db(&path).await.unwrap();
        assert_eq!(
            std::fs::read_dir(directory.path())
                .unwrap()
                .flatten()
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration"))
                .count(),
            1
        );
        restarted.close().await;
    }

    #[tokio::test]
    async fn migration_nineteen_histories_preserve_saved_state_and_metadata() {
        for (legacy_twelve, legacy) in [(false, true), (false, false), (true, true), (true, false)]
        {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("version19.db");
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&path)
                        .create_if_missing(true),
                )
                .await
                .unwrap();
            let legacy12 = sqlx::migrate::Migration::new(
                12,
                "posting removal".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0012_posting_removal.sql").into(),
                false,
            );
            let applied = if legacy_twelve {
                vec![(12, legacy12.checksum.to_vec(), true)]
            } else {
                vec![]
            };
            let mut manifest = history::manifest(&applied);
            manifest.migrations = manifest.iter().take(19).map(|migration| {
                if legacy && migration.version == 19 {
                    sqlx::migrate::Migration::new(19, migration.description.clone(), migration.migration_type,
                        "ALTER TABLE jobs ADD COLUMN description_source TEXT;\nALTER TABLE jobs ADD COLUMN description_json TEXT;\n".into(), false)
                } else { migration.clone() }
            }).collect::<Vec<_>>().into();
            manifest.run(&pool).await.unwrap();
            sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at,enabled,cadence) VALUES ('saved-source','greenhouse','Saved workflow','acme','2026-01-01',0,'manual')").execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,description_source,description_json) VALUES ('saved','saved-source:42','saved-source','42','Engineer','Acme','full_time','https://example.com/42','2026-01-01','2026-01-01','applied','2026-01-01','Preserved note',1,'<p>Evidence</p>','{}')").execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO job_status_events(job_id,from_status,to_status,changed_at) VALUES ('saved','scraped','applied','2026-01-01')").execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO app_settings VALUES ('custom.preserved','42','2026-01-01')")
                .execute(&pool)
                .await
                .unwrap();
            let before: Vec<(i64, Vec<u8>, String)> = sqlx::query_as(
                "SELECT version,checksum,installed_on FROM _sqlx_migrations ORDER BY version",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            pool.close().await;
            let upgraded = init_db(&path).await.unwrap();
            let after: Vec<(i64, Vec<u8>, String)> = sqlx::query_as("SELECT version,checksum,installed_on FROM _sqlx_migrations WHERE version<=19 ORDER BY version").fetch_all(&upgraded).await.unwrap();
            assert_eq!(before, after);
            let saved: (String, bool, String, String, String) = sqlx::query_as("SELECT notes,favorited,status,description_source,description_json FROM jobs WHERE id='saved'").fetch_one(&upgraded).await.unwrap();
            assert_eq!(
                saved,
                (
                    "Preserved note".into(),
                    true,
                    "applied".into(),
                    "<p>Evidence</p>".into(),
                    "{}".into()
                )
            );
            let format: Option<String> =
                sqlx::query_scalar("SELECT description_format FROM jobs WHERE id='saved'")
                    .fetch_one(&upgraded)
                    .await
                    .unwrap();
            assert_eq!(format, None);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM job_status_events WHERE job_id='saved'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                1
            );
            assert_eq!(
                sqlx::query_scalar::<_, String>(
                    "SELECT value FROM app_settings WHERE key='custom.preserved'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                "42"
            );
            assert!(std::fs::read_dir(directory.path())
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")));
            upgraded.close().await;
            init_db(&path).await.unwrap().close().await;
        }
    }

    #[tokio::test]
    async fn migration_nineteen_unknown_or_drift_is_rejected_without_writes() {
        for drift in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("rejected19.db");
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&path)
                        .create_if_missing(true),
                )
                .await
                .unwrap();
            let legacy = sqlx::migrate::Migration::new(
                19,
                "structured job descriptions".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0019_structured_job_descriptions.sql")
                    .into(),
                false,
            );
            let mut manifest = history::manifest(&[(19, legacy.checksum.to_vec(), true)]);
            manifest.migrations = manifest.iter().take(19).cloned().collect::<Vec<_>>().into();
            manifest.run(&pool).await.unwrap();
            if drift {
                sqlx::query("ALTER TABLE jobs ADD COLUMN unexpected TEXT")
                    .execute(&pool)
                    .await
                    .unwrap();
            } else {
                sqlx::query("UPDATE _sqlx_migrations SET checksum=X'12345678' WHERE version=19")
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            pool.close().await;
            let before = std::fs::read(&path).unwrap();
            assert!(init_db(&path).await.is_err());
            assert_eq!(before, std::fs::read(&path).unwrap());
            assert!(!std::fs::read_dir(directory.path())
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")));
        }
    }

    #[tokio::test]
    async fn migration_twenty_three_histories_preserve_saved_state_and_metadata() {
        use scrape_core::branding::{
            cache_asset, relationship_key, CompanyLogo, LogoEvidence, LogoMethod, LogoStatus,
        };
        let evidence = LogoEvidence {
            url: "https://fixture.test/logo.svg".into(),
            page_url: "https://fixture.test/jobs".into(),
            method: LogoMethod::EmployerLabel,
        };
        let asset_data = cache_asset(
            br##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><path fill="#f80" d="M0 0h80v20H0z"/></svg>"##,
        )
        .unwrap();
        let mut verified = CompanyLogo {
            evidence: Some(evidence),
            checked_at: "2026-10-16T00:00:00Z".into(),
            status: LogoStatus::Verified,
            cached_asset_reference: None,
            asset: Some(asset_data.clone()),
            diagnostic: None,
        };
        let reference = verified.asset_reference().unwrap();
        verified.cached_asset_reference = Some(reference.clone());

        for (legacy_twelve, legacy_nineteen, legacy_twenty_three) in [
            (false, false, true),
            (false, false, false),
            (false, true, true),
            (false, true, false),
            (true, false, true),
            (true, false, false),
            (true, true, true),
            (true, true, false),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("version23.db");
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
            let legacy12 = sqlx::migrate::Migration::new(
                12,
                "posting removal".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0012_posting_removal.sql").into(),
                false,
            );
            let legacy19 = sqlx::migrate::Migration::new(
                19,
                "structured job descriptions".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0019_structured_job_descriptions.sql")
                    .into(),
                false,
            );
            let legacy23 = sqlx::migrate::Migration::new(
                23,
                "employer branding".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0023_employer_branding.sql").into(),
                false,
            );
            let mut applied = Vec::new();
            if legacy_twelve {
                applied.push((12, legacy12.checksum.to_vec(), true));
            }
            if legacy_nineteen {
                applied.push((19, legacy19.checksum.to_vec(), true));
            }
            if legacy_twenty_three {
                applied.push((23, legacy23.checksum.to_vec(), true));
            }
            let mut manifest = history::manifest(&applied);
            manifest.migrations = manifest
                .iter()
                .take(23)
                .map(|migration| {
                    if legacy_twenty_three && migration.version == 23 {
                        legacy23.clone()
                    } else {
                        migration.clone()
                    }
                })
                .collect::<Vec<_>>()
                .into();
            manifest.run(&pool).await.unwrap();
            sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at,enabled,cadence) VALUES ('saved-source','greenhouse','Saved workflow','acme','2026-01-01',0,'manual')").execute(&pool).await.unwrap();
            let branding_key = relationship_key("saved-source", "Acme");
            if legacy_twenty_three {
                let inline_json = serde_json::to_string(&verified).unwrap();
                sqlx::query("INSERT INTO employer_branding(key,source_id,company,logo_json,last_attempt_json,asset_reference,updated_at) VALUES (?,?,?,?,?,?,?)")
                    .bind(&branding_key)
                    .bind("saved-source")
                    .bind("Acme")
                    .bind(&inline_json)
                    .bind(&inline_json)
                    .bind(&reference)
                    .bind("2026-10-16T00:00:00Z")
                    .execute(&pool)
                    .await
                    .unwrap();
            } else {
                branding::record(&pool, "saved-source", "Acme", &verified)
                    .await
                    .unwrap();
            }
            sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,description_source,description_json,company_branding_key) VALUES ('saved','saved-source:42','saved-source','42','Engineer','Acme','full_time','https://example.com/42','2026-01-01','2026-01-01','applied','2026-01-01','Preserved note',1,'<p>Evidence</p>','{}',?)")
                .bind(&branding_key)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO job_status_events(job_id,from_status,to_status,changed_at) VALUES ('saved','scraped','applied','2026-01-01')").execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO app_settings VALUES ('custom.preserved','42','2026-01-01')")
                .execute(&pool)
                .await
                .unwrap();
            let before: Vec<(i64, Vec<u8>, String)> = sqlx::query_as(
                "SELECT version,checksum,installed_on FROM _sqlx_migrations ORDER BY version",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            // Keep pool open or close without checkpointing WAL to exercise uncheckpointed WAL state.
            let upgraded = init_db(&path).await.unwrap();
            let after: Vec<(i64, Vec<u8>, String)> = sqlx::query_as(
                "SELECT version,checksum,installed_on FROM _sqlx_migrations WHERE version<=23 ORDER BY version",
            )
            .fetch_all(&upgraded)
            .await
            .unwrap();
            assert_eq!(before, after);
            let saved: (String, bool, String, String, String) = sqlx::query_as(
                "SELECT notes,favorited,status,description_source,description_json FROM jobs WHERE id='saved'",
            )
            .fetch_one(&upgraded)
            .await
            .unwrap();
            assert_eq!(
                saved,
                (
                    "Preserved note".into(),
                    true,
                    "applied".into(),
                    "<p>Evidence</p>".into(),
                    "{}".into()
                )
            );
            assert_eq!(
                branding::asset(&upgraded, &reference).await.unwrap(),
                asset_data
            );
            branding::record(&upgraded, "saved-source", "Acme", &verified)
                .await
                .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM job_status_events WHERE job_id='saved'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                1
            );
            assert_eq!(
                sqlx::query_scalar::<_, String>(
                    "SELECT value FROM app_settings WHERE key='custom.preserved'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                "42"
            );
            assert_eq!(
                std::fs::read_dir(directory.path())
                    .unwrap()
                    .flatten()
                    .filter(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-migration"))
                    .count(),
                1
            );
            upgraded.close().await;
            pool.close().await;
            init_db(&path).await.unwrap().close().await;
            assert_eq!(
                std::fs::read_dir(directory.path())
                    .unwrap()
                    .flatten()
                    .filter(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-migration"))
                    .count(),
                1
            );
        }
    }

    #[tokio::test]
    async fn migration_twenty_three_unknown_or_drift_is_rejected_without_writes() {
        for drift in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("rejected23.db");
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&path)
                        .create_if_missing(true),
                )
                .await
                .unwrap();
            let legacy = sqlx::migrate::Migration::new(
                23,
                "employer branding".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0023_employer_branding.sql").into(),
                false,
            );
            let mut manifest = history::manifest(&[(23, legacy.checksum.to_vec(), true)]);
            manifest.migrations = manifest.iter().take(23).cloned().collect::<Vec<_>>().into();
            manifest.run(&pool).await.unwrap();
            if drift {
                sqlx::query("ALTER TABLE employer_branding ADD COLUMN unexpected TEXT")
                    .execute(&pool)
                    .await
                    .unwrap();
            } else {
                sqlx::query("UPDATE _sqlx_migrations SET checksum=X'12345678' WHERE version=23")
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            pool.close().await;
            let before = std::fs::read(&path).unwrap();
            let err = init_db(&path).await.unwrap_err().to_string();
            if drift {
                assert!(err.contains("schema does not match"));
            } else {
                assert!(err.contains("Migration history mismatch at version 23"));
            }
            assert_eq!(before, std::fs::read(&path).unwrap());
            assert!(!std::fs::read_dir(directory.path())
                .unwrap()
                .any(|entry| entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")));
        }
    }

    #[tokio::test]
    async fn failed_forward_repair_rolls_back_and_can_resume() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("interrupted.db");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let legacy = sqlx::migrate::Migration::new(
            12,
            "posting removal".into(),
            sqlx::migrate::MigrationType::Simple,
            include_str!("../../historical-migrations/0012_posting_removal.sql").into(),
            false,
        );
        let applied = [(12, legacy.checksum.to_vec(), true)];
        let mut manifest = history::manifest(&applied);
        manifest.migrations = manifest
            .iter()
            .map(|migration| {
                if migration.version == 18 {
                    sqlx::migrate::Migration::new(
                        18,
                        migration.description.clone(),
                        migration.migration_type,
                        format!("{}; SELECT * FROM missing_failure_fixture;", migration.sql).into(),
                        false,
                    )
                } else {
                    migration.clone()
                }
            })
            .collect::<Vec<_>>()
            .into();
        assert!(manifest.run(&pool).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .unwrap(),
            17
        );
        let names: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('jobs')")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert!(!names.contains(&"source_name_snapshot".into()));
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM sqlite_master WHERE name='posting_legacy_blocks'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        let destination = directory.path().join("occupied.db");
        std::fs::write(&destination, b"Do not replace").unwrap();
        assert!(backup_to(&pool, &destination).await.is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"Do not replace");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .unwrap(),
            17
        );
        pool.close().await;
        let restarted = init_db(&path).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&restarted)
                .await
                .unwrap(),
            sqlx::migrate!("./migrations")
                .iter()
                .last()
                .unwrap()
                .version
        );
        assert_eq!(
            sqlx::query_scalar::<_, Vec<u8>>(
                "SELECT checksum FROM _sqlx_migrations WHERE version=12"
            )
            .fetch_one(&restarted)
            .await
            .unwrap(),
            legacy.checksum.to_vec()
        );
    }

    #[tokio::test]
    async fn both_version_twelve_histories_upgrade_preserving_state_and_checksums() {
        for legacy in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("historical.db");
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
            let legacy_migration = sqlx::migrate::Migration::new(
                12,
                "posting removal".into(),
                sqlx::migrate::MigrationType::Simple,
                include_str!("../../historical-migrations/0012_posting_removal.sql").into(),
                false,
            );
            let applied = if legacy {
                vec![(12, legacy_migration.checksum.to_vec(), true)]
            } else {
                vec![]
            };
            let mut manifest = history::manifest(&applied);
            manifest.migrations = manifest.iter().take(12).cloned().collect::<Vec<_>>().into();
            manifest.run(&pool).await.unwrap();
            sqlx::query("INSERT INTO sources (id,kind,name,identifier,created_at,filters) VALUES ('preserved','greenhouse','Preserved workflow','acme','2026-01-01',?)")
                .bind(r#"{"include_keywords":[],"locations":["remote"]}"#).execute(&pool).await.unwrap();
            for (index, status) in ["applied", "interviewing", "offer", "rejected"]
                .iter()
                .enumerate()
            {
                sqlx::query("INSERT INTO jobs (id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,removed_at) VALUES (?,?, 'preserved',?, 'Engineer','Acme','full_time',?,'2026-01-01','2026-01-01',?,'2026-01-01','Saved note',1,?)")
                    .bind(format!("saved-{index}")).bind(format!("preserved:{index}")).bind(index.to_string())
                    .bind(format!("https://example.com/posting/{index}")).bind(status)
                    .bind((index == 0).then_some("2026-01-02"))
                    .execute(&pool).await.unwrap();
                sqlx::query("INSERT INTO job_status_events (job_id,from_status,to_status,changed_at) VALUES (?,'scraped',?,'2026-01-01')")
                    .bind(format!("saved-{index}")).bind(status).execute(&pool).await.unwrap();
            }
            sqlx::query("INSERT INTO blocked_postings VALUES ('preserved-block','Removed engineer','Acme','https://example.com/blocked','Preserved workflow','2026-01-01',NULL)")
                .execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO deleted_jobs VALUES ('preserved:99','2026-01-01')")
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO app_settings VALUES ('custom.preserved','42','2026-01-01')")
                .execute(&pool)
                .await
                .unwrap();
            let before: Vec<(i64, Vec<u8>, String)> = sqlx::query_as(
                "SELECT version,checksum,installed_on FROM _sqlx_migrations ORDER BY version",
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            let upgraded = init_db(&path).await.unwrap();
            let after: Vec<(i64, Vec<u8>, String)> = sqlx::query_as("SELECT version,checksum,installed_on FROM _sqlx_migrations WHERE version <= 12 ORDER BY version").fetch_all(&upgraded).await.unwrap();
            assert_eq!(before, after);
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                    .fetch_one(&upgraded)
                    .await
                    .unwrap(),
                sqlx::migrate!("./migrations")
                    .iter()
                    .last()
                    .unwrap()
                    .version
            );
            let states: Vec<(String, bool, String, Option<String>, String)> = sqlx::query_as("SELECT notes,favorited,status,removed_at,source_name_snapshot FROM jobs ORDER BY id").fetch_all(&upgraded).await.unwrap();
            assert_eq!(states.len(), 4);
            for (index, row) in states.iter().enumerate() {
                assert_eq!(row.0, "Saved note");
                assert!(row.1);
                assert_eq!(
                    row.2,
                    ["applied", "interviewing", "offer", "rejected"][index]
                );
                assert_eq!(row.3.is_some(), index == 0);
                assert_eq!(row.4, "Preserved workflow");
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM job_status_events")
                    .fetch_one(&upgraded)
                    .await
                    .unwrap(),
                4
            );
            assert_eq!(
                sqlx::query_scalar::<_, String>(
                    "SELECT value FROM app_settings WHERE key='custom.preserved'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                "42"
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM blocked_postings WHERE identity='preserved-block'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                1
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM posting_legacy_blocks WHERE legacy_key='preserved:99'"
                )
                .fetch_one(&upgraded)
                .await
                .unwrap(),
                1
            );
            // The backup includes committed WAL data and the exact pre-repair history.
            let backups: Vec<_> = std::fs::read_dir(directory.path())
                .unwrap()
                .flatten()
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-migration")
                })
                .collect();
            assert_eq!(backups.len(), 1);
            let backup = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(backups[0].path())
                        .read_only(true),
                )
                .await
                .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs")
                    .fetch_one(&backup)
                    .await
                    .unwrap(),
                4
            );
            assert_eq!(
                sqlx::query_scalar::<_, Vec<u8>>(
                    "SELECT checksum FROM _sqlx_migrations WHERE version=12"
                )
                .fetch_one(&backup)
                .await
                .unwrap(),
                before[11].1
            );
            backup.close().await;
            upgraded.close().await;
            pool.close().await;
            let restarted = init_db(&path).await.unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM posting_legacy_blocks")
                    .fetch_one(&restarted)
                    .await
                    .unwrap(),
                1
            );
            restarted.close().await;
            assert_eq!(
                std::fs::read_dir(directory.path())
                    .unwrap()
                    .flatten()
                    .filter(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-migration"))
                    .count(),
                1
            );
        }
    }

    #[tokio::test]
    async fn recognized_legacy_checksum_does_not_accept_schema_drift() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let legacy = sqlx::migrate::Migration::new(
            12,
            "posting removal".into(),
            sqlx::migrate::MigrationType::Simple,
            include_str!("../../historical-migrations/0012_posting_removal.sql").into(),
            false,
        );
        let mut manifest = history::manifest(&[(12, legacy.checksum.to_vec(), true)]);
        manifest.migrations = manifest.iter().take(12).cloned().collect::<Vec<_>>().into();
        manifest.run(&pool).await.unwrap();
        sqlx::query("ALTER TABLE jobs ADD COLUMN unknown_data TEXT")
            .execute(&pool)
            .await
            .unwrap();
        assert!(run_migrations(&pool)
            .await
            .unwrap_err()
            .to_string()
            .contains("schema does not match"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .unwrap(),
            12
        );
    }

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

#[cfg(test)]
mod execution_outcome_upgrade_tests {
    use super::*;
    use crate::db::{
        runs::{RunCounts, RunStatus, RunsRepo, TriggerType},
        sources::SourcesRepo,
    };

    #[tokio::test]
    async fn populated_v21_upgrade_keeps_state_checksums_backup_and_restart_after_failure() {
        for inject_failure in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("outcomes.db");
            let original = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(&path)
                        .create_if_missing(true)
                        .journal_mode(SqliteJournalMode::Wal)
                        .foreign_keys(true),
                )
                .await
                .unwrap();
            let mut historical = sqlx::migrate!("./migrations");
            historical.migrations = historical
                .iter()
                .filter(|m| m.version <= 21)
                .cloned()
                .collect::<Vec<_>>()
                .into();
            historical.run(&original).await.unwrap();
            sqlx::query("INSERT INTO app_settings(key,value,updated_at) VALUES ('migration.workflow_settings_v4','true','2026-01-01')").execute(&original).await.unwrap();
            sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at,enabled,cadence,next_run_at,last_status,last_error,config) VALUES ('source','greenhouse','Fixture','board','2026-01-01',1,'daily','2030-01-01T09:00:00Z','error','Historical failure','{}')").execute(&original).await.unwrap();
            sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited) VALUES ('job','source:42','source','42','Engineer','Fixture','full_time','https://fixture.test/42','2026-01-01','2026-01-01','applied','2026-01-01','Protected note',1)").execute(&original).await.unwrap();
            sqlx::query("INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES ('job','applied','2026-01-01')").execute(&original).await.unwrap();
            sqlx::query("INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES ('protected-block','Blocked job','Fixture','2026-01-01')").execute(&original).await.unwrap();
            sqlx::query("INSERT INTO scrape_runs(id,trigger,status,started_at,finished_at,jobs_found,jobs_new,source_id,counts_json) VALUES ('historical','single_source','partial','2026-01-01','2026-01-01',2,1,'source','{\"fetched\":2,\"inserted\":1}')").execute(&original).await.unwrap();
            sqlx::query("INSERT INTO scrape_logs(id,run_id,timestamp,level,message) VALUES ('log','historical','2026-01-01','warn','Historical log')").execute(&original).await.unwrap();
            let checksums: Vec<(i64, String)> = sqlx::query_as(
                "SELECT version,hex(checksum) FROM _sqlx_migrations ORDER BY version",
            )
            .fetch_all(&original)
            .await
            .unwrap();
            if inject_failure {
                let mut failing = sqlx::migrate!("./migrations");
                failing.migrations = failing
                    .iter()
                    .map(|migration| {
                        if migration.version == 22 {
                            sqlx::migrate::Migration::new(
                                22,
                                migration.description.clone(),
                                migration.migration_type,
                                format!(
                                    "{}\nSELECT missing_column FROM missing_table;",
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
                assert!(failing.run(&original).await.is_err());
                let columns: Vec<String> =
                    sqlx::query_scalar("SELECT name FROM pragma_table_info('scrape_runs')")
                        .fetch_all(&original)
                        .await
                        .unwrap();
                assert!(!columns.contains(&"outcome".to_string()));
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                        .fetch_one(&original)
                        .await
                        .unwrap(),
                    21
                );
            }
            let upgraded = init_db(&path).await.unwrap();
            let backups: Vec<_> = std::fs::read_dir(directory.path())
                .unwrap()
                .flatten()
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-migration")
                })
                .collect();
            assert_eq!(backups.len(), 1);
            let backup = SqlitePoolOptions::new()
                .max_connections(1)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(backups[0].path())
                        .read_only(true),
                )
                .await
                .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                    .fetch_one(&backup)
                    .await
                    .unwrap(),
                21
            );
            for pool in [&backup, &upgraded] {
                let state: (String, bool, String) =
                    sqlx::query_as("SELECT notes,favorited,status FROM jobs WHERE id='job'")
                        .fetch_one(pool)
                        .await
                        .unwrap();
                assert_eq!(state, ("Protected note".into(), true, "applied".into()));
                let schedule: (String, String, String) = sqlx::query_as(
                    "SELECT cadence,next_run_at,last_error FROM sources WHERE id='source'",
                )
                .fetch_one(pool)
                .await
                .unwrap();
                assert_eq!(
                    schedule,
                    (
                        "daily".into(),
                        "2030-01-01T09:00:00Z".into(),
                        "Historical failure".into()
                    )
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM job_status_events")
                        .fetch_one(pool)
                        .await
                        .unwrap(),
                    1
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM blocked_postings")
                        .fetch_one(pool)
                        .await
                        .unwrap(),
                    1
                );
                assert_eq!(
                    sqlx::query_scalar::<_, String>(
                        "SELECT message FROM scrape_logs WHERE id='log'"
                    )
                    .fetch_one(pool)
                    .await
                    .unwrap(),
                    "Historical log"
                );
                assert!(sqlx::query("PRAGMA foreign_key_check")
                    .fetch_all(pool)
                    .await
                    .unwrap()
                    .is_empty());
                let prefix:Vec<(i64,String)>=sqlx::query_as("SELECT version,hex(checksum) FROM _sqlx_migrations WHERE version<=21 ORDER BY version").fetch_all(pool).await.unwrap();
                assert_eq!(prefix, checksums);
            }
            assert_eq!(
                RunsRepo::get(&upgraded, "historical").await.unwrap().status,
                RunStatus::Partial
            );
            assert!(SourcesRepo::get(&upgraded, "source")
                .await
                .unwrap()
                .last_outcome
                .is_none());
            for (outcome, source_status) in [
                (RunStatus::SuccessWithWarnings, "success_with_warnings"),
                (RunStatus::Cancelled, "cancelled"),
                (RunStatus::Partial, "partial"),
            ] {
                let run = RunsRepo::start_with_source(
                    &upgraded,
                    TriggerType::SingleSource,
                    Some("source"),
                )
                .await
                .unwrap();
                let counts = RunCounts {
                    inserted: 1,
                    enrichment_warnings: Some(2),
                    ..Default::default()
                };
                RunsRepo::record_counts(&upgraded, &run.id, &counts)
                    .await
                    .unwrap();
                RunsRepo::finish(
                    &upgraded,
                    &run.id,
                    outcome,
                    1,
                    1,
                    Some("Diagnostic fixture"),
                )
                .await
                .unwrap();
                SourcesRepo::set_run_result(
                    &upgraded,
                    "source",
                    source_status,
                    Some("Diagnostic fixture"),
                    Some(5),
                    Some("2030-01-01T09:00:00Z"),
                )
                .await
                .unwrap();
                assert_eq!(
                    RunsRepo::get(&upgraded, &run.id).await.unwrap().status,
                    outcome
                );
                assert_eq!(
                    RunsRepo::list_by_source(&upgraded, "source", 100)
                        .await
                        .unwrap()
                        .into_iter()
                        .find(|r| r.id == run.id)
                        .unwrap()
                        .counts,
                    Some(counts)
                );
                assert_eq!(
                    serde_json::to_value(
                        SourcesRepo::get(&upgraded, "source")
                            .await
                            .unwrap()
                            .last_outcome
                    )
                    .unwrap(),
                    serde_json::json!(source_status)
                );
            }
            backup.close().await;
            upgraded.close().await;
            original.close().await;
            let restarted = init_db(&path).await.unwrap();
            assert_eq!(
                SourcesRepo::get(&restarted, "source")
                    .await
                    .unwrap()
                    .last_outcome,
                Some(scrape_core::protocol::ScanOutcome::Partial)
            );
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                    .fetch_one(&restarted)
                    .await
                    .unwrap(),
                sqlx::migrate!("./migrations")
                    .iter()
                    .last()
                    .unwrap()
                    .version
            );
            assert_eq!(
                std::fs::read_dir(directory.path())
                    .unwrap()
                    .flatten()
                    .filter(|entry| entry
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-migration"))
                    .count(),
                1
            );
            restarted.close().await;
        }
    }

    #[tokio::test]
    async fn test_populated_migration_0024_multi_source_workflows() {
        use crate::db::jobs::JobsRepo;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("populated_24.db");
        let old = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();

        // Run migrations up to 23
        let mut historical = sqlx::migrate!("./migrations");
        historical.migrations = historical
            .iter()
            .filter(|m| m.version <= 23)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&old).await.unwrap();

        // Populate v4 valid source
        let v4_config = serde_json::json!({
            "url": "https://boards.greenhouse.io/acme",
            "__company": "Acme Corp",
            "__workflow_spec": {
                "version": 4,
                "metadata": {
                    "name": "Acme Engineering",
                    "company": "Acme Corp"
                },
                "extractor": {
                    "kind": "greenhouse",
                    "url": "https://boards.greenhouse.io/acme"
                },
                "schedule": {
                    "cadence": "daily",
                    "time": "09:00",
                    "timezone": "UTC"
                },
                "filters": {
                    "include_keywords": ["engineer"],
                    "exclude_keywords": ["senior"]
                },
                "retention": {
                    "days": 45,
                    "policy": "untracked_only"
                },
                "max_jobs_per_run": 25
            }
        });
        sqlx::query(
                "INSERT INTO sources (id, kind, name, identifier, config, enabled, created_at, cadence, cadence_cron, retention_days, retention_policy, filters)
                 VALUES ('acme-source', 'greenhouse', 'Acme Engineering', 'acme', ?, 1, '2026-10-01T00:00:00Z', 'daily', NULL, 45, 'untracked_only', '{}')"
            )
            .bind(v4_config.to_string())
            .execute(&old)
            .await
            .unwrap();

        // Populate an invalid source to test needs_review preservation
        let broken_config = serde_json::json!({
            "__workflow_spec": {
                "version": 4,
                "metadata": {
                    "name": ""
                },
                "extractor": {
                    "kind": "greenhouse",
                    "url": "https://boards.greenhouse.io/broken"
                }
            }
        });
        sqlx::query(
                "INSERT INTO sources (id, kind, name, identifier, config, enabled, created_at, cadence, cadence_cron, retention_days, retention_policy, filters)
                 VALUES ('broken-source', 'greenhouse', 'Broken Source', 'broken', ?, 1, '2026-10-01T00:00:00Z', 'daily', NULL, 30, 'untracked_only', '{}')"
            )
            .bind(broken_config.to_string())
            .execute(&old)
            .await
            .unwrap();

        // Populate jobs under acme-source
        sqlx::query(
                "INSERT INTO jobs (id, dedupe_key, source_id, external_id, title, company, url, date_added, last_seen_at, status, status_changed_at, notes, favorited)
                 VALUES ('job-1', 'acme:1', 'acme-source', '1', 'Software Engineer', 'Acme Corp', 'https://boards.greenhouse.io/acme/jobs/1', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z', 'scraped', '2026-10-01T00:00:00Z', 'My favorite note', 1)"
            )
            .execute(&old)
            .await
            .unwrap();

        sqlx::query(
                "INSERT INTO jobs (id, dedupe_key, source_id, external_id, title, company, url, date_added, last_seen_at, status, status_changed_at, notes, favorited)
                 VALUES ('job-2', 'acme:2', 'acme-source', '2', 'Product Manager', 'Acme Corp', 'https://boards.greenhouse.io/acme/jobs/2', '2026-10-01T00:00:00Z', '2026-10-01T00:00:00Z', 'applied', '2026-10-01T00:00:00Z', NULL, 0)"
            )
            .execute(&old)
            .await
            .unwrap();

        // Populate scrape_runs
        sqlx::query(
                "INSERT INTO scrape_runs (id, trigger, status, started_at, finished_at, jobs_found, jobs_new, source_id)
                 VALUES ('run-1', 'single_source', 'success', '2026-10-01T00:00:00Z', '2026-10-01T00:05:00Z', 10, 2, 'acme-source')"
            )
            .execute(&old)
            .await
            .unwrap();

        // 1. Failure injection on migration 24: verify rollback
        let mut failing = sqlx::migrate!("./migrations");
        failing.migrations = failing
            .iter()
            .map(|m| {
                if m.version == 24 {
                    sqlx::migrate::Migration::new(
                        24,
                        m.description.clone(),
                        m.migration_type,
                        format!("{}\nSELECT syntax_error FROM nonexistent_table;", m.sql).into(),
                        false,
                    )
                } else {
                    m.clone()
                }
            })
            .collect::<Vec<_>>()
            .into();

        assert!(failing.run(&old).await.is_err());

        // Verify failure rollback: workflows table does not exist, max migration is 23
        let workflows_table_exists: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='workflows'",
        )
        .fetch_one(&old)
        .await
        .unwrap();
        assert_eq!(workflows_table_exists, 0);

        let max_v: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&old)
            .await
            .unwrap();
        assert_eq!(max_v, 23);

        old.close().await;

        // 2. Real upgrade via init_db (runs migrations + upgrade_workflow_settings_v5)
        let upgraded = init_db(&path).await.unwrap();

        // Verify migration 24 and latest migrations applied
        let v24_success: Option<bool> =
            sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version = 24")
                .fetch_optional(&upgraded)
                .await
                .unwrap();
        assert_eq!(v24_success, Some(true));
        let current_v: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&upgraded)
            .await
            .unwrap();
        assert_eq!(
            current_v,
            sqlx::migrate!("./migrations")
                .iter()
                .last()
                .unwrap()
                .version
        );

        // Verify parent workflows table exists and contains acme-source and broken-source
        let workflow_count: i64 = sqlx::query_scalar("SELECT count(*) FROM workflows")
            .fetch_one(&upgraded)
            .await
            .unwrap();
        assert_eq!(workflow_count, 2);

        // Verify acme workflow
        let (w_name, w_max_jobs, w_spec_str, w_needs_review): (String, i64, String, i64) =
                sqlx::query_as("SELECT name, max_jobs_per_run, config, needs_review FROM workflows WHERE id='acme-source'")
                    .fetch_one(&upgraded)
                    .await
                    .unwrap();
        assert_eq!(w_name, "Acme Engineering");
        assert_eq!(w_max_jobs, 25);
        assert_eq!(w_needs_review, 0);
        let w_cfg: serde_json::Value = serde_json::from_str(&w_spec_str).unwrap();
        assert_eq!(w_cfg["__workflow_spec"]["version"], 5);
        assert_eq!(w_cfg["__workflow_spec"]["sources"][0]["id"], "acme-source");
        assert_eq!(
            w_cfg["__workflow_spec"]["sources"][0]["label"],
            "Acme Engineering"
        );

        // Verify broken workflow preserved as needs_review = 1
        let broken_needs_review: i64 =
            sqlx::query_scalar("SELECT needs_review FROM workflows WHERE id='broken-source'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(broken_needs_review, 1);

        // Verify sources table ownership and fields
        let (src_wf_id, src_needs_review): (String, i64) =
            sqlx::query_as("SELECT workflow_id, needs_review FROM sources WHERE id='acme-source'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(src_wf_id, "acme-source");
        assert_eq!(src_needs_review, 0);

        // Verify jobs ownership and user state preservation
        let (j1_wf_id, j1_notes, j1_fav, j1_status): (Option<String>, Option<String>, i64, String) =
            sqlx::query_as(
                "SELECT workflow_id, notes, favorited, status FROM jobs WHERE id='job-1'",
            )
            .fetch_one(&upgraded)
            .await
            .unwrap();
        assert_eq!(j1_wf_id.as_deref(), Some("acme-source"));
        assert_eq!(j1_notes.as_deref(), Some("My favorite note"));
        assert_eq!(j1_fav, 1);
        assert_eq!(j1_status, "scraped");

        let (j2_wf_id, j2_status): (Option<String>, String) =
            sqlx::query_as("SELECT workflow_id, status FROM jobs WHERE id='job-2'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(j2_wf_id.as_deref(), Some("acme-source"));
        assert_eq!(j2_status, "applied");

        // Verify scrape_runs ownership
        let run_wf_id: Option<String> =
            sqlx::query_scalar("SELECT workflow_id FROM scrape_runs WHERE id='run-1'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(run_wf_id.as_deref(), Some("acme-source"));

        // Verify retention pruning by workflow
        let pruned =
            JobsRepo::prune_expired_by_source(&upgraded, "acme-source", 1, "untracked_only")
                .await
                .unwrap();
        assert_eq!(pruned, 0);

        upgraded.close().await;

        // 3. Idempotent restart
        let restarted = init_db(&path).await.unwrap();
        let restarted_wf_count: i64 = sqlx::query_scalar("SELECT count(*) FROM workflows")
            .fetch_one(&restarted)
            .await
            .unwrap();
        assert_eq!(restarted_wf_count, 2);
        restarted.close().await;
    }
}

#[cfg(test)]
mod geography_parity_tests;
