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

#[tauri::command]
pub async fn toggle_job_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Job> {
    let job = JobsRepo::toggle_favorite(&state.db, &id).await?;
    let _ = app.emit("jobs://changed", &job);
    Ok(job)
}

#[tauri::command]
pub async fn set_job_favorite(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    favorited: bool,
) -> Result<Job> {
    let job = JobsRepo::set_favorite(&state.db, &id, favorited).await?;
    let _ = app.emit("jobs://changed", &job);
    Ok(job)
}

#[tauri::command]
pub async fn cleanup_expired_jobs(
    state: State<'_, AppState>,
    source_id: Option<String>,
) -> Result<u64> {
    if let Some(src_id) = source_id {
        use crate::db::sources::SourcesRepo;
        let source = SourcesRepo::get(&state.db, &src_id).await?;
        JobsRepo::prune_expired_by_source(
            &state.db,
            &source.id,
            source.retention_days,
            &source.retention_policy,
        )
        .await
    } else {
        JobsRepo::prune_all_expired(&state.db).await
    }
}

#[tauri::command]
pub async fn delete_job(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<bool> {
    let deleted = JobsRepo::delete(&state.db, &id).await?;
    if deleted {
        let _ = app.emit("jobs://changed", ());
    }
    Ok(deleted)
}

#[tauri::command]
pub async fn remove_job(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    block: bool,
) -> Result<crate::db::postings::RemovalResult> {
    let result = crate::db::postings::remove(&state.db, &id, block).await?;
    let _ = app.emit("jobs://changed", ());
    Ok(result)
}

#[tauri::command]
pub async fn restore_posting(
    app: AppHandle,
    state: State<'_, AppState>,
    identity: String,
) -> Result<()> {
    crate::db::postings::restore(&state.db, &identity).await?;
    let _ = app.emit("jobs://changed", ());
    Ok(())
}

#[tauri::command]
pub async fn set_posting_blocked(
    app: AppHandle,
    state: State<'_, AppState>,
    identity: String,
    blocked: bool,
) -> Result<()> {
    crate::db::postings::set_blocked(&state.db, &identity, blocked).await?;
    let _ = app.emit("jobs://changed", ());
    Ok(())
}

#[tauri::command]
pub async fn list_removed_postings(
    state: State<'_, AppState>,
    search: String,
    view: String,
    offset: i64,
    limit: i64,
) -> Result<Page<crate::db::postings::RemovedPosting>> {
    crate::db::postings::list(&state.db, &search, &view, offset, limit).await
}

#[tauri::command]
pub async fn posting_saved_copies(state: State<'_, AppState>, id: String) -> Result<i64> {
    crate::db::postings::backfill(&state.db).await?;
    Ok(sqlx::query_scalar("SELECT count(*) FROM jobs WHERE removed_at IS NULL AND posting_identity = (SELECT posting_identity FROM jobs WHERE id = ?)")
        .bind(id).fetch_one(&state.db).await?)
}
