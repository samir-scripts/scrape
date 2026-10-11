use crate::db::notifications::{AppNotification, CreateNotificationInput, NotificationKind, NotificationRepo};
use crate::db::settings::SettingsRepo;
use crate::error::Result;
use crate::state::AppState;
use scrape_core::now_iso;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn list_notifications(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<AppNotification>> {
    let lim = limit.unwrap_or(50);
    NotificationRepo::list(&state.db, lim).await
}

#[tauri::command]
pub async fn mark_notification_read(state: State<'_, AppState>, id: String) -> Result<()> {
    NotificationRepo::mark_as_read(&state.db, &id).await
}

#[tauri::command]
pub async fn mark_all_notifications_read(state: State<'_, AppState>) -> Result<()> {
    NotificationRepo::mark_all_as_read(&state.db).await
}

#[tauri::command]
pub async fn clear_all_notifications(state: State<'_, AppState>) -> Result<()> {
    NotificationRepo::clear_all(&state.db).await
}

#[tauri::command]
pub async fn count_unread_notifications(state: State<'_, AppState>) -> Result<i64> {
    NotificationRepo::count_unread(&state.db).await
}

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<UpdateCheckResult> {
    let current_version = app.package_info().version.to_string();
    let now = now_iso();

    // Persist last check timestamp
    let _ = SettingsRepo::update_setting(
        &state.db,
        "app.last_update_check",
        serde_json::Value::String(now),
    )
    .await;

    // Check with tauri-plugin-updater
    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                let latest_version = update.version.clone();
                let release_notes = update.body.clone();

                // Post in-app notification if not already notified
                let _ = NotificationRepo::create(
                    &state.db,
                    CreateNotificationInput {
                        kind: NotificationKind::Update,
                        title: format!("SCRAPE UPDATE AVAILABLE (v{})", latest_version),
                        message: release_notes
                            .clone()
                            .unwrap_or_else(|| format!("Version v{} is available for download.", latest_version)),
                        action_label: Some("INSTALL UPDATE".to_string()),
                        action_payload: Some(latest_version.clone()),
                    },
                )
                .await;

                Ok(UpdateCheckResult {
                    available: true,
                    current_version,
                    latest_version: Some(latest_version),
                    release_notes,
                    error: None,
                })
            }
            Ok(None) => Ok(UpdateCheckResult {
                available: false,
                current_version,
                latest_version: None,
                release_notes: None,
                error: None,
            }),
            Err(e) => {
                tracing::warn!("Updater check failed: {}", e);
                Ok(UpdateCheckResult {
                    available: false,
                    current_version,
                    latest_version: None,
                    release_notes: None,
                    error: Some(e.to_string()),
                })
            }
        },
        Err(e) => {
            tracing::info!("Updater not configured (dev mode or unconfigured): {}", e);
            Ok(UpdateCheckResult {
                available: false,
                current_version,
                latest_version: None,
                release_notes: None,
                error: Some("Updater not configured (development mode)".to_string()),
            })
        }
    }
}

#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<()> {
    match app.updater() {
        Ok(updater) => {
            if let Some(update) = updater.check().await.map_err(|e| crate::error::AppError::Internal(e.to_string()))? {
                update
                    .download_and_install(|_chunk, _total| {}, || {})
                    .await
                    .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

                app.restart();
            }
            Ok(())
        }
        Err(e) => Err(crate::error::AppError::Internal(format!(
            "Updater unavailable: {}",
            e
        ))),
    }
}
