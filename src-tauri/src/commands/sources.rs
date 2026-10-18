use crate::db::sources::{CreateSourceInput, Source, SourcesRepo, UpdateSourceInput};
use crate::error::{AppError, Result};
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

// Legacy single-source CRUD must not bypass atomic workflow ownership persistence.
fn require_single_source_crud(config: Option<&serde_json::Value>) -> Result<()> {
    if config
        .and_then(|c| c.pointer("/__workflow_spec/sources"))
        .and_then(|s| s.as_array())
        .is_some_and(|s| s.len() > 1)
    {
        return Err(AppError::Validation("Use Save workflow to create or edit multiple sources atomically; legacy source CRUD supports one source.".into()));
    }
    Ok(())
}

#[tauri::command]
pub async fn list_sources(state: State<'_, AppState>) -> Result<Vec<Source>> {
    SourcesRepo::list(&state.db).await
}

#[tauri::command]
pub async fn get_source(state: State<'_, AppState>, id: String) -> Result<Source> {
    SourcesRepo::get(&state.db, &id).await
}

#[tauri::command]
pub async fn create_source(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CreateSourceInput,
) -> Result<Source> {
    let _permit = state.runner.write_permit()?;
    require_single_source_crud(input.config.as_ref())?;
    let source = SourcesRepo::create(&state.db, input).await?;
    let _ = app.emit("sources://changed", &source);
    Ok(source)
}

#[tauri::command]
pub async fn update_source(
    app: AppHandle,
    state: State<'_, AppState>,
    input: UpdateSourceInput,
) -> Result<Source> {
    let _permit = state.runner.write_permit()?;
    require_single_source_crud(input.config.as_ref())?;
    let source = SourcesRepo::update(&state.db, input).await?;
    let _ = app.emit("sources://changed", &source);
    Ok(source)
}

#[tauri::command]
pub async fn delete_source(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    let _permit = state.runner.write_permit()?;
    SourcesRepo::delete(&state.db, &id).await?;
    let _ = app.emit("sources://changed", ());
    Ok(())
}

#[tauri::command]
pub async fn seed_sample_workflow(app: AppHandle, state: State<'_, AppState>) -> Result<Source> {
    let _permit = state.runner.write_permit()?;
    let source = crate::services::sample_workflows::seed_default(&state.db).await?;
    let _ = app.emit("sources://changed", &source);
    Ok(source)
}

#[tauri::command]
pub async fn seed_all_sample_workflows(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::services::sample_workflows::SeedWorkflowReport> {
    let _permit = state.runner.write_permit()?;
    let result = crate::services::sample_workflows::seed_all_report(&state.db).await;
    // Also refresh after a partial batch failure: earlier creations may be saved.
    let _ = app.emit("sources://changed", ());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_crud_cannot_store_a_multi_source_workflow_without_owned_rows() {
        assert!(require_single_source_crud(None).is_ok());
        assert!(require_single_source_crud(Some(
            &serde_json::json!({"__workflow_spec":{"sources":[{"id":"one"}]}})
        ))
        .is_ok());
        let error = require_single_source_crud(Some(
            &serde_json::json!({"__workflow_spec":{"sources":[{"id":"one"},{"id":"two"}]}}),
        ))
        .unwrap_err();
        assert!(error.to_string().contains("Save workflow"));
    }
}
