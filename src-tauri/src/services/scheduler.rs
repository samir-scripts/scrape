use crate::db::runs::TriggerType;
use crate::db::settings::SettingsRepo;
use crate::error::{AppError, Result};
use crate::services::runner::ScrapeRunner;
use chrono::SecondsFormat;
use chrono_tz::Tz;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CronValidationResult {
    pub ok: bool,
    pub next_runs: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SchedulerStatus {
    pub enabled: bool,
    pub next_run_at: Option<String>,
    pub running: bool,
}

pub struct SchedulerService {
    scheduler: JobScheduler,
    job_id: Option<Uuid>,
    is_paused: bool,
    pool: SqlitePool,
    runner: ScrapeRunner,
    app: AppHandle,
}

impl SchedulerService {
    pub async fn init(
        pool: SqlitePool,
        runner: ScrapeRunner,
        app: AppHandle,
    ) -> Result<Arc<Mutex<Self>>> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to create scheduler: {}", e)))?;

        let service = Arc::new(Mutex::new(Self {
            scheduler,
            job_id: None,
            is_paused: false,
            pool: pool.clone(),
            runner: runner.clone(),
            app: app.clone(),
        }));

        // Read settings
        let enabled: bool = SettingsRepo::get_value(&pool, "scraper.enabled")
            .await
            .ok()
            .flatten()
            .unwrap_or(true);
        let cron_expr: String = SettingsRepo::get_value(&pool, "scraper.cron")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "0 0 12 * * *".to_string());
        let run_on_startup: bool = SettingsRepo::get_value(&pool, "scraper.run_on_startup")
            .await
            .ok()
            .flatten()
            .unwrap_or(false);

        if enabled {
            let mut s = service.lock().await;
            if let Err(e) = s.schedule_job(&cron_expr).await {
                tracing::warn!("Failed to schedule scrape job with '{}': {}", cron_expr, e);
            }
        }

        // Start scheduler loop
        {
            let s = service.lock().await;
            s.scheduler
                .start()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to start scheduler: {}", e)))?;
        }

        // Run on startup delay (~10s) if enabled
        if run_on_startup {
            let app_clone = app.clone();
            let runner_clone = runner.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                tracing::info!("Executing startup scrape trigger...");
                let _ = runner_clone.trigger(&app_clone, TriggerType::Startup).await;
            });
        }

        Ok(service)
    }

    pub async fn schedule_job(&mut self, cron_expr: &str) -> Result<()> {
        if let Some(id) = self.job_id.take() {
            let _ = self.scheduler.remove(&id).await;
        }

        let tz_str: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());
        let tz = Tz::from_str(&tz_str).unwrap_or(chrono_tz::UTC);

        let runner = self.runner.clone();
        let app = self.app.clone();

        let job = Job::new_async_tz(cron_expr, tz, move |_uuid, _lock| {
            let runner = runner.clone();
            let app = app.clone();
            Box::pin(async move {
                tracing::info!("Scheduler tick: triggering scrape...");
                let _ = runner.trigger(&app, TriggerType::Schedule).await;
            })
        })
        .map_err(|e| {
            AppError::Validation(format!("Invalid cron expression '{}': {}", cron_expr, e))
        })?;

        let id = self
            .scheduler
            .add(job)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to add scheduled job: {}", e)))?;

        self.job_id = Some(id);
        Ok(())
    }

    pub async fn reschedule(&mut self, cron_expr: &str) -> Result<()> {
        self.schedule_job(cron_expr).await
    }

    pub async fn set_paused(&mut self, paused: bool) -> Result<()> {
        self.is_paused = paused;
        if paused {
            if let Some(id) = self.job_id.take() {
                let _ = self.scheduler.remove(&id).await;
            }
        } else {
            let cron_expr: String = SettingsRepo::get_value(&self.pool, "scraper.cron")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "0 0 12 * * *".to_string());
            self.schedule_job(&cron_expr).await?;
        }
        Ok(())
    }

    pub async fn get_status(&self) -> Result<SchedulerStatus> {
        let enabled = !self.is_paused && self.job_id.is_some();
        let running = self.runner.is_running();

        let cron_expr: String = SettingsRepo::get_value(&self.pool, "scraper.cron")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "0 0 12 * * *".to_string());
        let tz_str: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());

        let next_run_at = calculate_next_run(&cron_expr, &tz_str);

        Ok(SchedulerStatus {
            enabled,
            next_run_at,
            running,
        })
    }
}

pub fn validate_cron_expression(expr: &str, tz_str: &str) -> CronValidationResult {
    let tz = Tz::from_str(tz_str).unwrap_or(chrono_tz::UTC);
    match Schedule::from_str(expr) {
        Ok(schedule) => {
            let now = chrono::Utc::now().with_timezone(&tz);
            let next_runs: Vec<String> = schedule
                .after(&now)
                .take(3)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
                .collect();
            CronValidationResult {
                ok: true,
                next_runs,
                error: None,
            }
        }
        Err(e) => CronValidationResult {
            ok: false,
            next_runs: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}

fn calculate_next_run(cron_expr: &str, tz_str: &str) -> Option<String> {
    let tz = Tz::from_str(tz_str).unwrap_or(chrono_tz::UTC);
    let schedule = Schedule::from_str(cron_expr).ok()?;
    let now = chrono::Utc::now().with_timezone(&tz);
    schedule
        .after(&now)
        .next()
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cron_valid() {
        let res = validate_cron_expression("0 0 12 * * *", "America/New_York");
        assert!(res.ok);
        assert_eq!(res.next_runs.len(), 3);
        assert!(res.error.is_none());
    }

    #[test]
    fn test_validate_cron_invalid() {
        let res = validate_cron_expression("invalid cron", "UTC");
        assert!(!res.ok);
        assert!(res.next_runs.is_empty());
        assert!(res.error.is_some());
    }

    #[test]
    fn test_calculate_next_run() {
        let next_utc = calculate_next_run("0 0 12 * * *", "UTC");
        assert!(next_utc.is_some());

        let next_ny = calculate_next_run("0 0 12 * * *", "America/New_York");
        assert!(next_ny.is_some());

        let invalid = calculate_next_run("invalid-cron", "UTC");
        assert!(invalid.is_none());
    }

    #[tokio::test]
    async fn test_scheduler_lifecycle() {
        let sched = JobScheduler::new().await.unwrap();
        sched.start().await.unwrap();

        let tz = chrono_tz::UTC;
        let job = Job::new_async_tz("0 0 12 * * *", tz, |_uuid, _lock| Box::pin(async {})).unwrap();

        let id = sched.add(job).await.unwrap();
        assert!(sched.remove(&id).await.is_ok());
    }
}
