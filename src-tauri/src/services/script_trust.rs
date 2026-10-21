//! Local execution trust is database state, never authority carried by YAML.
use crate::error::{AppError, Result};
use scrape_core::extraction::{ScriptEnvironmentStatus, ScriptRuntime, WorkflowScriptConfig};
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
    if !std::path::Path::new(path).is_absolute() {
        return Err(AppError::Validation("script.path: select an absolute path to a local script file, or embed code before saving; repository-relative paths are not portable".into()));
    }
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
            "Enable at least one source in the workflow editor before preview or execution.".into(),
        ));
    }
    for source in sources {
        crate::services::workflow::validated_stored_config(source.kind, &source.config)?;
        require_trusted(pool, &source.id, &source.config).await?;
        match scrape_core::extraction::mode(&source.config).map_err(AppError::Validation)? {
            scrape_core::extraction::ExtractionMode::Static => {}
            scrape_core::extraction::ExtractionMode::Browser => {
                if let Some(session_id) = source
                    .config
                    .get("session_id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let valid_id =
                        scrape_core::browser::StoredBrowserSession::validate_session_id(session_id)
                            .map_err(AppError::Validation)?;
                    let status: Option<String> =
                        sqlx::query_scalar("SELECT status FROM browser_sessions WHERE id = ?")
                            .bind(&valid_id)
                            .fetch_optional(pool)
                            .await?;
                    match status.as_deref() {
                        Some("ready") => {}
                        Some(_) => {
                            return Err(AppError::Validation(format!(
                                "Browser session '{valid_id}' requires login. Open and save the login session before running."
                            )));
                        }
                        None => {
                            return Err(AppError::Validation(format!(
                                "Browser session '{valid_id}' does not exist. Open and save the login session before running."
                            )));
                        }
                    }
                }
                if std::env::var_os("SCRAPE_CDP_WS_URL").is_none() {
                    scrape_core::browser::resolve_browser_runtime()
                        .map_err(AppError::Validation)?;
                }
            }
            scrape_core::extraction::ExtractionMode::Script => {
                let script_cfg = script(&source.config)?
                    .ok_or_else(|| AppError::Validation("Missing script configuration".into()))?;
                let env = check_script_environment(&script_cfg)?;
                if !env.available {
                    return Err(AppError::Validation(format!(
                        "Script source '{}' cannot run: {}",
                        source.id,
                        env.message
                            .unwrap_or_else(|| "Script runtime unavailable".into())
                    )));
                }
            }
        }
    }
    Ok(())
}

fn resolve_executable_path(name: &str) -> Option<String> {
    if name.contains('/') || name.contains('\\') {
        let path = std::path::Path::new(name);
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

pub fn check_script_environment(script: &WorkflowScriptConfig) -> Result<ScriptEnvironmentStatus> {
    if script.path.is_some() {
        fingerprint(script)?;
    }
    match script.runtime {
        ScriptRuntime::Js => {
            let Some(binary) = script.interpreter.as_deref() else {
                return Ok(ScriptEnvironmentStatus {
                    available: false, runtime: ScriptRuntime::Js, executable_path: None,
                    version: None, missing_modules: vec![],
                    message: Some("Setup required: select an absolute path to a Node.js executable (Node.js 18 or newer). No runtime is downloaded or selected implicitly.".into()),
                });
            };
            if !std::path::Path::new(binary).is_absolute() {
                return Ok(ScriptEnvironmentStatus {
                    available: false, runtime: ScriptRuntime::Js, executable_path: None,
                    version: None, missing_modules: vec![],
                    message: Some("Setup required: choose the absolute path to Node.js; a shell command such as node depends on the developer PATH.".into()),
                });
            }
            let output = std::process::Command::new(binary).arg("--version").output();
            match output {
                Ok(out) if out.status.success() => {
                    let version_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    let exec_path =
                        resolve_executable_path(binary).or_else(|| Some(binary.to_string()));
                    let supported = version_str
                        .strip_prefix('v')
                        .and_then(|version| version.split('.').next())
                        .and_then(|major| major.parse::<u32>().ok())
                        .is_some_and(|major| major >= 18);
                    Ok(ScriptEnvironmentStatus {
                        available: supported,
                        runtime: ScriptRuntime::Js,
                        executable_path: exec_path,
                        version: Some(version_str),
                        missing_modules: Vec::new(),
                        message: (!supported).then(|| "Select Node.js 18 or newer; the configured executable did not report a supported Node.js version.".into()),
                    })
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    Ok(ScriptEnvironmentStatus {
                        available: false,
                        runtime: ScriptRuntime::Js,
                        executable_path: Some(binary.to_string()),
                        version: None,
                        missing_modules: Vec::new(),
                        message: Some(format!(
                            "Node.js check failed with exit code {:?}: {err}",
                            out.status.code()
                        )),
                    })
                }
                Err(e) => Ok(ScriptEnvironmentStatus {
                    available: false,
                    runtime: ScriptRuntime::Js,
                    executable_path: Some(binary.to_string()),
                    version: None,
                    missing_modules: Vec::new(),
                    message: Some(format!("Node.js runtime not found or not executable: {e}")),
                }),
            }
        }
        ScriptRuntime::Python => {
            let Some(binary) = &script.interpreter else {
                return Ok(ScriptEnvironmentStatus {
                    available: false,
                    runtime: ScriptRuntime::Python,
                    executable_path: None,
                    version: None,
                    missing_modules: Vec::new(),
                    message: Some(
                        "Python runtime requires an explicit interpreter path (e.g. /usr/bin/python3 or .venv/bin/python3)".into(),
                    ),
                });
            };
            let output = std::process::Command::new(binary).arg("--version").output();
            match output {
                Ok(out) if out.status.success() => {
                    let mut version_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if version_str.is_empty() {
                        version_str = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    }

                    // Extract imported modules
                    let mut required_modules = Vec::new();
                    let script_content = if let Some(code) = &script.code {
                        Some(code.clone())
                    } else if let Some(path) = &script.path {
                        std::fs::read_to_string(path).ok()
                    } else {
                        None
                    };

                    if let Some(content) = script_content {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if let Some(rest) = trimmed.strip_prefix("import ") {
                                for part in rest.split(',') {
                                    if let Some(mod_name) = part.split_whitespace().next() {
                                        let root = mod_name.split('.').next().unwrap_or(mod_name);
                                        if !required_modules.contains(&root.to_string()) {
                                            required_modules.push(root.to_string());
                                        }
                                    }
                                }
                            } else if let Some(rest) = trimmed.strip_prefix("from ") {
                                if let Some(mod_name) = rest.split_whitespace().next() {
                                    let root = mod_name.split('.').next().unwrap_or(mod_name);
                                    if !required_modules.contains(&root.to_string()) {
                                        required_modules.push(root.to_string());
                                    }
                                }
                            }
                        }
                    }

                    let mut missing_modules = Vec::new();
                    if !required_modules.is_empty() {
                        let probe_code = format!(
                            "import importlib.util\nfor m in {:?}:\n    if importlib.util.find_spec(m) is None:\n        print(m)",
                            required_modules
                        );
                        if let Ok(probe_out) = std::process::Command::new(binary)
                            .args(["-c", &probe_code])
                            .output()
                        {
                            if probe_out.status.success() {
                                for line in String::from_utf8_lossy(&probe_out.stdout).lines() {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        missing_modules.push(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }

                    let available = missing_modules.is_empty();
                    let message = if !available {
                        Some(format!(
                            "Missing required Python modules: {}",
                            missing_modules.join(", ")
                        ))
                    } else {
                        None
                    };

                    let exec_path =
                        resolve_executable_path(binary).or_else(|| Some(binary.clone()));
                    Ok(ScriptEnvironmentStatus {
                        available,
                        runtime: ScriptRuntime::Python,
                        executable_path: exec_path,
                        version: Some(version_str),
                        missing_modules,
                        message,
                    })
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    Ok(ScriptEnvironmentStatus {
                        available: false,
                        runtime: ScriptRuntime::Python,
                        executable_path: Some(binary.clone()),
                        version: None,
                        missing_modules: Vec::new(),
                        message: Some(format!(
                            "Python interpreter check failed with exit code {:?}: {err}",
                            out.status.code()
                        )),
                    })
                }
                Err(e) => Ok(ScriptEnvironmentStatus {
                    available: false,
                    runtime: ScriptRuntime::Python,
                    executable_path: Some(binary.clone()),
                    version: None,
                    missing_modules: Vec::new(),
                    message: Some(format!(
                        "Python interpreter not found or not executable: {e}"
                    )),
                }),
            }
        }
        ScriptRuntime::Custom => {
            let Some(binary) = &script.interpreter else {
                return Ok(ScriptEnvironmentStatus {
                    available: false,
                    runtime: ScriptRuntime::Custom,
                    executable_path: None,
                    version: None,
                    missing_modules: Vec::new(),
                    message: Some("Custom runtime requires an explicit executable path".into()),
                });
            };
            if !std::path::Path::new(binary).exists() {
                return Ok(ScriptEnvironmentStatus {
                    available: false,
                    runtime: ScriptRuntime::Custom,
                    executable_path: Some(binary.clone()),
                    version: None,
                    missing_modules: Vec::new(),
                    message: Some(format!("Custom executable does not exist at '{binary}'")),
                });
            }
            Ok(ScriptEnvironmentStatus {
                available: true,
                runtime: ScriptRuntime::Custom,
                executable_path: Some(binary.clone()),
                version: None,
                missing_modules: Vec::new(),
                message: None,
            })
        }
    }
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
        let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM sources WHERE workflow_id=? AND (needs_review=1 OR (script_sha256 IS NOT NULL AND script_trusted=0)) AND (json_type((SELECT config FROM sources WHERE id=?),'$.__workflow_spec.sources') IS NULL OR id IN (SELECT json_extract(value,'$.id') FROM json_each((SELECT config FROM sources WHERE id=?),'$.__workflow_spec.sources')))").bind(&id).bind(&id).bind(&id).fetch_one(pool).await?;
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
        serde_json::from_value(serde_json::json!({"kind":"selector","name":"Script fixture","identifier":"https://fixture.test/jobs","enabled":true,"cadence":"daily","config":{"url":"https://fixture.test/jobs","mode":"script","script":{"runtime":"js","interpreter":resolve_executable_path("node").expect("Node.js fixture runtime"),"code":code}}})).unwrap()
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
        let mut worker_source = scrape_core::WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: source.identifier.clone(),
            config: source.config.clone(),
        };
        assert!(validate_execution(&pool, &[])
            .await
            .unwrap_err()
            .to_string()
            .contains("Enable at least one source"));
        assert!(
            validate_execution(&pool, std::slice::from_ref(&worker_source))
                .await
                .unwrap_err()
                .to_string()
                .contains("untrusted")
        );
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
        validate_execution(&pool, std::slice::from_ref(&worker_source))
            .await
            .unwrap();
        let mut broken_input = input("broken");
        broken_input.identifier = "https://fixture.test/broken-jobs".into();
        broken_input.config.as_mut().unwrap()["script"] = serde_json::json!({
            "runtime": "js",
            "interpreter": "/nonexistent/node_binary",
            "code": "console.log('broken')"
        });
        let broken_src = SourcesRepo::create(&pool, broken_input).await.unwrap();
        trust(
            &pool,
            &broken_src.id,
            broken_src.script_sha256.as_deref().unwrap(),
            true,
        )
        .await
        .unwrap();
        let broken_worker = scrape_core::WorkerSource {
            id: broken_src.id.clone(),
            kind: broken_src.kind,
            name: broken_src.name.clone(),
            identifier: broken_src.identifier.clone(),
            config: broken_src.config.clone(),
        };
        assert!(
            validate_execution(&pool, std::slice::from_ref(&broken_worker))
                .await
                .unwrap_err()
                .to_string()
                .contains("cannot run")
        );
        worker_source.config = serde_json::json!({
            "mode":"browser",
            "session_id":"missing-session",
            "item":"article",
            "title":"h2",
            "link":"a"
        });
        assert!(
            validate_execution(&pool, std::slice::from_ref(&worker_source))
                .await
                .unwrap_err()
                .to_string()
                .contains("Browser session 'missing-session' does not exist")
        );
        worker_source.config.as_object_mut().unwrap().remove("mode");
        worker_source
            .config
            .as_object_mut()
            .unwrap()
            .remove("session_id");
        validate_execution(&pool, &[worker_source]).await.unwrap();
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
        let child_id = parent.config["__workflow_spec"]["sources"][1]["id"]
            .as_str()
            .unwrap();
        let child = SourcesRepo::get(&pool, child_id).await.unwrap();
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

    #[test]
    fn portable_script_setup_never_uses_a_shell_runtime_or_relative_file() {
        let mut script: WorkflowScriptConfig =
            serde_json::from_value(serde_json::json!({"runtime":"js","code":"fixture"})).unwrap();
        let missing = check_script_environment(&script).unwrap();
        assert!(!missing.available);
        assert!(missing.message.unwrap().contains("absolute path"));
        script.interpreter = Some("node".into());
        assert!(!check_script_environment(&script).unwrap().available);
        script.interpreter = Some("/nonexistent/scrape-fixture-node".into());
        let missing = check_script_environment(&script).unwrap();
        assert!(!missing.available);
        assert!(missing
            .message
            .unwrap()
            .contains("not found or not executable"));
        script.path = Some("repo/extractor.js".into());
        script.code = None;
        assert!(fingerprint(&script)
            .unwrap_err()
            .to_string()
            .contains("absolute path"));
        script.path = Some("/nonexistent/scrape-fixture-script.js".into());
        assert!(fingerprint(&script)
            .unwrap_err()
            .to_string()
            .contains("script.path"));
    }

    #[test]
    fn check_script_environment_reports_expected_diagnostics() {
        // Node / JS
        let js_config: WorkflowScriptConfig = serde_json::from_value(serde_json::json!({
            "runtime": "js",
            "code": "console.log('test')"
        }))
        .unwrap();
        let js_status = check_script_environment(&js_config).unwrap();
        // If node is installed, it is available
        if js_status.available {
            assert!(js_status.executable_path.is_some());
            assert!(js_status.version.is_some());
            assert!(js_status.missing_modules.is_empty());
        }

        // Python without interpreter path
        let py_no_interp: WorkflowScriptConfig = serde_json::from_value(serde_json::json!({
            "runtime": "python",
            "code": "print('test')"
        }))
        .unwrap();
        let py_no_status = check_script_environment(&py_no_interp).unwrap();
        assert!(!py_no_status.available);
        assert!(py_no_status
            .message
            .unwrap()
            .contains("requires an explicit interpreter path"));

        // Python with non-existent interpreter path
        let py_invalid: WorkflowScriptConfig = serde_json::from_value(serde_json::json!({
            "runtime": "python",
            "interpreter": "/nonexistent/python_binary_xyz",
            "code": "print('test')"
        }))
        .unwrap();
        let py_status = check_script_environment(&py_invalid).unwrap();
        assert!(!py_status.available);
        assert!(py_status
            .message
            .unwrap()
            .contains("not found or not executable"));

        // Custom with non-existent executable
        let custom_invalid: WorkflowScriptConfig = serde_json::from_value(serde_json::json!({
            "runtime": "custom",
            "interpreter": "/nonexistent/custom_runner_123",
            "code": "echo test"
        }))
        .unwrap();
        let custom_status = check_script_environment(&custom_invalid).unwrap();
        assert!(!custom_status.available);
        assert!(custom_status.message.unwrap().contains("does not exist"));
    }
}
