use crate::error::{AppError, Result};
use scrape_core::{now_iso, RoleType};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Settings {
    pub system_locale: String,
    pub system_timezone: String,
    pub system_week_start: String,
    pub system_time_format: String,
    pub system_date_format: String,
    pub ui_theme: String,
    pub ui_reduce_motion: bool,
    pub ui_tooltips_enabled: bool,
    pub ui_font_family: String,
    pub scraper_enabled: bool,
    pub scraper_cron: String,
    pub scraper_run_on_startup: bool,
    pub filters_include_keywords: Vec<String>,
    pub filters_exclude_keywords: Vec<String>,
    pub filters_locations: Vec<String>,
    pub filters_role_types: Vec<RoleType>,
    pub notifications_enabled: bool,
    pub app_start_hidden: bool,
    pub app_launch_at_login: bool,
    pub app_auto_update: bool,
    pub app_update_channel: String,
    pub app_last_update_check: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            system_locale: "en-US".to_string(),
            system_timezone: "UTC".to_string(),
            system_week_start: "sunday".to_string(),
            system_time_format: "12h".to_string(),
            system_date_format: "MM/DD/YYYY".to_string(),
            ui_theme: "paper".to_string(),
            ui_reduce_motion: false,
            ui_tooltips_enabled: true,
            ui_font_family: "default".to_string(),
            scraper_enabled: true,
            scraper_cron: "0 0 */6 * * *".to_string(),
            scraper_run_on_startup: false,
            filters_include_keywords: Vec::new(),
            filters_exclude_keywords: Vec::new(),
            filters_locations: Vec::new(),
            filters_role_types: Vec::new(),
            notifications_enabled: true,
            app_start_hidden: true,
            app_launch_at_login: false,
            app_auto_update: true,
            app_update_channel: "stable".to_string(),
            app_last_update_check: None,
        }
    }
}

pub struct SettingsRepo;

impl SettingsRepo {
    const KEY_LOCALE: &'static str = "system.locale";
    const KEY_TIMEZONE: &'static str = "system.timezone";
    const KEY_WEEK_START: &'static str = "system.week_start";
    const KEY_TIME_FORMAT: &'static str = "system.time_format";
    const KEY_DATE_FORMAT: &'static str = "system.date_format";
    const KEY_UI_THEME: &'static str = "ui.theme";
    const KEY_UI_REDUCE_MOTION: &'static str = "ui.reduce_motion";
    const KEY_UI_TOOLTIPS_ENABLED: &'static str = "ui.tooltips_enabled";
    const KEY_UI_FONT_FAMILY: &'static str = "ui.font_family";
    const KEY_SCRAPER_ENABLED: &'static str = "scraper.enabled";
    const KEY_SCRAPER_CRON: &'static str = "scraper.cron";
    const KEY_SCRAPER_RUN_ON_STARTUP: &'static str = "scraper.run_on_startup";
    const KEY_FILTERS_INCLUDE: &'static str = "filters.include_keywords";
    const KEY_FILTERS_EXCLUDE: &'static str = "filters.exclude_keywords";
    const KEY_FILTERS_LOCATIONS: &'static str = "filters.locations";
    const KEY_FILTERS_ROLE_TYPES: &'static str = "filters.role_types";
    const KEY_NOTIFICATIONS_ENABLED: &'static str = "notifications.enabled";
    const KEY_APP_START_HIDDEN: &'static str = "app.start_hidden";
    const KEY_APP_LAUNCH_AT_LOGIN: &'static str = "app.launch_at_login";
    const KEY_APP_AUTO_UPDATE: &'static str = "app.auto_update";
    const KEY_APP_UPDATE_CHANNEL: &'static str = "app.update_channel";
    const KEY_APP_LAST_UPDATE_CHECK: &'static str = "app.last_update_check";

    pub async fn ensure_defaults(pool: &SqlitePool, defaults: &Settings) -> Result<()> {
        let entries = vec![
            (
                Self::KEY_LOCALE,
                serde_json::to_string(&defaults.system_locale)?,
            ),
            (
                Self::KEY_TIMEZONE,
                serde_json::to_string(&defaults.system_timezone)?,
            ),
            (
                Self::KEY_WEEK_START,
                serde_json::to_string(&defaults.system_week_start)?,
            ),
            (
                Self::KEY_TIME_FORMAT,
                serde_json::to_string(&defaults.system_time_format)?,
            ),
            (
                Self::KEY_DATE_FORMAT,
                serde_json::to_string(&defaults.system_date_format)?,
            ),
            (
                Self::KEY_UI_THEME,
                serde_json::to_string(&defaults.ui_theme)?,
            ),
            (
                Self::KEY_UI_REDUCE_MOTION,
                serde_json::to_string(&defaults.ui_reduce_motion)?,
            ),
            (
                Self::KEY_UI_TOOLTIPS_ENABLED,
                serde_json::to_string(&defaults.ui_tooltips_enabled)?,
            ),
            (
                Self::KEY_UI_FONT_FAMILY,
                serde_json::to_string(&defaults.ui_font_family)?,
            ),
            (
                Self::KEY_SCRAPER_ENABLED,
                serde_json::to_string(&defaults.scraper_enabled)?,
            ),
            (
                Self::KEY_SCRAPER_CRON,
                serde_json::to_string(&defaults.scraper_cron)?,
            ),
            (
                Self::KEY_SCRAPER_RUN_ON_STARTUP,
                serde_json::to_string(&defaults.scraper_run_on_startup)?,
            ),
            (
                Self::KEY_FILTERS_INCLUDE,
                serde_json::to_string(&defaults.filters_include_keywords)?,
            ),
            (
                Self::KEY_FILTERS_EXCLUDE,
                serde_json::to_string(&defaults.filters_exclude_keywords)?,
            ),
            (
                Self::KEY_FILTERS_LOCATIONS,
                serde_json::to_string(&defaults.filters_locations)?,
            ),
            (
                Self::KEY_FILTERS_ROLE_TYPES,
                serde_json::to_string(&defaults.filters_role_types)?,
            ),
            (
                Self::KEY_NOTIFICATIONS_ENABLED,
                serde_json::to_string(&defaults.notifications_enabled)?,
            ),
            (
                Self::KEY_APP_START_HIDDEN,
                serde_json::to_string(&defaults.app_start_hidden)?,
            ),
            (
                Self::KEY_APP_LAUNCH_AT_LOGIN,
                serde_json::to_string(&defaults.app_launch_at_login)?,
            ),
            (
                Self::KEY_APP_AUTO_UPDATE,
                serde_json::to_string(&defaults.app_auto_update)?,
            ),
            (
                Self::KEY_APP_UPDATE_CHANNEL,
                serde_json::to_string(&defaults.app_update_channel)?,
            ),
            (
                Self::KEY_APP_LAST_UPDATE_CHECK,
                serde_json::to_string(&defaults.app_last_update_check)?,
            ),
        ];

        let now = now_iso();
        for (key, val) in entries {
            sqlx::query(
                "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?, ?, ?)",
            )
            .bind(key)
            .bind(val)
            .bind(&now)
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_value<T: for<'de> Deserialize<'de>>(
        pool: &SqlitePool,
        key: &str,
    ) -> Result<Option<T>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT value FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(r) => {
                let val_str: String = r.get("value");
                let parsed: T = serde_json::from_str(&val_str)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn set_value(pool: &SqlitePool, key: &str, value: serde_json::Value) -> Result<()> {
        let serialized = serde_json::to_string(&value)?;
        let now = now_iso();

        sqlx::query(
            r#"
            INSERT INTO app_settings (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3
            "#,
        )
        .bind(key)
        .bind(serialized)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_all(pool: &SqlitePool) -> Result<Settings> {
        use sqlx::Row;
        let rows = sqlx::query("SELECT key, value FROM app_settings")
            .fetch_all(pool)
            .await?;

        let mut settings = Settings::default();

        for r in rows {
            let key: String = r.get("key");
            let val_str: String = r.get("value");

            match key.as_str() {
                Self::KEY_LOCALE => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.system_locale = v;
                    }
                }
                Self::KEY_TIMEZONE => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.system_timezone = v;
                    }
                }
                Self::KEY_WEEK_START => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.system_week_start = v;
                    }
                }
                Self::KEY_TIME_FORMAT => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.system_time_format = v;
                    }
                }
                Self::KEY_DATE_FORMAT => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.system_date_format = v;
                    }
                }
                Self::KEY_UI_THEME => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.ui_theme = v;
                    }
                }
                Self::KEY_UI_REDUCE_MOTION => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.ui_reduce_motion = v;
                    }
                }
                Self::KEY_UI_TOOLTIPS_ENABLED => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.ui_tooltips_enabled = v;
                    }
                }
                Self::KEY_UI_FONT_FAMILY => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.ui_font_family = v;
                    }
                }
                Self::KEY_SCRAPER_ENABLED => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.scraper_enabled = v;
                    }
                }
                Self::KEY_SCRAPER_CRON => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.scraper_cron = v;
                    }
                }
                Self::KEY_SCRAPER_RUN_ON_STARTUP => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.scraper_run_on_startup = v;
                    }
                }
                Self::KEY_FILTERS_INCLUDE => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.filters_include_keywords = v;
                    }
                }
                Self::KEY_FILTERS_EXCLUDE => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.filters_exclude_keywords = v;
                    }
                }
                Self::KEY_FILTERS_LOCATIONS => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.filters_locations = v;
                    }
                }
                Self::KEY_FILTERS_ROLE_TYPES => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.filters_role_types = v;
                    }
                }
                Self::KEY_NOTIFICATIONS_ENABLED => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.notifications_enabled = v;
                    }
                }
                Self::KEY_APP_START_HIDDEN => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.app_start_hidden = v;
                    }
                }
                Self::KEY_APP_LAUNCH_AT_LOGIN => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.app_launch_at_login = v;
                    }
                }
                Self::KEY_APP_AUTO_UPDATE => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.app_auto_update = v;
                    }
                }
                Self::KEY_APP_UPDATE_CHANNEL => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.app_update_channel = v;
                    }
                }
                Self::KEY_APP_LAST_UPDATE_CHECK => {
                    if let Ok(v) = serde_json::from_str(&val_str) {
                        settings.app_last_update_check = v;
                    }
                }
                _ => {}
            }
        }

        Ok(settings)
    }

    pub async fn update_setting(
        pool: &SqlitePool,
        key: &str,
        value: serde_json::Value,
    ) -> Result<Settings> {
        let valid_keys = [
            Self::KEY_LOCALE,
            Self::KEY_TIMEZONE,
            Self::KEY_WEEK_START,
            Self::KEY_TIME_FORMAT,
            Self::KEY_DATE_FORMAT,
            Self::KEY_UI_THEME,
            Self::KEY_UI_REDUCE_MOTION,
            Self::KEY_UI_TOOLTIPS_ENABLED,
            Self::KEY_UI_FONT_FAMILY,
            Self::KEY_SCRAPER_ENABLED,
            Self::KEY_SCRAPER_CRON,
            Self::KEY_SCRAPER_RUN_ON_STARTUP,
            Self::KEY_FILTERS_INCLUDE,
            Self::KEY_FILTERS_EXCLUDE,
            Self::KEY_FILTERS_LOCATIONS,
            Self::KEY_FILTERS_ROLE_TYPES,
            Self::KEY_NOTIFICATIONS_ENABLED,
            Self::KEY_APP_START_HIDDEN,
            Self::KEY_APP_LAUNCH_AT_LOGIN,
            Self::KEY_APP_AUTO_UPDATE,
            Self::KEY_APP_UPDATE_CHANNEL,
            Self::KEY_APP_LAST_UPDATE_CHECK,
        ];

        if !valid_keys.contains(&key) {
            return Err(AppError::Validation(format!(
                "Unknown setting key: {}",
                key
            )));
        }

        Self::set_value(pool, key, value).await?;
        Self::get_all(pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[tokio::test]
    async fn test_ensure_defaults_and_get_all() {
        let pool = init_memory_db().await.unwrap();
        let defaults = Settings {
            system_locale: "fr-FR".to_string(),
            scraper_cron: "0 0 12 * * *".to_string(),
            ..Default::default()
        };

        SettingsRepo::ensure_defaults(&pool, &defaults)
            .await
            .unwrap();

        let fetched = SettingsRepo::get_all(&pool).await.unwrap();
        assert_eq!(fetched.system_locale, "fr-FR");
        assert_eq!(fetched.scraper_cron, "0 0 12 * * *");
        assert_eq!(fetched.ui_theme, "paper");
        assert_eq!(fetched.ui_tooltips_enabled, true);
        assert_eq!(fetched.ui_font_family, "default");
        assert_eq!(fetched.system_time_format, "12h");
        assert_eq!(fetched.system_date_format, "MM/DD/YYYY");

        // Ensure subsequent defaults call does NOT overwrite existing values
        let second_defaults = Settings {
            system_locale: "de-DE".to_string(),
            system_time_format: "24h".to_string(),
            ..Default::default()
        };
        SettingsRepo::ensure_defaults(&pool, &second_defaults)
            .await
            .unwrap();

        let fetched_again = SettingsRepo::get_all(&pool).await.unwrap();
        assert_eq!(fetched_again.system_locale, "fr-FR");
        assert_eq!(fetched_again.system_time_format, "12h");
    }

    #[tokio::test]
    async fn test_update_setting() {
        let pool = init_memory_db().await.unwrap();
        SettingsRepo::ensure_defaults(&pool, &Settings::default())
            .await
            .unwrap();

        let updated =
            SettingsRepo::update_setting(&pool, "ui.theme", serde_json::json!("obsidian"))
                .await
                .unwrap();

        assert_eq!(updated.ui_theme, "obsidian");

        let updated_tooltips =
            SettingsRepo::update_setting(&pool, "ui.tooltips_enabled", serde_json::json!(false))
                .await
                .unwrap();
        assert_eq!(updated_tooltips.ui_tooltips_enabled, false);

        let updated_font =
            SettingsRepo::update_setting(&pool, "ui.font_family", serde_json::json!("jetbrains"))
                .await
                .unwrap();
        assert_eq!(updated_font.ui_font_family, "jetbrains");

        let updated_time =
            SettingsRepo::update_setting(&pool, "system.time_format", serde_json::json!("24h"))
                .await
                .unwrap();
        assert_eq!(updated_time.system_time_format, "24h");

        let updated_date = SettingsRepo::update_setting(
            &pool,
            "system.date_format",
            serde_json::json!("DD/MM/YYYY"),
        )
        .await
        .unwrap();
        assert_eq!(updated_date.system_date_format, "DD/MM/YYYY");

        let err =
            SettingsRepo::update_setting(&pool, "invalid.key", serde_json::json!("foo")).await;

        assert!(err.is_err());
    }
}
