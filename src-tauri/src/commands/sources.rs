use crate::db::sources::{CreateSourceInput, Source, SourcesRepo, UpdateSourceInput};
use crate::error::Result;
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

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
    let source = SourcesRepo::update(&state.db, input).await?;
    let _ = app.emit("sources://changed", &source);
    Ok(source)
}

#[tauri::command]
pub async fn delete_source(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<()> {
    SourcesRepo::delete(&state.db, &id).await?;
    let _ = app.emit("sources://changed", ());
    Ok(())
}
