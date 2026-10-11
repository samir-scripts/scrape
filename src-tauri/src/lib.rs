pub mod commands;
pub mod db;
pub mod error;
pub mod services;
pub mod state;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            services::tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            let db_path = app_dir.join("scrape_data.db");
            tracing::info!("Initializing SQLite database at: {:?}", db_path);

            let pool = tauri::async_runtime::block_on(async {
                db::init_db(&db_path)
                    .await
                    .expect("Failed to initialize database")
            });

            tauri::async_runtime::block_on(async {
                services::system::SystemService::initialize_system_defaults(&pool)
                    .await
                    .expect("Failed to initialize system defaults");
                let _ = db::sources::SourcesRepo::seed_default_sample_if_empty(&pool).await;
            });

            let runner = services::runner::ScrapeRunner::new(pool.clone());
            let scheduler = tauri::async_runtime::block_on(async {
                services::scheduler::SchedulerService::init(
                    pool.clone(),
                    runner.clone(),
                    app.handle().clone(),
                )
                .await
                .expect("Failed to initialize scheduler service")
            });

            app.manage(state::AppState {
                db: pool.clone(),
                runner,
                scheduler,
            });

            // Prevent app quit on main window close; hide window instead
            if let Some(window) = app.get_webview_window("main") {
                let w_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_clone.hide();
                        #[cfg(target_os = "macos")]
                        let _ = w_clone
                            .app_handle()
                            .set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                });
            }

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            services::tray::setup_tray(app.handle(), &pool)
                .expect("Failed to initialize system tray");

            // Sync autostart setting on app startup
            tauri::async_runtime::block_on(async {
                use tauri_plugin_autostart::ManagerExt;
                if let Ok(Some(launch_at_login)) =
                    db::settings::SettingsRepo::get_value::<bool>(&pool, "app.launch_at_login")
                        .await
                {
                    let autolaunch = app.autolaunch();
                    if launch_at_login {
                        let _ = autolaunch.enable();
                    } else {
                        let _ = autolaunch.disable();
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::jobs::list_jobs,
            commands::jobs::get_job,
            commands::jobs::get_job_locations,
            commands::jobs::update_job_status,
            commands::jobs::bulk_update_job_status,
            commands::jobs::update_job_notes,
            commands::jobs::get_job_status_events,
            commands::jobs::reclassify_jobs,
            commands::jobs::preview_filter_impact,
            commands::jobs::list_location_options,
            commands::jobs::toggle_job_favorite,
            commands::jobs::set_job_favorite,
            commands::jobs::cleanup_expired_jobs,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::sources::list_sources,
            commands::sources::get_source,
            commands::sources::create_source,
            commands::sources::update_source,
            commands::sources::delete_source,
            commands::sources::seed_sample_workflow,
            commands::scrape::run_scrape_now,
            commands::scrape::run_source_now,
            commands::scrape::list_scrape_runs,
            commands::scrape::list_source_runs,
            commands::scrape::get_scrape_run,
            commands::scrape::get_scrape_logs,
            commands::scrape::validate_cron,
            commands::scrape::get_scheduler_status,
            commands::scrape::set_scheduler_paused,
            commands::scrape::detect_source,
            commands::scrape::test_source,
            commands::scrape::test_source_transient,
            commands::metrics::get_metrics,
            commands::notifications::list_notifications,
            commands::notifications::mark_notification_read,
            commands::notifications::mark_all_notifications_read,
            commands::notifications::clear_all_notifications,
            commands::notifications::count_unread_notifications,
            commands::notifications::check_app_update,
            commands::notifications::install_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod bindings_test {
    use super::*;
    use ts_rs::TS;

    #[test]
    fn export_bindings() {
        scrape_core::RoleType::export().unwrap();
        scrape_core::JobStatus::export().unwrap();
        scrape_core::SourceKind::export().unwrap();
        scrape_core::SourceCadence::export().unwrap();
        scrape_core::TriState::export().unwrap();
        scrape_core::WorkplaceType::export().unwrap();
        scrape_core::SalaryInterval::export().unwrap();
        scrape_core::SalarySource::export().unwrap();
        scrape_core::ClearanceLevel::export().unwrap();
        scrape_core::enrich::Salary::export().unwrap();
        scrape_core::enrich::ParsedLocation::export().unwrap();
        scrape_core::enrich::ClassifiedSponsorship::export().unwrap();
        scrape_core::enrich::ClassifiedClearance::export().unwrap();
        scrape_core::enrich::ExperienceClassification::export().unwrap();
        scrape_core::WorkerConfig::export().unwrap();
        scrape_core::WorkerSource::export().unwrap();
        scrape_core::WorkerFilters::export().unwrap();
        scrape_core::WorkerLimits::export().unwrap();
        scrape_core::WorkerEvent::export().unwrap();
        scrape_core::WorkerStats::export().unwrap();
        db::jobs::Job::export().unwrap();
        db::jobs::JobFilter::export().unwrap();
        db::jobs::JobSort::export().unwrap();
        db::jobs::Page::<db::jobs::Job>::export().unwrap();
        db::jobs::JobStatusEvent::export().unwrap();
        db::jobs::ScrapedJob::export().unwrap();
        db::jobs::UpsertStats::export().unwrap();
        db::jobs::FilterImpactPreview::export().unwrap();
        db::jobs::LocationOptions::export().unwrap();
        db::settings::Settings::export().unwrap();
        db::sources::Source::export().unwrap();
        db::sources::CreateSourceInput::export().unwrap();
        db::sources::UpdateSourceInput::export().unwrap();
        db::runs::ScrapeRun::export().unwrap();
        db::runs::TriggerType::export().unwrap();
        db::runs::RunStatus::export().unwrap();
        db::runs::LogLevel::export().unwrap();
        db::runs::ScrapeLogEntry::export().unwrap();
        db::metrics::Metrics::export().unwrap();
        db::metrics::MetricsTotals::export().unwrap();
        db::metrics::MetricsSeriesBucket::export().unwrap();
        db::metrics::RoleTypeMetric::export().unwrap();
        db::metrics::SalaryDistributionBucket::export().unwrap();
        db::metrics::MetricsStreak::export().unwrap();
        db::notifications::AppNotification::export().unwrap();
        db::notifications::NotificationKind::export().unwrap();
        db::notifications::CreateNotificationInput::export().unwrap();
        commands::notifications::UpdateCheckResult::export().unwrap();
        services::scheduler::CronValidationResult::export().unwrap();
        services::scheduler::SchedulerStatus::export().unwrap();
        commands::scrape::TransientSourceInput::export().unwrap();
    }
}
