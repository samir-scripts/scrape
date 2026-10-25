use crate::db::sources::{CreateSourceInput, Source, SourcesRepo, UpdateSourceInput};
use crate::error::{AppError, Result};
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, State};

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
pub async fn preview_test_workflow_replacement(
    state: State<'_, AppState>,
) -> Result<crate::services::sample_workflows::WorkflowReplacementPreview> {
    let timezone =
        crate::db::settings::SettingsRepo::get_value::<String>(&state.db, "system.timezone")
            .await?
            .unwrap_or_else(|| "UTC".into());
    crate::services::sample_workflows::replacement_preview(&state.db, &timezone).await
}

#[tauri::command]
pub async fn replace_test_workflows(
    app: AppHandle,
    state: State<'_, AppState>,
    selected: Vec<crate::services::sample_workflows::WorkflowReplacementSelection>,
) -> Result<crate::services::sample_workflows::WorkflowReplacementReport> {
    let _permit = state.runner.write_permit()?;
    let timezone =
        crate::db::settings::SettingsRepo::get_value::<String>(&state.db, "system.timezone")
            .await?
            .unwrap_or_else(|| "UTC".into());
    let database = app
        .path()
        .app_data_dir()
        .map_err(|error| {
            AppError::Internal(format!("Unable to locate application database: {error}"))
        })?
        .join("scrape_data.db");
    let backup = database.with_file_name(format!(
        "{}.pre-workflow-replacement-{}.db",
        database.file_name().unwrap_or_default().to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    let result =
        crate::services::sample_workflows::replace(&state.db, &selected, &backup, &timezone).await;
    if result.is_ok() {
        let _ = app.emit("sources://changed", ());
    }
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
