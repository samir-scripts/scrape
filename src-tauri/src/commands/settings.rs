use crate::db::settings::{Settings, SettingsRepo};
use crate::error::Result;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    SettingsRepo::get_all(&state.db).await
}
#[tauri::command]
pub async fn get_adzuna_credential_status(
) -> Result<crate::services::provider_credentials::ProviderCredentialStatus> {
    crate::services::provider_credentials::status().await
}

#[tauri::command]
pub async fn set_adzuna_credentials(
    app_id: String,
    app_key: String,
) -> Result<crate::services::provider_credentials::ProviderCredentialStatus> {
    crate::services::provider_credentials::set(app_id, app_key).await
}

#[tauri::command]
pub async fn clear_adzuna_credentials(
) -> Result<crate::services::provider_credentials::ProviderCredentialStatus> {
    crate::services::provider_credentials::clear().await
}

#[tauri::command]
pub async fn get_http_proxy_status() -> Result<crate::services::http_proxy::HttpProxyStatus> {
    crate::services::http_proxy::status().await
}

#[tauri::command]
pub async fn set_http_proxy(
    server: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<crate::services::http_proxy::HttpProxyStatus> {
    crate::services::http_proxy::set(server, username, password).await
}

#[tauri::command]
pub async fn clear_http_proxy() -> Result<crate::services::http_proxy::HttpProxyStatus> {
    crate::services::http_proxy::clear().await
}

#[tauri::command]
pub async fn update_setting(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    value: serde_json::Value,
) -> Result<Settings> {
    let _permit = state.runner.write_permit()?;
    let settings = SettingsRepo::update_setting(&state.db, &key, value.clone()).await?;

    if key == "scraper.cron" {
        if let Some(cron_str) = value.as_str() {
            let mut sched = state.scheduler.lock().await;
            sched.reschedule(cron_str).await?;
        }
    } else if key == "scraper.enabled" {
        if let Some(enabled) = value.as_bool() {
            let mut sched = state.scheduler.lock().await;
            let _ = sched.set_paused(!enabled).await;
        }
    } else if key == "system.timezone" {
        state
            .scheduler
            .lock()
            .await
            .reschedule(&settings.scraper_cron)
            .await?;
        for source in crate::db::sources::SourcesRepo::list(&state.db).await? {
            let next = crate::services::scheduler::calculate_next_run_for_source(
                &source,
                &settings.system_timezone,
            );
            sqlx::query("UPDATE sources SET next_run_at = ? WHERE id = ?")
                .bind(next)
                .bind(source.id)
                .execute(&state.db)
                .await?;
        }
    } else if key == "app.launch_at_login" {
        use tauri_plugin_autostart::ManagerExt;
        if let Some(launch) = value.as_bool() {
            let autolaunch = app.autolaunch();
            if launch {
                if let Err(e) = autolaunch.enable() {
                    tracing::warn!("Failed to enable autostart: {}", e);
                }
            } else {
                if let Err(e) = autolaunch.disable() {
                    tracing::warn!("Failed to disable autostart: {}", e);
                }
            }
        }
    }

    let _ = app.emit("settings://changed", &settings);
    Ok(settings)
}
