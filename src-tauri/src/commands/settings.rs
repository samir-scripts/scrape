use crate::db::settings::{Settings, SettingsRepo};
use crate::error::Result;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    SettingsRepo::get_all(&state.db).await
}

#[tauri::command]
pub async fn update_setting(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<Settings> {
    let settings = SettingsRepo::update_setting(&state.db, &key, value.clone()).await?;

    if key == "scraper.cron" {
        if let Some(cron_str) = value.as_str() {
            let mut sched = state.scheduler.lock().await;
            if let Err(e) = sched.reschedule(cron_str).await {
                tracing::warn!("Failed to reschedule cron to '{}': {}", cron_str, e);
            }
        }
    } else if key == "scraper.enabled" {
        if let Some(enabled) = value.as_bool() {
            let mut sched = state.scheduler.lock().await;
            let _ = sched.set_paused(!enabled).await;
        }
    }

    let _ = app.emit("settings://changed", &settings);
    Ok(settings)
}
