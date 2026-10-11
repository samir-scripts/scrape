use crate::db::settings::{Settings, SettingsRepo};
use crate::error::Result;
use sqlx::SqlitePool;

pub struct SystemService;

impl SystemService {
    pub fn detect_default_settings() -> Settings {
        let mut settings = Settings::default();

        if let Some(locale) = sys_locale::get_locale() {
            settings.system_locale = locale.clone();
            settings.system_week_start = Self::infer_week_start(&locale).to_string();
        }

        if let Ok(tz) = iana_time_zone::get_timezone() {
            settings.system_timezone = tz;
        }

        settings
    }

    pub fn infer_week_start(locale: &str) -> &'static str {
        let upper = locale.to_uppercase();
        if upper.ends_with("-US")
            || upper.ends_with("-CA")
            || upper.ends_with("-MX")
            || upper.ends_with("-JP")
            || upper.ends_with("-IL")
            || upper.ends_with("_US")
            || upper.ends_with("_CA")
            || upper.ends_with("_MX")
            || upper.ends_with("_JP")
            || upper.ends_with("_IL")
        {
            "sunday"
        } else {
            "monday"
        }
    }

    pub async fn initialize_system_defaults(pool: &SqlitePool) -> Result<Settings> {
        let defaults = Self::detect_default_settings();
        SettingsRepo::ensure_defaults(pool, &defaults).await?;
        SettingsRepo::get_all(pool).await
    }

    pub fn get_log_file_path(app: &tauri::AppHandle) -> std::path::PathBuf {
        use tauri::Manager;
        let app_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let _ = std::fs::create_dir_all(&app_dir);
        app_dir.join("scrape.log")
    }

    pub fn append_log(app: &tauri::AppHandle, level: &str, source_id: Option<&str>, message: &str) {
        use std::io::Write;
        let path = Self::get_log_file_path(app);
        let now = scrape_core::now_iso();
        let src_tag = source_id.map(|s| format!(" [{}]", s)).unwrap_or_default();
        let line = format!("[{}] [{:>5}]{} {}\n", now, level.to_uppercase(), src_tag, message);
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn launch_terminal_console(app: &tauri::AppHandle) -> Result<()> {
        use tauri::Manager;
        let log_path = Self::get_log_file_path(app);
        if !log_path.exists() {
            let _ = std::fs::write(&log_path, b"");
        }

        let app_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let script_path = app_dir.join("open_console.command");

        let log_path_str = log_path.to_string_lossy();
        let script_content = format!(
            r#"#!/usr/bin/env bash
clear
printf "\033[1;37m======================================================================\033[0m\n"
printf "\033[1;32m  SCRAPE // REAL-TIME SYSTEM TELEMETRY & EXECUTION CONSOLE\033[0m\n"
printf "  LOG PATH: %s\n" "{}"
printf "  PRESS CTRL+C TO CLOSE STREAM\n"
printf "\033[1;37m======================================================================\033[0m\n\n"
tail -f -n 100 "{}"
"#,
            log_path_str, log_path_str
        );

        std::fs::write(&script_path, script_content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg(&script_path)
                .spawn()
                .map_err(|e| crate::error::AppError::Internal(format!("Failed to launch Terminal.app: {}", e)))?;
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("x-terminal-emulator")
                .arg("-e")
                .arg(&script_path)
                .spawn()
                .or_else(|_| {
                    std::process::Command::new("xterm")
                        .arg("-e")
                        .arg(&script_path)
                        .spawn()
                });
        }

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "powershell", "-NoExit", "-Command", &format!("Get-Content -Path '{}' -Wait -Tail 100", log_path_str)])
                .spawn();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[test]
    fn test_infer_week_start() {
        assert_eq!(SystemService::infer_week_start("en-US"), "sunday");
        assert_eq!(SystemService::infer_week_start("en_CA"), "sunday");
        assert_eq!(SystemService::infer_week_start("en-GB"), "monday");
        assert_eq!(SystemService::infer_week_start("fr-FR"), "monday");
        assert_eq!(SystemService::infer_week_start("de-DE"), "monday");
    }

    #[tokio::test]
    async fn test_initialize_system_defaults() {
        let pool = init_memory_db().await.unwrap();
        let settings = SystemService::initialize_system_defaults(&pool)
            .await
            .unwrap();
        assert!(!settings.system_locale.is_empty());
        assert!(!settings.system_timezone.is_empty());
    }
}
