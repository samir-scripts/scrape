use crate::{
    db::cleanup::{self, CleanupMode, CleanupPreview, CleanupRequest, CleanupResult},
    error::Result,
    state::AppState,
};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn preview_data_cleanup(
    state: State<'_, AppState>,
    mode: CleanupMode,
) -> Result<CleanupPreview> {
    cleanup::preview(&state.db, mode).await
}

#[tauri::command]
pub async fn execute_data_cleanup(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CleanupRequest,
) -> Result<CleanupResult> {
    request.validate()?;
    let maintenance = state.runner.begin_maintenance()?;
    let _exclusive = maintenance.exclusive().await?;
    // Keep scheduler mutex until its in-memory pause state reflects the commit.
    let mut scheduler = state.scheduler.lock().await;
    let backup = cleanup::backup_path(&state.db).await?;
    let mut result = cleanup::execute(&state.db, &request, &backup).await?;
    if let Err(error) = scheduler
        .sync_after_cleanup(
            result.scheduling_paused,
            result.counts.mode == CleanupMode::ResetAll,
        )
        .await
    {
        result.warnings.push(format!("Data was cleared and scheduling paused, but its next-run calendar could not be refreshed: {error}. Restart Scrape before resuming automatic runs."));
    }
    if result.counts.mode == CleanupMode::ResetAll {
        // Native harnesses must never change the user's OS login preference.
        if crate::services::native_smoke::directory().is_none() {
            use tauri_plugin_autostart::ManagerExt;
            if let Err(error) = app.autolaunch().disable() {
                result.warnings.push(format!("Data was reset, but Launch at login could not be disabled: {error}. Disable it in your system login settings."));
            }
        }
        let log = backup
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("scrape.log"));
        if let Some(log) = log {
            if let Err(error) = tokio::fs::remove_file(log).await {
                if error.kind() != std::io::ErrorKind::NotFound {
                    result.warnings.push(format!(
                        "Data was reset, but the diagnostic log could not be cleared: {error}"
                    ));
                }
            }
        }
    }

    let _ = app.emit("data://cleaned", &result);
    Ok(result)
}
