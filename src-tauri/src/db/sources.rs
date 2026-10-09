use crate::error::{AppError, Result};
use scrape_core::{now_iso, SourceKind};
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
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateSourceInput {
    pub kind: SourceKind,
    pub name: String,
    pub identifier: String,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSourceInput {
    pub id: String,
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
}

pub struct SourcesRepo;

impl SourcesRepo {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Source>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at
            FROM sources
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(pool)
        .await?;

        let mut sources = Vec::new();
        for r in rows {
            let kind_str: String = r.get("kind");
            let config_str: String = r.get("config");
            let kind: SourceKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
            let config: serde_json::Value = serde_json::from_str(&config_str)?;
            let enabled_int: i64 = r.get("enabled");

            sources.push(Source {
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
            });
        }

        Ok(sources)
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Source> {
        use sqlx::Row;
        let r = sqlx::query(
            r#"
            SELECT id, kind, name, identifier, config, enabled, last_run_at, last_status, last_error, created_at
            FROM sources
            WHERE id = ?
            "#
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Source {} not found", id)))?;

        let kind_str: String = r.get("kind");
        let config_str: String = r.get("config");
        let kind: SourceKind = serde_json::from_value(serde_json::Value::String(kind_str))?;
        let config: serde_json::Value = serde_json::from_str(&config_str)?;
        let enabled_int: i64 = r.get("enabled");

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
        })
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

        sqlx::query(
            r#"
            INSERT INTO sources (id, kind, name, identifier, config, enabled, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(&id)
        .bind(kind_str)
        .bind(input.name)
        .bind(input.identifier)
        .bind(config_str)
        .bind(enabled_val)
        .bind(created_at)
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

        sqlx::query(
            r#"
            UPDATE sources
            SET name = ?1, identifier = ?2, config = ?3, enabled = ?4
            WHERE id = ?5
            "#,
        )
        .bind(name)
        .bind(identifier)
        .bind(config_str)
        .bind(enabled_val)
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
            },
        )
        .await
        .unwrap();

        assert_eq!(created.name, "Stripe");
        assert_eq!(created.identifier, "stripe");
        assert_eq!(created.kind, SourceKind::Greenhouse);
        assert!(created.enabled);

        let list = SourcesRepo::list(&pool).await.unwrap();
        assert_eq!(list.len(), 1);

        let updated = SourcesRepo::update(
            &pool,
            UpdateSourceInput {
                id: created.id.clone(),
                name: Some("Stripe Global".to_string()),
                identifier: None,
                config: None,
                enabled: Some(false),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "Stripe Global");
        assert!(!updated.enabled);

        SourcesRepo::set_last_result(&pool, &created.id, "ok", None)
            .await
            .unwrap();

        let after_run = SourcesRepo::get(&pool, &created.id).await.unwrap();
        assert_eq!(after_run.last_status, Some("ok".to_string()));
        assert!(after_run.last_run_at.is_some());

        SourcesRepo::delete(&pool, &created.id).await.unwrap();
        assert!(SourcesRepo::get(&pool, &created.id).await.is_err());
    }
}
