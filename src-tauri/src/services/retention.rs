use crate::db::jobs::JobsRepo;
use crate::db::settings::SettingsRepo;
use crate::db::sources::{Source, SourcesRepo};
use crate::error::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use ts_rs::TS;

const STATUS_KEY: &str = "data.retention_status";
const RETRY_BUSY_AFTER: Duration = Duration::from_secs(60);
const SWEEP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RetentionStatus {
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    #[ts(type = "number")]
    pub last_deleted_records: u64,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RetentionOutcome {
    pub attempted_at: String,
    pub deleted_records: u64,
    pub failures: Vec<String>,
}

impl RetentionOutcome {
    fn new(now: DateTime<Utc>) -> Self {
        Self {
            attempted_at: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            deleted_records: 0,
            failures: Vec::new(),
        }
    }
}

pub struct RetentionService;

impl RetentionService {
    /// Apply the single repository retention predicate to a fixed source set and instant.
    pub async fn prune_sources(
        pool: &SqlitePool,
        sources: &[Source],
        now: DateTime<Utc>,
    ) -> RetentionOutcome {
        let mut outcome = RetentionOutcome::new(now);
        for source in sources {
            if source.retention_days == 0 {
                continue;
            }
            match JobsRepo::prune_expired_by_source_at(
                pool,
                &source.id,
                source.retention_days,
                &source.retention_policy,
                now,
            )
            .await
            {
                Ok(deleted) => outcome.deleted_records += deleted,
                Err(error) => outcome
                    .failures
                    .push(format!("{} ({}): {error}", source.name, source.id)),
            }
        }
        outcome
    }

    pub async fn sweep(pool: &SqlitePool, now: DateTime<Utc>) -> RetentionOutcome {
        match SourcesRepo::list(pool).await {
            Ok(sources) => Self::prune_sources(pool, &sources, now).await,
            Err(error) => {
                let mut outcome = RetentionOutcome::new(now);
                outcome
                    .failures
                    .push(format!("Could not load workflows: {error}"));
                outcome
            }
        }
    }

    pub async fn prune_source_ids(
        pool: &SqlitePool,
        source_ids: impl IntoIterator<Item = String>,
        now: DateTime<Utc>,
    ) -> RetentionOutcome {
        let mut outcome = RetentionOutcome::new(now);
        let mut seen = HashSet::new();
        for id in source_ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            match SourcesRepo::get(pool, &id).await {
                Ok(source) => {
                    let one = Self::prune_sources(pool, std::slice::from_ref(&source), now).await;
                    outcome.deleted_records += one.deleted_records;
                    outcome.failures.extend(one.failures);
                }
                Err(error) => outcome.failures.push(format!("Workflow {id}: {error}")),
            }
        }
        outcome
    }

    pub async fn record(pool: &SqlitePool, outcome: &RetentionOutcome) -> Result<RetentionStatus> {
        let mut status = SettingsRepo::get_value::<RetentionStatus>(pool, STATUS_KEY)
            .await?
            .unwrap_or_default();
        status.last_attempt_at = Some(outcome.attempted_at.clone());
        status.last_deleted_records = outcome.deleted_records;
        if outcome.failures.is_empty() {
            status.last_success_at = Some(outcome.attempted_at.clone());
            status.consecutive_failures = 0;
            status.last_error = None;
        } else {
            status.consecutive_failures = status.consecutive_failures.saturating_add(1);
            status.last_error = Some(outcome.failures.join("; "));
        }
        SettingsRepo::set_value(pool, STATUS_KEY, serde_json::to_value(&status)?).await?;
        Ok(status)
    }

    pub async fn get_status(pool: &SqlitePool) -> Result<RetentionStatus> {
        Ok(SettingsRepo::get_value(pool, STATUS_KEY)
            .await?
            .unwrap_or_default())
    }

    pub async fn publish(pool: &SqlitePool, app: &AppHandle, outcome: &RetentionOutcome) {
        match Self::record(pool, outcome).await {
            Ok(status) => Self::notify_for_command(app, &status, outcome),
            Err(error) => tracing::error!("Could not save retention outcome: {error}"),
        }
    }

    pub(crate) fn notify_for_command(
        app: &AppHandle,
        status: &RetentionStatus,
        outcome: &RetentionOutcome,
    ) {
        let _ = app.emit("retention://updated", status);
        for failure in &outcome.failures {
            tracing::error!("Automatic retention failed: {failure}");
        }
    }

    /// Startup sweep and 24-hour elapsed-time sweeps continue while scraping is paused.
    /// Busy coordination defers for one minute and never cancels active work.
    pub fn start(pool: SqlitePool, runner: crate::services::runner::ScrapeRunner, app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            loop {
                let Some(_permit) = runner.try_retention_permit() else {
                    tokio::time::sleep(RETRY_BUSY_AFTER).await;
                    continue;
                };
                let now = Utc::now();
                let outcome = Self::sweep(&pool, now).await;
                let status = Self::record(&pool, &outcome).await;
                drop(_permit);
                match status {
                    Ok(status) => Self::notify_for_command(&app, &status, &outcome),
                    Err(error) => tracing::error!("Could not save retention outcome: {error}"),
                }
                tokio::time::sleep(SWEEP_INTERVAL).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sources::{CreateSourceInput, SourcesRepo};
    use chrono::TimeZone;
    use scrape_core::SourceKind;

    async fn source(pool: &SqlitePool, id: &str, days: i64, enabled: bool) -> Source {
        SourcesRepo::create(
            pool,
            CreateSourceInput {
                kind: SourceKind::Greenhouse,
                name: id.into(),
                identifier: id.into(),
                config: None,
                enabled: Some(enabled),
                cadence: None,
                cadence_cron: None,
                retention_days: Some(days),
                retention_policy: Some("untracked_only".into()),
                filters: None,
            },
        )
        .await
        .unwrap()
    }

    async fn old_job(pool: &SqlitePool, source: &Source, id: &str) {
        sqlx::query("INSERT INTO jobs (id,dedupe_key,source_id,title,company,url,date_added,last_seen_at,status,status_changed_at) VALUES (?1,?1,?2,'Role','Org','https://example.test/job','2000-01-01T00:00:00Z','2026-10-15T12:00:00Z','scraped','2000-01-01T00:00:00Z')")
            .bind(id).bind(&source.id).execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn sweep_runs_while_paused_and_includes_disabled_workflows_but_zero_days_keep_forever() {
        let pool = crate::db::init_memory_db().await.unwrap();
        SettingsRepo::set_value(&pool, "scraper.enabled", serde_json::json!(false))
            .await
            .unwrap();
        let disabled = source(&pool, "disabled", 30, false).await;
        let forever = source(&pool, "forever", 0, true).await;
        old_job(&pool, &disabled, "expired-disabled").await;
        old_job(&pool, &forever, "kept-forever").await;
        let now = Utc.with_ymd_and_hms(2026, 10, 15, 12, 0, 0).unwrap();
        let outcome = RetentionService::sweep(&pool, now).await;
        assert!(outcome.failures.is_empty());
        assert_eq!(outcome.deleted_records, 1);
        assert!(JobsRepo::get(&pool, "expired-disabled").await.is_err());
        assert!(JobsRepo::get(&pool, "kept-forever").await.is_ok());
        let paused: bool = SettingsRepo::get_value(&pool, "scraper.enabled")
            .await
            .unwrap()
            .unwrap();
        assert!(!paused);
    }

    #[tokio::test]
    async fn cleanup_status_reports_failures_and_resets_after_a_success() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let now = Utc.with_ymd_and_hms(2026, 10, 15, 12, 0, 0).unwrap();
        let mut failed = RetentionOutcome::new(now);
        failed.failures.push("Fixture database failure".into());
        let status = RetentionService::record(&pool, &failed).await.unwrap();
        assert_eq!(status.consecutive_failures, 1);
        assert!(status.last_success_at.is_none());
        assert_eq!(
            status.last_error.as_deref(),
            Some("Fixture database failure")
        );

        let success = RetentionOutcome::new(now + chrono::Duration::days(1));
        let status = RetentionService::record(&pool, &success).await.unwrap();
        assert_eq!(status.consecutive_failures, 0);
        assert!(status.last_error.is_none());
        assert_eq!(status.last_success_at, Some(success.attempted_at));
    }
}
