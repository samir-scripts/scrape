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
use std::sync::atomic::{AtomicBool, Ordering};
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
    is_paused: Arc<AtomicBool>,
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
            is_paused: Arc::new(AtomicBool::new(true)),
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

        service
            .lock()
            .await
            .is_paused
            .store(!enabled, Ordering::SeqCst);
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
        if enabled && run_on_startup {
            let paused = service.lock().await.is_paused.clone();
            let app_clone = app.clone();
            let runner_clone = runner.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if paused.load(Ordering::SeqCst) {
                    return;
                }
                tracing::info!("Executing startup scrape trigger...");
                let _ = runner_clone.trigger(&app_clone, TriggerType::Startup).await;
            });
        }

        // Background ticker for custom cadence sources (checks every 60 seconds)
        let paused = service.lock().await.is_paused.clone();
        let ticker_pool = pool.clone();
        let ticker_runner = runner.clone();
        let ticker_app = app.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if paused.load(Ordering::SeqCst) {
                    continue;
                }
                if let Ok(due_sources) =
                    crate::db::sources::SourcesRepo::get_due_sources(&ticker_pool).await
                {
                    for source in due_sources {
                        if paused.load(Ordering::SeqCst) {
                            break;
                        }
                        tracing::info!(
                            "Custom cadence trigger for source {} ({})",
                            source.name,
                            source.id
                        );
                        let _ = ticker_runner
                            .trigger_single_source(&ticker_app, &source.id)
                            .await;
                    }
                }
            }
        });

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

        if self.is_paused.load(Ordering::SeqCst) {
            return Ok(());
        }
        let paused = self.is_paused.clone();
        let runner = self.runner.clone();
        let app = self.app.clone();

        let job = Job::new_async_tz(cron_expr, tz, move |_uuid, _lock| {
            let paused = paused.clone();
            let runner = runner.clone();
            let app = app.clone();
            Box::pin(async move {
                if paused.load(Ordering::SeqCst) {
                    return;
                }
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
        self.is_paused.store(paused, Ordering::SeqCst);
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
        let enabled = !self.is_paused.load(Ordering::SeqCst);
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

        let next_run_at = if enabled {
            calculate_next_run(&cron_expr, &tz_str)
        } else {
            None
        };

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
                .map(|dt| {
                    dt.with_timezone(&chrono::Utc)
                        .to_rfc3339_opts(SecondsFormat::Millis, true)
                })
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

pub fn calculate_next_run(cron_expr: &str, tz_str: &str) -> Option<String> {
    let tz = Tz::from_str(tz_str).unwrap_or(chrono_tz::UTC);
    let schedule = Schedule::from_str(cron_expr).ok()?;
    let now = chrono::Utc::now().with_timezone(&tz);
    schedule.after(&now).next().map(|dt| {
        dt.with_timezone(&chrono::Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, true)
    })
}

pub fn calculate_next_run_for_source(
    source: &crate::db::sources::Source,
    tz_str: &str,
) -> Option<String> {
    use scrape_core::SourceCadence;
    let cron_str = match source.cadence {
        SourceCadence::Global | SourceCadence::Manual => return None,
        SourceCadence::Cron => source.cadence_cron.as_deref()?,
        other => other.default_cron()?,
    };
    calculate_next_run(cron_str, tz_str)
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

        // Verify clock schedule expressions
        for expr in &[
            "0 30 14 * * *",       // Daily at 14:30
            "0 30 14 * * 1-5",     // Weekdays numeric
            "0 30 14 * * MON-FRI", // Weekdays named
            "0 30 14 * * 6,7",     // Weekends numeric
            "0 30 14 * * SAT,SUN", // Weekends named
            "0 30 */12 * * *",     // Every 12 hours
            "0 30 */6 * * *",      // Every 6 hours
            "0 30 * * * *",        // Hourly
        ] {
            let res = validate_cron_expression(expr, "UTC");
            assert!(
                res.ok,
                "Expected expr '{}' to be valid, got error: {:?}",
                expr, res.error
            );
        }
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

    #[test]
    fn test_calculate_next_run_for_source() {
        use scrape_core::SourceCadence;

        let mut source = crate::db::sources::Source {
            id: "src-1".to_string(),
            kind: scrape_core::SourceKind::Greenhouse,
            name: "Acme".to_string(),
            identifier: "acme".to_string(),
            config: serde_json::json!({}),
            enabled: true,
            last_run_at: None,
            last_status: None,
            last_error: None,
            created_at: "2026-10-10T00:00:00Z".to_string(),
            cadence: SourceCadence::Global,
            cadence_cron: None,
            next_run_at: None,
            last_run_duration_ms: None,
            retention_days: 30,
            retention_policy: "untracked_only".to_string(),
            filters: serde_json::json!({}),
        };

        // Global cadence has no independent next run
        assert!(calculate_next_run_for_source(&source, "UTC").is_none());

        // Every 12h preset calculates next run
        source.cadence = SourceCadence::Every12h;
        assert!(calculate_next_run_for_source(&source, "UTC").is_some());

        // Custom Cron
        source.cadence = SourceCadence::Cron;
        source.cadence_cron = Some("0 30 9 * * *".to_string());
        assert!(calculate_next_run_for_source(&source, "UTC").is_some());
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
