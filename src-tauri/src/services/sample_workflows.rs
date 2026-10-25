//! Shared built-in workflow seeding and explicit recoverable replacement.
use crate::db::sources::{Source, SourcesRepo};
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::Path;

const TARGET_IDS: [&str; 3] = [
    "software-engineering-last-12-hours",
    "cherry-hill-fast-food-last-month",
    "remote-usd-over-60000",
];

#[derive(Serialize, Deserialize, ts_rs::TS, Clone, Debug)]
#[ts(export)]
pub struct WorkflowReplacementCandidate {
    pub id: String,
    pub name: String,
    pub fingerprint: String,
    pub selected_by_default: bool,
    pub reason: String,
}

#[derive(Serialize, Deserialize, ts_rs::TS, Clone, Debug)]
#[ts(export)]
pub struct WorkflowReplacementPreview {
    pub candidates: Vec<WorkflowReplacementCandidate>,
    pub target_template_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, ts_rs::TS, Clone, Debug)]
#[ts(export)]
pub struct WorkflowReplacementSelection {
    pub id: String,
    pub fingerprint: String,
}
#[derive(Serialize, Deserialize, ts_rs::TS, Clone, Debug)]
#[ts(export)]
pub struct WorkflowReplacementReport {
    pub created: Vec<Source>,
    pub removed_ids: Vec<String>,
    pub preserved_ids: Vec<String>,
    pub backup_path: Option<String>,
    pub setup_required: Vec<String>,
}

pub async fn seed_default(pool: &SqlitePool) -> Result<Source> {
    if let Some(existing) = SourcesRepo::list(pool).await?.into_iter().find(|source| {
        source
            .config
            .pointer("/__builtin_workflow/template_id")
            .and_then(Value::as_str)
            == Some(TARGET_IDS[0])
    }) {
        return Ok(existing);
    }
    seed_input(
        pool,
        super::template_catalog::seed_inputs()?.remove(0),
        TARGET_IDS[0],
    )
    .await
}

pub async fn seed_all(pool: &SqlitePool) -> Result<Vec<Source>> {
    let existing = SourcesRepo::list(pool).await?;
    let inputs = super::template_catalog::seed_inputs()?;
    if inputs.len() != TARGET_IDS.len() {
        return Err(AppError::Validation(
            "Built-in workflow catalog must contain exactly three profiles".into(),
        ));
    }
    let timezone = crate::db::settings::SettingsRepo::get_value::<String>(pool, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let missing = TARGET_IDS
        .iter()
        .zip(inputs)
        .filter(|(template_id, _)| {
            !existing.iter().any(|source| {
                source
                    .config
                    .pointer("/__builtin_workflow/template_id")
                    .and_then(Value::as_str)
                    == Some(*template_id)
            })
        })
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(Vec::new());
    }
    let mut tx = pool.begin().await?;
    let mut created = Vec::with_capacity(missing.len());
    for (template_id, input) in missing {
        let spec = crate::services::workflow::validate(
            input.config.as_ref().unwrap()["__workflow_spec"].clone(),
            &timezone,
        )?;
        let origin = origin_marker(&spec, template_id)?;
        let id = crate::services::workflow_store::save_in_transaction(
            &mut tx, spec, None, true, &timezone, false,
        )
        .await?;
        mark_origin_tx(&mut tx, &id, &origin).await?;
        created.push(SourcesRepo::get_tx(&mut tx, &id).await?);
    }
    tx.commit().await?;
    Ok(created)
}

async fn seed_input(
    pool: &SqlitePool,
    input: crate::db::sources::CreateSourceInput,
    template_id: &str,
) -> Result<Source> {
    let timezone = crate::db::settings::SettingsRepo::get_value::<String>(pool, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let spec = crate::services::workflow::validate(
        input.config.as_ref().unwrap()["__workflow_spec"].clone(),
        &timezone,
    )?;
    let origin = origin_marker(&spec, template_id)?;
    let source = crate::services::workflow::persist(pool, spec, None, true, &timezone).await?;
    mark_origin(pool, &source.id, &origin).await?;
    SourcesRepo::get(pool, &source.id).await
}

fn origin_marker(
    spec: &crate::services::workflow::WorkflowSpec,
    template_id: &str,
) -> Result<Value> {
    Ok(
        json!({"catalog_version": super::template_catalog::CATALOG_VERSION, "template_id": template_id, "fingerprint": fingerprint(&normalized_spec(spec)?)?}),
    )
}
async fn mark_origin(pool: &SqlitePool, id: &str, origin: &Value) -> Result<()> {
    let mut tx = pool.begin().await?;
    mark_origin_tx(&mut tx, id, origin).await?;
    tx.commit().await?;
    Ok(())
}

async fn mark_origin_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
    origin: &Value,
) -> Result<()> {
    let marker = serde_json::to_string(origin)?;
    let parent = sqlx::query(
        "UPDATE workflows SET config=json_set(config,'$.__builtin_workflow',json(?)) WHERE id=?",
    )
    .bind(&marker)
    .bind(id)
    .execute(&mut **tx)
    .await?;
    if parent.rows_affected() != 1 {
        return Err(AppError::Validation(
            "Built-in workflow origin could not be recorded".into(),
        ));
    }
    let sources = sqlx::query("UPDATE sources SET config=json_set(config,'$.__builtin_workflow',json(?)) WHERE workflow_id=?")
        .bind(&marker).bind(id).execute(&mut **tx).await?;
    if sources.rows_affected() == 0 {
        return Err(AppError::Validation(
            "Built-in workflow source ownership could not be recorded".into(),
        ));
    }
    Ok(())
}
pub async fn replacement_preview(
    pool: &SqlitePool,
    timezone: &str,
) -> Result<WorkflowReplacementPreview> {
    let recognized = super::legacy_sample_workflows::recognition_specs()?;
    let mut candidates = Vec::new();
    let signatures = recognized
        .iter()
        .map(|s| {
            let resolved = crate::services::workflow::validate(serde_json::to_value(s)?, timezone)?;
            fingerprint(&normalized_spec(&resolved)?)
        })
        .collect::<Result<Vec<_>>>()?;
    for source in SourcesRepo::list(pool).await? {
        let aggregate = aggregate_fingerprint(pool, &source.id).await?;
        let stored = source.config.get("__workflow_spec").cloned();
        let exact = match stored {
            Some(value) => crate::services::workflow::validate(value, timezone)
                .ok()
                .and_then(|spec| normalized_spec(&spec).ok())
                .and_then(|value| fingerprint(&value).ok())
                .is_some_and(|hash| signatures.contains(&hash)),
            None => false,
        };
        candidates.push(WorkflowReplacementCandidate {
            id: source.id.clone(),
            name: source.name.clone(),
            fingerprint: aggregate,
            selected_by_default: exact,
            reason: if exact {
                "Matches an exact shipped example".into()
            } else {
                "Other workflow — preserved unless explicitly selected".into()
            },
        });
    }
    Ok(WorkflowReplacementPreview {
        candidates,
        target_template_ids: TARGET_IDS.iter().map(|s| (*s).to_string()).collect(),
    })
}

pub async fn replace(
    pool: &SqlitePool,
    selected: &[WorkflowReplacementSelection],
    backup_path: &Path,
    timezone: &str,
) -> Result<WorkflowReplacementReport> {
    let preview = replacement_preview(pool, timezone).await?;
    let mut ids = std::collections::BTreeSet::new();
    for selection in selected {
        if !ids.insert(selection.id.as_str()) {
            return Err(AppError::Validation(
                "Duplicate workflow replacement selection".into(),
            ));
        }
        let candidate = preview
            .candidates
            .iter()
            .find(|candidate| candidate.id == selection.id)
            .ok_or_else(|| {
                AppError::Validation(
                    "Selected workflow no longer exists. Reload and review the selection.".into(),
                )
            })?;
        if candidate.fingerprint != selection.fingerprint {
            return Err(AppError::Validation(
                "Workflow changed while replacing test workflows. Reload and review the selection."
                    .into(),
            ));
        }
    }
    let before = SourcesRepo::list(pool).await?;
    let mut removed_ids: Vec<String> = selected.iter().map(|s| s.id.clone()).collect();
    removed_ids.sort();
    let mut preserved_ids: Vec<String> = before
        .iter()
        .filter(|s| !ids.contains(s.id.as_str()))
        .map(|s| s.id.clone())
        .collect();
    preserved_ids.sort();
    let targets = super::template_catalog::seed_inputs()?;
    let mut existing_targets = std::collections::HashSet::new();
    for source in &before {
        if ids.contains(source.id.as_str()) {
            continue;
        }
        if let Some(template_id) = source
            .config
            .pointer("/__builtin_workflow/template_id")
            .and_then(Value::as_str)
        {
            if TARGET_IDS.contains(&template_id) {
                existing_targets.insert(template_id.to_string());
            }
        }
    }
    let missing = TARGET_IDS.iter().any(|id| !existing_targets.contains(*id));
    if (!ids.is_empty() || missing) && !before.is_empty() {
        crate::db::backup_to(pool, backup_path)
            .await
            .map_err(|error| {
                AppError::Validation(format!(
                    "Recovery backup failed. No workflows were changed: {error}"
                ))
            })?;
    }
    let mut tx = pool.begin().await?;
    for selection in selected {
        let current = aggregate_fingerprint_tx(&mut tx, &selection.id).await?;
        if current != selection.fingerprint {
            return Err(AppError::Validation(
                "Workflow changed while replacing test workflows. Reload and review the selection."
                    .into(),
            ));
        }
        // The parent owns these source rows. Deleting it cascades owned rows while jobs/runs
        // retain their independent history and their source/workflow links are nulled by FK.
        sqlx::query("DELETE FROM workflows WHERE id=?")
            .bind(&selection.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sources WHERE id=? AND workflow_id IS NULL")
            .bind(&selection.id)
            .execute(&mut *tx)
            .await?;
    }
    let mut created = Vec::new();
    for (index, input) in targets.into_iter().enumerate() {
        let template_id = TARGET_IDS[index];
        if existing_targets.contains(template_id) {
            continue;
        }
        let spec_value = input
            .config
            .as_ref()
            .and_then(|c| c.get("__workflow_spec"))
            .cloned()
            .ok_or_else(|| {
                AppError::Validation("Built-in workflow specification is missing".into())
            })?;
        let spec = crate::services::workflow::validate(spec_value, timezone)?;
        let origin = origin_marker(&spec, template_id)?;
        let id = crate::services::workflow_store::save_in_transaction(
            &mut tx, spec, None, true, timezone, false,
        )
        .await?;
        mark_origin_tx(&mut tx, &id, &origin).await?;
        created.push(SourcesRepo::get_tx(&mut tx, &id).await?);
    }
    tx.commit().await?;
    let after = SourcesRepo::list(pool).await?;
    let setup_required = after
        .iter()
        .filter(|source| {
            source
                .config
                .pointer("/__builtin_workflow/template_id")
                .and_then(Value::as_str)
                == Some("cherry-hill-fast-food-last-month")
                && source
                    .config
                    .pointer("/__workflow_spec/sources")
                    .and_then(Value::as_array)
                    .is_some_and(|sources| {
                        sources.iter().any(|candidate| {
                            candidate["extractor"]["kind"] == "adzuna"
                                && candidate["enabled"] != true
                        })
                    })
        })
        .map(|_| {
            "Adzuna is not enabled — setup required for broader local-search coverage.".to_string()
        })
        .collect();
    Ok(WorkflowReplacementReport {
        created,
        removed_ids,
        preserved_ids,
        backup_path: if before.is_empty() || (ids.is_empty() && !missing) {
            None
        } else {
            Some(backup_path.to_string_lossy().into_owned())
        },
        setup_required,
    })
}

fn normalized_spec(spec: &crate::services::workflow::WorkflowSpec) -> Result<Value> {
    let mut value = serde_json::to_value(spec)?;
    if let Some(sources) = value.get_mut("sources").and_then(Value::as_array_mut) {
        for (index, source) in sources.iter_mut().enumerate() {
            source["id"] = Value::String(format!("source-{index}"));
        }
    }
    fn strip_timezone(value: &mut Value) {
        match value {
            Value::Object(map) => {
                map.remove("timezone");
                for value in map.values_mut() {
                    strip_timezone(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    strip_timezone(value);
                }
            }
            _ => {}
        }
    }
    strip_timezone(&mut value);
    Ok(value)
}

fn fingerprint(value: &Value) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}
pub fn refreshed_origin_marker(
    spec: &crate::services::workflow::WorkflowSpec,
    mut marker: Value,
) -> Result<Value> {
    if let Some(template_id) = marker
        .get("template_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
    {
        marker["catalog_version"] = json!(super::template_catalog::CATALOG_VERSION);
        marker["fingerprint"] = json!(fingerprint(&normalized_spec(spec)?)?);
        marker["template_id"] = json!(template_id);
        Ok(marker)
    } else {
        Err(AppError::Validation(
            "Built-in workflow origin marker is malformed".into(),
        ))
    }
}

async fn aggregate_fingerprint(pool: &SqlitePool, id: &str) -> Result<String> {
    let mut tx = pool.begin().await?;
    let hash = aggregate_fingerprint_tx(&mut tx, id).await?;
    tx.rollback().await?;
    Ok(hash)
}

async fn aggregate_fingerprint_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    id: &str,
) -> Result<String> {
    let workflow: Option<String> = sqlx::query_scalar(
        "SELECT json_object('id',id,'name',name,'enabled',enabled,'cadence',cadence,'cadence_cron',cadence_cron,'next_run_at',next_run_at,'retention_days',retention_days,'retention_policy',retention_policy,'filters',filters,'config',config,'max_jobs_per_run',max_jobs_per_run,'starting_source_index',starting_source_index,'last_run_at',last_run_at,'last_status',last_status,'last_outcome',last_outcome,'last_error',last_error,'needs_review',needs_review,'created_at',created_at) FROM workflows WHERE id=?")
        .bind(id).fetch_optional(&mut **tx).await?;
    let source_rows: String = sqlx::query_scalar(
        "SELECT COALESCE(json_group_array(json_object('id',id,'workflow_id',workflow_id,'kind',kind,'name',name,'identifier',identifier,'config',config,'enabled',enabled,'last_run_at',last_run_at,'last_status',last_status,'last_outcome',last_outcome,'last_error',last_error,'created_at',created_at,'cadence',cadence,'cadence_cron',cadence_cron,'next_run_at',next_run_at,'last_run_duration_ms',last_run_duration_ms,'retention_days',retention_days,'retention_policy',retention_policy,'filters',filters,'needs_review',needs_review,'script_trusted',script_trusted,'script_sha256',script_sha256,'employer_hint',employer_hint,'order_index',order_index,'company_icon_url',company_icon_url)), '[]') FROM (SELECT * FROM sources WHERE workflow_id=? OR id=? ORDER BY id)")
        .bind(id).bind(id).fetch_one(&mut **tx).await?;
    let sources: Value = serde_json::from_str(&source_rows)?;
    if workflow.is_none() && sources.as_array().is_none_or(Vec::is_empty) {
        return Err(AppError::Validation(
            "Workflow changed while replacing test workflows. Reload and review the selection."
                .into(),
        ));
    }
    let value = json!({"workflow": workflow.map(|v| serde_json::from_str::<Value>(&v)).transpose()?.unwrap_or(Value::Null), "sources": sources});
    fingerprint(&value)
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn temporary_pool() -> (TempDir, SqlitePool) {
        let directory = tempfile::tempdir().unwrap();
        let pool = crate::db::init_db(&directory.path().join("replacement.db"))
            .await
            .unwrap();
        (directory, pool)
    }

    async fn saved_legacy_example(pool: &SqlitePool) -> Source {
        let spec = super::super::legacy_sample_workflows::recognition_specs()
            .unwrap()
            .into_iter()
            .find_map(|spec| {
                crate::services::workflow::validate(serde_json::to_value(&spec).ok()?, "UTC").ok()
            })
            .expect("frozen legacy examples contain a resolvable workflow");
        crate::services::workflow::persist(pool, spec, None, false, "UTC")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn ordinary_saves_refresh_builtin_origin_fingerprints() {
        let (_directory, pool) = temporary_pool().await;
        let source = seed_all(&pool).await.unwrap().remove(0);
        let previous = source.config["__builtin_workflow"]["fingerprint"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut spec: crate::services::workflow::WorkflowSpec =
            serde_json::from_value(source.config["__workflow_spec"].clone()).unwrap();
        spec.metadata.name.push_str(" — edited");
        let saved =
            crate::services::workflow::persist(&pool, spec, Some(source.id.clone()), false, "UTC")
                .await
                .unwrap();
        let marker = &saved.config["__builtin_workflow"];
        assert_eq!(
            marker["template_id"],
            source.config["__builtin_workflow"]["template_id"]
        );
        assert_ne!(marker["fingerprint"].as_str().unwrap(), previous);
        let child_marker: String = sqlx::query_scalar(
            "SELECT config FROM sources WHERE workflow_id=? AND id<>? ORDER BY order_index LIMIT 1",
        )
        .bind(&source.id)
        .bind(&source.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        let child_marker: Value = serde_json::from_str(&child_marker).unwrap();
        assert_eq!(child_marker["__builtin_workflow"], *marker);
        pool.close().await;
    }

    #[tokio::test]
    async fn preview_recognizes_only_exact_examples_and_replacement_preserves_job_history() {
        let (directory, pool) = temporary_pool().await;
        let legacy = saved_legacy_example(&pool).await;
        let custom_spec = crate::services::workflow::validate(
            super::super::template_catalog::seed_inputs().unwrap()[0]
                .config
                .as_ref()
                .unwrap()["__workflow_spec"]
                .clone(),
            "UTC",
        )
        .unwrap();
        let custom = crate::services::workflow::persist(&pool, custom_spec, None, false, "UTC")
            .await
            .unwrap();
        let preview = replacement_preview(&pool, "UTC").await.unwrap();
        let exact = preview
            .candidates
            .iter()
            .find(|candidate| candidate.id == legacy.id)
            .unwrap();
        let modified = preview
            .candidates
            .iter()
            .find(|candidate| candidate.id == custom.id)
            .unwrap();
        assert!(exact.selected_by_default);
        assert!(!modified.selected_by_default);
        sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,favorited_at,source_name_snapshot) VALUES('protected','identity:protected',?,?, 'Software Engineer','Example','https://example.com/job','2026-10-23','2026-10-23','applied','2026-10-23','keep me',1,'2026-10-23',?)")
            .bind(&legacy.id).bind(&legacy.id).bind(&legacy.name).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO job_status_events(job_id,from_status,to_status,changed_at) VALUES('protected','viewed','applied','2026-10-23')").execute(&pool).await.unwrap();
        let backup = directory.path().join("before.db");
        let report = replace(
            &pool,
            &[WorkflowReplacementSelection {
                id: legacy.id.clone(),
                fingerprint: exact.fingerprint.clone(),
            }],
            &backup,
            "UTC",
        )
        .await
        .unwrap();
        assert_eq!(report.removed_ids, vec![legacy.id.clone()]);
        assert!(backup.exists());
        assert!(SourcesRepo::get(&pool, &custom.id).await.is_ok());
        assert_eq!(
            SourcesRepo::list(&pool)
                .await
                .unwrap()
                .iter()
                .filter(|s| s
                    .config
                    .pointer("/__builtin_workflow/template_id")
                    .is_some())
                .count(),
            3
        );
        let job: (Option<String>, Option<String>, String, i64, String) = sqlx::query_as(
            "SELECT source_id,workflow_id,notes,favorited,status FROM jobs WHERE id='protected'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(job, (None, None, "keep me".into(), 1, "applied".into()));
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM job_status_events WHERE job_id='protected'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        let again = replace(&pool, &[], &directory.path().join("unused.db"), "UTC")
            .await
            .unwrap();
        assert!(again.created.is_empty());
        assert!(again.backup_path.is_none());
        pool.close().await;
    }

    #[tokio::test]
    async fn stale_preview_backup_failure_and_target_save_failure_leave_existing_workflow_unchanged(
    ) {
        let (directory, pool) = temporary_pool().await;
        let legacy = saved_legacy_example(&pool).await;
        let preview = replacement_preview(&pool, "UTC").await.unwrap();
        let exact = preview
            .candidates
            .iter()
            .find(|candidate| candidate.id == legacy.id)
            .unwrap()
            .clone();
        sqlx::query("UPDATE workflows SET name='Concurrent edit' WHERE id=?")
            .bind(&legacy.id)
            .execute(&pool)
            .await
            .unwrap();
        let stale = replace(
            &pool,
            &[WorkflowReplacementSelection {
                id: legacy.id.clone(),
                fingerprint: exact.fingerprint.clone(),
            }],
            &directory.path().join("stale.db"),
            "UTC",
        )
        .await
        .unwrap_err();
        assert!(stale
            .to_string()
            .contains("Workflow changed while replacing test workflows"));
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT name FROM workflows WHERE id=?")
                .bind(&legacy.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "Concurrent edit"
        );

        let current = replacement_preview(&pool, "UTC")
            .await
            .unwrap()
            .candidates
            .into_iter()
            .find(|c| c.id == legacy.id)
            .unwrap();
        let failed_backup = directory.path().join("not-a-backup.db");
        std::fs::write(&failed_backup, b"preserve").unwrap();
        assert!(replace(
            &pool,
            &[WorkflowReplacementSelection {
                id: legacy.id.clone(),
                fingerprint: current.fingerprint.clone()
            }],
            &failed_backup,
            "UTC"
        )
        .await
        .is_err());
        assert_eq!(std::fs::read(&failed_backup).unwrap(), b"preserve");
        assert!(SourcesRepo::get(&pool, &legacy.id).await.is_ok());

        std::fs::remove_file(&failed_backup).unwrap();
        sqlx::query("CREATE TRIGGER fail_replacement_target BEFORE INSERT ON workflows WHEN NEW.name LIKE 'Software Engineering%' BEGIN SELECT RAISE(ABORT,'injected target failure'); END;").execute(&pool).await.unwrap();
        assert!(replace(
            &pool,
            &[WorkflowReplacementSelection {
                id: legacy.id.clone(),
                fingerprint: current.fingerprint
            }],
            &directory.path().join("rollback.db"),
            "UTC"
        )
        .await
        .is_err());
        assert!(SourcesRepo::get(&pool, &legacy.id).await.is_ok());
        assert_eq!(
            SourcesRepo::list(&pool)
                .await
                .unwrap()
                .iter()
                .filter(|s| s
                    .config
                    .pointer("/__builtin_workflow/template_id")
                    .is_some())
                .count(),
            0
        );
        pool.close().await;
    }
}
