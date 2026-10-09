use crate::error::{AppError, Result};
use scrape_core::now_iso;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TriggerType {
    Schedule,
    Manual,
    Startup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RunStatus {
    Running,
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct ScrapeRun {
    pub id: String,
    pub trigger: TriggerType,
    pub status: RunStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub jobs_found: i64,
    pub jobs_new: i64,
    pub error: Option<String>,
}

pub struct RunsRepo;

impl RunsRepo {
    pub async fn start(pool: &SqlitePool, trigger: TriggerType) -> Result<ScrapeRun> {
        let id = Uuid::now_v7().to_string();
        let started_at = now_iso();
        let trigger_str = match serde_json::to_value(trigger)? {
            serde_json::Value::String(s) => s,
            _ => unreachable!(),
        };

        sqlx::query(
            r#"
            INSERT INTO scrape_runs (id, trigger, status, started_at)
            VALUES (?1, ?2, 'running', ?3)
            "#,
        )
        .bind(&id)
        .bind(trigger_str)
        .bind(&started_at)
        .execute(pool)
        .await?;

        Ok(ScrapeRun {
            id,
            trigger,
            status: RunStatus::Running,
            started_at,
            finished_at: None,
            jobs_found: 0,
            jobs_new: 0,
            error: None,
        })
    }

    pub async fn finish(
        pool: &SqlitePool,
        id: &str,
        status: RunStatus,
        jobs_found: i64,
        jobs_new: i64,
        error: Option<&str>,
    ) -> Result<ScrapeRun> {
        let finished_at = now_iso();
        let status_str = match serde_json::to_value(status)? {
            serde_json::Value::String(s) => s,
            _ => unreachable!(),
        };

        sqlx::query(
            r#"
            UPDATE scrape_runs
            SET status = ?1, finished_at = ?2, jobs_found = ?3, jobs_new = ?4, error = ?5
            WHERE id = ?6
            "#,
        )
        .bind(status_str)
        .bind(finished_at)
        .bind(jobs_found)
        .bind(jobs_new)
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;

        Self::get(pool, id).await
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<ScrapeRun> {
        use sqlx::Row;
        let r = sqlx::query(
            r#"
            SELECT id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error
            FROM scrape_runs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Scrape run {} not found", id)))?;

        let trigger_str: String = r.get("trigger");
        let status_str: String = r.get("status");
        let trigger: TriggerType = serde_json::from_value(serde_json::Value::String(trigger_str))?;
        let status: RunStatus = serde_json::from_value(serde_json::Value::String(status_str))?;

        Ok(ScrapeRun {
            id: r.get("id"),
            trigger,
            status,
            started_at: r.get("started_at"),
            finished_at: r.get("finished_at"),
            jobs_found: r.get("jobs_found"),
            jobs_new: r.get("jobs_new"),
            error: r.get("error"),
        })
    }

    pub async fn recent(pool: &SqlitePool, limit: i64) -> Result<Vec<ScrapeRun>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error
            FROM scrape_runs
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let mut runs = Vec::new();
        for r in rows {
            let trigger_str: String = r.get("trigger");
            let status_str: String = r.get("status");
            let trigger: TriggerType =
                serde_json::from_value(serde_json::Value::String(trigger_str))?;
            let status: RunStatus = serde_json::from_value(serde_json::Value::String(status_str))?;
            runs.push(ScrapeRun {
                id: r.get("id"),
                trigger,
                status,
                started_at: r.get("started_at"),
                finished_at: r.get("finished_at"),
                jobs_found: r.get("jobs_found"),
                jobs_new: r.get("jobs_new"),
                error: r.get("error"),
            });
        }

        Ok(runs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[tokio::test]
    async fn test_scrape_run_lifecycle() {
        let pool = init_memory_db().await.unwrap();

        let run = RunsRepo::start(&pool, TriggerType::Manual).await.unwrap();
        assert_eq!(run.status, RunStatus::Running);

        let finished = RunsRepo::finish(&pool, &run.id, RunStatus::Success, 120, 14, None)
            .await
            .unwrap();

        assert_eq!(finished.status, RunStatus::Success);
        assert_eq!(finished.jobs_found, 120);
        assert_eq!(finished.jobs_new, 14);
        assert!(finished.finished_at.is_some());

        let list = RunsRepo::recent(&pool, 10).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, run.id);
    }
}
