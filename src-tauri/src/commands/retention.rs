use crate::error::Result;
use crate::services::retention::{RetentionService, RetentionStatus};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_retention_status(state: State<'_, AppState>) -> Result<RetentionStatus> {
    RetentionService::get_status(&state.db).await
}
