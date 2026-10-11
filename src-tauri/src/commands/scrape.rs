use crate::db::runs::{RunsRepo, ScrapeRun, TriggerType};
use crate::db::settings::SettingsRepo;
use crate::db::sources::SourcesRepo;
use crate::error::{AppError, Result};
use crate::services::scheduler::{validate_cron_expression, CronValidationResult, SchedulerStatus};
use crate::state::AppState;
use scrape_core::{
    WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, PROTOCOL_VERSION,
};
use tauri::{AppHandle, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn run_scrape_now(app: AppHandle, state: State<'_, AppState>) -> Result<ScrapeRun> {
    state.runner.trigger(&app, TriggerType::Manual).await
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
pub async fn set_scheduler_paused(state: State<'_, AppState>, paused: bool) -> Result<()> {
    state.scheduler.lock().await.set_paused(paused).await
}

#[tauri::command]
pub async fn detect_source(app: AppHandle, url: String) -> Result<serde_json::Value> {
    let output = app
        .shell()
        .sidecar("scrape-worker")
        .map_err(|e| AppError::Internal(format!("Failed to find sidecar: {}", e)))?
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
}

async fn execute_dry_run_worker(
    app: &AppHandle,
    worker_source: WorkerSource,
) -> Result<Vec<WorkerEvent>> {
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![worker_source],
        filters: WorkerFilters::default(),
        limits: WorkerLimits::default(),
    };

    let (mut rx, mut child) = app
        .shell()
        .sidecar("scrape-worker")
        .map_err(|e| AppError::Internal(format!("Failed to find sidecar: {}", e)))?
        .args(["--stdin", "--dry-run"])
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to spawn sidecar: {}", e)))?;

    let config_bytes = serde_json::to_vec(&config)?;
    child
        .write(&config_bytes)
        .map_err(|e| AppError::Internal(format!("Failed to write to sidecar stdin: {}", e)))?;

    let mut events = Vec::new();

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(bytes) = event {
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(worker_event) = serde_json::from_str::<WorkerEvent>(trimmed) {
                    events.push(worker_event);
                }
            }
        }
    }

    Ok(events)
}

#[tauri::command]
pub async fn test_source_transient(
    app: AppHandle,
    input: TransientSourceInput,
) -> Result<Vec<WorkerEvent>> {
    let worker_source = WorkerSource {
        id: "transient-test".to_string(),
        kind: input.kind,
        name: input.name,
        identifier: input.identifier,
        config: input.config.unwrap_or_else(|| serde_json::json!({})),
    };
    execute_dry_run_worker(&app, worker_source).await
}

#[tauri::command]
pub async fn test_source(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<WorkerEvent>> {
    let source = SourcesRepo::get(&state.db, &id).await?;
    let worker_source = WorkerSource {
        id: source.id,
        kind: source.kind,
        name: source.name,
        identifier: source.identifier,
        config: source.config,
    };
    execute_dry_run_worker(&app, worker_source).await
}

#[tauri::command]
pub async fn open_terminal_console(app: AppHandle) -> Result<()> {
    crate::services::system::SystemService::launch_terminal_console(&app)
}
