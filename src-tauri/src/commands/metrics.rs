use crate::db::metrics::{Metrics, MetricsRepo};
use crate::error::Result;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_metrics(
    state: State<'_, AppState>,
    granularity: Option<String>,
    periods: Option<u32>,
) -> Result<Metrics> {
    let gran = granularity.as_deref().unwrap_or("week");
    let per = periods.unwrap_or(12);
    MetricsRepo::get_metrics(&state.db, gran, per).await
}
