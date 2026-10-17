use crate::db::jobs::{JobsRepo, ScrapedJob};
use crate::db::runs::{LogLevel, RunCounts, RunStatus, RunsRepo, ScrapeRun, TriggerType};
use crate::db::settings::SettingsRepo;
use crate::db::sources::SourcesRepo;
use crate::error::{AppError, Result};
use scrape_core::{
    WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, PROTOCOL_VERSION,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::process::CommandEvent;
use tokio::time::{timeout_at, Instant};

pub struct RunLockGuard {
    flag: Arc<AtomicBool>,
    _permit: tokio::sync::OwnedRwLockReadGuard<()>,
}

impl RunLockGuard {
    // Status readers responding to completion must observe the released run lock.
    fn finish(self, notify: impl FnOnce()) {
        drop(self);
        notify();
    }
}

impl Drop for RunLockGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

pub struct MaintenanceGuard {
    flag: Arc<AtomicBool>,
    writes: Arc<tokio::sync::RwLock<()>>,
}
impl MaintenanceGuard {
    pub async fn exclusive(&self) -> Result<tokio::sync::OwnedRwLockWriteGuard<()>> {
        tokio::time::timeout(Duration::from_secs(75), self.writes.clone().write_owned()).await
            .map_err(|_| AppError::Validation("Cleanup could not stop active work within 75 seconds. No data was deleted. Try again once the run has stopped.".into()))
    }
}
impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

fn aggregate_run_status(
    statuses: &HashMap<String, (String, Option<String>)>,
    found: i64,
) -> RunStatus {
    let values: Vec<_> = statuses
        .values()
        .map(|(status, _)| RunStatus::from_source_status(status))
        .collect();
    if values.contains(&RunStatus::Cancelled) {
        RunStatus::Cancelled
    } else if !values.is_empty()
        && found == 0
        && values.iter().all(|status| *status == RunStatus::Failed)
    {
        RunStatus::Failed
    } else if values
        .iter()
        .any(|status| matches!(status, RunStatus::Failed | RunStatus::Partial))
    {
        RunStatus::Partial
    } else if values.contains(&RunStatus::SuccessWithWarnings) {
        RunStatus::SuccessWithWarnings
    } else {
        RunStatus::Success
    }
}

pub fn resolve_worker_filters(
    source_filters: &serde_json::Value,
    global_filters: WorkerFilters,
) -> Result<WorkerFilters> {
    let mut effective = serde_json::to_value(global_filters)?;
    if !source_filters.is_null() {
        let overrides = source_filters
            .as_object()
            .ok_or_else(|| AppError::Validation("Workflow filters must be an object".into()))?;
        for (key, value) in overrides {
            effective[key] = value.clone();
        }
    }
    let filters: WorkerFilters = serde_json::from_value(effective)
        .map_err(|e| AppError::Validation(format!("workflow.filters: {e}")))?;
    filters.to_expr().map_err(AppError::Validation)?;
    Ok(filters)
}

pub async fn load_global_filters(pool: &SqlitePool) -> Result<WorkerFilters> {
    Ok(WorkerFilters {
        include_keywords: SettingsRepo::get_value(pool, "filters.include_keywords")
            .await?
            .unwrap_or_default(),
        exclude_keywords: SettingsRepo::get_value(pool, "filters.exclude_keywords")
            .await?
            .unwrap_or_default(),
        locations: SettingsRepo::get_value(pool, "filters.locations")
            .await?
            .unwrap_or_default(),
        role_types: SettingsRepo::get_value(pool, "filters.role_types")
            .await?
            .unwrap_or_default(),
        query: None,
        policy: None,
        compensation: None,
        matching: None,
    })
}

pub(crate) fn worker_failure(
    exit_code: Option<i32>,
    done: bool,
    error: Option<&str>,
) -> Option<String> {
    if let Some(error) = error {
        return Some(error.to_string());
    }
    if exit_code != Some(0) {
        return Some(format!(
            "Worker exited without success (code {exit_code:?})"
        ));
    }
    if !done {
        return Some("Worker exited without its required completion event".into());
    }
    None
}

#[derive(Clone)]
pub struct ScrapeRunner {
    pool: SqlitePool,
    is_running: Arc<AtomicBool>,
    cancel_requested: Arc<AtomicBool>,
    maintenance: Arc<AtomicBool>,
    writes: Arc<tokio::sync::RwLock<()>>,
}

impl ScrapeRunner {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
            maintenance: Arc::new(AtomicBool::new(false)),
            writes: Arc::new(tokio::sync::RwLock::new(())),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn cancel_current(&self) -> bool {
        if !self.is_running() {
            return false;
        }
        self.cancel_requested.store(true, Ordering::SeqCst);
        true
    }

    /// Every app mutation holds a permit; maintenance rejects new mutations
    /// before waiting for existing workers/commands to finish.
    pub fn write_permit(&self) -> Result<tokio::sync::OwnedRwLockReadGuard<()>> {
        if self.maintenance.load(Ordering::SeqCst) {
            return Err(AppError::Validation(
                "Data cleanup is in progress. Try again after it finishes.".into(),
            ));
        }
        self.writes.clone().try_read_owned().map_err(|_| {
            AppError::Validation("Data cleanup is in progress. Try again after it finishes.".into())
        })
    }

    /// Routine housekeeping never cancels a scrape or waits behind explicit
    /// maintenance. `None` means another coordinated write currently owns the window.
    pub fn try_retention_permit(&self) -> Option<tokio::sync::OwnedRwLockWriteGuard<()>> {
        if self.maintenance.load(Ordering::SeqCst) {
            return None;
        }
        let permit = self.writes.clone().try_write_owned().ok()?;
        if self.maintenance.load(Ordering::SeqCst) {
            return None;
        }
        Some(permit)
    }

    pub fn begin_maintenance(&self) -> Result<MaintenanceGuard> {
        self.maintenance
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| {
                AppError::Validation("Another data cleanup is already in progress.".into())
            })?;
        self.cancel_current();
        Ok(MaintenanceGuard {
            flag: self.maintenance.clone(),
            writes: self.writes.clone(),
        })
    }

    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.cancel_requested.clone()
    }

    pub fn try_acquire_lock(&self) -> Result<RunLockGuard> {
        let permit = self.write_permit()?;
        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            tracing::warn!("Scrape run skipped: another run is already in progress");
            return Err(AppError::AlreadyRunning);
        }
        self.cancel_requested.store(false, Ordering::SeqCst);
        if self.maintenance.load(Ordering::SeqCst) {
            self.is_running.store(false, Ordering::SeqCst);
            return Err(AppError::Validation("Data cleanup is in progress.".into()));
        }
        Ok(RunLockGuard {
            flag: self.is_running.clone(),
            _permit: permit,
        })
    }

    async fn log_event(
        &self,
        app: &AppHandle,
        run_id: &str,
        level: LogLevel,
        source_id: Option<&str>,
        message: &str,
    ) {
        let level_str = match level {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        let _ = RunsRepo::add_log(&self.pool, run_id, level, source_id, message).await;
        crate::services::system::SystemService::append_log(app, level_str, source_id, message);
    }

    pub async fn trigger(&self, app: &AppHandle, trigger: TriggerType) -> Result<ScrapeRun> {
        // 1. Overlap guard
        let _guard = self.try_acquire_lock()?;

        // 3. Load sources & settings
        let sources = SourcesRepo::list(&self.pool).await?;
        let sources: Vec<_> = sources
            .into_iter()
            .filter(|s| {
                s.enabled
                    && !s.needs_review
                    && (trigger == TriggerType::Manual
                        || s.cadence == scrape_core::SourceCadence::Global)
            })
            .collect();
        for source in &sources {
            super::workflow::validated_stored_config(source.kind, &source.config)?;
        }
        let run = RunsRepo::start(&self.pool, trigger).await?;
        let _ = app.emit("scrape://started", serde_json::json!({ "run_id": run.id }));

        let tz_name: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());

        if sources.is_empty() {
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
            _guard.finish(|| {
                let _ = app.emit("scrape://finished", &finished);
            });

            let tz: chrono_tz::Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
            let now_tz = chrono::Utc::now().with_timezone(&tz);
            let time_str = now_tz.format("%H:%M").to_string();
            crate::services::tray::update_last_run(app, &time_str, 0);

            return Ok(finished);
        }

        let mut total_found = 0;
        let mut total_new = 0;
        let mut source_statuses = HashMap::new();

        for source in &sources {
            if self.cancel_requested.load(Ordering::SeqCst) {
                source_statuses.insert(
                    source.id.clone(),
                    (
                        "cancelled".to_string(),
                        Some("Run cancelled by user".to_string()),
                    ),
                );
                break;
            }

            let spec_opt: Option<super::workflow::WorkflowSpec> = source
                .config
                .get("__workflow_spec")
                .and_then(|v| serde_json::from_value(v.clone()).ok());

            let (worker_sources, max_jobs) = if let Some(spec) = &spec_opt {
                let inputs = super::workflow::workflow_source_inputs(spec, &tz_name)?;
                let w_sources: Vec<WorkerSource> = spec
                    .sources
                    .iter()
                    .zip(inputs.iter())
                    .filter(|(s, _)| s.enabled)
                    .map(|(s, input)| WorkerSource {
                        id: if spec.sources.len() == 1 {
                            source.id.clone()
                        } else {
                            s.id.clone()
                        },
                        kind: input.kind,
                        name: s
                            .employer_hint
                            .clone()
                            .or_else(|| spec.metadata.company.clone())
                            .unwrap_or_else(|| s.label.clone()),
                        identifier: input.identifier.clone(),
                        config: input.config.clone().unwrap_or(serde_json::Value::Null),
                    })
                    .collect();
                let cap = spec
                    .max_jobs_per_run
                    .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN);
                (w_sources, Some(cap))
            } else {
                let single = vec![WorkerSource {
                    id: source.id.clone(),
                    kind: source.kind,
                    name: source
                        .config
                        .get("__company")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown company")
                        .to_owned(),
                    identifier: source.identifier.clone(),
                    config: source.config.clone(),
                }];
                (single, None)
            };

            if worker_sources.is_empty() {
                continue;
            }

            let workflow_filters =
                resolve_worker_filters(&source.filters, scrape_core::WorkerFilters::default())?;
            let starting_index = source
                .config
                .get("__starting_source_index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;

            let config = WorkerConfig {
                version: PROTOCOL_VERSION,
                run_started_at: Some(run.started_at.clone()),
                sources: worker_sources.clone(),
                filters: workflow_filters,
                filters_by_source: Default::default(),
                limits: WorkerLimits::default(),
                blocked_postings: crate::db::postings::suppression_snapshot(&self.pool).await?,
                max_jobs_per_run: max_jobs,
                starting_source_index: Some(starting_index),
            };

            let start_time = Instant::now();
            let step_result = self.execute_sidecar(app, &run.id, config).await;
            let duration_ms = start_time.elapsed().as_millis() as i64;
            let next_run =
                crate::services::scheduler::calculate_next_run_for_source(source, &tz_name);

            match step_result {
                Ok((found, new, statuses)) => {
                    total_found += found;
                    total_new += new;
                    for (src_id, status_info) in statuses {
                        source_statuses.insert(src_id, status_info);
                    }
                    if !worker_sources.is_empty() {
                        let next_index = (starting_index + 1) % worker_sources.len();
                        let mut updated_config = source.config.clone();
                        updated_config["__starting_source_index"] =
                            serde_json::Value::from(next_index as u64);
                        let _ = sqlx::query(
                            "UPDATE workflows SET starting_source_index = ? WHERE id = ?",
                        )
                        .bind(next_index as i64)
                        .bind(&source.id)
                        .execute(&self.pool)
                        .await;
                        let _ = sqlx::query("UPDATE sources SET config = ? WHERE id = ?")
                            .bind(serde_json::to_string(&updated_config).unwrap_or_default())
                            .bind(&source.id)
                            .execute(&self.pool)
                            .await;
                    }
                    let (status, err_msg) = source_statuses
                        .get(&source.id)
                        .cloned()
                        .unwrap_or_else(|| ("ok".to_string(), None));
                    SourcesRepo::set_run_result(
                        &self.pool,
                        &source.id,
                        &status,
                        err_msg.as_deref(),
                        Some(duration_ms),
                        next_run.as_deref(),
                    )
                    .await?;
                }
                Err(err) => {
                    source_statuses.insert(
                        source.id.clone(),
                        ("error".to_string(), Some(err.to_string())),
                    );
                    SourcesRepo::set_last_result(
                        &self.pool,
                        &source.id,
                        "error",
                        Some(&err.to_string()),
                    )
                    .await?;
                }
            }
        }

        let final_status = aggregate_run_status(&source_statuses, total_found);
        let err_summary = match final_status {
            RunStatus::Failed | RunStatus::Partial => {
                Some("One or more sources had incomplete processing; inspect source scan details")
            }
            RunStatus::Cancelled => Some("Worker cancelled; partial results retained"),
            _ => None,
        };

        let finished = RunsRepo::finish(
            &self.pool,
            &run.id,
            final_status,
            total_found,
            total_new,
            err_summary,
        )
        .await?;

        self.log_event(
            app,
            &run.id,
            if final_status == RunStatus::Failed {
                LogLevel::Error
            } else if final_status == RunStatus::Partial {
                LogLevel::Warn
            } else {
                LogLevel::Info
            },
            None,
            &format!(
                "Run finished: {:?}. Found {} postings, {} new.",
                final_status, total_found, total_new
            ),
        )
        .await;

        if final_status == RunStatus::Failed || final_status == RunStatus::Partial {
            let _ = crate::db::notifications::NotificationRepo::create(
                &self.pool,
                crate::db::notifications::CreateNotificationInput {
                    kind: crate::db::notifications::NotificationKind::Error,
                    title: if final_status == RunStatus::Failed {
                        "SCRAPE RUN FAILED".to_string()
                    } else {
                        "SCRAPE RUN PARTIAL FAILURE".to_string()
                    },
                    message: err_summary
                        .unwrap_or("One or more workflows encountered fatal scrape errors")
                        .to_string(),
                    action_label: Some("VIEW RUN".to_string()),
                    action_payload: Some(run.id.clone()),
                },
            )
            .await;
        }

        _guard.finish(|| {
            let _ = app.emit("scrape://finished", &finished);
        });
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

    pub async fn trigger_single_source(
        &self,
        app: &AppHandle,
        source_id: &str,
    ) -> Result<ScrapeRun> {
        self.trigger_single_source_as(app, source_id, TriggerType::SingleSource)
            .await
    }

    pub async fn trigger_single_source_as(
        &self,
        app: &AppHandle,
        source_id: &str,
        trigger: TriggerType,
    ) -> Result<ScrapeRun> {
        let _guard = self.try_acquire_lock()?;

        let source = SourcesRepo::get(&self.pool, source_id).await?;
        if source.needs_review {
            return Err(AppError::Validation(
                "Workflow needs review before it can run. Open its configuration and save a valid specification.".into(),
            ));
        }
        super::workflow::validated_stored_config(source.kind, &source.config)?;
        let filters =
            resolve_worker_filters(&source.filters, scrape_core::WorkerFilters::default())?;
        let run = RunsRepo::start_with_source(&self.pool, trigger, Some(source_id)).await?;
        let _ = app.emit(
            "scrape://started",
            serde_json::json!({ "run_id": run.id, "source_id": source_id }),
        );

        self.log_event(
            app,
            &run.id,
            LogLevel::Info,
            Some(source_id),
            &format!(
                "Scrape workflow initiated: {} ({:?})",
                source.name, source.kind
            ),
        )
        .await;

        let tz_str: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());

        let spec_opt: Option<super::workflow::WorkflowSpec> = source
            .config
            .get("__workflow_spec")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let (worker_sources, max_jobs) = if let Some(spec) = &spec_opt {
            let inputs = super::workflow::workflow_source_inputs(spec, &tz_str)?;
            let sources: Vec<WorkerSource> = spec
                .sources
                .iter()
                .zip(inputs.iter())
                .filter(|(s, _)| s.enabled)
                .map(|(s, input)| WorkerSource {
                    id: if spec.sources.len() == 1 {
                        source.id.clone()
                    } else {
                        s.id.clone()
                    },
                    kind: input.kind,
                    name: s
                        .employer_hint
                        .clone()
                        .or_else(|| spec.metadata.company.clone())
                        .unwrap_or_else(|| s.label.clone()),
                    identifier: input.identifier.clone(),
                    config: input.config.clone().unwrap_or(serde_json::Value::Null),
                })
                .collect();
            let cap = spec
                .max_jobs_per_run
                .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN);
            (sources, Some(cap))
        } else {
            let single = vec![WorkerSource {
                id: source.id.clone(),
                kind: source.kind,
                name: source
                    .config
                    .get("__company")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown company")
                    .to_owned(),
                identifier: source.identifier.clone(),
                config: source.config.clone(),
            }];
            (single, None)
        };

        let starting_index = source
            .config
            .get("__starting_source_index")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;

        let config = WorkerConfig {
            version: PROTOCOL_VERSION,
            run_started_at: Some(run.started_at.clone()),
            filters_by_source: Default::default(),
            sources: worker_sources.clone(),
            filters,
            limits: WorkerLimits::default(),
            blocked_postings: crate::db::postings::suppression_snapshot(&self.pool).await?,
            max_jobs_per_run: max_jobs,
            starting_source_index: Some(starting_index),
        };

        let start_time = Instant::now();
        let run_result = self.execute_sidecar(app, &run.id, config).await;
        let duration_ms = start_time.elapsed().as_millis() as i64;

        if !worker_sources.is_empty() {
            let next_index = (starting_index + 1) % worker_sources.len();
            let mut updated_config = source.config.clone();
            updated_config["__starting_source_index"] = serde_json::Value::from(next_index as u64);
            let _ = sqlx::query("UPDATE workflows SET starting_source_index = ? WHERE id = ?")
                .bind(next_index as i64)
                .bind(source_id)
                .execute(&self.pool)
                .await;
            let _ = sqlx::query("UPDATE sources SET config = ? WHERE id = ?")
                .bind(serde_json::to_string(&updated_config).unwrap_or_default())
                .bind(source_id)
                .execute(&self.pool)
                .await;
        }

        let next_run = crate::services::scheduler::calculate_next_run_for_source(&source, &tz_str);

        match run_result {
            Ok((total_found, total_new, source_statuses)) => {
                let (status, err_msg) = source_statuses
                    .get(source_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        if !source_statuses.is_empty() {
                            let agg = aggregate_run_status(&source_statuses, total_found);
                            let first_err = source_statuses.values().find_map(|(_, err)| err.clone());
                            let status_str = match agg {
                                RunStatus::Success | RunStatus::SuccessWithWarnings | RunStatus::Partial => "ok",
                                _ => "error",
                            };
                            (status_str.to_string(), first_err)
                        } else {
                            (
                                "error".to_string(),
                                Some("Worker process terminated without reporting status for this workflow".to_string()),
                            )
                        }
                    });

                SourcesRepo::set_run_result(
                    &self.pool,
                    source_id,
                    &status,
                    err_msg.as_deref(),
                    Some(duration_ms),
                    next_run.as_deref(),
                )
                .await?;

                let final_status = aggregate_run_status(&source_statuses, total_found);

                let finished = RunsRepo::finish(
                    &self.pool,
                    &run.id,
                    final_status,
                    total_found,
                    total_new,
                    err_msg.as_deref(),
                )
                .await?;

                self.log_event(
                    app,
                    &run.id,
                    match final_status {
                        RunStatus::Failed => LogLevel::Error,
                        RunStatus::Partial | RunStatus::SuccessWithWarnings => LogLevel::Warn,
                        _ => LogLevel::Info,
                    },
                    Some(source_id),
                    &format!(
                        "Workflow run completed in {:.2}s with status: {:?}",
                        duration_ms as f64 / 1000.0,
                        final_status
                    ),
                )
                .await;

                if final_status == RunStatus::Failed {
                    let _ = crate::db::notifications::NotificationRepo::create(
                        &self.pool,
                        crate::db::notifications::CreateNotificationInput {
                            kind: crate::db::notifications::NotificationKind::Error,
                            title: format!("WORKFLOW FAILED: {}", source.name),
                            message: err_msg
                                .clone()
                                .unwrap_or_else(|| "Execution terminated with errors".to_string()),
                            action_label: Some("INSPECT WORKFLOW".to_string()),
                            action_payload: Some(source.id.clone()),
                        },
                    )
                    .await;
                }

                _guard.finish(|| {
                    let _ = app.emit("scrape://finished", &finished);
                });
                let _ = app.emit(
                    "scrape://source_finished",
                    serde_json::json!({
                        "source_id": source_id,
                        "run": finished,
                        "duration_ms": duration_ms,
                    }),
                );
                let _ = app.emit("sources://changed", ());

                if total_new > 0 {
                    let _ = app.emit("jobs://changed", ());
                    let notifications_enabled: bool =
                        SettingsRepo::get_value(&self.pool, "notifications.enabled")
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or(true);
                    if notifications_enabled {
                        let _ = app
                            .notification()
                            .builder()
                            .title("SCRAPE COMPLETE")
                            .body(format!("{} NEW POSTINGS", total_new))
                            .show();
                    }
                }

                Ok(finished)
            }
            Err(err) => {
                let err_str = err.to_string();
                tracing::error!("Single source scrape error: {}", err_str);
                self.log_event(
                    app,
                    &run.id,
                    LogLevel::Error,
                    Some(source_id),
                    &format!("Workflow execution failed: {}", err_str),
                )
                .await;
                SourcesRepo::set_run_result(
                    &self.pool,
                    source_id,
                    "error",
                    Some(&err_str),
                    Some(duration_ms),
                    next_run.as_deref(),
                )
                .await?;
                let finished =
                    RunsRepo::finish(&self.pool, &run.id, RunStatus::Failed, 0, 0, Some(&err_str))
                        .await?;
                _guard.finish(|| {
                    let _ = app.emit("scrape://finished", &finished);
                });
                let _ = app.emit("sources://changed", ());
                Err(err)
            }
        }
    }

    async fn execute_sidecar(
        &self,
        app: &AppHandle,
        run_id: &str,
        config: WorkerConfig,
    ) -> Result<(i64, i64, HashMap<String, (String, Option<String>)>)> {
        let mut budget = super::ingestion::RunPostingBudget::new(&config.sources)?;
        RunsRepo::record_config(&self.pool, run_id, &config).await?;
        let mut counts = RunCounts {
            examination_incomplete: Some(true),
            ..Default::default()
        };
        let (mut rx, child) = crate::services::process::worker_command(app)?
            .args(["--stdin"])
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to spawn sidecar: {}", e)))?;

        let mut child = super::process::ChildGuard::new(child);
        let single_source_id = if config.sources.len() == 1 {
            Some(config.sources[0].id.as_str())
        } else {
            None
        };

        self.log_event(
            app,
            run_id,
            LogLevel::Info,
            single_source_id,
            &format!(
                "Sidecar worker spawned. Scraping {} sources.",
                config.sources.len()
            ),
        )
        .await;

        let mut config_bytes = serde_json::to_vec(&config)?;
        config_bytes.push(b'\n');
        if let Err(error) = child.write(&config_bytes) {
            let _ = child.kill();
            return Err(AppError::Internal(format!(
                "Failed to write to sidecar stdin: {error}"
            )));
        }

        let mut job_buffer: Vec<ScrapedJob> = Vec::new();
        let mut last_flush = Instant::now();
        let mut total_found = 0i64;
        let mut total_new = 0i64;
        let mut source_statuses: HashMap<String, (String, Option<String>)> = HashMap::new();

        let mut termination_code: Option<i32> = None;
        let mut sidecar_err: Option<String> = None;

        let mut saw_done = false;
        let deadline = Instant::now()
            + Duration::from_secs(super::process::execution_seconds(
                if config.sources.len() == 1 { 180 } else { 600 },
            ));
        loop {
            let received = tokio::select! {
                event = timeout_at(deadline, rx.recv()) => Some(event),
                _ = tokio::time::sleep(Duration::from_millis(100)) => None,
            };
            let cancelled = self.cancel_requested.load(Ordering::SeqCst);
            if received.is_none() && !cancelled {
                continue;
            }
            let received = if cancelled { None } else { received };
            let event = match received {
                Some(Ok(Some(event))) => event,
                Some(Ok(None)) => break,
                Some(Err(_)) | None => {
                    sidecar_err = Some(
                        if cancelled {
                            "Worker cancelled; partial results retained"
                        } else {
                            "Worker execution timed out; partial results retained"
                        }
                        .into(),
                    );
                    let _ = child.kill();
                    // The shell plugin owns reaping; consume its terminal notification.
                    let reap_deadline = Instant::now() + Duration::from_secs(5);
                    while let Ok(Some(event)) = timeout_at(reap_deadline, rx.recv()).await {
                        if let CommandEvent::Terminated(status) = event {
                            termination_code = status.code;
                            break;
                        }
                    }
                    break;
                }
            };
            match event {
                CommandEvent::Stdout(bytes) => {
                    let text = match std::str::from_utf8(&bytes) {
                        Ok(text) => text,
                        Err(error) => {
                            sidecar_err =
                                Some(format!("Worker output is not valid UTF-8: {error}"));
                            continue;
                        }
                    };
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if saw_done {
                            sidecar_err = Some("Worker emitted data after completion".into());
                            continue;
                        }
                        let worker_event = match serde_json::from_str::<WorkerEvent>(trimmed) {
                            Ok(event) => event,
                            Err(error) => {
                                sidecar_err = Some(format!("Malformed worker event: {error}"));
                                continue;
                            }
                        };
                        if let Some(source_id) = worker_event.source_id() {
                            if !config.sources.iter().any(|source| source.id == source_id) {
                                sidecar_err = Some(format!(
                                    "Worker emitted an unknown workflow ID: {source_id}"
                                ));
                                counts.failed += 1;
                                continue;
                            }
                        }
                        {
                            match worker_event {
                                event @ WorkerEvent::Job { .. } => {
                                    if let WorkerEvent::Job { ref source_id, .. } = event {
                                        if let Err(error) = budget.take(source_id) {
                                            sidecar_err = Some(error.to_string());
                                            counts.failed += 1;
                                            continue;
                                        }
                                    }
                                    job_buffer
                                        .push(super::ingestion::scraped_job_from_event(event)?);
                                    total_found += 1;

                                    if job_buffer.len() >= 50
                                        || last_flush.elapsed() >= Duration::from_millis(500)
                                    {
                                        let stats =
                                            JobsRepo::upsert_scraped(&self.pool, &job_buffer)
                                                .await?;
                                        total_new += stats.inserted;
                                        counts.inserted += stats.inserted;
                                        counts.updated += stats.updated;
                                        counts.blocked += stats.blocked;
                                        RunsRepo::record_counts(&self.pool, run_id, &counts)
                                            .await?;
                                        job_buffer.clear();
                                        last_flush = Instant::now();
                                    }
                                }
                                WorkerEvent::Progress {
                                    source_id,
                                    fetched,
                                    matched,
                                } => {
                                    counts.fetched += fetched as i64;
                                    RunsRepo::record_counts(&self.pool, run_id, &counts).await?;
                                    source_statuses
                                        .entry(source_id.clone())
                                        .or_insert_with(|| ("success".to_string(), None));
                                    if fetched > 0 && matched == 0 {
                                        self.log_event(
                                            app,
                                            run_id,
                                            LogLevel::Warn,
                                            Some(&source_id),
                                            &format!(
                                                "Examined {} candidates with 0 confirmed matches; inspect scan details for rejections, invalid records, and missing eligibility evidence",
                                                fetched
                                            ),
                                        )
                                        .await;
                                    } else {
                                        self.log_event(
                                            app,
                                            run_id,
                                            LogLevel::Info,
                                            Some(&source_id),
                                            &format!(
                                                "Examined {} candidates, matched {} jobs",
                                                fetched, matched
                                            ),
                                        )
                                        .await;
                                    }
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
                                    invalid_posting: _,
                                } => {
                                    counts.failed += 1;
                                    RunsRepo::record_counts(&self.pool, run_id, &counts).await?;
                                    source_statuses.insert(
                                        source_id.clone(),
                                        ("error".to_string(), Some(message.clone())),
                                    );
                                    self.log_event(
                                        app,
                                        run_id,
                                        LogLevel::Error,
                                        Some(&source_id),
                                        &message,
                                    )
                                    .await;
                                    let _ = app.emit(
                                        "scrape://progress",
                                        serde_json::json!({
                                            "source_id": source_id,
                                            "error": message,
                                            "retryable": retryable,
                                        }),
                                    );
                                }
                                WorkerEvent::Warning {
                                    source_id, message, ..
                                } => {
                                    *counts.enrichment_warnings.get_or_insert(0) += 1;
                                    source_statuses.entry(source_id.clone()).or_insert_with(|| {
                                        ("success_with_warnings".into(), Some(message.clone()))
                                    });
                                    self.log_event(
                                        app,
                                        run_id,
                                        LogLevel::Warn,
                                        Some(&source_id),
                                        &message,
                                    )
                                    .await;
                                    RunsRepo::record_counts(&self.pool, run_id, &counts).await?;
                                }
                                WorkerEvent::Branding {
                                    source_id,
                                    company,
                                    logo,
                                } => {
                                    if let Err(error) = logo.validate().and_then(|_| {
                                        if company.trim().is_empty() {
                                            Err("Missing employer identity".into())
                                        } else {
                                            Ok(())
                                        }
                                    }) {
                                        sidecar_err.get_or_insert_with(|| {
                                            format!("Invalid employer branding report: {error}")
                                        });
                                        continue;
                                    }
                                    super::super::db::branding::record(
                                        &self.pool, &source_id, &company, &logo,
                                    )
                                    .await?;
                                    self.log_event(
                                        app,
                                        run_id,
                                        LogLevel::Info,
                                        Some(&source_id),
                                        &format!(
                                            "Employer logo {company}: {:?}; {}",
                                            logo.status,
                                            logo.diagnostic
                                                .as_deref()
                                                .unwrap_or("source evidence recorded")
                                        ),
                                    )
                                    .await;
                                }
                                WorkerEvent::SourceInfo { .. } => {}
                                WorkerEvent::Done { stats } => {
                                    let ids: std::collections::HashSet<_> = stats
                                        .sources
                                        .iter()
                                        .map(|scan| scan.source_id.as_str())
                                        .collect();
                                    if ids.len() != config.sources.len()
                                        || ids.len() != stats.sources.len()
                                        || config
                                            .sources
                                            .iter()
                                            .any(|source| !ids.contains(source.id.as_str()))
                                        || stats.sources.iter().any(|scan| scan.outcome.is_none())
                                    {
                                        sidecar_err.get_or_insert_with(|| "Worker completion must contain one summary and outcome for every configured source".into());
                                        continue;
                                    }
                                    if let Some(error) =
                                        stats.sources.iter().find_map(|scan| scan.validate().err())
                                    {
                                        sidecar_err.get_or_insert(error);
                                        continue;
                                    }
                                    counts.enrichment_warnings = Some(
                                        stats
                                            .sources
                                            .iter()
                                            .map(|scan| scan.enrichment_warnings)
                                            .sum(),
                                    );
                                    counts.rejected =
                                        stats.sources.iter().map(|scan| scan.rejected as i64).sum();
                                    counts.sources = Some(stats.sources.clone());
                                    counts.examination_incomplete = Some(false);
                                    for scan in stats.sources {
                                        if !config.sources.iter().any(|s| s.id == scan.source_id) {
                                            sidecar_err = Some(
                                                "Worker summary contains an unknown workflow ID"
                                                    .into(),
                                            );
                                            continue;
                                        }
                                        let outcome =
                                            scan.outcome.expect("Validated source outcome");
                                        let status = match outcome {
                                            scrape_core::protocol::ScanOutcome::Success => "success",
                                            scrape_core::protocol::ScanOutcome::SuccessWithWarnings => "success_with_warnings",
                                            scrape_core::protocol::ScanOutcome::Partial => "partial",
                                            scrape_core::protocol::ScanOutcome::Failed => "error",
                                            scrape_core::protocol::ScanOutcome::Cancelled => "cancelled",
                                        };
                                        let message = if matches!(
                                            outcome,
                                            scrape_core::protocol::ScanOutcome::Partial
                                                | scrape_core::protocol::ScanOutcome::Failed
                                        ) {
                                            Some(format!("Source processing is {status}: {} candidates lack required eligibility evidence; {} invalid; coverage {:?}. {} Inspect warnings and retry.",scan.unresolved,scan.invalid,scan.termination,source_statuses.get(&scan.source_id).and_then(|(_,message)| message.as_deref()).unwrap_or("")))
                                        } else {
                                            source_statuses
                                                .get(&scan.source_id)
                                                .and_then(|(_, message)| message.clone())
                                        };
                                        source_statuses.insert(
                                            scan.source_id.clone(),
                                            (status.into(), message),
                                        );
                                        counts.blocked += scan.blocked as i64;
                                        self.log_event(app, run_id, LogLevel::Info, Some(&scan.source_id), &format!("Examined {} postings; {} matched, {} blocked, {} duplicates; limit {}{}. Counts cover examined postings only.", scan.examined, scan.matched, scan.blocked, scan.duplicates, scan.limit, if scan.limit_reached { " reached; stopped early, source coverage may be incomplete" } else { " not reached" })).await;
                                    }
                                    RunsRepo::record_counts(&self.pool, run_id, &counts).await?;
                                    saw_done = true;
                                }
                            }
                        }
                    }
                }
                CommandEvent::Stderr(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let line = text.trim();
                    if !line.is_empty() {
                        tracing::info!("[sidecar] {}", line);
                        if line.contains("ERROR") || line.contains("error") {
                            self.log_event(app, run_id, LogLevel::Warn, single_source_id, line)
                                .await;
                        }
                    }
                }
                CommandEvent::Error(err) => {
                    tracing::error!("[sidecar error] {}", err);
                    sidecar_err = Some(err.to_string());
                    self.log_event(
                        app,
                        run_id,
                        LogLevel::Error,
                        single_source_id,
                        &format!("Sidecar process error: {}", err),
                    )
                    .await;
                }
                CommandEvent::Terminated(status) => {
                    child.disarm();
                    tracing::info!("[sidecar exit] status: {:?}", status.code);
                    termination_code = status.code;
                    if let Some(code) = status.code {
                        if code != 0 {
                            self.log_event(
                                app,
                                run_id,
                                LogLevel::Error,
                                single_source_id,
                                &format!("Sidecar exited with non-zero status code: {}", code),
                            )
                            .await;
                        }
                    }
                    break;
                }
                _ => {}
            }
        }

        // Terminal failures override earlier progress; partial rows remain recoverable.
        if let Some(message) = worker_failure(termination_code, saw_done, sidecar_err.as_deref()) {
            let cancelled = self.cancel_requested.load(Ordering::SeqCst);
            if !cancelled {
                counts.failed = counts.failed.max(1);
            }
            counts.examination_incomplete = Some(!saw_done);
            counts.fetched = counts.fetched.max(total_found);
            if let Some(scans) = counts.sources.as_mut() {
                for scan in scans {
                    scan.termination = if cancelled {
                        scrape_core::protocol::CoverageTermination::Cancelled
                    } else {
                        scrape_core::protocol::CoverageTermination::Failed
                    };
                    scan.outcome = Some(if cancelled {
                        scrape_core::protocol::ScanOutcome::Cancelled
                    } else if scan.accepted > 0 {
                        scrape_core::protocol::ScanOutcome::Partial
                    } else {
                        scrape_core::protocol::ScanOutcome::Failed
                    });
                }
            }
            for source in &config.sources {
                source_statuses.insert(
                    source.id.clone(),
                    (
                        (if cancelled {
                            "cancelled"
                        } else if total_found > 0 {
                            "partial"
                        } else {
                            "error"
                        })
                        .into(),
                        Some(message.clone()),
                    ),
                );
            }
        }

        // Ensure every configured source has a recorded status
        for src in &config.sources {
            source_statuses.entry(src.id.clone()).or_insert_with(|| {
                (
                    "error".to_string(),
                    Some(
                        "Worker terminated unexpectedly without reporting status for this workflow"
                            .to_string(),
                    ),
                )
            });
        }

        // Flush remaining buffered jobs
        if !job_buffer.is_empty() {
            let stats = JobsRepo::upsert_scraped(&self.pool, &job_buffer).await?;
            total_new += stats.inserted;
            counts.inserted += stats.inserted;
            counts.updated += stats.updated;
            counts.blocked += stats.blocked;
            job_buffer.clear();
        }

        self.log_event(
            app,
            run_id,
            LogLevel::Info,
            single_source_id,
            &format!(
                "Execution finished: committed {} new jobs (found {} total)",
                total_new, total_found
            ),
        )
        .await;

        // Post-run retention shares the same eligibility and fixed-clock service as
        // independent housekeeping. Report errors to both run logs and Data management.
        let retention = super::retention::RetentionService::prune_source_ids(
            &self.pool,
            source_statuses.keys().cloned(),
            chrono::Utc::now(),
        )
        .await;
        counts.retention_removed += i64::try_from(retention.deleted_records).unwrap_or(i64::MAX);
        for failure in &retention.failures {
            self.log_event(
                app,
                run_id,
                LogLevel::Warn,
                None,
                &format!("Automatic retention cleanup failed: {failure}"),
            )
            .await;
        }
        super::retention::RetentionService::publish(&self.pool, app, &retention).await;

        RunsRepo::record_counts(&self.pool, run_id, &counts).await?;
        Ok((total_found, total_new, source_statuses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::{RoleType, TriState};

    #[tokio::test]
    async fn completion_notification_observes_released_worker() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let runner = ScrapeRunner::new(pool);
        let guard = runner.try_acquire_lock().unwrap();
        assert!(runner.is_running());
        guard.finish(|| assert!(!runner.is_running()));
        assert!(runner.try_acquire_lock().is_ok());
    }

    #[tokio::test]
    async fn retention_defers_during_a_scrape_without_cancelling_it() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let runner = ScrapeRunner::new(pool);
        let guard = runner.try_acquire_lock().unwrap();
        assert!(runner.try_retention_permit().is_none());
        assert!(runner.is_running());
        assert!(!runner.cancellation_flag().load(Ordering::SeqCst));
        drop(guard);
        assert!(runner.try_retention_permit().is_some());
    }

    #[tokio::test]
    async fn test_overlap_guard() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let runner = ScrapeRunner::new(pool);
        assert!(!runner.is_running());

        let guard = runner.try_acquire_lock().unwrap();
        assert!(runner.is_running());
        assert!(runner.cancel_current());
        assert!(runner.cancel_requested.load(Ordering::SeqCst));

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
        assert!(!runner.cancel_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn aggregate_outcomes_distinguish_warnings_partial_failures_and_cancellation() {
        for (statuses, found, expected) in [
            (vec!["success"], 0, RunStatus::Success),
            (
                vec!["success_with_warnings", "success"],
                1,
                RunStatus::SuccessWithWarnings,
            ),
            (vec!["partial"], 0, RunStatus::Partial),
            (vec!["error"], 0, RunStatus::Failed),
            (vec!["error"], 1, RunStatus::Partial),
            (vec!["error", "success"], 0, RunStatus::Partial),
            (vec!["cancelled", "success"], 1, RunStatus::Cancelled),
        ] {
            let statuses = statuses
                .into_iter()
                .enumerate()
                .map(|(i, s)| (i.to_string(), (s.to_string(), None)))
                .collect();
            assert_eq!(aggregate_run_status(&statuses, found), expected);
        }
    }

    #[test]
    fn completion_requires_zero_exit_done_and_valid_protocol() {
        assert!(worker_failure(Some(0), true, None).is_none());
        for (exit, done, error) in [
            (Some(1), true, None),
            (None, true, None),
            (Some(0), false, None),
            (Some(0), true, Some("Malformed event")),
        ] {
            assert!(worker_failure(exit, done, error).is_some());
        }
    }

    #[test]
    fn test_resolve_worker_filters_with_overrides() {
        let global = WorkerFilters {
            include_keywords: vec!["GlobalKW".to_string()],
            exclude_keywords: vec!["GlobalExclude".to_string()],
            locations: vec!["GlobalLoc".to_string()],
            role_types: vec![RoleType::FullTime],
            query: Some("title LIKE '%Global%'".to_string()),
            policy: None,
            compensation: None,
            matching: None,
        };

        let source_filters = serde_json::json!({
            "include_keywords": ["CustomKW", "Rust"],
            "role_types": ["internship"],
            "query": "title LIKE '%Engineer%' AND location LIKE '%Remote%'"
        });

        let resolved = resolve_worker_filters(&source_filters, global).unwrap();
        assert_eq!(
            resolved.include_keywords,
            vec!["CustomKW".to_string(), "Rust".to_string()]
        );
        assert_eq!(resolved.role_types, vec![RoleType::Internship]);
        assert_eq!(
            resolved.query.as_deref(),
            Some("title LIKE '%Engineer%' AND location LIKE '%Remote%'")
        );
        // Fallback to global for omitted fields
        assert_eq!(resolved.exclude_keywords, vec!["GlobalExclude".to_string()]);
        assert_eq!(resolved.locations, vec!["GlobalLoc".to_string()]);
    }

    #[test]
    fn explicit_empty_overrides_and_invalid_filters_fail() {
        let global = WorkerFilters {
            include_keywords: vec!["Engineer".into()],
            ..Default::default()
        };
        let result =
            resolve_worker_filters(&serde_json::json!({"include_keywords": []}), global.clone())
                .unwrap();
        assert!(result.include_keywords.is_empty());
        for invalid in [
            serde_json::json!({"include_keywords": "Engineer"}),
            serde_json::json!({"query": "title LIKE"}),
            serde_json::json!({"unsupported": true}),
        ] {
            assert!(resolve_worker_filters(&invalid, global.clone()).is_err());
        }
    }

    #[test]
    fn test_resolve_worker_filters_fallback() {
        let global = WorkerFilters {
            include_keywords: vec!["Engineer".to_string()],
            exclude_keywords: vec!["Manager".to_string()],
            locations: vec!["Remote".to_string()],
            role_types: vec![RoleType::FullTime],
            query: None,
            policy: None,
            compensation: None,
            matching: None,
        };

        let null_filters = serde_json::Value::Null;
        let resolved = resolve_worker_filters(&null_filters, global.clone()).unwrap();
        assert_eq!(resolved.include_keywords, global.include_keywords);
        assert_eq!(resolved.exclude_keywords, global.exclude_keywords);
        assert_eq!(resolved.locations, global.locations);
        assert_eq!(resolved.role_types, global.role_types);

        let empty_filters = serde_json::json!({});
        let resolved_empty = resolve_worker_filters(&empty_filters, global.clone()).unwrap();
        assert_eq!(resolved_empty.include_keywords, global.include_keywords);
    }

    #[tokio::test]
    async fn source_run_log_records_remain_queryable_by_source() {
        let pool = crate::db::init_memory_db().await.unwrap();

        // 1. Create a test source
        let source = crate::db::sources::SourcesRepo::create(
            &pool,
            crate::db::sources::CreateSourceInput {
                kind: scrape_core::SourceKind::Greenhouse,
                name: "Cloudflare Tests".to_string(),
                identifier: "cloudflare".to_string(),
                config: None,
                enabled: Some(true),
                cadence: Some(scrape_core::SourceCadence::Daily),
                cadence_cron: None,
                retention_days: Some(30),
                retention_policy: Some("untracked_only".to_string()),
                filters: None,
            },
        )
        .await
        .unwrap();

        // 2. Start a workflow execution run
        let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
            .await
            .unwrap();
        assert_eq!(run.source_id.as_deref(), Some(source.id.as_str()));
        assert_eq!(run.status, RunStatus::Running);

        // 3. Emulate execution logs generated throughout runner lifecycle
        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            Some(&source.id),
            "Scrape workflow initiated: Cloudflare Tests (Greenhouse)",
        )
        .await
        .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            Some(&source.id),
            "Sidecar worker spawned. Scraping 1 sources.",
        )
        .await
        .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            Some(&source.id),
            "Fetched 142 postings, matched 15 jobs",
        )
        .await
        .unwrap();

        let finished = RunsRepo::finish(&pool, &run.id, RunStatus::Success, 142, 15, None)
            .await
            .unwrap();
        assert_eq!(finished.status, RunStatus::Success);

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            Some(&source.id),
            "Workflow run completed in 1.25s with status: SUCCESS",
        )
        .await
        .unwrap();

        // 4. Assert logs are persisted and queryable by run ID
        let logs = RunsRepo::get_logs(&pool, &run.id).await.unwrap();
        assert_eq!(logs.len(), 4, "Expected 4 lifecycle logs to be recorded");
        assert_eq!(
            logs[0].message,
            "Scrape workflow initiated: Cloudflare Tests (Greenhouse)"
        );
        assert_eq!(logs[0].source_id.as_deref(), Some(source.id.as_str()));
        assert_eq!(logs[0].level, LogLevel::Info);

        assert_eq!(
            logs[1].message,
            "Sidecar worker spawned. Scraping 1 sources."
        );
        assert_eq!(logs[2].message, "Fetched 142 postings, matched 15 jobs");
        assert_eq!(
            logs[3].message,
            "Workflow run completed in 1.25s with status: SUCCESS"
        );

        // 5. Assert that list_by_source correctly returns the run
        let source_runs = RunsRepo::list_by_source(&pool, &source.id, 10)
            .await
            .unwrap();
        assert_eq!(source_runs.len(), 1);
        assert_eq!(source_runs[0].id, run.id);
        assert_eq!(source_runs[0].jobs_found, 142);
        assert_eq!(source_runs[0].jobs_new, 15);
    }

    #[tokio::test]
    async fn failed_source_run_log_records_preserve_errors() {
        let pool = crate::db::init_memory_db().await.unwrap();

        let source = crate::db::sources::SourcesRepo::create(
            &pool,
            crate::db::sources::CreateSourceInput {
                kind: scrape_core::SourceKind::Selector,
                name: "Broken Careers".to_string(),
                identifier: "https://invalid.example.com".to_string(),
                config: Some(
                    serde_json::json!({"item":".job-item","title":".job-title","link":"a"}),
                ),
                enabled: Some(true),
                cadence: Some(scrape_core::SourceCadence::Manual),
                cadence_cron: None,
                retention_days: Some(30),
                retention_policy: Some("untracked_only".to_string()),
                filters: None,
            },
        )
        .await
        .unwrap();

        let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
            .await
            .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Info,
            Some(&source.id),
            "Scrape workflow initiated: Broken Careers (Selector)",
        )
        .await
        .unwrap();

        let err_msg = "Workflow execution failed: HTTP 500 Internal Server Error";
        RunsRepo::add_log(&pool, &run.id, LogLevel::Error, Some(&source.id), err_msg)
            .await
            .unwrap();

        let _ = RunsRepo::finish(&pool, &run.id, RunStatus::Failed, 0, 0, Some(err_msg))
            .await
            .unwrap();

        RunsRepo::add_log(
            &pool,
            &run.id,
            LogLevel::Error,
            Some(&source.id),
            "Workflow run completed in 0.45s with status: FAILED",
        )
        .await
        .unwrap();

        let logs = RunsRepo::get_logs(&pool, &run.id).await.unwrap();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].level, LogLevel::Info);
        assert_eq!(logs[1].level, LogLevel::Error);
        assert_eq!(logs[1].message, err_msg);
        assert_eq!(logs[2].level, LogLevel::Error);

        let error_logs: Vec<_> = logs.iter().filter(|l| l.level == LogLevel::Error).collect();
        assert_eq!(error_logs.len(), 2, "Expected 2 error level logs");

        let source_runs = RunsRepo::list_by_source(&pool, &source.id, 10)
            .await
            .unwrap();
        assert_eq!(source_runs.len(), 1);
        assert_eq!(source_runs[0].status, RunStatus::Failed);
        assert_eq!(source_runs[0].error.as_deref(), Some(err_msg));
    }

    #[tokio::test]
    async fn repository_ingestion_preserves_discovery_and_user_state() {
        use crate::db::jobs::{JobFilter, JobsRepo, ScrapedJob};
        use crate::db::sources::{CreateSourceInput, SourcesRepo};
        use scrape_core::{JobStatus, WorkplaceType};

        let pool = crate::db::init_memory_db().await.unwrap();

        // 1. Seed sample workflow source
        let source = SourcesRepo::create(
            &pool,
            CreateSourceInput {
                kind: scrape_core::SourceKind::Greenhouse,
                name: "Cloudflare — Software Engineering & Internships".to_string(),
                identifier: "cloudflare".to_string(),
                config: Some(serde_json::json!({
                    "url": "https://boards.greenhouse.io/cloudflare"
                })),
                enabled: Some(true),
                cadence: Some(scrape_core::SourceCadence::Daily),
                cadence_cron: Some("0 0 9 * * *".to_string()),
                retention_days: Some(45),
                retention_policy: Some("untracked_only".to_string()),
                filters: Some(serde_json::json!({
                    "role_types": ["internship", "full_time"],
                    "query": "title LIKE '%Engineer%' OR title LIKE '%Intern%'",
                    "locations": ["Remote", "San Francisco", "Austin"]
                })),
            },
        )
        .await
        .unwrap();

        // 2. Start a single-source execution run
        let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
            .await
            .unwrap();

        // 3. Emulate sidecar emitting scraped postings matching the workflow
        let batch = vec![
            ScrapedJob {
                compensation: vec![],
                dedupe_key: format!("{}:job-101", source.id),
                source_id: Some(source.id.clone()),
                external_id: Some("job-101".to_string()),
                title: "Software Engineering Intern".to_string(),
                company: "Cloudflare".to_string(),
                role_type: RoleType::Internship,
                location: Some("San Francisco, CA".to_string()),
                url: "https://boards.greenhouse.io/cloudflare/jobs/101".to_string(),
                provider_updated_at: None,
                location_data_complete: true,
                date_posted: Some("2026-10-10".to_string()),
                salary_min: None,
                salary_max: None,
                salary_currency: None,
                salary_interval: None,
                salary_raw: None,
                salary_source: None,
                workplace_type: WorkplaceType::Hybrid,
                sponsorship: TriState::Yes,
                sponsorship_evidence: Some("Sponsorship available".to_string()),
                work_auth: scrape_core::WorkAuthRequirement::Unknown,
                work_auth_evidence: None,
                clearance: TriState::No,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(0),
                experience_max: Some(1),
                experience_raw: Some("0-1 years".to_string()),
                is_new_grad: TriState::Yes,
                experience_evidence: None,
                description_text: Some("Join our summer intern program".to_string()),
                description: None,
                description_source: None,
                description_format: None,
                company_icon_url: None,
                company_logo: None,
                locations: vec![],
            },
            ScrapedJob {
                compensation: vec![],
                dedupe_key: format!("{}:job-102", source.id),
                source_id: Some(source.id.clone()),
                external_id: Some("job-102".to_string()),
                title: "Systems Engineer".to_string(),
                company: "Cloudflare".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Austin, TX".to_string()),
                url: "https://boards.greenhouse.io/cloudflare/jobs/102".to_string(),
                provider_updated_at: None,
                location_data_complete: true,
                date_posted: Some("2026-10-11".to_string()),
                salary_min: Some(140000),
                salary_max: Some(180000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(scrape_core::SalaryInterval::Year),
                salary_raw: Some("$140k - $180k".to_string()),
                salary_source: Some(scrape_core::SalarySource::Structured),
                workplace_type: WorkplaceType::Remote,
                sponsorship: TriState::Unknown,
                sponsorship_evidence: None,
                work_auth: scrape_core::WorkAuthRequirement::Unknown,
                work_auth_evidence: None,
                clearance: TriState::Unknown,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(2),
                experience_max: Some(5),
                experience_raw: None,
                is_new_grad: TriState::No,
                experience_evidence: None,
                description_text: Some("Build edge networking systems".to_string()),
                description: None,
                description_source: None,
                description_format: None,
                company_icon_url: None,
                company_logo: None,
                locations: vec![],
            },
        ];

        // 4. Upsert into database
        let stats = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(stats.inserted, 2, "Expected 2 new jobs to be inserted");
        assert_eq!(stats.updated, 0);

        // Finish the run
        let _ = RunsRepo::finish(&pool, &run.id, RunStatus::Success, 2, 2, None)
            .await
            .unwrap();

        // 5. Query DISCOVERED inbox: statuses = ['scraped', 'viewed']
        let discovered_page = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                statuses: Some(vec![JobStatus::Scraped, JobStatus::Viewed]),
                role_types: None,
                search: None,
                source_id: None,
                sort: None,
                limit: Some(50),
                offset: Some(0),
                favorited_only: None,
                salary_min: None,
                salary_max: None,
                salary_currency: None,
                salary_include_unknown: None,
                salary_interval: None,
                experience_min: None,
                experience_max: None,
                experience_include_unknown: None,
                new_grad: None,
                sponsorship: None,
                work_auth: None,
                clearance: None,
                countries: None,
                regions: None,
                remote: None,
                location_include_unknown: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            discovered_page.items.len(),
            2,
            "Both scraped jobs must appear in Discovered inbox"
        );
        assert!(discovered_page
            .items
            .iter()
            .all(|j| j.status == JobStatus::Scraped));

        // 6. Query TRACKED pipeline: statuses = ['applied', 'interviewing', 'offer', 'rejected']
        let tracked_page = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                statuses: Some(vec![
                    JobStatus::Applied,
                    JobStatus::Interviewing,
                    JobStatus::Offer,
                    JobStatus::Rejected,
                ]),
                role_types: None,
                search: None,
                source_id: None,
                sort: None,
                limit: Some(50),
                offset: Some(0),
                favorited_only: None,
                salary_min: None,
                salary_max: None,
                salary_currency: None,
                salary_include_unknown: None,
                salary_interval: None,
                experience_min: None,
                experience_max: None,
                experience_include_unknown: None,
                new_grad: None,
                sponsorship: None,
                work_auth: None,
                clearance: None,
                countries: None,
                regions: None,
                remote: None,
                location_include_unknown: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(tracked_page.items.len(), 0, "No jobs should be tracked yet");

        // 7. Move job-101 to Applied
        let job101 = &discovered_page.items[0];
        let moved = JobsRepo::update_status(&pool, &job101.id, JobStatus::Applied)
            .await
            .unwrap();
        assert_eq!(moved.status, JobStatus::Applied);

        // 8. Re-query Discovered: now only 1 job remains
        let discovered_after = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                statuses: Some(vec![JobStatus::Scraped, JobStatus::Viewed]),
                role_types: None,
                search: None,
                source_id: None,
                sort: None,
                limit: Some(50),
                offset: Some(0),
                favorited_only: None,
                salary_min: None,
                salary_max: None,
                salary_currency: None,
                salary_include_unknown: None,
                salary_interval: None,
                experience_min: None,
                experience_max: None,
                experience_include_unknown: None,
                new_grad: None,
                sponsorship: None,
                work_auth: None,
                clearance: None,
                countries: None,
                regions: None,
                remote: None,
                location_include_unknown: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            discovered_after.items.len(),
            1,
            "Discovered count must decrease to 1 when a job is tracked"
        );

        // 9. Re-scrape same jobs on next run: verify user status is NOT overwritten
        let stats_second = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(stats_second.updated, 2);
        assert_eq!(stats_second.inserted, 0);

        let job101_after = JobsRepo::get(&pool, &job101.id).await.unwrap();
        assert_eq!(
            job101_after.status,
            JobStatus::Applied,
            "Re-scraping must preserve user Applied state"
        );
    }
}
