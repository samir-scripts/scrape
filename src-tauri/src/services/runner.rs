use crate::db::jobs::{JobsRepo, ScrapedJob};
use crate::db::runs::{RunStatus, RunsRepo, ScrapeRun, TriggerType};
use crate::db::settings::SettingsRepo;
use crate::db::sources::SourcesRepo;
use crate::error::{AppError, Result};
use scrape_core::{
    RoleType, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource,
    PROTOCOL_VERSION,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::time::{timeout, Instant};

pub struct RunLockGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for RunLockGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct ScrapeRunner {
    pool: SqlitePool,
    is_running: Arc<AtomicBool>,
}

impl ScrapeRunner {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn try_acquire_lock(&self) -> Result<RunLockGuard> {
        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            tracing::warn!("Scrape run skipped: another run is already in progress");
            return Err(AppError::AlreadyRunning);
        }
        Ok(RunLockGuard {
            flag: self.is_running.clone(),
        })
    }

    pub async fn trigger(&self, app: &AppHandle, trigger: TriggerType) -> Result<ScrapeRun> {
        // 1. Overlap guard
        let _guard = self.try_acquire_lock()?;

        // 2. Start scrape run in DB
        let run = RunsRepo::start(&self.pool, trigger).await?;
        let _ = app.emit("scrape://started", serde_json::json!({ "run_id": run.id }));

        // 3. Load sources & settings
        let sources = SourcesRepo::list(&self.pool).await?;
        let enabled_sources: Vec<WorkerSource> = sources
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| WorkerSource {
                id: s.id,
                kind: s.kind,
                name: s.name,
                identifier: s.identifier,
                config: s.config,
            })
            .collect();

        if enabled_sources.is_empty() {
            tracing::info!("No enabled sources to scrape");
            let finished = RunsRepo::finish(
                &self.pool,
                &run.id,
                RunStatus::Success,
                0,
                0,
                Some("No enabled sources"),
            )
            .await?;
            let _ = app.emit("scrape://finished", &finished);

            let tz_name: String = SettingsRepo::get_value(&self.pool, "system.timezone")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "UTC".to_string());
            let tz: chrono_tz::Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
            let now_tz = chrono::Utc::now().with_timezone(&tz);
            let time_str = now_tz.format("%H:%M").to_string();
            crate::services::tray::update_last_run(app, &time_str, 0);

            return Ok(finished);
        }

        let include_keywords: Vec<String> =
            SettingsRepo::get_value(&self.pool, "filters.include_keywords")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
        let exclude_keywords: Vec<String> =
            SettingsRepo::get_value(&self.pool, "filters.exclude_keywords")
                .await
                .ok()
                .flatten()
                .unwrap_or_default();
        let locations: Vec<String> = SettingsRepo::get_value(&self.pool, "filters.locations")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        let role_types: Vec<RoleType> = SettingsRepo::get_value(&self.pool, "filters.role_types")
            .await
            .ok()
            .flatten()
            .unwrap_or_default();

        let config = WorkerConfig {
            version: PROTOCOL_VERSION,
            sources: enabled_sources,
            filters: WorkerFilters {
                include_keywords,
                exclude_keywords,
                locations,
                role_types,
            },
            limits: WorkerLimits::default(),
        };

        // 4. Run sidecar with 10 minute timeout
        let run_result = timeout(
            Duration::from_secs(600),
            self.execute_sidecar(app, &run.id, config),
        )
        .await;

        match run_result {
            Ok(Ok((total_found, total_new, source_statuses))) => {
                // Update source statuses
                for (src_id, (status, err_msg)) in source_statuses {
                    let _ = SourcesRepo::set_last_result(
                        &self.pool,
                        &src_id,
                        &status,
                        err_msg.as_deref(),
                    )
                    .await;
                }

                let final_status = RunStatus::Success;
                let finished = RunsRepo::finish(
                    &self.pool,
                    &run.id,
                    final_status,
                    total_found,
                    total_new,
                    None,
                )
                .await?;

                let _ = app.emit("scrape://finished", &finished);
                let _ = app.emit("jobs://changed", ());

                // Send desktop notification if enabled
                let notifications_enabled: bool =
                    SettingsRepo::get_value(&self.pool, "notifications.enabled")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or(true);

                if notifications_enabled && total_new > 0 {
                    let _ = app
                        .notification()
                        .builder()
                        .title("SCRAPE COMPLETE")
                        .body(format!("{} NEW POSTINGS", total_new))
                        .show();
                }

                // Update system tray label
                let tz_name: String = SettingsRepo::get_value(&self.pool, "system.timezone")
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "UTC".to_string());
                let tz: chrono_tz::Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
                let now_tz = chrono::Utc::now().with_timezone(&tz);
                let time_str = now_tz.format("%H:%M").to_string();
                crate::services::tray::update_last_run(app, &time_str, total_new);

                Ok(finished)
            }
            Ok(Err(err)) => {
                let err_str = err.to_string();
                tracing::error!("Scrape execution error: {}", err_str);
                let finished =
                    RunsRepo::finish(&self.pool, &run.id, RunStatus::Failed, 0, 0, Some(&err_str))
                        .await?;
                let _ = app.emit("scrape://finished", &finished);
                Err(err)
            }
            Err(_) => {
                tracing::error!("Scrape execution timed out (hard 10m limit)");
                let finished = RunsRepo::finish(
                    &self.pool,
                    &run.id,
                    RunStatus::Failed,
                    0,
                    0,
                    Some("Execution timed out (10 minute limit)"),
                )
                .await?;
                let _ = app.emit("scrape://finished", &finished);
                Err(AppError::Internal("Scrape execution timed out".to_string()))
            }
        }
    }

    async fn execute_sidecar(
        &self,
        app: &AppHandle,
        _run_id: &str,
        config: WorkerConfig,
    ) -> Result<(i64, i64, HashMap<String, (String, Option<String>)>)> {
        let (mut rx, mut child) = app
            .shell()
            .sidecar("scrape-worker")
            .map_err(|e| AppError::Internal(format!("Failed to find sidecar: {}", e)))?
            .args(["--stdin"])
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to spawn sidecar: {}", e)))?;

        let config_bytes = serde_json::to_vec(&config)?;
        child
            .write(&config_bytes)
            .map_err(|e| AppError::Internal(format!("Failed to write to sidecar stdin: {}", e)))?;

        let mut job_buffer: Vec<ScrapedJob> = Vec::new();
        let mut last_flush = Instant::now();
        let mut total_found = 0i64;
        let mut total_new = 0i64;
        let mut source_statuses: HashMap<String, (String, Option<String>)> = HashMap::new();

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(worker_event) = serde_json::from_str::<WorkerEvent>(trimmed) {
                            match worker_event {
                                WorkerEvent::Job {
                                    source_id,
                                    external_id,
                                    title,
                                    company,
                                    role_type,
                                    location,
                                    url,
                                    date_posted,
                                } => {
                                    let dedupe_key = format!("{}:{}", source_id, external_id);
                                    let loc_opt = if location.is_empty() {
                                        None
                                    } else {
                                        Some(location)
                                    };

                                    job_buffer.push(ScrapedJob {
                                        dedupe_key,
                                        source_id: Some(source_id),
                                        external_id: Some(external_id),
                                        title,
                                        company,
                                        role_type,
                                        location: loc_opt,
                                        url,
                                        date_posted,
                                    });
                                    total_found += 1;

                                    if job_buffer.len() >= 50
                                        || last_flush.elapsed() >= Duration::from_millis(500)
                                    {
                                        let stats =
                                            JobsRepo::upsert_scraped(&self.pool, &job_buffer)
                                                .await?;
                                        total_new += stats.inserted as i64;
                                        job_buffer.clear();
                                        last_flush = Instant::now();
                                    }
                                }
                                WorkerEvent::Progress {
                                    source_id,
                                    fetched,
                                    matched,
                                } => {
                                    source_statuses
                                        .entry(source_id.clone())
                                        .or_insert_with(|| ("success".to_string(), None));
                                    let _ = app.emit(
                                        "scrape://progress",
                                        serde_json::json!({
                                            "source_id": source_id,
                                            "fetched": fetched,
                                            "matched": matched,
                                        }),
                                    );
                                }
                                WorkerEvent::Error {
                                    source_id,
                                    message,
                                    retryable,
                                } => {
                                    source_statuses.insert(
                                        source_id.clone(),
                                        ("error".to_string(), Some(message.clone())),
                                    );
                                    let _ = app.emit(
                                        "scrape://progress",
                                        serde_json::json!({
                                            "source_id": source_id,
                                            "error": message,
                                            "retryable": retryable,
                                        }),
                                    );
                                }
                                WorkerEvent::Done { .. } => {}
                            }
                        }
                    }
                }
                CommandEvent::Stderr(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    tracing::info!("[sidecar] {}", text.trim());
                }
                CommandEvent::Error(err) => {
                    tracing::error!("[sidecar error] {}", err);
                }
                CommandEvent::Terminated(status) => {
                    tracing::info!("[sidecar exit] status: {:?}", status.code);
                    break;
                }
                _ => {}
            }
        }

        // Flush remaining buffered jobs
        if !job_buffer.is_empty() {
            let stats = JobsRepo::upsert_scraped(&self.pool, &job_buffer).await?;
            total_new += stats.inserted as i64;
            job_buffer.clear();
        }

        Ok((total_found, total_new, source_statuses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_overlap_guard() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let runner = ScrapeRunner::new(pool);
        assert!(!runner.is_running());

        let guard = runner.try_acquire_lock().unwrap();
        assert!(runner.is_running());

        // Concurrently acquiring lock fails with AlreadyRunning
        let second = runner.try_acquire_lock();
        match second {
            Err(AppError::AlreadyRunning) => {}
            _ => panic!("Expected AlreadyRunning error"),
        }

        // Drop guard and verify lock is released
        drop(guard);
        assert!(!runner.is_running());

        let third = runner.try_acquire_lock();
        assert!(third.is_ok());
    }
}
