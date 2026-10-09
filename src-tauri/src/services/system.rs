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
