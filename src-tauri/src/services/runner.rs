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
}

impl ScrapeRunner {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_requested: Arc::new(AtomicBool::new(false)),
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

    pub fn try_acquire_lock(&self) -> Result<RunLockGuard> {
        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            tracing::warn!("Scrape run skipped: another run is already in progress");
            return Err(AppError::AlreadyRunning);
        }
        self.cancel_requested.store(false, Ordering::SeqCst);
        Ok(RunLockGuard {
            flag: self.is_running.clone(),
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
                    && (trigger == TriggerType::Manual
                        || s.cadence == scrape_core::SourceCadence::Global)
            })
            .collect();
        for source in &sources {
            super::workflow::validated_stored_config(source.kind, &source.config)?;
        }
        let globals = load_global_filters(&self.pool).await?;
        let filters_by_source = sources
            .iter()
            .map(|s| {
                Ok((
                    s.id.clone(),
                    resolve_worker_filters(&s.filters, globals.clone())?,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        let enabled_sources: Vec<WorkerSource> = sources
            .into_iter()
            .map(|s| WorkerSource {
                id: s.id,
                kind: s.kind,
                name: s
                    .config
                    .get("__company")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown company")
                    .to_owned(),
                identifier: s.identifier,
                config: s.config,
            })
            .collect();

        // 2. Start scrape run in DB
        let run = RunsRepo::start(&self.pool, trigger).await?;
        let _ = app.emit("scrape://started", serde_json::json!({ "run_id": run.id }));

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
            _guard.finish(|| {
                let _ = app.emit("scrape://finished", &finished);
            });

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

        let config = WorkerConfig {
            version: PROTOCOL_VERSION,
            run_started_at: Some(run.started_at.clone()),
            sources: enabled_sources,
            filters: globals,
            filters_by_source,
            limits: WorkerLimits::default(),
        };

        // 4. Run sidecar with 10 minute timeout
        let run_result = self.execute_sidecar(app, &run.id, config).await;

        match run_result {
            Ok((total_found, total_new, source_statuses)) => {
                // Update source statuses
                for (src_id, (status, err_msg)) in &source_statuses {
                    let _ = SourcesRepo::set_last_result(
                        &self.pool,
                        src_id,
                        status,
                        err_msg.as_deref(),
                    )
                    .await;
                }

                let has_errors = source_statuses.values().any(|(s, _)| s == "error");
                let all_failed = !source_statuses.is_empty()
                    && source_statuses.values().all(|(s, _)| s == "error");
                let final_status = if all_failed {
                    RunStatus::Failed
                } else if has_errors {
                    RunStatus::Partial
                } else {
                    RunStatus::Success
                };

                let err_summary = if has_errors {
                    Some("One or more sources encountered errors during scrape")
                } else {
                    None
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
            Err(err) => {
                let err_str = err.to_string();
                tracing::error!("Scrape execution error: {}", err_str);
                let finished =
                    RunsRepo::finish(&self.pool, &run.id, RunStatus::Failed, 0, 0, Some(&err_str))
                        .await?;
                _guard.finish(|| {
                    let _ = app.emit("scrape://finished", &finished);
                });
                Err(err)
            }
        }
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
        super::workflow::validated_stored_config(source.kind, &source.config)?;
        let filters =
            resolve_worker_filters(&source.filters, load_global_filters(&self.pool).await?)?;
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

        let config = WorkerConfig {
            version: PROTOCOL_VERSION,
            run_started_at: Some(run.started_at.clone()),
            filters_by_source: Default::default(),
            sources: vec![WorkerSource {
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
            }],
            filters,
            limits: WorkerLimits::default(),
        };

        let start_time = Instant::now();
        let run_result = self.execute_sidecar(app, &run.id, config).await;
        let duration_ms = start_time.elapsed().as_millis() as i64;

        let tz_str: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());
        let next_run = crate::services::scheduler::calculate_next_run_for_source(&source, &tz_str);

        match run_result {
            Ok((total_found, total_new, source_statuses)) => {
                let (status, err_msg) = source_statuses
                    .get(source_id)
                    .cloned()
                    .unwrap_or_else(|| {
                        (
                            "error".to_string(),
                            Some("Worker process terminated without reporting status for this workflow".to_string()),
                        )
                    });

                let _ = SourcesRepo::set_run_result(
                    &self.pool,
                    source_id,
                    &status,
                    err_msg.as_deref(),
                    Some(duration_ms),
                    next_run.as_deref(),
                )
                .await;

                let final_status = if status == "error" {
                    RunStatus::Failed
                } else {
                    RunStatus::Success
                };

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
                    if final_status == RunStatus::Success {
                        LogLevel::Info
                    } else {
                        LogLevel::Error
                    },
                    Some(source_id),
                    &format!(
                        "Workflow run completed in {:.2}s with status: {}",
                        duration_ms as f64 / 1000.0,
                        if final_status == RunStatus::Success {
                            "SUCCESS"
                        } else {
                            "FAILED"
                        }
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
                let _ = SourcesRepo::set_run_result(
                    &self.pool,
                    source_id,
                    "error",
                    Some(&err_str),
                    Some(duration_ms),
                    next_run.as_deref(),
                )
                .await;
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
        RunsRepo::record_config(&self.pool, run_id, &config).await?;
        let mut counts = RunCounts::default();
        let mut invalid = 0i64;
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
                        if let WorkerEvent::Job { source_id, .. }
                        | WorkerEvent::Progress { source_id, .. }
                        | WorkerEvent::Error { source_id, .. } = &worker_event
                        {
                            if !config.sources.iter().any(|source| &source.id == source_id) {
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
                                    counts.rejected +=
                                        (fetched as i64 - matched as i64 - invalid).max(0);
                                    invalid = 0;
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
                                                "Fetched {} postings from remote board, but 0 matched workflow filters (check keywords, role types, and locations)",
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
                                                "Fetched {} postings, matched {} jobs",
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
                                    invalid_posting,
                                } => {
                                    counts.failed += 1;
                                    if invalid_posting == Some(true) {
                                        invalid += 1;
                                    }
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
                                WorkerEvent::Done { .. } => {
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
            counts.failed = counts.failed.max(1);
            for source in &config.sources {
                source_statuses.insert(source.id.clone(), ("error".into(), Some(message.clone())));
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

        // Task 5 in DAG: Post-run retention pruning for participating sources
        for src_id in source_statuses.keys() {
            if let Ok(source) = SourcesRepo::get(&self.pool, src_id).await {
                if source.retention_days > 0 {
                    if let Ok(pruned) = JobsRepo::prune_expired_by_source(
                        &self.pool,
                        &source.id,
                        source.retention_days,
                        &source.retention_policy,
                    )
                    .await
                    {
                        counts.retention_removed += pruned as i64;
                        if pruned > 0 {
                            tracing::info!(
                                "Retention pruned {} expired jobs for workflow '{}' ({})",
                                pruned,
                                source.name,
                                source.id
                            );
                        }
                    }
                }
            }
        }

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
