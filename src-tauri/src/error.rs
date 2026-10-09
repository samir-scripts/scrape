use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Scrape run already in progress")]
    AlreadyRunning,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        let kind = match self {
            AppError::AlreadyRunning => "already_running",
            AppError::Database(_) => "database",
            AppError::Migration(_) => "migration",
            AppError::Json(_) => "json",
            AppError::NotFound(_) => "not_found",
            AppError::Validation(_) => "validation",
            AppError::Internal(_) => "internal",
        };
        state.serialize_field("kind", kind)?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
