pub mod commands;
pub mod db;
pub mod error;
pub mod services;
pub mod state;

use tauri::Manager;

pub async fn bootstrap_database_and_services(
    db_path: &std::path::Path,
) -> Result<(sqlx::SqlitePool, services::runner::ScrapeRunner), error::AppError> {
    tracing::info!("Initializing SQLite database at: {:?}", db_path);
    let pool = db::init_db(db_path).await?;
    // Reprocess historical descriptions in bounded, versioned batches after
    // migrations; the repair service takes its own consistent SQLite backup.
    db::jobs::JobsRepo::reclassify_jobs(&pool).await?;
    services::system::SystemService::initialize_system_defaults(&pool).await?;
    let _ = db::sources::SourcesRepo::seed_default_sample_if_empty(&pool).await;
    if services::native_smoke::directory().is_some() {
        db::settings::SettingsRepo::set_value(&pool, "scraper.enabled", serde_json::json!(false))
            .await?;
        for source in db::sources::SourcesRepo::list(&pool).await? {
            db::sources::SourcesRepo::update(
                &pool,
                db::sources::UpdateSourceInput {
                    id: source.id,
                    name: None,
                    identifier: None,
                    config: None,
                    enabled: Some(false),
                    cadence: None,
                    cadence_cron: None,
                    retention_days: None,
                    retention_policy: None,
                    filters: None,
                },
            )
            .await?;
        }
    }
    let runner = services::runner::ScrapeRunner::new(pool.clone());
    Ok((pool, runner))
}

pub fn run() {
    let builder = tauri::Builder::default();
    // A validated debug smoke directory owns an isolated database and lifecycle.
    // It must not forward to a user's running application and exit before testing.
    let builder = if services::native_smoke::directory().is_none() {
        builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            services::tray::show_main_window(app);
        }))
    } else {
        builder
    };
    let outcome = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_page_load(|webview, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                if let Some(directory) = services::native_smoke::directory() {
                    let script = std::fs::read_to_string(directory.join("smoke.js"))
                        .expect("Read native smoke script");
                    if let Err(error) = webview.eval(&script) {
                        eprintln!("Native smoke injection failed: {error}");
                        webview.app_handle().exit(1);
                    }
                }
            }
        })
        .setup(|app| {
            let app_dir = match services::native_smoke::directory()
                .map(Ok)
                .unwrap_or_else(|| app.path().app_data_dir())
            {
                Ok(directory) => directory,
                Err(error) => {
                    services::startup::failure(app, &error.to_string());
                    return Ok(());
                }
            };
            let db_path = app_dir.join("scrape_data.db");
            let (pool, runner) =
                match tauri::async_runtime::block_on(bootstrap_database_and_services(&db_path)) {
                    Ok(services) => services,
                    Err(error) => {
                        services::startup::failure(app, &error.to_string());
                        return Ok(());
                    }
                };
            let scheduler =
                match tauri::async_runtime::block_on(services::scheduler::SchedulerService::init(
                    pool.clone(),
                    runner.clone(),
                    app.handle().clone(),
                )) {
                    Ok(scheduler) => scheduler,
                    Err(error) => {
                        tauri::async_runtime::block_on(pool.close());
                        services::startup::failure(app, &error.to_string());
                        return Ok(());
                    }
                };

            app.manage(state::AppState {
                db: pool.clone(),
                runner: runner.clone(),
                scheduler,
            });

            services::retention::RetentionService::start(
                pool.clone(),
                runner,
                app.handle().clone(),
            );

            if services::native_smoke::install(app) {
                return Ok(());
            }

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

            if let Err(error) = services::tray::setup_tray(app.handle(), &pool) {
                services::startup::failure(app, &error.to_string());
                return Ok(());
            }

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
            commands::cleanup::preview_data_cleanup,
            commands::cleanup::execute_data_cleanup,
            commands::retention::get_retention_status,
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
            commands::jobs::delete_job,
            commands::jobs::remove_job,
            commands::jobs::restore_posting,
            commands::jobs::set_posting_blocked,
            commands::jobs::list_removed_postings,
            commands::jobs::posting_saved_copies,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::sources::list_sources,
            commands::sources::get_source,
            commands::sources::create_source,
            commands::sources::update_source,
            commands::sources::delete_source,
            commands::sources::seed_sample_workflow,
            commands::scrape::run_scrape_now,
            commands::scrape::cancel_scrape,
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
            commands::scrape::validate_workflow,
            commands::scrape::preview_workflow,
            commands::scrape::save_workflow,
            commands::scrape::cancel_workflow_preview,
            commands::metrics::get_metrics,
            commands::notifications::create_notification,
            commands::notifications::list_notifications,
            commands::notifications::mark_notification_read,
            commands::notifications::mark_all_notifications_read,
            commands::notifications::clear_all_notifications,
            commands::notifications::count_unread_notifications,
            commands::notifications::check_app_update,
            commands::notifications::install_app_update,
            commands::scrape::open_terminal_console,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = outcome {
        eprintln!("STARTUP_FAILED: {error}");
        std::process::exit(1);
    }
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
        scrape_core::enrich::DescriptionInline::export().unwrap();
        scrape_core::enrich::DescriptionBlock::export().unwrap();
        scrape_core::enrich::DescriptionFormat::export().unwrap();
        scrape_core::enrich::JobDescription::export().unwrap();
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
        services::retention::RetentionStatus::export().unwrap();
        commands::scrape::TransientSourceInput::export().unwrap();
    }

    #[tokio::test]
    async fn test_cold_application_startup_from_disk_path() {
        let temp_dir =
            std::env::temp_dir().join(format!("scrape_boot_cold_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("scrape_data.db");

        let res = bootstrap_database_and_services(&db_path).await;
        assert!(
            res.is_ok(),
            "Cold startup must succeed on fresh disk path: {:?}",
            res.err()
        );

        let (pool, _runner) = res.unwrap();
        assert!(db_path.exists(), "Database file must exist on disk");

        // Verify default system settings initialized
        let settings = db::settings::SettingsRepo::get_all(&pool).await.unwrap();
        assert_eq!(settings.app_update_channel, "stable");
        assert!(settings.app_auto_update);

        // Verify sample SWE workflow seeded
        let sources = db::sources::SourcesRepo::list(&pool).await.unwrap();
        assert!(
            !sources.is_empty(),
            "Initial bootstrap must seed sample workflow"
        );

        pool.close().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_warm_application_startup_restart_on_existing_db() {
        let temp_dir =
            std::env::temp_dir().join(format!("scrape_boot_warm_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("scrape_data.db");

        // First boot
        let (pool1, _runner1) = bootstrap_database_and_services(&db_path).await.unwrap();
        db::settings::SettingsRepo::set_value(
            &pool1,
            "appearance.font_family",
            serde_json::json!("ia_writer_duo"),
        )
        .await
        .unwrap();
        let sources_count1 = db::sources::SourcesRepo::list(&pool1).await.unwrap().len();
        pool1.close().await;

        // Second boot (restart)
        let res2 = bootstrap_database_and_services(&db_path).await;
        assert!(
            res2.is_ok(),
            "Warm restart must succeed on existing database: {:?}",
            res2.err()
        );

        let (pool2, _runner2) = res2.unwrap();
        let font: Option<String> =
            db::settings::SettingsRepo::get_value(&pool2, "appearance.font_family")
                .await
                .unwrap();
        assert_eq!(
            font,
            Some("ia_writer_duo".to_string()),
            "Custom settings must persist across restarts"
        );

        let sources_count2 = db::sources::SourcesRepo::list(&pool2).await.unwrap().len();
        assert_eq!(
            sources_count1, sources_count2,
            "Sources must not be duplicated on warm restart"
        );

        pool2.close().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_app_startup_refuses_corrupted_migration_checksum() {
        let temp_dir =
            std::env::temp_dir().join(format!("scrape_boot_corrupt_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("scrape_data.db");

        let (pool1, _runner1) = bootstrap_database_and_services(&db_path).await.unwrap();
        // Simulate a corrupted checksum in _sqlx_migrations
        sqlx::query("UPDATE _sqlx_migrations SET checksum = X'12345678' WHERE version = 6")
            .execute(&pool1)
            .await
            .unwrap();
        pool1.close().await;

        let res2 = bootstrap_database_and_services(&db_path).await;
        assert!(res2.is_err(), "Startup must refuse a modified history");
        let pool2 = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(&db_path))
            .await
            .unwrap();
        let checksum: Vec<u8> =
            sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 6")
                .fetch_one(&pool2)
                .await
                .unwrap();
        assert_eq!(checksum, vec![0x12, 0x34, 0x56, 0x78]);
        pool2.close().await;
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
