use crate::db::jobs::{JobsRepo, ScrapedJob};
use crate::db::runs::{LogLevel, RunStatus, RunsRepo, ScrapeRun, TriggerType};
use crate::db::settings::SettingsRepo;
use crate::db::sources::SourcesRepo;
use crate::error::{AppError, Result};
use scrape_core::{
    RoleType, TriState, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource,
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

pub fn resolve_worker_filters(
    source_filters: &serde_json::Value,
    global_filters: WorkerFilters,
) -> WorkerFilters {
    if !source_filters.is_object() {
        return global_filters;
    }

    let include_keywords = source_filters
        .get("include_keywords")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .filter(|v| !v.is_empty())
        .unwrap_or(global_filters.include_keywords);

    let exclude_keywords = source_filters
        .get("exclude_keywords")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .filter(|v| !v.is_empty())
        .unwrap_or(global_filters.exclude_keywords);

    let locations = source_filters
        .get("locations")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
        .filter(|v| !v.is_empty())
        .unwrap_or(global_filters.locations);

    let role_types = source_filters
        .get("role_types")
        .and_then(|v| serde_json::from_value::<Vec<RoleType>>(v.clone()).ok())
        .filter(|v| !v.is_empty())
        .unwrap_or(global_filters.role_types);

    let query = source_filters
        .get("query")
        .and_then(|v| v.as_str().map(ToString::to_string))
        .or(global_filters.query);

    let policy = source_filters
        .get("policy")
        .cloned()
        .or(global_filters.policy);

    WorkerFilters {
        include_keywords,
        exclude_keywords,
        locations,
        role_types,
        query,
        policy,
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
                query: None,
                policy: None,
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

    pub async fn trigger_single_source(
        &self,
        app: &AppHandle,
        source_id: &str,
    ) -> Result<ScrapeRun> {
        let _guard = self.try_acquire_lock()?;

        let source = SourcesRepo::get(&self.pool, source_id).await?;
        let run =
            RunsRepo::start_with_source(&self.pool, TriggerType::SingleSource, Some(source_id))
                .await?;
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

        let global_filters = WorkerFilters {
            include_keywords,
            exclude_keywords,
            locations,
            role_types,
            query: None,
            policy: None,
        };
        let filters = resolve_worker_filters(&source.filters, global_filters);

        let config = WorkerConfig {
            version: PROTOCOL_VERSION,
            sources: vec![WorkerSource {
                id: source.id.clone(),
                kind: source.kind,
                name: source.name.clone(),
                identifier: source.identifier.clone(),
                config: source.config.clone(),
            }],
            filters,
            limits: WorkerLimits::default(),
        };

        let start_time = Instant::now();
        let run_result = timeout(
            Duration::from_secs(180),
            self.execute_sidecar(app, &run.id, config),
        )
        .await;
        let duration_ms = start_time.elapsed().as_millis() as i64;

        let tz_str: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());
        let next_run = crate::services::scheduler::calculate_next_run_for_source(&source, &tz_str);

        match run_result {
            Ok(Ok((total_found, total_new, source_statuses))) => {
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

                let _ = app.emit("scrape://finished", &finished);
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
            Ok(Err(err)) => {
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
                let _ = app.emit("scrape://finished", &finished);
                let _ = app.emit("sources://changed", ());
                Err(err)
            }
            Err(_) => {
                tracing::error!("Single source scrape timed out (3m limit)");
                let err_str = "Execution timed out (3 minute limit)";
                self.log_event(
                    app,
                    &run.id,
                    LogLevel::Error,
                    Some(source_id),
                    "Workflow execution timed out (3 minute limit)",
                )
                .await;
                let _ = SourcesRepo::set_run_result(
                    &self.pool,
                    source_id,
                    "error",
                    Some(err_str),
                    Some(duration_ms),
                    next_run.as_deref(),
                )
                .await;
                let finished =
                    RunsRepo::finish(&self.pool, &run.id, RunStatus::Failed, 0, 0, Some(err_str))
                        .await?;
                let _ = app.emit("scrape://finished", &finished);
                let _ = app.emit("sources://changed", ());
                Err(AppError::Internal(
                    "Single source scrape timed out".to_string(),
                ))
            }
        }
    }

    async fn execute_sidecar(
        &self,
        app: &AppHandle,
        run_id: &str,
        config: WorkerConfig,
    ) -> Result<(i64, i64, HashMap<String, (String, Option<String>)>)> {
        let (mut rx, mut child) = app
            .shell()
            .sidecar("scrape-worker")
            .map_err(|e| AppError::Internal(format!("Failed to find sidecar: {}", e)))?
            .args(["--stdin"])
            .spawn()
            .map_err(|e| AppError::Internal(format!("Failed to spawn sidecar: {}", e)))?;

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
        child
            .write(&config_bytes)
            .map_err(|e| AppError::Internal(format!("Failed to write to sidecar stdin: {}", e)))?;

        let mut job_buffer: Vec<ScrapedJob> = Vec::new();
        let mut last_flush = Instant::now();
        let mut total_found = 0i64;
        let mut total_new = 0i64;
        let mut source_statuses: HashMap<String, (String, Option<String>)> = HashMap::new();

        let mut termination_code: Option<i32> = None;
        let mut sidecar_err: Option<String> = None;

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
                                    salary,
                                    locations,
                                    workplace_type,
                                    sponsorship,
                                    clearance,
                                    experience,
                                    is_new_grad,
                                    description_text,
                                } => {
                                    let dedupe_key = format!("{}:{}", source_id, external_id);
                                    let loc_opt = if location.is_empty() {
                                        None
                                    } else {
                                        Some(location)
                                    };

                                    let (
                                        salary_min,
                                        salary_max,
                                        salary_currency,
                                        salary_interval,
                                        salary_raw,
                                        salary_source,
                                    ) = if let Some(s) = salary {
                                        (
                                            s.min,
                                            s.max,
                                            Some(s.currency),
                                            Some(s.interval),
                                            Some(s.raw),
                                            Some(s.source),
                                        )
                                    } else {
                                        (None, None, None, None, None, None)
                                    };

                                    let (sponsorship_val, sponsorship_evidence) =
                                        if let Some(sp) = sponsorship {
                                            (sp.value, sp.evidence)
                                        } else {
                                            (TriState::Unknown, None)
                                        };

                                    let (clearance_val, clearance_level, clearance_evidence) =
                                        if let Some(cl) = clearance {
                                            (cl.value, cl.level, cl.evidence)
                                        } else {
                                            (TriState::Unknown, None, None)
                                        };

                                    let (
                                        experience_min,
                                        experience_max,
                                        experience_raw,
                                        experience_evidence,
                                    ) = if let Some(exp) = experience {
                                        (exp.min_years, exp.max_years, exp.raw, exp.evidence)
                                    } else {
                                        (None, None, None, None)
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
                                        salary_min,
                                        salary_max,
                                        salary_currency,
                                        salary_interval,
                                        salary_raw,
                                        salary_source,
                                        workplace_type,
                                        sponsorship: sponsorship_val,
                                        sponsorship_evidence,
                                        clearance: clearance_val,
                                        clearance_level,
                                        clearance_evidence,
                                        experience_min,
                                        experience_max,
                                        experience_raw,
                                        is_new_grad,
                                        experience_evidence,
                                        description_text,
                                        locations,
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
                                } => {
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
                                WorkerEvent::Done { .. } => {}
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

        // If the sidecar crashed or exited with an error code, ensure all sources are marked failed
        if let Some(code) = termination_code {
            if code != 0 {
                let fatal_msg = format!("Sidecar process exited with non-zero exit code: {}", code);
                for src in &config.sources {
                    source_statuses
                        .entry(src.id.clone())
                        .or_insert_with(|| ("error".to_string(), Some(fatal_msg.clone())));
                }
            }
        }
        if let Some(ref err) = sidecar_err {
            let fatal_msg = format!("Sidecar process error: {}", err);
            for src in &config.sources {
                source_statuses
                    .entry(src.id.clone())
                    .or_insert_with(|| ("error".to_string(), Some(fatal_msg.clone())));
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
            total_new += stats.inserted as i64;
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

    #[test]
    fn test_resolve_worker_filters_with_overrides() {
        let global = WorkerFilters {
            include_keywords: vec!["GlobalKW".to_string()],
            exclude_keywords: vec!["GlobalExclude".to_string()],
            locations: vec!["GlobalLoc".to_string()],
            role_types: vec![RoleType::FullTime],
            query: Some("title LIKE '%Global%'".to_string()),
            policy: None,
        };

        let source_filters = serde_json::json!({
            "include_keywords": ["CustomKW", "Rust"],
            "role_types": ["internship"],
            "query": "title LIKE '%Engineer%' AND location LIKE '%Remote%'"
        });

        let resolved = resolve_worker_filters(&source_filters, global);
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
    fn test_resolve_worker_filters_fallback() {
        let global = WorkerFilters {
            include_keywords: vec!["Engineer".to_string()],
            exclude_keywords: vec!["Manager".to_string()],
            locations: vec!["Remote".to_string()],
            role_types: vec![RoleType::FullTime],
            query: None,
            policy: None,
        };

        let null_filters = serde_json::Value::Null;
        let resolved = resolve_worker_filters(&null_filters, global.clone());
        assert_eq!(resolved.include_keywords, global.include_keywords);
        assert_eq!(resolved.exclude_keywords, global.exclude_keywords);
        assert_eq!(resolved.locations, global.locations);
        assert_eq!(resolved.role_types, global.role_types);

        let empty_filters = serde_json::json!({});
        let resolved_empty = resolve_worker_filters(&empty_filters, global.clone());
        assert_eq!(resolved_empty.include_keywords, global.include_keywords);
    }

    #[tokio::test]
    async fn test_single_source_workflow_execution_logs_pipeline() {
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
    async fn test_failed_source_workflow_execution_logs_pipeline() {
        let pool = crate::db::init_memory_db().await.unwrap();

        let source = crate::db::sources::SourcesRepo::create(
            &pool,
            crate::db::sources::CreateSourceInput {
                kind: scrape_core::SourceKind::Selector,
                name: "Broken Careers".to_string(),
                identifier: "https://invalid.example.com".to_string(),
                config: None,
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
}
