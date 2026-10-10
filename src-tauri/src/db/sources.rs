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
}

pub struct SourcesRepo;

impl SourcesRepo {
    fn row_to_source(r: &sqlx::sqlite::SqliteRow) -> Result<Source> {
        use sqlx::Row;
        let kind_str: String = r.get("kind");
        let config_str: String = r.get("config");
        let kind: SourceKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
        let config: serde_json::Value = serde_json::from_str(&config_str)?;
        let enabled_int: i64 = r.get("enabled");
        let cadence_str: Option<String> = r.get("cadence");
        let cadence: SourceCadence = match cadence_str {
            Some(s) => serde_json::from_value(serde_json::Value::String(s))
                .unwrap_or(SourceCadence::Global),
            None => SourceCadence::Global,
        };

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
        })
    }

    pub async fn list(pool: &SqlitePool) -> Result<Vec<Source>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms
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
                   cadence, cadence_cron, next_run_at, last_run_duration_ms
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

    pub async fn create(pool: &SqlitePool, input: CreateSourceInput) -> Result<Source> {
        let id = Uuid::now_v7().to_string();
        let created_at = now_iso();
        let kind_str = match serde_json::to_value(input.kind)? {
            serde_json::Value::String(s) => s,
            _ => unreachable!(),
        };
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

        sqlx::query(
            r#"
            INSERT INTO sources (id, kind, name, identifier, config, enabled, created_at, cadence, cadence_cron, next_run_at, last_run_duration_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, NULL)
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
        .execute(pool)
        .await?;

        Self::get(pool, &id).await
    }

    pub async fn update(pool: &SqlitePool, input: UpdateSourceInput) -> Result<Source> {
        let existing = Self::get(pool, &input.id).await?;

        let name = input.name.unwrap_or(existing.name);
        let identifier = input.identifier.unwrap_or(existing.identifier);
        let config = input.config.unwrap_or(existing.config);
        let config_str = serde_json::to_string(&config)?;
        let enabled = input.enabled.unwrap_or(existing.enabled);
        let enabled_val: i64 = if enabled { 1 } else { 0 };
        let cadence = input.cadence.unwrap_or(existing.cadence);
        let cadence_str = match serde_json::to_value(cadence)? {
            serde_json::Value::String(s) => s,
            _ => "global".to_string(),
        };
        let cadence_cron = input.cadence_cron.or(existing.cadence_cron);

        sqlx::query(
            r#"
            UPDATE sources
            SET name = ?1, identifier = ?2, config = ?3, enabled = ?4, cadence = ?5, cadence_cron = ?6
            WHERE id = ?7
            "#,
        )
        .bind(name)
        .bind(identifier)
        .bind(config_str)
        .bind(enabled_val)
        .bind(cadence_str)
        .bind(cadence_cron)
        .bind(input.id.clone())
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
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at,
                   cadence, cadence_cron, next_run_at, last_run_duration_ms
            FROM sources
            WHERE enabled = 1
              AND cadence != 'global'
              AND cadence != 'manual'
              AND next_run_at IS NOT NULL
              AND next_run_at <= datetime('now')
            ORDER BY next_run_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut sources = Vec::new();
        for r in rows {
            sources.push(Self::row_to_source(&r)?);
        }
        Ok(sources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

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
            },
        )
        .await
        .unwrap();

        assert_eq!(created.name, "Stripe");
        assert_eq!(created.identifier, "stripe");
        assert_eq!(created.kind, SourceKind::Greenhouse);
        assert_eq!(created.cadence, SourceCadence::Every12h);
        assert!(created.enabled);

        let list = SourcesRepo::list(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].cadence, SourceCadence::Every12h);

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
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "Stripe Global");
        assert_eq!(updated.cadence, SourceCadence::Cron);
        assert_eq!(updated.cadence_cron.as_deref(), Some("0 30 9 * * *"));
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
        assert_eq!(after_run.next_run_at.as_deref(), Some("2026-10-10T12:00:00Z"));
        assert!(after_run.last_run_at.is_some());

        SourcesRepo::delete(&pool, &created.id).await.unwrap();
        assert!(SourcesRepo::get(&pool, &created.id).await.is_err());
    }
}
