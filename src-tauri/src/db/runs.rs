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
    SuccessWithWarnings,
    Cancelled,
    Partial,
    Failed,
}

impl RunStatus {
    pub fn from_source_status(status: &str) -> Self {
        match status {
            "success" | "ok" | "empty" => Self::Success,
            "success_with_warnings" => Self::SuccessWithWarnings,
            "partial" => Self::Partial,
            "cancelled" => Self::Cancelled,
            _ => Self::Failed,
        }
    }
    fn storage_status(self) -> Self {
        match self {
            Self::SuccessWithWarnings => Self::Success,
            Self::Cancelled => Self::Partial,
            other => other,
        }
    }
}
impl From<scrape_core::protocol::ScanOutcome> for RunStatus {
    fn from(outcome: scrape_core::protocol::ScanOutcome) -> Self {
        use scrape_core::protocol::ScanOutcome;
        match outcome {
            ScanOutcome::Success => Self::Success,
            ScanOutcome::SuccessWithWarnings => Self::SuccessWithWarnings,
            ScanOutcome::Partial => Self::Partial,
            ScanOutcome::Failed => Self::Failed,
            ScanOutcome::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[serde(default)]
#[ts(export)]
pub struct RunCounts {
    // Detailed summaries are absent on historical runs.
    #[serde(default)]
    #[ts(optional = nullable)]
    pub sources: Option<Vec<scrape_core::protocol::SourceScanStats>>,
    #[ts(type = "number")]
    pub fetched: i64,
    #[ts(type = "number")]
    pub rejected: i64,
    #[ts(type = "number")]
    pub blocked: i64,
    #[ts(type = "number")]
    pub inserted: i64,
    #[ts(type = "number")]
    pub updated: i64,
    #[ts(type = "number")]
    pub failed: i64,
    #[ts(type = "number")]
    pub retention_removed: i64,
    #[serde(default)]
    #[ts(optional = nullable)]
    #[ts(type = "number | null")]
    pub enrichment_warnings: Option<u64>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub examination_incomplete: Option<bool>,
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
    #[serde(default)]
    #[ts(optional = nullable)]
    pub counts: Option<RunCounts>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub effective_config: Option<scrape_core::WorkerConfig>,
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
            counts: None,
            effective_config: None,
        })
    }

    pub async fn record_config(
        pool: &SqlitePool,
        id: &str,
        config: &scrape_core::WorkerConfig,
    ) -> Result<()> {
        sqlx::query("UPDATE scrape_runs SET effective_config_json=? WHERE id=?")
            .bind(serde_json::to_string(config)?)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
    pub async fn record_counts(pool: &SqlitePool, id: &str, counts: &RunCounts) -> Result<()> {
        sqlx::query("UPDATE scrape_runs SET counts_json=? WHERE id=?")
            .bind(serde_json::to_string(counts)?)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
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
        let outcome = serde_json::to_value(status)?.as_str().unwrap().to_owned();
        let status_str = match serde_json::to_value(status.storage_status())? {
            serde_json::Value::String(s) => s,
            _ => unreachable!(),
        };

        sqlx::query(
            r#"
            UPDATE scrape_runs
            SET status = ?1, finished_at = ?2, jobs_found = ?3, jobs_new = ?4, error = ?5, outcome = ?7
            WHERE id = ?6
            "#,
        )
        .bind(status_str)
        .bind(finished_at)
        .bind(jobs_found)
        .bind(jobs_new)
        .bind(error)
        .bind(id)
        .bind(outcome)
        .execute(pool)
        .await?;

        Self::get(pool, id).await
    }

    pub async fn get(pool: &SqlitePool, id: &str) -> Result<ScrapeRun> {
        use sqlx::Row;
        let r = sqlx::query(
            r#"
            SELECT id, trigger, COALESCE(outcome, status) AS status, started_at, finished_at, jobs_found, jobs_new, error, source_id, counts_json, effective_config_json
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
            counts: r
                .get::<Option<String>, _>("counts_json")
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            effective_config: r
                .get::<Option<String>, _>("effective_config_json")
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }

    pub async fn recent(pool: &SqlitePool, limit: i64) -> Result<Vec<ScrapeRun>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, trigger, COALESCE(outcome, status) AS status, started_at, finished_at, jobs_found, jobs_new, error, source_id, counts_json, effective_config_json
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
                counts: r
                    .get::<Option<String>, _>("counts_json")
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?,
                effective_config: r
                    .get::<Option<String>, _>("effective_config_json")
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?,
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
            SELECT id, trigger, COALESCE(outcome, status) AS status, started_at, finished_at, jobs_found, jobs_new, error, source_id, counts_json, effective_config_json
            FROM scrape_runs
            WHERE source_id = ?1
               OR id IN (SELECT DISTINCT run_id FROM scrape_logs WHERE source_id = ?1)
            ORDER BY started_at DESC
            LIMIT ?2
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
                counts: r
                    .get::<Option<String>, _>("counts_json")
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?,
                effective_config: r
                    .get::<Option<String>, _>("effective_config_json")
                    .map(|value| serde_json::from_str(&value))
                    .transpose()?,
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
    async fn effective_rules_and_stage_counts_survive_failure_and_restart_queries() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let run = RunsRepo::start(&pool, TriggerType::Manual).await.unwrap();
        let config = scrape_core::WorkerConfig {
            version: scrape_core::PROTOCOL_VERSION,
            sources: vec![],
            filters: Default::default(),
            filters_by_source: Default::default(),
            run_started_at: Some(run.started_at.clone()),
            limits: Default::default(),
            blocked_postings: vec![],
            max_jobs_per_run: None,
            starting_source_index: None,
        };
        let counts = RunCounts {
            fetched: 12,
            rejected: 4,
            blocked: 2,
            inserted: 3,
            updated: 1,
            failed: 2,
            retention_removed: 5,
            enrichment_warnings: Some(1),
            examination_incomplete: None,
            sources: Some(vec![scrape_core::protocol::SourceScanStats {
                source_id: "fixture".into(),
                limit: 20,
                examined: 12,
                rejected: 4,
                matched: 6,
                accepted: 4,
                blocked: 2,
                invalid: 2,
                rejections: vec![scrape_core::protocol::RejectionCount {
                    reason: scrape_core::matching::RejectionReason::Geography,
                    count: 4,
                }],
                ..Default::default()
            }]),
        };
        RunsRepo::record_config(&pool, &run.id, &config)
            .await
            .unwrap();
        RunsRepo::record_counts(&pool, &run.id, &counts)
            .await
            .unwrap();
        RunsRepo::finish(
            &pool,
            &run.id,
            RunStatus::Partial,
            6,
            3,
            Some("Worker cancelled"),
        )
        .await
        .unwrap();
        for saved in [
            RunsRepo::get(&pool, &run.id).await.unwrap(),
            RunsRepo::recent(&pool, 1).await.unwrap().remove(0),
        ] {
            assert_eq!(saved.counts, Some(counts.clone()));
            assert_eq!(saved.effective_config, Some(config.clone()));
            assert_eq!(saved.status, RunStatus::Partial);
        }
    }

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
                retention_days: None,
                retention_policy: None,
                filters: None,
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

        let source_runs = RunsRepo::list_by_source(&pool, source_id, 10)
            .await
            .unwrap();
        assert_eq!(source_runs.len(), 1);
        assert_eq!(source_runs[0].id, run.id);
        assert_eq!(source_runs[0].trigger, TriggerType::SingleSource);

        let other_runs = RunsRepo::list_by_source(&pool, "non-existent-source", 10)
            .await
            .unwrap();
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

    #[tokio::test]
    async fn test_list_by_source_includes_global_runs_with_logs() {
        let pool = init_memory_db().await.unwrap();

        // Global run has source_id: None on scrape_runs
        let global_run = RunsRepo::start(&pool, TriggerType::Manual).await.unwrap();
        let _ = RunsRepo::finish(&pool, &global_run.id, RunStatus::Success, 50, 10, None)
            .await
            .unwrap();

        // Record a log entry under this global run for a specific source
        RunsRepo::add_log(
            &pool,
            &global_run.id,
            LogLevel::Info,
            Some("workflow-source-xyz"),
            "Scraped 50 jobs for workflow-source-xyz",
        )
        .await
        .unwrap();

        // Querying runs for workflow-source-xyz should return the global run
        let runs = RunsRepo::list_by_source(&pool, "workflow-source-xyz", 10)
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, global_run.id);

        // Querying an unrelated source with no logs returns empty
        let empty = RunsRepo::list_by_source(&pool, "unrelated-source", 10)
            .await
            .unwrap();
        assert!(empty.is_empty());
    }
}
