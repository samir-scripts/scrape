//! Local execution trust is database state, never authority carried by YAML.
use crate::error::{AppError, Result};
use scrape_core::extraction::WorkflowScriptConfig;
use serde_json::Value;
use sqlx::SqlitePool;
use std::io::Read;

fn script(config: &Value) -> Result<Option<WorkflowScriptConfig>> {
    scrape_core::extraction::script(config).map_err(AppError::Validation)
}

/// Path scripts include the current bytes as well as the declared executable
/// inputs. Editing a file in place must invalidate a previous trust decision.
pub fn fingerprint(script: &WorkflowScriptConfig) -> Result<String> {
    let canonical = script.canonical_sha256().map_err(AppError::Validation)?;
    let Some(path) = &script.path else {
        return Ok(canonical);
    };
    if !std::fs::metadata(path)
        .map_err(|error| AppError::Validation(format!("script.path: {error}")))?
        .is_file()
    {
        return Err(AppError::Validation(
            "script.path: select a regular script file".into(),
        ));
    }
    let file = std::fs::File::open(path).map_err(|error| {
        AppError::Validation(format!(
            "script.path: cannot read the selected script: {error}"
        ))
    })?;
    let mut bytes = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|error| AppError::Validation(format!("script.path: {error}")))?;
    if bytes.len() > 1_048_576 {
        return Err(AppError::Validation(
            "script.path: maximum script size is 1 MiB".into(),
        ));
    }
    script.file_sha256(&bytes).map_err(AppError::Validation)
}

pub fn saved_state(
    config: &Value,
    trusted: bool,
    old_hash: Option<&str>,
    imported: bool,
) -> Result<(bool, Option<String>)> {
    let Some(script) = script(config)? else {
        return Ok((false, None));
    };
    // Imports may reference paths that do not exist on this machine. They can
    // be saved disabled, but trust always requires reading the actual file.
    let hash = fingerprint(&script)
        .or_else(|_| script.canonical_sha256().map_err(AppError::Validation))?;
    Ok((
        trusted && !imported && old_hash == Some(hash.as_str()),
        Some(hash),
    ))
}

pub fn quarantine_import(
    mut spec: crate::services::workflow::WorkflowSpec,
) -> Result<crate::services::workflow::WorkflowSpec> {
    let inputs = crate::services::workflow::workflow_source_inputs(&spec, "UTC")?;
    for (source, input) in spec.sources.iter_mut().zip(inputs) {
        if script(input.config.as_ref().unwrap_or(&Value::Null))?.is_some() {
            source.enabled = false;
            spec.metadata.enabled = Some(false);
        }
    }
    Ok(spec)
}

pub async fn trust(
    pool: &SqlitePool,
    id: &str,
    expected_hash: &str,
    confirmed: bool,
) -> Result<crate::db::sources::Source> {
    if !confirmed {
        return Err(AppError::Validation("Explicit confirmation is required: this custom script runs with your local privileges; process isolation is not an OS sandbox.".into()));
    }
    let source = crate::db::sources::SourcesRepo::get(pool, id).await?;
    crate::services::workflow::validated_stored_config(source.kind, &source.config)?;
    let script = script(&source.config)?
        .ok_or_else(|| AppError::Validation("This source has no custom script to trust".into()))?;
    let current = fingerprint(&script)?;
    if source.script_sha256.as_deref() != Some(expected_hash) || current != expected_hash {
        return Err(AppError::Validation(
            "Script changed since review. Save and review the current script before trusting it."
                .into(),
        ));
    }
    let result = sqlx::query("UPDATE sources SET script_trusted=1,needs_review=0 WHERE id=? AND script_sha256=? AND config=?")
        .bind(id).bind(expected_hash).bind(serde_json::to_string(&source.config)?).execute(pool).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Validation(
            "Source changed during script review; reload and review it again".into(),
        ));
    }
    reconcile_workflow(pool, id).await?;
    crate::db::sources::SourcesRepo::get(pool, id).await
}

/// Recheck executable content before every preview/run, including path scripts.
/// A caller cannot grant trust by placing flags or hashes in a worker config.
pub async fn require_trusted(pool: &SqlitePool, id: &str, config: &Value) -> Result<()> {
    let Some(script) = script(config)? else {
        return Ok(());
    };
    let source = match crate::db::sources::SourcesRepo::get(pool, id).await {
        Err(AppError::NotFound(_)) => return Err(AppError::Validation("Custom script needs local review and explicit trust before preview or execution. Save it disabled first.".into())),
        result => result?,
    };
    let current = fingerprint(&script);
    if !source.script_trusted || current.as_ref().ok() != source.script_sha256.as_ref() {
        sqlx::query("UPDATE sources SET script_trusted=0,needs_review=1,enabled=0,next_run_at=NULL WHERE id=?").bind(id).execute(pool).await?;
        reconcile_workflow(pool, id).await?;
        return Err(AppError::Validation("Custom script is untrusted or changed. Review and explicitly trust its current contents before running.".into()));
    }
    Ok(())
}

pub async fn validate_execution(
    pool: &SqlitePool,
    sources: &[scrape_core::WorkerSource],
) -> Result<()> {
    if sources.is_empty() {
        return Err(AppError::Validation(
            "Enable at least one source before preview or execution.".into(),
        ));
    }
    for source in sources {
        crate::services::workflow::validated_stored_config(source.kind, &source.config)?;
        require_trusted(pool, &source.id, &source.config).await?;
        match scrape_core::extraction::mode(&source.config).map_err(AppError::Validation)? {
            scrape_core::extraction::ExtractionMode::Static => {},
            mode => return Err(AppError::Validation(format!("{mode:?} extraction is configured but its runner is not installed in this application build. Browser execution follows in Phase 6C; custom script execution follows in Phase 6D."))),
        }
    }
    Ok(())
}

/// Aggregate review state without enabling anything as a side effect of trust.
pub async fn reconcile_workflow(pool: &SqlitePool, source_id: &str) -> Result<()> {
    let workflow_id: Option<String> =
        sqlx::query_scalar("SELECT workflow_id FROM sources WHERE id=?")
            .bind(source_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    if let Some(id) = workflow_id {
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM sources WHERE workflow_id=? AND (needs_review=1 OR (script_sha256 IS NOT NULL AND script_trusted=0))").bind(&id).fetch_one(pool).await?;
        sqlx::query("UPDATE workflows SET needs_review=?, enabled=CASE WHEN ? THEN 0 ELSE enabled END, next_run_at=CASE WHEN ? THEN NULL ELSE next_run_at END WHERE id=?")
            .bind(pending > 0).bind(pending > 0).bind(pending > 0).bind(&id).execute(pool).await?;
        // The current IPC presents the parent's source row as the workflow.
        if pending > 0 {
            sqlx::query("UPDATE sources SET enabled=0,next_run_at=NULL WHERE id=?")
                .bind(&id)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::sources::{CreateSourceInput, SourcesRepo, UpdateSourceInput};
    fn input(code: &str) -> CreateSourceInput {
        serde_json::from_value(serde_json::json!({"kind":"selector","name":"Script fixture","identifier":"https://fixture.test/jobs","enabled":true,"cadence":"daily","config":{"url":"https://fixture.test/jobs","mode":"script","script":{"runtime":"js","code":code}}})).unwrap()
    }
    fn update(id: &str, config: Value) -> UpdateSourceInput {
        serde_json::from_value(serde_json::json!({"id":id,"config":config,"enabled":true})).unwrap()
    }
    #[tokio::test]
    async fn save_trust_edit_import_and_execution_guard_use_persisted_authority() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let source = SourcesRepo::create(&pool, input("fixture script"))
            .await
            .unwrap();
        assert!(!source.enabled && source.needs_review && !source.script_trusted);
        assert!(source.next_run_at.is_none());
        let hash = source.script_sha256.clone().unwrap();
        assert!(require_trusted(&pool, &source.id, &source.config)
            .await
            .is_err());
        assert!(trust(&pool, &source.id, &hash, false).await.is_err());
        assert!(trust(&pool, &source.id, "stale", true).await.is_err());
        let trusted = trust(&pool, &source.id, &hash, true).await.unwrap();
        assert!(trusted.script_trusted && !trusted.needs_review && !trusted.enabled);
        require_trusted(&pool, &source.id, &source.config)
            .await
            .unwrap();
        let unchanged = SourcesRepo::update(&pool, update(&source.id, source.config.clone()))
            .await
            .unwrap();
        assert!(unchanged.script_trusted && unchanged.enabled);
        let mut edited = source.config.clone();
        edited["script"]["code"] = "changed code".into();
        let edited = SourcesRepo::update(&pool, update(&source.id, edited))
            .await
            .unwrap();
        assert!(!edited.script_trusted && edited.needs_review && !edited.enabled);
        assert_ne!(edited.script_sha256, Some(hash));
        assert!(
            SourcesRepo::mark_reviewed(&pool, &source.id)
                .await
                .unwrap()
                .needs_review
        );
        assert!(
            !saved_state(&edited.config, true, edited.script_sha256.as_deref(), true)
                .unwrap()
                .0
        );
        pool.close().await;
        assert!(matches!(
            require_trusted(&pool, &source.id, &source.config).await,
            Err(AppError::Database(_))
        ));
    }

    #[tokio::test]
    async fn imported_multi_source_scripts_stay_disabled_and_reimport_resets_trust() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let spec = crate::services::workflow::validate(serde_json::json!({
            "version":6,"metadata":{"name":"Multi-source fixture"},
            "sources":[
                {"id":"primary","label":"Static fixture","enabled":true,"extractor":{"kind":"selector","url":"https://fixture.test/static","job_container":"article","title_selector":"h2","url_selector":"a"}},
                {"id":"child-script","label":"Script fixture","enabled":true,"extractor":{"kind":"script","url":"https://fixture.test/dynamic","script":{"runtime":"js","code":"fixture"}}}
            ],"filters":{"include_keywords":["Engineer"]}
        }), "UTC").unwrap();
        let imported = quarantine_import(spec).unwrap();
        assert_eq!(imported.metadata.enabled, Some(false));
        assert!(!imported.sources[1].enabled);
        let parent = crate::services::workflow::persist(&pool, imported, None, true, "UTC")
            .await
            .unwrap();
        assert!(parent.needs_review && !parent.enabled);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sources")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        let child = SourcesRepo::get(&pool, "child-script").await.unwrap();
        assert!(!child.script_trusted && !child.enabled && child.needs_review);
        trust(
            &pool,
            &child.id,
            child.script_sha256.as_deref().unwrap(),
            true,
        )
        .await
        .unwrap();
        assert!(
            !SourcesRepo::get(&pool, &parent.id)
                .await
                .unwrap()
                .needs_review
        );
        let spec =
            crate::services::workflow::validate(parent.config["__workflow_spec"].clone(), "UTC")
                .unwrap();
        let parent = crate::services::workflow::persist(
            &pool,
            spec.clone(),
            Some(parent.id.clone()),
            false,
            "UTC",
        )
        .await
        .unwrap();
        assert!(
            SourcesRepo::get(&pool, &child.id)
                .await
                .unwrap()
                .script_trusted
        );
        crate::services::workflow::persist(
            &pool,
            quarantine_import(spec.clone()).unwrap(),
            Some(parent.id.clone()),
            true,
            "UTC",
        )
        .await
        .unwrap();
        assert!(
            !SourcesRepo::get(&pool, &child.id)
                .await
                .unwrap()
                .script_trusted
        );
        assert!(
            SourcesRepo::get(&pool, &parent.id)
                .await
                .unwrap()
                .needs_review
        );
        assert!(
            crate::services::workflow::persist(&pool, spec, None, true, "UTC")
                .await
                .is_err()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sources")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn file_edit_at_same_path_invalidates_trust_before_execution() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.js");
        let non_file: WorkflowScriptConfig =
            serde_json::from_value(serde_json::json!({"runtime":"js","path":directory.path()}))
                .unwrap();
        assert!(fingerprint(&non_file)
            .unwrap_err()
            .to_string()
            .contains("regular script file"));
        std::fs::write(&path, "initial fixture").unwrap();
        let pool = crate::db::init_memory_db().await.unwrap();
        let mut input = input("unused");
        input.config.as_mut().unwrap()["script"] = serde_json::json!({"runtime":"js","path":path});
        let source = SourcesRepo::create(&pool, input).await.unwrap();
        trust(
            &pool,
            &source.id,
            source.script_sha256.as_deref().unwrap(),
            true,
        )
        .await
        .unwrap();
        std::fs::write(&path, "changed fixture").unwrap();
        assert!(require_trusted(&pool, &source.id, &source.config)
            .await
            .is_err());
        let invalidated = SourcesRepo::get(&pool, &source.id).await.unwrap();
        assert!(!invalidated.enabled && invalidated.needs_review && !invalidated.script_trusted);
        assert!(trust(
            &pool,
            &source.id,
            source.script_sha256.as_deref().unwrap(),
            true
        )
        .await
        .is_err());
        pool.close().await;
    }
}
