use crate::db::runs::{RunsRepo, ScrapeRun, TriggerType};
use crate::db::settings::SettingsRepo;
use crate::db::sources::SourcesRepo;
use crate::error::{AppError, Result};
use crate::services::scheduler::{validate_cron_expression, CronValidationResult, SchedulerStatus};
use crate::state::AppState;
use scrape_core::{
    WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, LazyLock, Mutex as StdMutex,
};
use tauri::{AppHandle, State};
use tauri_plugin_shell::process::CommandEvent;
use ts_rs::TS;

#[tauri::command]
pub async fn run_scrape_now(app: AppHandle, state: State<'_, AppState>) -> Result<ScrapeRun> {
    state.runner.trigger(&app, TriggerType::Manual).await
}

#[tauri::command]
pub fn cancel_scrape(state: State<'_, AppState>) -> bool {
    state.runner.cancel_current()
}

#[tauri::command]
pub async fn list_scrape_runs(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<ScrapeRun>> {
    RunsRepo::recent(&state.db, limit.unwrap_or(20)).await
}

#[tauri::command]
pub async fn get_scrape_run(state: State<'_, AppState>, run_id: String) -> Result<ScrapeRun> {
    RunsRepo::get(&state.db, &run_id).await
}

#[tauri::command]
pub async fn get_scrape_logs(
    state: State<'_, AppState>,
    run_id: String,
) -> Result<Vec<crate::db::runs::ScrapeLogEntry>> {
    RunsRepo::get_logs(&state.db, &run_id).await
}

#[tauri::command]
pub async fn validate_cron(
    state: State<'_, AppState>,
    expr: String,
) -> Result<CronValidationResult> {
    let tz_str: String = SettingsRepo::get_value(&state.db, "system.timezone")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "UTC".to_string());
    Ok(validate_cron_expression(&expr, &tz_str))
}

#[tauri::command]
pub async fn get_scheduler_status(state: State<'_, AppState>) -> Result<SchedulerStatus> {
    state.scheduler.lock().await.get_status().await
}

#[tauri::command]
pub async fn set_scheduler_paused(
    app: AppHandle,
    state: State<'_, AppState>,
    paused: bool,
) -> Result<()> {
    let _permit = state.runner.write_permit()?;
    state.scheduler.lock().await.set_paused(paused).await?;
    use tauri::Emitter;
    let _ = app.emit("settings://changed", ());
    Ok(())
}

#[tauri::command]
pub async fn detect_source(app: AppHandle, url: String) -> Result<serde_json::Value> {
    let output = crate::services::process::worker_command(&app)?
        .args(["--detect", &url])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to execute detect: {}", e)))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(stdout_str.trim())
        .map_err(|e| AppError::Internal(format!("Failed to parse detect result: {}", e)))?;

    Ok(val)
}

#[tauri::command]
pub async fn run_source_now(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<ScrapeRun> {
    state.runner.trigger_single_source(&app, &id).await
}

#[tauri::command]
pub async fn list_source_runs(
    state: State<'_, AppState>,
    source_id: String,
    limit: Option<i64>,
) -> Result<Vec<ScrapeRun>> {
    RunsRepo::list_by_source(&state.db, &source_id, limit.unwrap_or(10)).await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct TransientSourceInput {
    pub kind: scrape_core::SourceKind,
    pub name: String,
    pub identifier: String,
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub filters: Option<serde_json::Value>,
}

static PREVIEW_CANCEL: LazyLock<StdMutex<std::collections::HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| StdMutex::new(std::collections::HashMap::new()));
struct PreviewRegistration(String);
impl Drop for PreviewRegistration {
    fn drop(&mut self) {
        PREVIEW_CANCEL.lock().unwrap().remove(&self.0);
    }
}
async fn terminate_preview(
    mut child: crate::services::process::ChildGuard,
    receiver: &mut tauri::async_runtime::Receiver<CommandEvent>,
) {
    let _ = child.kill();
    let until = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while let Ok(Some(event)) = tokio::time::timeout_at(until, receiver.recv()).await {
        if matches!(event, CommandEvent::Terminated(_)) {
            break;
        }
    }
}

struct PreviewHost<'a> {
    app: &'a AppHandle,
    pool: &'a sqlx::SqlitePool,
}

async fn execute_dry_run_worker(
    host: PreviewHost<'_>,
    sources: Vec<WorkerSource>,
    filters: WorkerFilters,
    blocked_postings: Vec<String>,
    cancel: Option<Arc<AtomicBool>>,
    run_cancel: Arc<AtomicBool>,
    max_jobs_per_run: Option<u32>,
) -> Result<Vec<WorkerEvent>> {
    let PreviewHost { app, pool } = host;
    crate::services::script_trust::validate_execution(pool, &sources).await?;
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        run_started_at: Some(scrape_core::now_iso()),
        filters_by_source: Default::default(),
        sources,
        filters,
        limits: WorkerLimits::default(),
        blocked_postings,
        max_jobs_per_run,
        starting_source_index: Some(0),
    };

    let (mut rx, child) = crate::services::process::worker_command(app)?
        .args(["--stdin", "--dry-run"])
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to spawn sidecar: {}", e)))?;

    let mut child = crate::services::process::ChildGuard::new(child);
    let mut config_bytes = serde_json::to_vec(&config)?;
    config_bytes.push(b'\n');
    if let Err(error) = child.write(&config_bytes) {
        let _ = child.kill();
        return Err(AppError::Internal(format!(
            "Failed to write to sidecar stdin: {error}"
        )));
    }

    let mut events = Vec::new();
    let mut done = false;
    let mut exit = None;
    let mut failure = None;
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_secs(crate::services::process::execution_seconds(60));
    loop {
        if run_cancel.load(Ordering::SeqCst)
            || cancel
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            terminate_preview(child, &mut rx).await;
            return Err(AppError::Internal("Preview cancelled".into()));
        }
        let incoming = tokio::select! { result=tokio::time::timeout_at(deadline,rx.recv())=>result, _=tokio::time::sleep(std::time::Duration::from_millis(100))=>continue };
        let event = match incoming {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(_) => {
                terminate_preview(child, &mut rx).await;
                return Err(AppError::Internal(
                    "Preview timed out after 60 seconds".into(),
                ));
            }
        };
        match event {
            CommandEvent::Stdout(bytes) => {
                let text = match std::str::from_utf8(&bytes) {
                    Ok(text) => text,
                    Err(error) => {
                        failure = Some(format!("Worker output is not valid UTF-8: {error}"));
                        continue;
                    }
                };
                for line in text.lines().filter(|line| !line.trim().is_empty()) {
                    if done {
                        failure = Some("Worker emitted data after completion".into());
                        continue;
                    }
                    match serde_json::from_str::<WorkerEvent>(line) {
                        Ok(event) => {
                            if event.source_id().is_some_and(|id| {
                                !config.sources.iter().any(|source| source.id == id)
                            }) {
                                failure.get_or_insert_with(|| {
                                    "Worker preview emitted an unknown source ID".into()
                                });
                                continue;
                            }
                            if let WorkerEvent::Branding { logo, .. } = &event {
                                if let Err(error) = logo.validate() {
                                    failure.get_or_insert_with(|| {
                                        format!("Invalid employer branding report: {error}")
                                    });
                                    continue;
                                }
                            }
                            if let WorkerEvent::Done { stats } = &event {
                                if stats.sources.len() != config.sources.len()
                                    || stats
                                        .sources
                                        .iter()
                                        .map(|scan| &scan.source_id)
                                        .collect::<std::collections::HashSet<_>>()
                                        .len()
                                        != stats.sources.len()
                                    || stats.sources.iter().any(|scan| {
                                        !config
                                            .sources
                                            .iter()
                                            .any(|source| source.id == scan.source_id)
                                            || scan.outcome.is_none()
                                            || scan.validate().is_err()
                                    })
                                {
                                    failure.get_or_insert_with(|| "Worker preview completion has missing or inconsistent source summaries".into());
                                }
                                if let Ok(session_dir) =
                                    crate::services::browser_session::session_dir_for_app(app)
                                {
                                    for scan in &stats.sources {
                                        if let Some(worker_source) =
                                            config.sources.iter().find(|s| s.id == scan.source_id)
                                        {
                                            let _ = crate::services::browser_session::reconcile_source_session_after_scan(
                                                pool,
                                                &session_dir,
                                                &worker_source.config,
                                                Some(scan.termination),
                                            )
                                            .await;
                                        }
                                    }
                                }
                            }
                            done = matches!(event, WorkerEvent::Done { .. });
                            events.push(event);
                        }
                        Err(error) => failure = Some(format!("Malformed worker event: {error}")),
                    }
                }
            }
            CommandEvent::Terminated(status) => {
                child.disarm();
                exit = status.code;
                break;
            }
            CommandEvent::Error(error) => {
                failure = Some(error);
            }
            _ => {}
        }
    }
    if let Some(message) = crate::services::runner::worker_failure(exit, done, failure.as_deref()) {
        return Err(AppError::Internal(message));
    }

    Ok(events)
}

#[tauri::command]
pub async fn test_source_transient(
    app: AppHandle,
    state: State<'_, AppState>,
    input: TransientSourceInput,
) -> Result<Vec<WorkerEvent>> {
    let _guard = state.runner.try_acquire_lock()?;
    let filters = crate::services::runner::resolve_worker_filters(
        &input.filters.unwrap_or(serde_json::Value::Null),
        scrape_core::WorkerFilters::default(),
    )?;
    let worker_source = WorkerSource {
        id: "transient-test".to_string(),
        kind: input.kind,
        name: input.name,
        identifier: input.identifier,
        config: input.config.unwrap_or_else(|| serde_json::json!({})),
    };
    execute_dry_run_worker(
        PreviewHost {
            app: &app,
            pool: &state.db,
        },
        vec![worker_source],
        filters,
        crate::db::postings::suppression_snapshot(&state.db).await?,
        None,
        state.runner.cancellation_flag(),
        None,
    )
    .await
}

#[tauri::command]
pub async fn test_source(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<WorkerEvent>> {
    let _guard = state.runner.try_acquire_lock()?;
    let source = SourcesRepo::get(&state.db, &id).await?;
    if source.needs_review {
        return Err(AppError::Validation(
            "Workflow needs review before it can run.".into(),
        ));
    }
    let filters = crate::services::runner::resolve_worker_filters(
        &source.filters,
        scrape_core::WorkerFilters::default(),
    )?;
    let worker_source = WorkerSource {
        id: source.id,
        kind: source.kind,
        name: source.name,
        identifier: source.identifier,
        config: source.config,
    };
    execute_dry_run_worker(
        PreviewHost {
            app: &app,
            pool: &state.db,
        },
        vec![worker_source],
        filters,
        crate::db::postings::suppression_snapshot(&state.db).await?,
        None,
        state.runner.cancellation_flag(),
        None,
    )
    .await
}

#[tauri::command]
pub async fn open_terminal_console(app: AppHandle) -> Result<()> {
    crate::services::system::SystemService::launch_terminal_console(&app)
}

#[tauri::command]
pub async fn validate_workflow(
    state: State<'_, AppState>,
    spec: serde_json::Value,
) -> Result<crate::services::workflow::WorkflowSpec> {
    validate_workflow_spec(&state.db, spec).await
}

#[tauri::command]
pub async fn import_workflow(
    state: State<'_, AppState>,
    spec: serde_json::Value,
) -> Result<crate::services::workflow::WorkflowSpec> {
    crate::services::script_trust::quarantine_import(validate_workflow_spec(&state.db, spec).await?)
}

#[tauri::command]
pub async fn trust_workflow_script(
    app: AppHandle,
    state: State<'_, AppState>,
    source_id: String,
    expected_hash: String,
    confirmed: bool,
) -> Result<crate::db::sources::Source> {
    let _permit = state.runner.write_permit()?;
    let source =
        crate::services::script_trust::trust(&state.db, &source_id, &expected_hash, confirmed)
            .await?;
    use tauri::Emitter;
    app.emit("sources://changed", &source)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(source)
}

async fn validate_workflow_spec(
    pool: &sqlx::SqlitePool,
    spec: serde_json::Value,
) -> Result<crate::services::workflow::WorkflowSpec> {
    let timezone: String = SettingsRepo::get_value(pool, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    if let Some(version) = spec.get("version") {
        let version = version
            .as_u64()
            .ok_or_else(|| AppError::Validation("version: expected an integer".into()))?;
        if version >= 4 {
            return crate::services::workflow::validate(spec, &timezone);
        }
    }
    let cron: String = SettingsRepo::get_value(pool, "scraper.cron")
        .await?
        .unwrap_or_else(|| "0 0 */6 * * *".into());
    let startup: bool = SettingsRepo::get_value(pool, "scraper.run_on_startup")
        .await?
        .unwrap_or(false);
    let filters = crate::services::runner::load_global_filters(pool).await?;
    crate::services::workflow::validate_legacy_import(spec, &timezone, &cron, &filters, startup)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PreviewPosting {
    pub identity: String,
    pub job: WorkerEvent,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowPreview {
    #[serde(default)]
    #[ts(optional = nullable)]
    pub warnings: Option<Vec<String>>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub sources: Option<Vec<scrape_core::protocol::SourceScanStats>>,
    pub spec: crate::services::workflow::WorkflowSpec,
    pub jobs: Vec<PreviewPosting>,
    #[ts(type = "number")]
    pub fetched: u64,
    #[ts(type = "number")]
    pub matched: u64,
    #[ts(type = "number")]
    pub rejected: u64,
    #[ts(type = "number")]
    pub blocked: u64,
    #[ts(type = "number")]
    pub invalid: u64,
    pub sampled: bool,
    pub errors: Vec<String>,
    pub next_run_at: Option<String>,
}
#[tauri::command]
pub async fn preview_workflow(
    app: AppHandle,
    state: State<'_, AppState>,
    spec: serde_json::Value,
    request_id: String,
) -> Result<WorkflowPreview> {
    let _guard = state.runner.try_acquire_lock()?;
    let cancel = Arc::new(AtomicBool::new(false));
    PREVIEW_CANCEL
        .lock()
        .unwrap()
        .insert(request_id.clone(), cancel.clone());
    let _registration = PreviewRegistration(request_id);

    let timezone: String = SettingsRepo::get_value(&state.db, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let spec = validate_workflow_spec(&state.db, spec).await?;
    let inputs = crate::services::workflow::workflow_source_inputs(&spec, &timezone)?;
    let filters = crate::services::runner::resolve_worker_filters(
        &serde_json::to_value(spec.filters.clone().unwrap_or_default())?,
        scrape_core::WorkerFilters::default(),
    )?;

    let worker_sources: Vec<WorkerSource> = spec
        .sources
        .iter()
        .zip(inputs.iter())
        .filter(|(s, _)| s.enabled)
        .map(|(s, input)| WorkerSource {
            id: s.id.clone(),
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

    let preview_limit = std::cmp::min(
        spec.max_jobs_per_run
            .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN),
        scrape_core::protocol::PREVIEW_MAX_JOBS,
    );

    let events = execute_dry_run_worker(
        PreviewHost {
            app: &app,
            pool: &state.db,
        },
        worker_sources,
        filters,
        crate::db::postings::suppression_snapshot(&state.db).await?,
        Some(cancel),
        state.runner.cancellation_flag(),
        Some(preview_limit),
    )
    .await?;
    let primary_input = &inputs[0];
    let mut result = WorkflowPreview {
        spec: spec.clone(),
        sources: None,
        warnings: Some(vec![]),
        jobs: vec![],
        fetched: 0,
        matched: 0,
        rejected: 0,
        blocked: 0,
        invalid: 0,
        sampled: false,
        errors: vec![],
        next_run_at: crate::services::calendar::next_for_source(
            primary_input.cadence.unwrap_or_default(),
            primary_input.cadence_cron.as_deref(),
            primary_input
                .config
                .as_ref()
                .unwrap_or(&serde_json::Value::Null),
            primary_input.enabled.unwrap_or(true),
            &timezone,
            chrono::Utc::now(),
        )?,
    };
    for event in events {
        match event {
            WorkerEvent::Job {
                ref external_id,
                ref url,
                ref source_id,
                ..
            } => {
                let source_match = spec.sources.iter().find(|s| s.id == *source_id);
                let (source_kind, source_identifier) = source_match
                    .map(|s| {
                        let input =
                            crate::services::workflow::extract_source_input(&spec, s, &timezone)
                                .ok();
                        (
                            s.extractor.kind,
                            input.map(|i| i.identifier).unwrap_or_default(),
                        )
                    })
                    .unwrap_or((primary_input.kind, primary_input.identifier.clone()));
                let identity = scrape_core::PostingIdentity::for_posting(
                    Some(source_kind),
                    Some(&source_identifier),
                    Some(external_id),
                    url,
                )
                .map_err(AppError::Validation)?
                .key;
                let blocked: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM blocked_postings WHERE identity=?)",
                )
                .bind(&identity)
                .fetch_one(&state.db)
                .await?;
                if blocked {
                    result.blocked += 1;
                } else if result.jobs.len() < scrape_core::protocol::PREVIEW_MAX_JOBS as usize {
                    result.jobs.push(PreviewPosting {
                        identity,
                        job: event,
                    });
                }
            }
            WorkerEvent::Progress {
                fetched, matched, ..
            } => {
                result.fetched += fetched;
                result.matched += matched;
            }
            WorkerEvent::Error {
                message,
                invalid_posting,
                ..
            } => {
                if invalid_posting == Some(true) {
                    result.invalid += 1;
                }
                result.errors.push(message);
            }
            WorkerEvent::Warning {
                message,
                eligibility_incomplete,
                ..
            } => {
                if eligibility_incomplete {
                    result.errors.push(message);
                } else {
                    result.warnings.get_or_insert_with(Vec::new).push(message);
                }
            }
            WorkerEvent::Branding { logo, .. } => {
                logo.validate().map_err(AppError::Validation)?;
            }
            WorkerEvent::SourceInfo { .. } => {}
            WorkerEvent::Done { stats } => {
                for scan in &stats.sources {
                    scan.validate().map_err(AppError::Validation)?;
                }
                result.sources = Some(stats.sources.clone());
                for scan in stats.sources {
                    result.blocked += scan.blocked;
                    result.sampled |= scan.limit_reached;
                    if matches!(
                        scan.outcome,
                        Some(
                            scrape_core::protocol::ScanOutcome::Partial
                                | scrape_core::protocol::ScanOutcome::Failed
                        )
                    ) {
                        result.errors.push(if scan.outcome == Some(scrape_core::protocol::ScanOutcome::Failed) { "Source could not complete usable processing. Inspect diagnostics and retry.".into() } else { format!("Source processing incomplete: {} candidates need eligibility evidence. Inspect scan details and retry.",scan.unresolved) });
                    }
                }
            }
        }
    }
    result.rejected = result
        .sources
        .as_ref()
        .map(|sources| sources.iter().map(|scan| scan.rejected).sum())
        .unwrap_or_else(|| {
            result
                .fetched
                .saturating_sub(result.matched + result.invalid)
        });
    result.sampled |= result.matched.saturating_sub(result.blocked) > result.jobs.len() as u64;
    Ok(result)
}
#[tauri::command]
pub async fn save_workflow(
    app: AppHandle,
    state: State<'_, AppState>,
    spec: serde_json::Value,
    id: Option<String>,
    imported: Option<bool>,
) -> Result<crate::db::sources::Source> {
    let _permit = state.runner.write_permit()?;
    let timezone: String = SettingsRepo::get_value(&state.db, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let spec = validate_workflow_spec(&state.db, spec).await?;
    let source = crate::services::workflow::persist(
        &state.db,
        spec,
        id,
        imported.unwrap_or(false),
        &timezone,
    )
    .await?;
    use tauri::Emitter;
    let _ = app.emit("sources://changed", &source);
    Ok(source)
}

#[tauri::command]
pub fn cancel_workflow_preview(request_id: String) -> bool {
    if let Some(flag) = PREVIEW_CANCEL.lock().unwrap().get(&request_id) {
        flag.store(true, Ordering::SeqCst);
        true
    } else {
        false
    }
}

#[tauri::command]
pub fn detect_browser_runtime() -> Result<scrape_core::protocol::BrowserRuntimeStatus> {
    Ok(crate::services::browser_session::detect_browser_runtime())
}

#[tauri::command]
pub async fn list_browser_sessions(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<scrape_core::protocol::BrowserSessionSummary>> {
    let session_dir = crate::services::browser_session::session_dir_for_app(&app)?;
    crate::services::browser_session::list_browser_sessions(&state.db, &session_dir).await
}

#[tauri::command]
pub async fn open_browser_session(
    app: AppHandle,
    state: State<'_, AppState>,
    label: String,
    login_url: String,
) -> Result<scrape_core::protocol::BrowserSessionSummary> {
    let _permit = state.runner.write_permit()?;
    let session_dir = crate::services::browser_session::session_dir_for_app(&app)?;
    crate::services::browser_session::open_browser_session(
        &state.db,
        &session_dir,
        &label,
        &login_url,
    )
    .await
}

#[tauri::command]
pub async fn save_browser_session(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<scrape_core::protocol::BrowserSessionSummary> {
    let _permit = state.runner.write_permit()?;
    let session_dir = crate::services::browser_session::session_dir_for_app(&app)?;
    crate::services::browser_session::save_browser_session(&state.db, &session_dir, &session_id)
        .await
}

#[tauri::command]
pub async fn clear_browser_session(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
) -> Result<()> {
    let _permit = state.runner.write_permit()?;
    let session_dir = crate::services::browser_session::session_dir_for_app(&app)?;
    crate::services::browser_session::clear_browser_session(&state.db, &session_dir, &session_id)
        .await
}

#[tauri::command]
pub async fn test_extractor(
    app: AppHandle,
    state: State<'_, AppState>,
    extractor: crate::services::workflow::WorkflowExtractor,
) -> Result<scrape_core::protocol::ExtractorTestResult> {
    let _guard = state.runner.try_acquire_lock()?;
    let spec = crate::services::workflow::WorkflowSpec {
        version: crate::services::workflow::WORKFLOW_SPEC_VERSION,
        metadata: crate::services::workflow::WorkflowMetadata {
            name: "Extractor Test".into(),
            company: None,
            enabled: Some(true),
        },
        sources: vec![crate::services::workflow::WorkflowSourceSpec {
            id: "extractor-test".into(),
            label: "Extractor Test".into(),
            employer_hint: None,
            enabled: true,
            extractor: extractor.clone(),
        }],
        schedule: None,
        filters: None,
        retention: None,
        max_jobs_per_run: Some(scrape_core::protocol::PREVIEW_MAX_JOBS),
    };
    let input = crate::services::workflow::extract_source_input(&spec, &spec.sources[0], "UTC")?;
    let worker_source = WorkerSource {
        id: "extractor-test".into(),
        kind: input.kind,
        name: "Extractor Test".into(),
        identifier: input.identifier,
        config: input.config.unwrap_or_else(|| serde_json::json!({})),
    };
    let events = execute_dry_run_worker(
        PreviewHost {
            app: &app,
            pool: &state.db,
        },
        vec![worker_source],
        scrape_core::WorkerFilters::default(),
        Vec::new(),
        None,
        state.runner.cancellation_flag(),
        Some(scrape_core::protocol::PREVIEW_MAX_JOBS),
    )
    .await?;

    let mut samples = Vec::new();
    let mut field_match_counts = std::collections::BTreeMap::new();
    for key in [
        "title",
        "url",
        "location",
        "salary",
        "description",
        "date",
        "id",
        "employer_logo",
    ] {
        field_match_counts.insert(key.to_string(), 0);
    }
    let mut warnings = Vec::new();
    let mut termination = scrape_core::protocol::CoverageTermination::Exhausted;

    for event in events {
        match event {
            WorkerEvent::Job {
                external_id,
                title,
                company,
                location,
                url,
                date_posted,
                salary,
                description_text,
                company_icon_url,
                company_logo,
                ..
            } => {
                if !title.trim().is_empty() {
                    *field_match_counts.entry("title".into()).or_insert(0) += 1;
                }
                if !url.trim().is_empty() {
                    *field_match_counts.entry("url".into()).or_insert(0) += 1;
                }
                if !location.trim().is_empty() {
                    *field_match_counts.entry("location".into()).or_insert(0) += 1;
                }
                if salary.is_some() {
                    *field_match_counts.entry("salary".into()).or_insert(0) += 1;
                }
                if description_text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
                {
                    *field_match_counts.entry("description".into()).or_insert(0) += 1;
                }
                if date_posted
                    .as_deref()
                    .is_some_and(|date| !date.trim().is_empty())
                {
                    *field_match_counts.entry("date".into()).or_insert(0) += 1;
                }
                if !external_id.trim().is_empty() {
                    *field_match_counts.entry("id".into()).or_insert(0) += 1;
                }
                let logo_url =
                    company_icon_url.or_else(|| company_logo.map(|evidence| evidence.url));
                if logo_url.is_some() {
                    *field_match_counts
                        .entry("employer_logo".into())
                        .or_insert(0) += 1;
                }
                if samples.len() < 5 {
                    samples.push(scrape_core::protocol::ExtractorSampleCandidate {
                        evidence_url: extractor.url.clone(),
                        external_id: Some(external_id).filter(|id| !id.trim().is_empty()),
                        title,
                        url,
                        company: Some(company).filter(|c| !c.trim().is_empty()),
                        location: Some(location).filter(|l| !l.trim().is_empty()),
                        salary: salary.map(|s| s.raw),
                        description: description_text,
                        date_posted,
                        employer_logo_url: logo_url,
                    });
                }
            }
            WorkerEvent::Warning { message, .. } | WorkerEvent::Error { message, .. } => {
                warnings.push(message);
            }
            WorkerEvent::Done { stats } => {
                if let Some(scan) = stats.sources.first() {
                    termination = scan.termination;
                }
            }
            _ => {}
        }
    }

    Ok(scrape_core::protocol::ExtractorTestResult {
        samples,
        field_match_counts,
        warnings,
        termination,
    })
}
