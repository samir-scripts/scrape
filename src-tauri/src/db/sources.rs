use crate::error::{AppError, Result};
use scrape_core::{now_iso, SourceCadence, SourceKind};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    pub name: String,
    pub identifier: String,
    pub config: serde_json::Value,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub last_outcome: Option<scrape_core::protocol::ScanOutcome>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub cadence: SourceCadence,
    pub cadence_cron: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_duration_ms: Option<i64>,
    pub retention_days: i64,
    pub retention_policy: String,
    pub filters: serde_json::Value,
    pub needs_review: bool,
    #[serde(default)]
    pub script_trusted: bool,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub script_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateSourceInput {
    pub kind: SourceKind,
    pub name: String,
    pub identifier: String,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub cadence: Option<SourceCadence>,
    pub cadence_cron: Option<String>,
    pub retention_days: Option<i64>,
    pub retention_policy: Option<String>,
    pub filters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSourceInput {
    pub id: String,
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub cadence: Option<SourceCadence>,
    pub cadence_cron: Option<String>,
    pub retention_days: Option<i64>,
    pub retention_policy: Option<String>,
    pub filters: Option<serde_json::Value>,
}

fn source_outcome(status: &str) -> Result<(&str, &str)> {
    match status {
        "ok" | "success" | "empty" => Ok((status, "success")),
        "success_with_warnings" => Ok(("success", "success_with_warnings")),
        "partial" => Ok(("error", "partial")),
        "cancelled" => Ok(("error", "cancelled")),
        "error" | "failed" => Ok(("error", "failed")),
        _ => Err(AppError::Validation(format!(
            "Unknown source outcome: {status}"
        ))),
    }
}

pub struct SourcesRepo;

impl SourcesRepo {
    pub async fn mark_reviewed(pool: &SqlitePool, id: &str) -> Result<Source> {
        sqlx::query("UPDATE sources SET needs_review=0 WHERE id=? AND (script_sha256 IS NULL OR script_trusted=1)")
            .bind(id)
            .execute(pool)
            .await?;
        Self::get(pool, id).await
    }

    pub(crate) fn row_to_source(r: &sqlx::sqlite::SqliteRow) -> Result<Source> {
        use sqlx::Row;
        let kind_str: String = r.get("kind");
        let config_str: String = r.get("config");
        let kind: SourceKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
        let parsed_config = serde_json::from_str::<serde_json::Value>(&config_str);
        let mut config = parsed_config
            .ok()
            .and_then(|config| crate::services::workflow::normalize_source_config(&config).ok())
            .unwrap_or_else(|| serde_json::json!({"__invalid_original_config": config_str}));
        let enabled_int: i64 = r.get("enabled");
        let cadence_str: Option<String> = r.get("cadence");
        let cadence: SourceCadence = match cadence_str {
            Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
            None => SourceCadence::Global,
        };
        let retention_days: i64 = r.try_get("retention_days")?;
        let retention_policy: String = r.try_get("retention_policy")?;
        let filters_str: String = r.try_get("filters")?;
        let (filters, invalid_filters) = match serde_json::from_str(&filters_str) {
            Ok(filters) => (filters, false),
            Err(_) => {
                config["__invalid_original_filters"] = serde_json::Value::String(filters_str);
                (serde_json::json!({}), true)
            }
        };
        let needs_review = r.get::<i64, _>("needs_review") == 1
            || r.try_get::<i64, _>("workflow_needs_review").unwrap_or(0) == 1
            || invalid_filters
            || config.get("__invalid_original_config").is_some();

        Ok(Source {
            id: r.get("id"),
            kind,
            name: r.get("name"),
            identifier: r.get("identifier"),
            config,
            enabled: enabled_int == 1,
            last_run_at: r.get("last_run_at"),
            last_status: r.get("last_status"),
            last_outcome: r
                .get::<Option<String>, _>("last_outcome")
                .map(|value| serde_json::from_value(serde_json::Value::String(value)))
                .transpose()?,
            last_error: r.get("last_error"),
            created_at: r.get("created_at"),
            cadence,
            cadence_cron: r.get("cadence_cron"),
            next_run_at: r.get("next_run_at"),
            last_run_duration_ms: r.get("last_run_duration_ms"),
            retention_days,
            retention_policy,
            filters,
            needs_review,
            script_trusted: r.get::<i64, _>("script_trusted") == 1,
            script_sha256: r.get("script_sha256"),
        })
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<Source>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_outcome, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters, needs_review, script_trusted, script_sha256,
                   COALESCE((SELECT needs_review FROM workflows WHERE workflows.id=sources.id),0) AS workflow_needs_review
            FROM sources
            WHERE workflow_id = id OR workflow_id IS NULL
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut sources = Vec::new();
        for r in rows {
            sources.push(Self::row_to_source(&r)?);
        }

        Ok(sources)
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Source> {
        Self::get_with(id, pool).await
    }

    pub(crate) async fn get_with<'e, E>(id: &str, executor: E) -> Result<Source>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_outcome, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters, needs_review, script_trusted, script_sha256,
                   COALESCE((SELECT needs_review FROM workflows WHERE workflows.id=sources.id),0) AS workflow_needs_review
            FROM sources
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Source {} not found", id)))?;
        Self::row_to_source(&r)
    }

    pub(crate) async fn get_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        id: &str,
    ) -> Result<Source> {
        Self::get_with(id, &mut **tx).await
    }

    async fn next_run_for_schedule(
        pool: &SqlitePool,
        cadence: SourceCadence,
        cron: Option<&str>,
        enabled: bool,
        config: &serde_json::Value,
    ) -> Result<Option<String>> {
        let timezone: String =
            crate::db::settings::SettingsRepo::get_value(pool, "system.timezone")
                .await?
                .unwrap_or_else(|| "UTC".into());
        crate::services::calendar::next_for_source(
            cadence,
            cron,
            config,
            enabled,
            &timezone,
            chrono::Utc::now(),
        )
    }

    pub async fn create(pool: &SqlitePool, mut input: CreateSourceInput) -> Result<Source> {
        let id = Uuid::now_v7().to_string();
        let created_at = now_iso();
        let kind_str = match serde_json::to_value(input.kind)? {
            serde_json::Value::String(s) => s,
            _ => unreachable!(),
        };
        input.config = Some(crate::services::workflow::validated_stored_config(
            input.kind,
            input.config.as_ref().unwrap_or(&serde_json::Value::Null),
        )?);
        let (_, script_hash) = crate::services::script_trust::saved_state(
            input.config.as_ref().unwrap(),
            false,
            None,
            false,
        )?;
        let needs_review = script_hash.is_some();
        let config_str = match input.config {
            Some(ref c) => serde_json::to_string(c)?,
            None => "{}".to_string(),
        };
        let enabled_val: i64 = if input.enabled.unwrap_or(true) && !needs_review {
            1
        } else {
            0
        };
        let cadence = input.cadence.unwrap_or_default();
        let cadence_str = match serde_json::to_value(cadence)? {
            serde_json::Value::String(s) => s,
            _ => "global".to_string(),
        };
        let retention_days = input.retention_days.unwrap_or(30).max(0);
        let retention_policy = input
            .retention_policy
            .unwrap_or_else(|| "untracked_only".to_string());
        let filters_str = match input.filters {
            Some(ref f) => serde_json::to_string(f)?,
            None => "{}".to_string(),
        };

        let next_run = Self::next_run_for_schedule(
            pool,
            cadence,
            input.cadence_cron.as_deref(),
            enabled_val == 1,
            input.config.as_ref().unwrap_or(&serde_json::Value::Null),
        )
        .await?;
        crate::services::runner::resolve_worker_filters(
            input.filters.as_ref().unwrap_or(&serde_json::Value::Null),
        )?;
        let has_workflows = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='workflows'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0)
            > 0;

        let mut tx = pool.begin().await?;
        if has_workflows {
            sqlx::query(
                r#"
                INSERT INTO workflows (
                    id, name, enabled, cadence, cadence_cron, next_run_at,
                    retention_days, retention_policy, filters, config,
                    max_jobs_per_run, starting_source_index, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 20, 0, ?11)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name, enabled = excluded.enabled, cadence = excluded.cadence,
                    cadence_cron = excluded.cadence_cron, next_run_at = excluded.next_run_at,
                    retention_days = excluded.retention_days, retention_policy = excluded.retention_policy,
                    filters = excluded.filters, config = excluded.config
                "#,
            )
            .bind(&id)
            .bind(&input.name)
            .bind(enabled_val)
            .bind(&cadence_str)
            .bind(&input.cadence_cron)
            .bind(&next_run)
            .bind(retention_days)
            .bind(&retention_policy)
            .bind(&filters_str)
            .bind(&config_str)
            .bind(&created_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO sources (id, workflow_id, kind, name, identifier, config, enabled, created_at, cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters)
                VALUES (?1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?13, NULL, ?10, ?11, ?12)
                "#,
            )
            .bind(&id)
            .bind(kind_str)
            .bind(input.name)
            .bind(input.identifier)
            .bind(config_str)
            .bind(enabled_val)
            .bind(created_at)
            .bind(cadence_str)
            .bind(input.cadence_cron)
            .bind(retention_days)
            .bind(retention_policy)
            .bind(filters_str)
            .bind(next_run)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO sources (id, kind, name, identifier, config, enabled, created_at, cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?13, NULL, ?10, ?11, ?12)
                "#,
            )
            .bind(&id)
            .bind(kind_str)
            .bind(input.name)
            .bind(input.identifier)
            .bind(config_str)
            .bind(enabled_val)
            .bind(created_at)
            .bind(cadence_str)
            .bind(input.cadence_cron)
            .bind(retention_days)
            .bind(retention_policy)
            .bind(filters_str)
            .bind(next_run)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "UPDATE sources SET script_trusted=0,script_sha256=?,needs_review=? WHERE id=?",
        )
        .bind(&script_hash)
        .bind(needs_review)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE workflows SET needs_review=? WHERE id=?")
            .bind(needs_review)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Self::get(pool, &id).await
    }

    pub async fn update(pool: &SqlitePool, input: UpdateSourceInput) -> Result<Source> {
        Self::update_with_origin(pool, input, false).await
    }

    pub async fn update_with_origin(
        pool: &SqlitePool,
        input: UpdateSourceInput,
        imported: bool,
    ) -> Result<Source> {
        let existing = Self::get(pool, &input.id).await?;
        let old_trust = existing.script_trusted;
        let old_hash = existing.script_sha256.clone();

        let name = input.name.unwrap_or(existing.name);
        let identifier = input.identifier.unwrap_or(existing.identifier);
        let config = crate::services::workflow::validated_stored_config(
            existing.kind,
            &input.config.unwrap_or(existing.config),
        )?;
        let (trusted, script_hash) = crate::services::script_trust::saved_state(
            &config,
            old_trust,
            old_hash.as_deref(),
            imported,
        )?;
        let needs_review = script_hash.is_some() && !trusted;
        let config_str = serde_json::to_string(&config)?;
        let enabled = input.enabled.unwrap_or(existing.enabled) && !needs_review;
        let enabled_val: i64 = if enabled { 1 } else { 0 };
        let cadence = input.cadence.unwrap_or(existing.cadence);
        let cadence_str = match serde_json::to_value(cadence)? {
            serde_json::Value::String(s) => s,
            _ => "global".to_string(),
        };
        let cadence_cron = input.cadence_cron.or(existing.cadence_cron);
        let retention_days = input
            .retention_days
            .unwrap_or(existing.retention_days)
            .max(0);
        let retention_policy = input.retention_policy.unwrap_or(existing.retention_policy);
        let filters = input.filters.unwrap_or(existing.filters);
        let filters_str = serde_json::to_string(&filters)?;
        crate::services::runner::resolve_worker_filters(&filters)?;
        let next_run =
            Self::next_run_for_schedule(pool, cadence, cadence_cron.as_deref(), enabled, &config)
                .await?;

        let mut tx = pool.begin().await?;
        sqlx::query(
            r#"
            UPDATE workflows
            SET name = ?1, config = ?2, enabled = ?3, cadence = ?4, cadence_cron = ?5,
                retention_days = ?6, retention_policy = ?7, filters = ?8, next_run_at = ?9
            WHERE id = ?10
            "#,
        )
        .bind(&name)
        .bind(&config_str)
        .bind(enabled_val)
        .bind(&cadence_str)
        .bind(&cadence_cron)
        .bind(retention_days)
        .bind(&retention_policy)
        .bind(&filters_str)
        .bind(&next_run)
        .bind(&input.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE sources
            SET name = ?1, identifier = ?2, config = ?3, enabled = ?4, cadence = ?5, cadence_cron = ?6,
                retention_days = ?7, retention_policy = ?8, filters = ?9, next_run_at = ?11
            WHERE id = ?10
            "#,
        )
        .bind(name)
        .bind(identifier)
        .bind(config_str)
        .bind(enabled_val)
        .bind(cadence_str)
        .bind(cadence_cron)
        .bind(retention_days)
        .bind(retention_policy)
        .bind(filters_str)
        .bind(input.id.clone())
        .bind(next_run)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE sources SET script_trusted=?,script_sha256=?,needs_review=? WHERE id=?",
        )
        .bind(trusted)
        .bind(&script_hash)
        .bind(needs_review)
        .bind(&input.id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE workflows SET needs_review=? WHERE id=?")
            .bind(needs_review)
            .bind(&input.id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        crate::services::script_trust::reconcile_workflow(pool, &input.id).await?;
        Self::get(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        let mut tx = pool.begin().await?;
        let wf_affected = sqlx::query("DELETE FROM workflows WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        let src_affected = if wf_affected == 0 {
            sqlx::query("DELETE FROM sources WHERE id = ? AND workflow_id IS NULL")
                .bind(id)
                .execute(&mut *tx)
                .await?
                .rows_affected()
        } else {
            0
        };

        if wf_affected == 0 && src_affected == 0 {
            return Err(AppError::NotFound(format!("Source {} not found", id)));
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn set_last_result(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let (storage_status, outcome) = source_outcome(status)?;
        sqlx::query(
            r#"
            UPDATE sources
            SET last_run_at = ?1, last_status = ?2, last_error = ?3, last_outcome = ?5
            WHERE id = ?4
            "#,
        )
        .bind(now)
        .bind(storage_status)
        .bind(error)
        .bind(id)
        .bind(outcome)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_run_result(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        error: Option<&str>,
        duration_ms: Option<i64>,
        next_run_at: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        let (storage_status, outcome) = source_outcome(status)?;
        sqlx::query(
            r#"
            UPDATE sources
            SET last_run_at = ?1, last_status = ?2, last_error = ?3, last_run_duration_ms = ?4, next_run_at = ?5, last_outcome = ?7
            WHERE id = ?6
            "#,
        )
        .bind(now)
        .bind(storage_status)
        .bind(error)
        .bind(duration_ms)
        .bind(next_run_at)
        .bind(id)
        .bind(outcome)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_due_sources(pool: &SqlitePool) -> Result<Vec<Source>> {
        Self::get_due_sources_at(pool, &now_iso()).await
    }

    pub async fn get_due_sources_at(pool: &SqlitePool, now: &str) -> Result<Vec<Source>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_outcome, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters, needs_review, script_trusted, script_sha256,
                   COALESCE((SELECT needs_review FROM workflows WHERE workflows.id=sources.id),0) AS workflow_needs_review
            FROM sources
            WHERE enabled = 1
              AND (workflow_id = id OR workflow_id IS NULL)
              AND cadence != 'global'
              AND cadence != 'manual'
              AND next_run_at IS NOT NULL
              AND julianday(next_run_at) <= julianday(?)
            ORDER BY julianday(next_run_at) ASC
            "#,
        )
        .bind(now)
        .fetch_all(pool)
        .await?;

        let mut sources = Vec::new();
        for r in rows {
            sources.push(Self::row_to_source(&r)?);
        }
        Ok(sources)
    }

    pub fn sample_workflows_suite() -> Vec<CreateSourceInput> {
        crate::services::template_catalog::seed_inputs().expect("built-in templates must validate")
    }

    pub async fn seed_default_sample_if_empty(pool: &SqlitePool) -> Result<Vec<Source>> {
        if crate::db::settings::SettingsRepo::get_value::<bool>(pool, "app.data_reset")
            .await?
            .unwrap_or(false)
        {
            return Ok(vec![]);
        }
        let count: (i64,) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM sources) + (SELECT COUNT(*) FROM workflows)",
        )
        .fetch_one(pool)
        .await?;
        if count.0 > 0 {
            return Ok(vec![]);
        }
        let seeded = crate::services::sample_workflows::seed_all(pool).await?;
        tracing::info!("Seeded {} built-in workflows", seeded.len());
        Ok(seeded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    async fn deletion_fixture(pool: &SqlitePool) {
        for id in ["delete-parent", "unrelated-parent"] {
            sqlx::query("INSERT INTO workflows(id,name,cadence,created_at) VALUES(?,?,'manual','2026-10-23T12:00:00Z')")
                .bind(id).bind(id).execute(pool).await.unwrap();
        }
        for (id, owner, enabled) in [
            ("delete-parent", "delete-parent", true),
            ("enabled-child", "delete-parent", true),
            ("disabled-child", "delete-parent", false),
            ("unrelated-parent", "unrelated-parent", true),
        ] {
            sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,enabled,cadence,created_at) VALUES(?,?,'greenhouse',?,?,?,'manual','2026-10-23T12:00:00Z')")
                .bind(id).bind(owner).bind(id).bind(id).bind(enabled)
                .execute(pool).await.unwrap();
            sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,favorited_at,source_name_snapshot) VALUES(?,?,?,?,'Saved engineer','Example','https://fixture.invalid/job','2026-10-23','2026-10-23','applied','2026-10-23','Preserved notes',1,'2026-10-23',?)")
                .bind(id).bind(format!("identity:{id}")).bind(id).bind(owner).bind(id)
                .execute(pool).await.unwrap();
            sqlx::query("INSERT INTO job_status_events(job_id,from_status,to_status,changed_at) VALUES(?,'viewed','applied','2026-10-23')")
                .bind(id).execute(pool).await.unwrap();
            sqlx::query("INSERT INTO scrape_runs(id,source_id,workflow_id,trigger,status,started_at,finished_at,jobs_found,jobs_new) VALUES(?,?,?,'single_source','success','2026-10-23T12:00:00Z','2026-10-23T12:01:00Z',3,1)")
                .bind(id).bind(id).bind(owner).execute(pool).await.unwrap();
            sqlx::query("INSERT INTO scrape_logs(id,run_id,timestamp,level,source_id,message) VALUES(?,?,'2026-10-23T12:00:30Z','info',?,'Preserved run history')")
                .bind(id).bind(id).bind(id).execute(pool).await.unwrap();
        }
    }

    // Relationship columns are checked separately; all saved content must survive deletion.
    async fn saved_deletion_state(pool: &SqlitePool) -> Vec<String> {
        let mut state = Vec::new();
        for query in [
            "SELECT json_array(id,dedupe_key,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,favorited_at,source_name_snapshot) FROM jobs ORDER BY id",
            "SELECT json_array(id,job_id,from_status,to_status,changed_at) FROM job_status_events ORDER BY id",
            "SELECT json_array(id,trigger,status,started_at,finished_at,jobs_found,jobs_new,error) FROM scrape_runs ORDER BY id",
            "SELECT json_array(id,run_id,timestamp,level,source_id,message) FROM scrape_logs ORDER BY id",
        ] {
            state.extend(
                sqlx::query_scalar::<_, String>(query)
                    .fetch_all(pool)
                    .await
                    .unwrap(),
            );
        }
        state
    }

    async fn deletion_links(pool: &SqlitePool) -> Vec<(String, Option<String>, Option<String>)> {
        let mut links = Vec::new();
        for query in [
            "SELECT id,source_id,workflow_id FROM jobs ORDER BY id",
            "SELECT id,source_id,workflow_id FROM scrape_runs ORDER BY id",
        ] {
            links.extend(sqlx::query_as(query).fetch_all(pool).await.unwrap());
        }
        links
    }

    async fn assert_workflow_deleted(pool: &SqlitePool, saved: &[String]) {
        assert_eq!(saved_deletion_state(pool).await, saved);
        let owners: Vec<(String, String)> =
            sqlx::query_as("SELECT id,workflow_id FROM sources ORDER BY id")
                .fetch_all(pool)
                .await
                .unwrap();
        assert_eq!(
            owners,
            vec![("unrelated-parent".into(), "unrelated-parent".into())]
        );
        let workflows: Vec<String> = sqlx::query_scalar("SELECT id FROM workflows ORDER BY id")
            .fetch_all(pool)
            .await
            .unwrap();
        assert_eq!(workflows, vec!["unrelated-parent"]);
        for (id, source, workflow) in deletion_links(pool).await {
            if id == "unrelated-parent" {
                assert_eq!(source.as_deref(), Some("unrelated-parent"));
                assert_eq!(workflow.as_deref(), Some("unrelated-parent"));
            } else {
                assert_eq!(source, None);
                assert_eq!(workflow, None);
            }
        }
    }

    #[tokio::test]
    async fn delete_workflow_cascades_all_owned_sources_and_preserves_saved_history() {
        let pool = init_memory_db().await.unwrap();
        deletion_fixture(&pool).await;
        let saved = saved_deletion_state(&pool).await;
        let unrelated = SourcesRepo::get(&pool, "unrelated-parent").await.unwrap();

        SourcesRepo::delete(&pool, "delete-parent").await.unwrap();

        assert_workflow_deleted(&pool, &saved).await;
        assert_eq!(
            SourcesRepo::get(&pool, "unrelated-parent").await.unwrap(),
            unrelated
        );
    }

    #[tokio::test]
    async fn delete_workflow_propagates_database_failure_rolls_back_and_can_retry() {
        let pool = init_memory_db().await.unwrap();
        deletion_fixture(&pool).await;
        let saved = saved_deletion_state(&pool).await;
        let links = deletion_links(&pool).await;
        let mut sources = Vec::new();
        for id in [
            "delete-parent",
            "enabled-child",
            "disabled-child",
            "unrelated-parent",
        ] {
            sources.push(SourcesRepo::get(&pool, id).await.unwrap());
        }
        let workflows: Vec<(String, String)> =
            sqlx::query_as("SELECT id,name FROM workflows ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        sqlx::query("CREATE TRIGGER fail_workflow_delete BEFORE DELETE ON workflows WHEN OLD.id='delete-parent' BEGIN SELECT RAISE(ABORT,'injected workflow delete failure'); END;")
            .execute(&pool).await.unwrap();

        let error = SourcesRepo::delete(&pool, "delete-parent")
            .await
            .unwrap_err();
        assert!(matches!(&error, AppError::Database(_)));
        assert!(error
            .to_string()
            .contains("injected workflow delete failure"));
        assert_eq!(saved_deletion_state(&pool).await, saved);
        assert_eq!(deletion_links(&pool).await, links);
        for source in sources {
            assert_eq!(SourcesRepo::get(&pool, &source.id).await.unwrap(), source);
        }
        assert_eq!(
            sqlx::query_as::<_, (String, String)>("SELECT id,name FROM workflows ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap(),
            workflows
        );

        sqlx::query("DROP TRIGGER fail_workflow_delete")
            .execute(&pool)
            .await
            .unwrap();
        SourcesRepo::delete(&pool, "delete-parent").await.unwrap();
        assert_workflow_deleted(&pool, &saved).await;
    }

    #[tokio::test]
    async fn delete_rejects_owned_children_and_missing_ids_but_accepts_unowned_legacy_source() {
        let pool = init_memory_db().await.unwrap();
        deletion_fixture(&pool).await;
        let saved = saved_deletion_state(&pool).await;
        let links = deletion_links(&pool).await;
        for id in ["enabled-child", "disabled-child", "missing"] {
            assert!(matches!(
                SourcesRepo::delete(&pool, id).await,
                Err(AppError::NotFound(message)) if message == format!("Source {id} not found")
            ));
        }
        assert_eq!(saved_deletion_state(&pool).await, saved);
        assert_eq!(deletion_links(&pool).await, links);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sources")
                .fetch_one(&pool)
                .await
                .unwrap(),
            4
        );
        sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at) VALUES('legacy','greenhouse','Legacy','legacy','2026-10-23T12:00:00Z')")
            .execute(&pool).await.unwrap();
        SourcesRepo::delete(&pool, "legacy").await.unwrap();
        assert!(matches!(
            SourcesRepo::get(&pool, "legacy").await,
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(
            SourcesRepo::delete(&pool, "legacy").await,
            Err(AppError::NotFound(_))
        ));
        assert_eq!(saved_deletion_state(&pool).await, saved);
        assert_eq!(deletion_links(&pool).await, links);
    }

    #[tokio::test]
    async fn invalid_stored_rules_fail_and_legacy_selectors_migrate_explicitly() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let input = CreateSourceInput {
            kind: SourceKind::Selector,
            name: "Legacy selectors".into(),
            identifier: "https://example.com/careers".into(),
            config: Some(serde_json::json!({"selectors":{"item":".job","title":"h2","link":"a"}})),
            enabled: Some(true),
            cadence: Some(SourceCadence::Manual),
            cadence_cron: None,
            retention_days: Some(30),
            retention_policy: None,
            filters: Some(serde_json::json!({"include_keywords":[]})),
        };
        let source = SourcesRepo::create(&pool, input).await.unwrap();
        assert_eq!(source.config["item"], ".job");
        assert!(source.config.get("selectors").is_none());
        sqlx::query("UPDATE sources SET filters='{invalid JSON' WHERE id=?")
            .bind(&source.id)
            .execute(&pool)
            .await
            .unwrap();
        let retained_source = SourcesRepo::get(&pool, &source.id).await.unwrap();
        assert!(retained_source.needs_review);
        assert_eq!(retained_source.filters, serde_json::json!({}));
        assert_eq!(
            retained_source.config["__invalid_original_filters"],
            "{invalid JSON"
        );
        let retained: String = sqlx::query_scalar("SELECT filters FROM sources WHERE id=?")
            .bind(&source.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(retained, "{invalid JSON");
    }

    #[tokio::test]
    async fn test_crud_source() {
        let pool = init_memory_db().await.unwrap();

        let created = SourcesRepo::create(
            &pool,
            CreateSourceInput {
                kind: SourceKind::Greenhouse,
                name: "Stripe".to_string(),
                identifier: "stripe".to_string(),
                config: Some(serde_json::json!({})),
                enabled: Some(true),
                cadence: Some(SourceCadence::Every12h),
                cadence_cron: None,
                retention_days: Some(45),
                retention_policy: Some("untracked_only".to_string()),
                filters: Some(serde_json::json!({"include_keywords": ["rust"]})),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.name, "Stripe");
        assert_eq!(created.identifier, "stripe");
        assert_eq!(created.kind, SourceKind::Greenhouse);
        assert_eq!(created.cadence, SourceCadence::Every12h);
        assert_eq!(created.retention_days, 45);
        assert_eq!(created.retention_policy, "untracked_only");
        assert_eq!(created.filters["include_keywords"][0], "rust");
        assert!(created.enabled);

        let list = SourcesRepo::list(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].cadence, SourceCadence::Every12h);
        assert_eq!(list[0].retention_days, 45);

        let updated = SourcesRepo::update(
            &pool,
            UpdateSourceInput {
                id: created.id.clone(),
                name: Some("Stripe Global".to_string()),
                identifier: None,
                config: None,
                enabled: Some(false),
                cadence: Some(SourceCadence::Cron),
                cadence_cron: Some("0 30 9 * * *".to_string()),
                retention_days: Some(60),
                retention_policy: None,
                filters: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "Stripe Global");
        assert_eq!(updated.cadence, SourceCadence::Cron);
        assert_eq!(updated.cadence_cron.as_deref(), Some("0 30 9 * * *"));
        assert_eq!(updated.retention_days, 60);
        assert!(!updated.enabled);

        SourcesRepo::set_run_result(
            &pool,
            &created.id,
            "ok",
            None,
            Some(1420),
            Some("2026-10-10T12:00:00Z"),
        )
        .await
        .unwrap();

        let after_run = SourcesRepo::get(&pool, &created.id).await.unwrap();
        assert_eq!(after_run.last_status, Some("ok".to_string()));
        assert_eq!(after_run.last_run_duration_ms, Some(1420));
        assert_eq!(
            after_run.next_run_at.as_deref(),
            Some("2026-10-10T12:00:00Z")
        );
        assert!(after_run.last_run_at.is_some());

        SourcesRepo::delete(&pool, &created.id).await.unwrap();
        assert!(SourcesRepo::get(&pool, &created.id).await.is_err());
    }

    #[tokio::test]
    async fn due_sources_use_instants_and_exclude_manual_and_disabled() {
        let pool = init_memory_db().await.unwrap();
        for (id, cadence, enabled, next) in [
            ("offset", "daily", true, "2026-10-13T09:00:00+02:00"),
            ("future", "daily", true, "2026-10-13T08:00:00Z"),
            ("manual", "manual", true, "2026-10-13T06:00:00Z"),
            ("disabled", "daily", false, "2026-10-13T06:00:00Z"),
        ] {
            sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at, cadence, enabled, next_run_at) VALUES (?1, 'greenhouse', ?1, ?1, '2026-01-01', ?2, ?3, ?4)")
                .bind(id).bind(cadence).bind(enabled).bind(next).execute(&pool).await.unwrap();
        }
        let due = SourcesRepo::get_due_sources_at(&pool, "2026-10-13T07:00:00Z")
            .await
            .unwrap();
        assert_eq!(
            due.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["offset"]
        );
    }

    #[tokio::test]
    async fn empty_startup_seeds_all_three_builtins_only_once() {
        let pool = init_memory_db().await.unwrap();
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "app.data_reset",
            serde_json::json!(true),
        )
        .await
        .unwrap();
        assert!(SourcesRepo::seed_default_sample_if_empty(&pool)
            .await
            .unwrap()
            .is_empty());
        assert!(SourcesRepo::list(&pool).await.unwrap().is_empty());
        crate::db::settings::SettingsRepo::set_value(
            &pool,
            "app.data_reset",
            serde_json::json!(false),
        )
        .await
        .unwrap();
        let seeded = SourcesRepo::seed_default_sample_if_empty(&pool)
            .await
            .unwrap();
        assert_eq!(seeded.len(), 3);
        let ids = seeded
            .iter()
            .map(|source| {
                source.config["__builtin_workflow"]["template_id"]
                    .as_str()
                    .unwrap()
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            ids,
            [
                "software-engineering-last-12-hours",
                "cherry-hill-fast-food-last-month",
                "remote-usd-over-60000"
            ]
            .into_iter()
            .collect()
        );
        let second = SourcesRepo::seed_default_sample_if_empty(&pool)
            .await
            .unwrap();
        assert!(second.is_empty());
        assert_eq!(SourcesRepo::list(&pool).await.unwrap().len(), 3);
    }
}
