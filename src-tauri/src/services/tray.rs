use crate::db::runs::{RunsRepo, TriggerType};
use crate::db::settings::SettingsRepo;
use crate::error::{AppError, Result};
use crate::state::AppState;
use sqlx::SqlitePool;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub struct TrayState {
    pub last_run_item: MenuItem<tauri::Wry>,
}

pub fn update_last_run(app: &AppHandle, time_str: &str, count: i64) {
    if let Some(state) = app.try_state::<TrayState>() {
        let text = format!("LAST RUN: {} · {} NEW", time_str, count);
        let _ = state.last_run_item.set_text(text);
    }
}

pub fn setup_tray(app: &AppHandle, pool: &SqlitePool) -> Result<()> {
    let initial_label = match tauri::async_runtime::block_on(RunsRepo::recent(pool, 1)) {
        Ok(recent) if !recent.is_empty() && recent[0].finished_at.is_some() => {
            let last_run = &recent[0];
            let tz_name: String = tauri::async_runtime::block_on(
                SettingsRepo::get_value::<String>(pool, "system.timezone"),
            )
            .ok()
            .flatten()
            .unwrap_or_else(|| "UTC".to_string());
            let tz: chrono_tz::Tz = tz_name.parse().unwrap_or(chrono_tz::UTC);
            let time_str = match chrono::DateTime::parse_from_rfc3339(
                last_run.finished_at.as_ref().unwrap(),
            ) {
                Ok(dt) => dt.with_timezone(&tz).format("%H:%M").to_string(),
                Err(_) => "UNKNOWN".to_string(),
            };
            format!("LAST RUN: {} · {} NEW", time_str, last_run.jobs_new)
        }
        _ => "LAST RUN: NEVER".to_string(),
    };

    let last_run_item = MenuItemBuilder::with_id("last_run", initial_label)
        .enabled(false)
        .build(app)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let menu = MenuBuilder::new(app)
        .text("open", "OPEN SCRAPE")
        .separator()
        .text("run_now", "RUN SCRAPE NOW")
        .text("toggle_schedule", "PAUSE / RESUME SCHEDULE")
        .item(&last_run_item)
        .separator()
        .text("quit", "QUIT")
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    app.manage(TrayState { last_run_item });

    let icon = app
        .default_window_icon()
        .ok_or_else(|| AppError::Internal("No default window icon found".to_string()))?
        .clone();

    let mut builder = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Scrape — Local Job Tracker");

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                show_main_window(app);
            }
            "run_now" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let _ = state.runner.trigger(&app_handle, TriggerType::Manual).await;
                });
            }
            "toggle_schedule" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let mut sched = state.scheduler.lock().await;
                    let status = sched.get_status().await.unwrap_or(
                        crate::services::scheduler::SchedulerStatus {
                            enabled: false,
                            next_run_at: None,
                            running: false,
                        },
                    );
                    let new_enabled = !status.enabled;
                    let _ = sched.set_paused(!new_enabled).await;
                    if let Ok(settings) = SettingsRepo::update_setting(
                        &state.db,
                        "scraper.enabled",
                        serde_json::json!(new_enabled),
                    )
                    .await
                    {
                        let _ = app_handle.emit("settings://changed", &settings);
                    }
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_main_window(app);
            }
        })
        .build(app)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            let _ = window.hide();
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        } else {
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}
