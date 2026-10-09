use crate::db::runs::TriggerType;
use crate::error::{AppError, Result};
use crate::state::AppState;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub fn setup_tray(app: &AppHandle) -> Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open", "OPEN SCRAPE")
        .separator()
        .text("run_now", "RUN SCRAPE NOW")
        .text("toggle_schedule", "PAUSE / RESUME SCHEDULE")
        .item(
            &MenuItemBuilder::with_id("last_run", "LAST RUN: NEVER")
                .enabled(false)
                .build(app)
                .map_err(|e| AppError::Internal(e.to_string()))?,
        )
        .separator()
        .text("quit", "QUIT")
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
                    let _ = sched.set_paused(status.enabled).await;
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
