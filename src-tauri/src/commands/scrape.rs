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

async fn execute_dry_run_worker(
    app: &AppHandle,
    worker_source: WorkerSource,
    filters: WorkerFilters,
    blocked_postings: Vec<String>,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<Vec<WorkerEvent>> {
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        run_started_at: Some(scrape_core::now_iso()),
        filters_by_source: Default::default(),
        sources: vec![worker_source],
        filters,
        limits: WorkerLimits::default(),
        blocked_postings,
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
        if cancel
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
    let filters = crate::services::runner::resolve_worker_filters(
        &input.filters.unwrap_or(serde_json::Value::Null),
        crate::services::runner::load_global_filters(&state.db).await?,
    )?;
    let worker_source = WorkerSource {
        id: "transient-test".to_string(),
        kind: input.kind,
        name: input.name,
        identifier: input.identifier,
        config: input.config.unwrap_or_else(|| serde_json::json!({})),
    };
    execute_dry_run_worker(
        &app,
        worker_source,
        filters,
        crate::db::postings::suppression_snapshot(&state.db).await?,
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
    let source = SourcesRepo::get(&state.db, &id).await?;
    let filters = crate::services::runner::resolve_worker_filters(
        &source.filters,
        crate::services::runner::load_global_filters(&state.db).await?,
    )?;
    let worker_source = WorkerSource {
        id: source.id,
        kind: source.kind,
        name: source.name,
        identifier: source.identifier,
        config: source.config,
    };
    execute_dry_run_worker(
        &app,
        worker_source,
        filters,
        crate::db::postings::suppression_snapshot(&state.db).await?,
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
    let timezone: String = SettingsRepo::get_value(&state.db, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    crate::services::workflow::validate(spec, &timezone)
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
    let spec = crate::services::workflow::validate(spec, &timezone)?;
    let source = crate::services::workflow::source_input(&spec, &timezone)?;
    let filters = crate::services::runner::resolve_worker_filters(
        source.filters.as_ref().unwrap_or(&serde_json::Value::Null),
        crate::services::runner::load_global_filters(&state.db).await?,
    )?;

    let events = execute_dry_run_worker(
        &app,
        WorkerSource {
            id: "preview".into(),
            kind: source.kind,
            name: spec
                .metadata
                .company
                .clone()
                .unwrap_or_else(|| "Unknown company".into()),
            identifier: source.identifier.clone(),
            config: source.config.clone().unwrap_or(serde_json::Value::Null),
        },
        filters,
        crate::db::postings::suppression_snapshot(&state.db).await?,
        Some(cancel),
    )
    .await?;
    let mut result = WorkflowPreview {
        spec: spec.clone(),
        jobs: vec![],
        fetched: 0,
        matched: 0,
        rejected: 0,
        blocked: 0,
        invalid: 0,
        sampled: false,
        errors: vec![],
        next_run_at: crate::services::calendar::next_for_source(
            source.cadence.unwrap_or_default(),
            source.cadence_cron.as_deref(),
            source.config.as_ref().unwrap_or(&serde_json::Value::Null),
            source.enabled.unwrap_or(true),
            &timezone,
            chrono::Utc::now(),
        )?,
    };
    for event in events {
        match event {
            WorkerEvent::Job {
                ref external_id,
                ref url,
                ..
            } => {
                let identity = scrape_core::PostingIdentity::for_posting(
                    Some(source.kind),
                    Some(&source.identifier),
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
            WorkerEvent::Done { stats } => {
                for scan in stats.sources {
                    result.blocked += scan.blocked;
                    result.sampled |= scan.limit_reached;
                }
            }
        }
    }
    result.rejected = result
        .fetched
        .saturating_sub(result.matched + result.invalid);
    result.sampled |= result.matched.saturating_sub(result.blocked) > result.jobs.len() as u64;
    Ok(result)
}
#[tauri::command]
pub async fn save_workflow(
    app: AppHandle,
    state: State<'_, AppState>,
    spec: serde_json::Value,
    id: Option<String>,
) -> Result<crate::db::sources::Source> {
    let timezone: String = SettingsRepo::get_value(&state.db, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let spec = crate::services::workflow::validate(spec, &timezone)?;
    let input = crate::services::workflow::source_input(&spec, &timezone)?;
    let source = if let Some(id) = id {
        let old = SourcesRepo::get(&state.db, &id).await?;
        if old.kind != input.kind {
            return Err(AppError::Validation(
                "Create a new workflow to change the source adapter".into(),
            ));
        }
        SourcesRepo::update(
            &state.db,
            crate::db::sources::UpdateSourceInput {
                id,
                name: Some(input.name),
                identifier: Some(input.identifier),
                config: input.config,
                enabled: input.enabled,
                cadence: input.cadence,
                cadence_cron: input.cadence_cron,
                retention_days: input.retention_days,
                retention_policy: input.retention_policy,
                filters: input.filters,
            },
        )
        .await?
    } else {
        SourcesRepo::create(&state.db, input).await?
    };
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
