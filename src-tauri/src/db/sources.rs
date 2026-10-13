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
    pub last_error: Option<String>,
    pub created_at: String,
    pub cadence: SourceCadence,
    pub cadence_cron: Option<String>,
    pub next_run_at: Option<String>,
    pub last_run_duration_ms: Option<i64>,
    pub retention_days: i64,
    pub retention_policy: String,
    pub filters: serde_json::Value,
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

pub struct SourcesRepo;

impl SourcesRepo {
    fn row_to_source(r: &sqlx::sqlite::SqliteRow) -> Result<Source> {
        use sqlx::Row;
        let kind_str: String = r.get("kind");
        let config_str: String = r.get("config");
        let kind: SourceKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
        let config = crate::services::workflow::normalize_source_config(&serde_json::from_str::<
            serde_json::Value,
        >(&config_str)?)?;
        let enabled_int: i64 = r.get("enabled");
        let cadence_str: Option<String> = r.get("cadence");
        let cadence: SourceCadence = match cadence_str {
            Some(s) => serde_json::from_value(serde_json::Value::String(s))?,
            None => SourceCadence::Global,
        };
        let retention_days: i64 = r.try_get("retention_days")?;
        let retention_policy: String = r.try_get("retention_policy")?;
        let filters_str: String = r.try_get("filters")?;
        let filters: serde_json::Value = serde_json::from_str(&filters_str).map_err(|error| {
            AppError::Validation(format!(
                "Workflow {} filters: invalid stored JSON: {error}",
                r.get::<String, _>("id")
            ))
        })?;

        Ok(Source {
            id: r.get("id"),
            kind,
            name: r.get("name"),
            identifier: r.get("identifier"),
            config,
            enabled: enabled_int == 1,
            last_run_at: r.get("last_run_at"),
            last_status: r.get("last_status"),
            last_error: r.get("last_error"),
            created_at: r.get("created_at"),
            cadence,
            cadence_cron: r.get("cadence_cron"),
            next_run_at: r.get("next_run_at"),
            last_run_duration_ms: r.get("last_run_duration_ms"),
            retention_days,
            retention_policy,
            filters,
        })
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<Source>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters
            FROM sources
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
        let r = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters
            FROM sources
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Source {} not found", id)))?;

        Self::row_to_source(&r)
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
        let config_str = match input.config {
            Some(ref c) => serde_json::to_string(c)?,
            None => "{}".to_string(),
        };
        let enabled_val: i64 = if input.enabled.unwrap_or(true) { 1 } else { 0 };
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
            scrape_core::WorkerFilters::default(),
        )?;
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
        .execute(pool)
        .await?;

        Self::get(pool, &id).await
    }

    pub async fn update(pool: &SqlitePool, input: UpdateSourceInput) -> Result<Source> {
        let existing = Self::get(pool, &input.id).await?;

        let name = input.name.unwrap_or(existing.name);
        let identifier = input.identifier.unwrap_or(existing.identifier);
        let config = crate::services::workflow::validated_stored_config(
            existing.kind,
            &input.config.unwrap_or(existing.config),
        )?;
        let config_str = serde_json::to_string(&config)?;
        let enabled = input.enabled.unwrap_or(existing.enabled);
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
        crate::services::runner::resolve_worker_filters(
            &filters,
            scrape_core::WorkerFilters::default(),
        )?;
        let next_run =
            Self::next_run_for_schedule(pool, cadence, cadence_cron.as_deref(), enabled, &config)
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
        .execute(pool)
        .await?;

        Self::get(pool, &input.id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
        let rows_affected = sqlx::query("DELETE FROM sources WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(AppError::NotFound(format!("Source {} not found", id)));
        }

        Ok(())
    }

    pub async fn set_last_result(
        pool: &SqlitePool,
        id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let now = now_iso();
        sqlx::query(
            r#"
            UPDATE sources
            SET last_run_at = ?1, last_status = ?2, last_error = ?3
            WHERE id = ?4
            "#,
        )
        .bind(now)
        .bind(status)
        .bind(error)
        .bind(id)
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
        sqlx::query(
            r#"
            UPDATE sources
            SET last_run_at = ?1, last_status = ?2, last_error = ?3, last_run_duration_ms = ?4, next_run_at = ?5
            WHERE id = ?6
            "#,
        )
        .bind(now)
        .bind(status)
        .bind(error)
        .bind(duration_ms)
        .bind(next_run_at)
        .bind(id)
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
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms, retention_days, retention_policy, filters
            FROM sources
            WHERE enabled = 1
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

    pub fn default_sample_workflow_input() -> CreateSourceInput {
        CreateSourceInput {
            kind: SourceKind::Greenhouse,
            name: "Cloudflare — Software Engineering & Internships".to_string(),
            identifier: "cloudflare".to_string(),
            config: Some(serde_json::json!({
                "url": "https://boards.greenhouse.io/cloudflare"
            })),
            enabled: Some(true),
            cadence: Some(SourceCadence::Daily),
            cadence_cron: Some("0 0 9 * * *".to_string()),
            retention_days: Some(45),
            retention_policy: Some("untracked_only".to_string()),
            filters: Some(serde_json::json!({
                "role_types": ["internship", "full_time"],
                "query": "title LIKE '%Engineer%' OR title LIKE '%Developer%' OR title LIKE '%Intern%'",
                "include_keywords": ["Software", "Engineer", "Developer", "Systems", "Intern"],
                "exclude_keywords": ["Recruiter", "Account Executive", "Sales Representative"],
                "locations": ["Remote", "San Francisco", "Austin", "London", "Lisbon"]
            })),
        }
    }

    pub async fn seed_default_sample_if_empty(pool: &SqlitePool) -> Result<Option<Source>> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sources")
            .fetch_one(pool)
            .await?;

        if count.0 > 0 {
            return Ok(None);
        }

        let input = Self::default_sample_workflow_input();
        let source = Self::create(pool, input).await?;
        tracing::info!(
            "Seeded sample Software Engineering Internships workflow: {}",
            source.id
        );
        Ok(Some(source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

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
        let error = SourcesRepo::get(&pool, &source.id)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(&source.id) && error.contains("invalid stored JSON"));
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
    async fn test_seed_default_sample_workflow() {
        let pool = init_memory_db().await.unwrap();

        // 1. Initially empty: seeding should insert sample SWE workflow
        let seeded = SourcesRepo::seed_default_sample_if_empty(&pool)
            .await
            .unwrap();
        assert!(seeded.is_some());
        let sample = seeded.unwrap();
        assert_eq!(
            sample.name,
            "Cloudflare — Software Engineering & Internships"
        );
        assert_eq!(sample.identifier, "cloudflare");
        assert_eq!(sample.kind, SourceKind::Greenhouse);
        assert_eq!(sample.cadence, SourceCadence::Daily);
        assert_eq!(sample.retention_days, 45);
        assert_eq!(sample.filters["role_types"][0], "internship");

        // 2. Second invocation should be a no-op since sources exist
        let second = SourcesRepo::seed_default_sample_if_empty(&pool)
            .await
            .unwrap();
        assert!(second.is_none());

        let all = SourcesRepo::list(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
    }
}
