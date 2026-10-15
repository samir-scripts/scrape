use crate::db::jobs::{
    FilterImpactPreview, Job, JobFilter, JobStatusEvent, JobsRepo, LocationOptions, Page,
};
use crate::error::{AppError, Result};
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
    let job = JobsRepo::set_favorite(&state.db, &id, favorited).await?;
    let _ = app.emit("jobs://changed", &job);
    Ok(job)
}

#[tauri::command]
pub async fn cleanup_expired_jobs(
    app: AppHandle,
    state: State<'_, AppState>,
    source_id: Option<String>,
) -> Result<u64> {
    let _permit = state.runner.try_retention_permit().ok_or_else(|| {
        AppError::Validation(
            "Retention was deferred because another write is active. Try again shortly.".into(),
        )
    })?;
    let now = chrono::Utc::now();
    let outcome = if let Some(source_id) = source_id {
        crate::services::retention::RetentionService::prune_source_ids(
            &state.db,
            std::iter::once(source_id),
            now,
        )
        .await
    } else {
        crate::services::retention::RetentionService::sweep(&state.db, now).await
    };
    let deleted = outcome.deleted_records;
    let status = crate::services::retention::RetentionService::record(&state.db, &outcome).await;
    drop(_permit);
    match status {
        Ok(status) => crate::services::retention::RetentionService::notify_for_command(
            &app, &status, &outcome,
        ),
        Err(error) => {
            return Err(AppError::Validation(format!(
            "Retention removed {deleted} records, but its outcome could not be recorded: {error}"
        )))
        }
    }
    if outcome.failures.is_empty() {
        Ok(deleted)
    } else {
        Err(AppError::Validation(format!(
            "Retention completed with errors: {}",
            outcome.failures.join("; ")
        )))
    }
}

#[tauri::command]
pub async fn delete_job(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<bool> {
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
    let _permit = state.runner.write_permit()?;
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
