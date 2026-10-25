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
pub async fn search_role_titles(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<crate::db::role_titles::RoleTitleSuggestion>> {
    crate::db::role_titles::search_role_titles(&state.db, &query).await
}

#[tauri::command]
pub async fn search_companies(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<crate::db::companies::CompanySuggestion>> {
    crate::db::companies::search_companies(&state.db, &query).await
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

#[tauri::command]
pub async fn get_company_logo_asset(
    state: State<'_, AppState>,
    asset_reference: String,
) -> Result<String> {
    crate::db::branding::asset(&state.db, &asset_reference).await
}

static BRANDING_LOOKUPS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

#[tauri::command]
pub async fn ensure_company_logo_for_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<scrape_core::branding::CompanyLogo>> {
    use sqlx::Row;

    let Some((Some(source_id), company, url)) =
        crate::db::branding::job_identity(&state.db, &job_id).await?
    else {
        return Ok(None);
    };
    let source_id = source_id.trim();
    let company = company.trim();
    if source_id.is_empty() || company.is_empty() || !scrape_core::branding::http_url(&url) {
        return Ok(None);
    }
    let relationship = scrape_core::branding::relationship_key(source_id, company);
    if let Some(row) = sqlx::query(
        "SELECT logo_json,asset_data FROM employer_branding WHERE key=? AND asset_reference IS NOT NULL",
    )
    .bind(&relationship)
    .fetch_optional(&state.db)
    .await?
    {
        let mut existing: scrape_core::branding::CompanyLogo =
            serde_json::from_str(&row.get::<String, _>("logo_json"))?;
        existing.asset = row.get("asset_data");
        if existing.status == scrape_core::branding::LogoStatus::Verified
            && !existing.evidence.as_ref().is_some_and(
                scrape_core::branding::is_legacy_ashby_platform_favicon,
            )
            && existing.validate().is_ok()
        {
            return Ok(Some(existing));
        }
    }

    let expected_identity = (Some(source_id.to_owned()), company.to_owned(), url.clone());
    if crate::db::branding::job_identity(&state.db, &job_id)
        .await?
        .as_ref()
        != Some(&expected_identity)
    {
        return Ok(None);
    }
    if let Some(curated) = crate::db::branding::resolve_curated_logo(company) {
        crate::db::branding::record(&state.db, source_id, company, &curated).await?;
        return Ok(Some(curated));
    }

    use tauri_plugin_shell::process::CommandEvent;
    let _permit = BRANDING_LOOKUPS
        .acquire()
        .await
        .map_err(|_| AppError::Internal("Company logo lookup service is unavailable".into()))?;
    let proxy = crate::services::http_proxy::configuration().await?;
    let (mut receiver, child) = crate::services::process::worker_command(&app)?
        .args(["--branding-proxy-stdin", "--branding", company, &url])
        .spawn()
        .map_err(|_| AppError::Internal("Failed to execute branding worker".into()))?;
    let mut child = crate::services::process::ChildGuard::new(child);
    let mut input = serde_json::to_vec(&proxy)?;
    input.push(b'\n');
    child.write(&input).map_err(|_| {
        AppError::Internal("Failed to send branding transport configuration".into())
    })?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut stdout = Vec::new();
    let mut exit_code = None;
    loop {
        match tokio::time::timeout_at(deadline, receiver.recv()).await {
            Ok(Some(CommandEvent::Stdout(bytes))) => {
                if stdout.len().saturating_add(bytes.len()) > 2 * 1024 * 1024 {
                    return Ok(None);
                }
                stdout.extend_from_slice(&bytes);
            }
            Ok(Some(CommandEvent::Terminated(status))) => {
                child.disarm();
                exit_code = status.code;
                break;
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
    if exit_code != Some(0) {
        return Ok(None);
    }
    let Ok(logo) = serde_json::from_slice::<scrape_core::branding::CompanyLogo>(&stdout) else {
        return Ok(None);
    };
    if logo.status != scrape_core::branding::LogoStatus::Verified || logo.validate().is_err() {
        return Ok(None);
    }
    if crate::db::branding::job_identity(&state.db, &job_id)
        .await?
        .as_ref()
        != Some(&expected_identity)
    {
        return Ok(None);
    }
    crate::db::branding::record(&state.db, source_id, company, &logo).await?;
    let _ = app.emit("jobs://branding-updated", ());
    Ok(Some(logo))
}
