pub mod commands;
pub mod db;
pub mod error;
pub mod services;
pub mod state;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
                    .expect("Failed to initialize system defaults")
            });

            app.manage(state::AppState { db: pool });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::jobs::list_jobs,
            commands::jobs::get_job,
            commands::jobs::update_job_status,
            commands::jobs::bulk_update_job_status,
            commands::jobs::update_job_notes,
            commands::jobs::get_job_status_events,
            commands::settings::get_settings,
            commands::settings::update_setting,
            commands::sources::list_sources,
            commands::sources::get_source,
            commands::sources::create_source,
            commands::sources::update_source,
            commands::sources::delete_source,
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
        db::jobs::Job::export().unwrap();
        db::jobs::JobFilter::export().unwrap();
        db::jobs::JobSort::export().unwrap();
        db::jobs::Page::<db::jobs::Job>::export().unwrap();
        db::jobs::JobStatusEvent::export().unwrap();
        db::jobs::ScrapedJob::export().unwrap();
        db::jobs::UpsertStats::export().unwrap();
        db::settings::Settings::export().unwrap();
        db::sources::Source::export().unwrap();
        db::sources::CreateSourceInput::export().unwrap();
        db::sources::UpdateSourceInput::export().unwrap();
        db::runs::ScrapeRun::export().unwrap();
        db::runs::TriggerType::export().unwrap();
        db::runs::RunStatus::export().unwrap();
    }
}
