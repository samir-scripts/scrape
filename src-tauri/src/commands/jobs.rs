use crate::db::jobs::{
    FilterImpactPreview, Job, JobFilter, JobStatusEvent, JobsRepo, LocationOptions, Page,
};
use crate::error::Result;
use crate::state::AppState;
use scrape_core::enrich::ParsedLocation;
use scrape_core::JobStatus;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn list_jobs(state: State<'_, AppState>, filter: Option<JobFilter>) -> Result<Page<Job>> {
    JobsRepo::list(&state.db, filter.unwrap_or_default()).await
}

#[tauri::command]
pub async fn get_job(state: State<'_, AppState>, id: String) -> Result<Job> {
    JobsRepo::get(&state.db, &id).await
}

#[tauri::command]
pub async fn get_job_locations(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Vec<ParsedLocation>> {
    JobsRepo::get_locations(&state.db, &job_id).await
}

#[tauri::command]
pub async fn update_job_status(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    status: JobStatus,
) -> Result<Job> {
    let job = JobsRepo::update_status(&state.db, &id, status).await?;
    let _ = app.emit("jobs://changed", &job);
    Ok(job)
}

#[tauri::command]
pub async fn bulk_update_job_status(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
    status: JobStatus,
) -> Result<u64> {
    let count = JobsRepo::bulk_update_status(&state.db, &ids, status).await?;
    let _ = app.emit("jobs://changed", ());
    Ok(count)
}

#[tauri::command]
pub async fn update_job_notes(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    notes: Option<String>,
) -> Result<Job> {
    let job = JobsRepo::update_notes(&state.db, &id, notes.as_deref()).await?;
    let _ = app.emit("jobs://changed", &job);
    Ok(job)
}

#[tauri::command]
pub async fn get_job_status_events(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Vec<JobStatusEvent>> {
    JobsRepo::get_status_events(&state.db, &job_id).await
}

#[tauri::command]
pub async fn reclassify_jobs(state: State<'_, AppState>) -> Result<i64> {
    JobsRepo::reclassify_jobs(&state.db).await
}

#[tauri::command]
pub async fn preview_filter_impact(
    state: State<'_, AppState>,
    spec: JobFilter,
) -> Result<FilterImpactPreview> {
    JobsRepo::preview_filter_impact(&state.db, spec).await
}

#[tauri::command]
pub async fn list_location_options(state: State<'_, AppState>) -> Result<LocationOptions> {
    JobsRepo::list_location_options(&state.db).await
}
