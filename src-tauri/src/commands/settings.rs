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
    let settings = SettingsRepo::update_setting(&state.db, &key, value).await?;
    let _ = app.emit("settings://changed", &settings);
    Ok(settings)
}
