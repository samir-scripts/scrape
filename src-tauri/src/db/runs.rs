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
    SingleSource,
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
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct ScrapeLogEntry {
    pub id: String,
    pub run_id: String,
    pub timestamp: String,
    pub level: LogLevel,
    pub source_id: Option<String>,
    pub message: String,
}

pub struct RunsRepo;

impl RunsRepo {
    pub async fn start(pool: &SqlitePool, trigger: TriggerType) -> Result<ScrapeRun> {
        Self::start_with_source(pool, trigger, None).await
    }

    pub async fn start_with_source(
        pool: &SqlitePool,
        trigger: TriggerType,
        source_id: Option<&str>,
    ) -> Result<ScrapeRun> {
        let id = Uuid::now_v7().to_string();
        let started_at = now_iso();
        let trigger_str = match serde_json::to_value(trigger)? {
            serde_json::Value::String(s) => s,
            _ => unreachable!(),
        };

        sqlx::query(
            r#"
            INSERT INTO scrape_runs (id, trigger, status, started_at, source_id)
            VALUES (?1, ?2, 'running', ?3, ?4)
            "#,
        )
        .bind(&id)
        .bind(trigger_str)
        .bind(&started_at)
        .bind(source_id)
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
            source_id: source_id.map(ToString::to_string),
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
            SELECT id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error, source_id
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
            source_id: r.get("source_id"),
        })
    }

    pub async fn recent(pool: &SqlitePool, limit: i64) -> Result<Vec<ScrapeRun>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error, source_id
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
                source_id: r.get("source_id"),
            });
        }

        Ok(runs)
    }

    pub async fn list_by_source(
        pool: &SqlitePool,
        source_id: &str,
        limit: i64,
    ) -> Result<Vec<ScrapeRun>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, trigger, status, started_at, finished_at, jobs_found, jobs_new, error, source_id
            FROM scrape_runs
            WHERE source_id = ?
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )
        .bind(source_id)
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
                source_id: r.get("source_id"),
            });
        }

        Ok(runs)
    }

    pub async fn add_log(
        pool: &SqlitePool,
        run_id: &str,
        level: LogLevel,
        source_id: Option<&str>,
        message: &str,
    ) -> Result<()> {
        let id = Uuid::now_v7().to_string();
        let timestamp = now_iso();
        let level_str = match level {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };

        sqlx::query(
            r#"
            INSERT INTO scrape_logs (id, run_id, timestamp, level, source_id, message)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&id)
        .bind(run_id)
        .bind(&timestamp)
        .bind(level_str)
        .bind(source_id)
        .bind(message)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_logs(pool: &SqlitePool, run_id: &str) -> Result<Vec<ScrapeLogEntry>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, run_id, timestamp, level, source_id, message
            FROM scrape_logs
            WHERE run_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(pool)
        .await?;

        let mut entries = Vec::new();
        for r in rows {
            let level_str: String = r.get("level");
            let level = match level_str.as_str() {
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => LogLevel::Info,
            };
            entries.push(ScrapeLogEntry {
                id: r.get("id"),
                run_id: r.get("run_id"),
                timestamp: r.get("timestamp"),
                level,
                source_id: r.get("source_id"),
                message: r.get("message"),
            });
        }
        Ok(entries)
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

    #[tokio::test]
    async fn test_scrape_run_with_source_id_and_single_source() {
        let pool = init_memory_db().await.unwrap();

        let source = crate::db::sources::SourcesRepo::create(
            &pool,
            crate::db::sources::CreateSourceInput {
                kind: scrape_core::SourceKind::Greenhouse,
                name: "Test Co".to_string(),
                identifier: "testco".to_string(),
                config: None,
                enabled: Some(true),
                cadence: Some(scrape_core::SourceCadence::Every12h),
                cadence_cron: None,
            },
        )
        .await
        .unwrap();

        let source_id = source.id.as_str();
        let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(source_id))
            .await
            .unwrap();

        assert_eq!(run.trigger, TriggerType::SingleSource);
        assert_eq!(run.source_id.as_deref(), Some(source_id));
        assert_eq!(run.status, RunStatus::Running);

        let finished = RunsRepo::finish(&pool, &run.id, RunStatus::Success, 15, 3, None)
            .await
            .unwrap();

        assert_eq!(finished.source_id.as_deref(), Some(source_id));
        assert_eq!(finished.jobs_found, 15);
        assert_eq!(finished.jobs_new, 3);

        let source_runs = RunsRepo::list_by_source(&pool, source_id, 10).await.unwrap();
        assert_eq!(source_runs.len(), 1);
        assert_eq!(source_runs[0].id, run.id);
        assert_eq!(source_runs[0].trigger, TriggerType::SingleSource);

        let other_runs = RunsRepo::list_by_source(&pool, "non-existent-source", 10).await.unwrap();
        assert!(other_runs.is_empty());
    }

    #[tokio::test]
    async fn test_scrape_run_logs_persistence_and_ordering() {
        let pool = init_memory_db().await.unwrap();

        let run = RunsRepo::start(&pool, TriggerType::Schedule).await.unwrap();

        // 1. Add log entries across different levels and source scopes
        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            Some("source-gh-stripe"),
            "Discovered 25 postings from Greenhouse API",
        )
        .await
        .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Warn,
            Some("source-gh-stripe"),
            "HTTP 429 received, backing off 1500ms",
        )
        .await
        .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Error,
            Some("source-lever-figma"),
            "Connection reset by peer while parsing payload",
        )
        .await
        .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            None,
            "Scrape run completed with 1 failure",
        )
        .await
        .unwrap();

        // 2. Query logs for the run
        let logs = RunsRepo::get_logs(&pool, &run.id).await.unwrap();
        assert_eq!(logs.len(), 4);

        // 3. Verify monotonic ordering and content preservation
        assert_eq!(logs[0].level, LogLevel::Info);
        assert_eq!(logs[0].source_id.as_deref(), Some("source-gh-stripe"));
        assert_eq!(logs[1].level, LogLevel::Warn);
        assert_eq!(logs[2].level, LogLevel::Error);
        assert_eq!(logs[2].source_id.as_deref(), Some("source-lever-figma"));
        assert_eq!(logs[3].level, LogLevel::Info);
        assert_eq!(logs[3].source_id, None);

        // Check timestamps are non-decreasing
        for i in 0..logs.len() - 1 {
            assert!(logs[i].timestamp <= logs[i + 1].timestamp);
        }

        // 4. Verify querying an unknown or separate run ID returns empty
        let empty_logs = RunsRepo::get_logs(&pool, "non-existent-run").await.unwrap();
        assert!(empty_logs.is_empty());
    }
}
