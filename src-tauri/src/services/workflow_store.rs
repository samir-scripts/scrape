//! Atomic persistence for a workflow and its owned extraction sources.
use super::workflow::{workflow_source_inputs, WorkflowSpec};
use crate::{
    db::sources::{Source, SourcesRepo},
    error::{AppError, Result},
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub(super) async fn save(
    pool: &SqlitePool,
    mut spec: WorkflowSpec,
    id: Option<String>,
    imported: bool,
    timezone: &str,
    repair: bool,
) -> Result<Source> {
    // Validate every child before writing; preserve the caller's effective filters.
    super::workflow::validate(serde_json::to_value(&spec)?, timezone)?;
    let old = match &id {
        Some(id) => Some(SourcesRepo::get(pool, id).await?),
        None => None,
    };
    let expected: Option<String> = if let Some(id) = &id {
        sqlx::query_scalar("SELECT config FROM sources WHERE id=?")
            .bind(id)
            .fetch_optional(pool)
            .await?
    } else {
        None
    };
    let workflow_id = id.unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let mut tx = pool.begin().await?;
    if old.is_some() {
        let owner: Option<String> =
            sqlx::query_scalar("SELECT workflow_id FROM sources WHERE id=?")
                .bind(&workflow_id)
                .fetch_one(&mut *tx)
                .await?;
        if owner.as_deref() != Some(workflow_id.as_str()) {
            return Err(AppError::Validation(
                "Save the parent workflow; a child source cannot become another workflow's parent."
                    .into(),
            ));
        }
    }

    if let Some(expected) = expected {
        let current: String = sqlx::query_scalar("SELECT config FROM sources WHERE id=?")
            .bind(&workflow_id)
            .fetch_one(&mut *tx)
            .await?;
        if current != expected {
            return Err(AppError::Validation(
                "Workflow changed while saving. Reload and retry.".into(),
            ));
        }
    }
    let mut owners = Vec::new();
    for source in &spec.sources {
        let owner: Option<Option<String>> =
            sqlx::query_scalar("SELECT workflow_id FROM sources WHERE id=?")
                .bind(&source.id)
                .fetch_optional(&mut *tx)
                .await?;
        if owner
            .as_ref()
            .is_some_and(|owner| owner.as_deref() != Some(workflow_id.as_str()))
        {
            return Err(AppError::Validation(format!(
                "sources.{}: belongs to another workflow; use a distinct source ID",
                source.id
            )));
        }
        owners.push(owner.is_some());
    }
    let original_inputs = workflow_source_inputs(&spec, timezone)?;
    let primary = if let Some(index) = spec.sources.iter().position(|s| s.id == workflow_id) {
        index
    } else if repair {
        let previous = old.as_ref().unwrap();
        original_inputs.iter().position(|input|input.kind==previous.kind &&
                (input.identifier==previous.identifier || input.config.as_ref().and_then(|c|c.get("url"))==previous.config.get("url")))
                .ok_or_else(||AppError::Validation("Cannot identify the historical parent source safely; review the saved workflow before repair.".into()))?
    } else {
        spec.sources.iter().position(|s| s.enabled).unwrap_or(0)
    };
    if !repair
        && old.is_some()
        && spec.sources[primary].id != workflow_id
        && owners.iter().any(|owned| *owned)
    {
        return Err(AppError::Validation(
            "Keep the parent source in this workflow; its identity is referenced by saved jobs."
                .into(),
        ));
    }
    for (index, entry) in spec.sources.iter_mut().enumerate() {
        if index == primary {
            entry.id = workflow_id.clone();
        } else if !owners[index] {
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT id FROM sources WHERE workflow_id=? AND kind=? AND identifier=? AND id!=?",
            )
            .bind(&workflow_id)
            .bind(
                serde_json::to_value(original_inputs[index].kind)?
                    .as_str()
                    .unwrap(),
            )
            .bind(&original_inputs[index].identifier)
            .bind(&workflow_id)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(existing) = existing {
                entry.id = existing;
                owners[index] = true;
            } else {
                entry.id = uuid::Uuid::now_v7().to_string();
            }
        }
    }
    let mut inputs = workflow_source_inputs(&spec, timezone)?;
    if old
        .as_ref()
        .is_some_and(|old| old.kind != inputs[primary].kind)
    {
        return Err(AppError::Validation(
            "Create a new workflow to change the source adapter".into(),
        ));
    }
    let mut states = Vec::new();
    for (entry, input) in spec.sources.iter().zip(&inputs) {
        let previous: Option<(bool, Option<String>)> = sqlx::query_as(
            "SELECT script_trusted,script_sha256 FROM sources WHERE id=? AND workflow_id=?",
        )
        .bind(&entry.id)
        .bind(&workflow_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (trusted, hash) = super::script_trust::saved_state(
            input.config.as_ref().unwrap_or(&Value::Null),
            previous.as_ref().is_some_and(|s| s.0),
            previous.as_ref().and_then(|s| s.1.as_deref()),
            imported,
        )?;
        states.push((trusted, hash.clone(), hash.is_some() && !trusted));
    }
    let pending = states.iter().any(|state| state.2);
    if pending {
        spec.metadata.enabled = Some(false);
        for (entry, state) in spec.sources.iter_mut().zip(&states) {
            if state.2 {
                entry.enabled = false;
            }
        }
        inputs = workflow_source_inputs(&spec, timezone)?;
    }
    let now = scrape_core::now_iso();
    let root = &inputs[primary];
    let mut root_config = root.config.clone().unwrap_or(json!({}));
    if let Some(index) = old
        .as_ref()
        .and_then(|s| s.config.get("__starting_source_index"))
    {
        root_config["__starting_source_index"] = index.clone();
    }
    let cadence = root.cadence.unwrap_or_default();
    let enabled = spec.metadata.enabled.unwrap_or(true) && !pending;
    let next = if repair {
        old.as_ref()
            .and_then(|s| s.next_run_at.clone())
            .filter(|_| !pending)
    } else {
        super::calendar::next_for_source(
            cadence,
            root.cadence_cron.as_deref(),
            &root_config,
            enabled,
            timezone,
            chrono::Utc::now(),
        )?
    };
    let filters = if repair {
        old.as_ref().unwrap().filters.clone()
    } else {
        root.filters.clone().unwrap_or(json!({}))
    };
    super::runner::resolve_worker_filters(&filters)?;
    if repair {
        let old = old
            .as_ref()
            .ok_or_else(|| AppError::Validation("Repair requires an existing workflow".into()))?;
        root_config = old.config.clone();
        root_config["__workflow_spec"] = serde_json::to_value(&spec)?;
        root_config["__source_id"] = json!(workflow_id);
        sqlx::query("UPDATE workflows SET config=json_set(config,'$.__workflow_spec',json(?)),needs_review=CASE WHEN ? THEN 1 ELSE needs_review END,enabled=CASE WHEN ? THEN 0 ELSE enabled END,next_run_at=CASE WHEN ? THEN NULL ELSE next_run_at END WHERE id=?")
            .bind(serde_json::to_string(&spec)?).bind(pending).bind(pending).bind(pending).bind(&workflow_id).execute(&mut *tx).await?;
    } else {
        sqlx::query("INSERT INTO workflows (id,name,enabled,cadence,cadence_cron,next_run_at,retention_days,retention_policy,filters,config,max_jobs_per_run,needs_review,created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,enabled=excluded.enabled,cadence=excluded.cadence,cadence_cron=excluded.cadence_cron,next_run_at=excluded.next_run_at,retention_days=excluded.retention_days,retention_policy=excluded.retention_policy,filters=excluded.filters,config=excluded.config,max_jobs_per_run=excluded.max_jobs_per_run,needs_review=excluded.needs_review")
            .bind(&workflow_id).bind(&spec.metadata.name).bind(enabled).bind(serde_json::to_value(cadence)?.as_str().unwrap()).bind(&root.cadence_cron).bind(&next)
            .bind(root.retention_days.unwrap_or(30)).bind(root.retention_policy.as_deref().unwrap_or("untracked_only")).bind(filters.to_string()).bind(root_config.to_string())
            .bind(spec.max_jobs_per_run.unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN)).bind(pending).bind(&now).execute(&mut *tx).await?;
    }
    for (index, ((entry, input), state)) in
        spec.sources.iter().zip(&inputs).zip(&states).enumerate()
    {
        let is_root = entry.id == workflow_id;
        if repair && owners[index] && !is_root {
            continue;
        }
        if repair && is_root {
            sqlx::query("UPDATE sources SET config=?,enabled=CASE WHEN ? THEN 0 ELSE enabled END,next_run_at=CASE WHEN ? THEN NULL ELSE next_run_at END WHERE id=?")
                .bind(root_config.to_string()).bind(pending).bind(pending).bind(&workflow_id).execute(&mut *tx).await?;
            continue;
        }
        let kind = serde_json::to_value(input.kind)?;
        let row_cadence = if is_root {
            cadence
        } else {
            scrape_core::SourceCadence::Manual
        };
        let row_enabled = if is_root {
            enabled
        } else {
            entry.enabled && !state.2
        };
        let config = if is_root {
            root_config.clone()
        } else {
            input.config.clone().unwrap_or(json!({}))
        };
        sqlx::query("INSERT INTO sources (id,workflow_id,kind,name,identifier,config,enabled,employer_hint,order_index,created_at,cadence,cadence_cron,next_run_at,retention_days,retention_policy,filters,script_trusted,script_sha256,needs_review) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET kind=excluded.kind,name=excluded.name,identifier=excluded.identifier,config=excluded.config,enabled=excluded.enabled,employer_hint=excluded.employer_hint,order_index=excluded.order_index,cadence=excluded.cadence,cadence_cron=excluded.cadence_cron,next_run_at=excluded.next_run_at,retention_days=excluded.retention_days,retention_policy=excluded.retention_policy,filters=excluded.filters,script_trusted=excluded.script_trusted,script_sha256=excluded.script_sha256,needs_review=excluded.needs_review")
            .bind(&entry.id).bind(&workflow_id).bind(kind.as_str().unwrap()).bind(if is_root {&spec.metadata.name} else {&entry.label}).bind(&input.identifier).bind(config.to_string()).bind(row_enabled)
            .bind(&entry.employer_hint).bind(index as i64).bind(&now).bind(serde_json::to_value(row_cadence)?.as_str().unwrap()).bind(if is_root {root.cadence_cron.as_deref()} else {None}).bind(if is_root {next.as_deref()} else {None})
            .bind(root.retention_days.unwrap_or(30)).bind(root.retention_policy.as_deref().unwrap_or("untracked_only")).bind(filters.to_string()).bind(state.0).bind(&state.1).bind(state.2).execute(&mut *tx).await?;
    }
    if !repair {
        let ids: Vec<_> = spec.sources.iter().map(|s| s.id.as_str()).collect();
        let old_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM sources WHERE workflow_id=?")
            .bind(&workflow_id)
            .fetch_all(&mut *tx)
            .await?;
        for id in old_ids {
            if !ids.contains(&id.as_str()) {
                sqlx::query(
                    "UPDATE sources SET enabled=0,cadence='manual',next_run_at=NULL WHERE id=?",
                )
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    tx.commit().await?;
    SourcesRepo::get(pool, &workflow_id).await
}

/// Explicit forward data repair. The caller supplies a fresh recovery path;
/// backup failure aborts before any mutation. No automatic seeding rewrites edits.
pub async fn repair_ownership(
    pool: &SqlitePool,
    id: &str,
    backup: &std::path::Path,
    timezone: &str,
) -> Result<Source> {
    let old = SourcesRepo::get(pool, id).await?;
    let spec: WorkflowSpec =
        serde_json::from_value(old.config.get("__workflow_spec").cloned().ok_or_else(|| {
            AppError::Validation("Workflow has no saved specification to repair".into())
        })?)?;
    if spec.sources.len() < 2 {
        return Ok(old);
    }
    let mut missing = false;
    for entry in &spec.sources {
        let owner: Option<Option<String>> =
            sqlx::query_scalar("SELECT workflow_id FROM sources WHERE id=?")
                .bind(&entry.id)
                .fetch_optional(pool)
                .await?;
        match owner {
            Some(owner) if owner.as_deref() != Some(id) => {
                return Err(AppError::Validation(format!(
                    "Source {} belongs to another workflow; repair refused",
                    entry.id
                )))
            }
            None => missing = true,
            _ => {}
        }
    }
    if !missing {
        return Ok(old);
    }
    crate::db::backup_to(pool, backup).await?;
    save(pool, spec, Some(id.into()), false, timezone, true).await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec() -> WorkflowSpec {
        super::super::workflow::validate(json!({"version":6,"metadata":{"name":"Authored atomic workflow"},"sources":[
            {"id":"first","label":"First","enabled":true,"extractor":{"kind":"greenhouse","url":"atomic"}},
            {"id":"second","label":"Second","enabled":true,"extractor":{"kind":"lever","url":"atomic"}}
        ],"schedule":{"cadence":"manual"}}),"UTC").unwrap()
    }
    #[tokio::test]
    async fn child_write_failure_rolls_back_create_and_edit_and_retry_preserves_identity() {
        let pool = crate::db::init_memory_db().await.unwrap();
        sqlx::query("CREATE TRIGGER fail_child BEFORE INSERT ON sources WHEN NEW.order_index=1 BEGIN SELECT RAISE(ABORT,'injected child failure'); END;").execute(&pool).await.unwrap();
        assert!(save(&pool, spec(), None, false, "UTC", false)
            .await
            .is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM workflows")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sources")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        sqlx::query("DROP TRIGGER fail_child")
            .execute(&pool)
            .await
            .unwrap();
        let parent = save(&pool, spec(), None, false, "UTC", false)
            .await
            .unwrap();
        let original = parent.config.clone();
        let mut edit: WorkflowSpec =
            serde_json::from_value(original["__workflow_spec"].clone()).unwrap();
        edit.metadata.name = "Changed name".into();
        sqlx::query("CREATE TRIGGER fail_child BEFORE INSERT ON sources WHEN NEW.order_index=1 BEGIN SELECT RAISE(ABORT,'injected child failure'); END;").execute(&pool).await.unwrap();
        assert!(save(
            &pool,
            edit.clone(),
            Some(parent.id.clone()),
            false,
            "UTC",
            false
        )
        .await
        .is_err());
        let after = SourcesRepo::get(&pool, &parent.id).await.unwrap();
        assert_eq!(after.config, original);
        assert_eq!(after.name, parent.name);
        sqlx::query("DROP TRIGGER fail_child")
            .execute(&pool)
            .await
            .unwrap();
        let after = save(&pool, edit, Some(parent.id.clone()), false, "UTC", false)
            .await
            .unwrap();
        assert_eq!(after.id, parent.id);
        assert_eq!(
            after.config["__workflow_spec"]["sources"],
            original["__workflow_spec"]["sources"]
        );
        // A separate workflow can use exactly the same external boards.
        let other = save(&pool, spec(), None, false, "UTC", false)
            .await
            .unwrap();
        assert_ne!(parent.id, other.id);
        let other_child = other.config["__workflow_spec"]["sources"][1]["id"]
            .as_str()
            .unwrap();
        assert!(
            save(&pool, spec(), Some(other_child.into()), false, "UTC", false)
                .await
                .is_err()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM workflows")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );

        let mut stealing: WorkflowSpec =
            serde_json::from_value(after.config["__workflow_spec"].clone()).unwrap();
        stealing.sources[1].id = other.config["__workflow_spec"]["sources"][1]["id"]
            .as_str()
            .unwrap()
            .into();
        assert!(save(&pool, stealing, Some(parent.id), false, "UTC", false)
            .await
            .is_err());
    }
    #[tokio::test]
    async fn reordering_removal_and_readding_keep_owned_ids_and_saved_jobs() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let parent = save(&pool, spec(), None, false, "UTC", false)
            .await
            .unwrap();
        let mut spec: WorkflowSpec =
            serde_json::from_value(parent.config["__workflow_spec"].clone()).unwrap();
        let child = spec.sources[1].clone();
        let job=crate::services::ingestion::scraped_job_from_event(serde_json::from_value(json!({"type":"job","source_id":child.id,"external_id":"retained","title":"Authored retained source job","company":"Authored","role_type":"unknown","url":"https://fixture.invalid/retained"})).unwrap()).unwrap();
        crate::db::jobs::JobsRepo::upsert_scraped(&pool, &[job])
            .await
            .unwrap();
        spec.sources.swap(0, 1);
        save(
            &pool,
            spec.clone(),
            Some(parent.id.clone()),
            false,
            "UTC",
            false,
        )
        .await
        .unwrap();
        spec.sources.retain(|s| s.id != child.id);
        save(
            &pool,
            spec.clone(),
            Some(parent.id.clone()),
            false,
            "UTC",
            false,
        )
        .await
        .unwrap();
        let removed = SourcesRepo::get(&pool, &child.id).await.unwrap();
        assert!(!removed.enabled && removed.next_run_at.is_none());
        let mut readded = child.clone();
        readded.id = "new-local-alias".into();
        spec.sources.push(readded);
        let restored = save(&pool, spec, Some(parent.id.clone()), false, "UTC", false)
            .await
            .unwrap();
        assert_eq!(
            restored.config["__workflow_spec"]["sources"][1]["id"],
            child.id
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT source_id FROM jobs")
                .fetch_one(&pool)
                .await
                .unwrap(),
            child.id
        );
        assert_eq!(SourcesRepo::list(&pool).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn explicit_repair_backs_up_preserves_edits_and_is_idempotent_after_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("repair.db");
        let pool = crate::db::init_db(&path).await.unwrap();
        let input = crate::db::sources::SourcesRepo::sample_workflows_suite().remove(4);
        let parent = SourcesRepo::create(&pool, input).await.unwrap();
        sqlx::query("UPDATE sources SET name='Personal name',filters='{}',next_run_at='2030-01-01T09:00:00Z' WHERE id=?").bind(&parent.id).execute(&pool).await.unwrap();
        let job=crate::services::ingestion::scraped_job_from_event(serde_json::from_value(json!({"type":"job","source_id":parent.id,"external_id":"protected","title":"Authored protected repair fixture","company":"Authored","role_type":"unknown","url":"https://fixture.invalid/protected"})).unwrap()).unwrap();
        crate::db::jobs::JobsRepo::upsert_scraped(&pool, &[job])
            .await
            .unwrap();
        sqlx::query("UPDATE jobs SET notes='Keep note',favorited=1,status='interviewing'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO job_status_events(job_id,to_status,changed_at) SELECT id,'interviewing','2026-01-01' FROM jobs").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES ('authored-block','Protected','Authored','2026-01-01')").execute(&pool).await.unwrap();
        let before = SourcesRepo::get(&pool, &parent.id).await.unwrap();
        let missing_dir = directory.path().join("missing/backup.db");
        assert!(repair_ownership(&pool, &parent.id, &missing_dir, "UTC")
            .await
            .is_err());
        assert_eq!(
            SourcesRepo::get(&pool, &parent.id).await.unwrap().config,
            before.config
        );
        sqlx::query("CREATE TRIGGER fail_repair BEFORE INSERT ON sources WHEN NEW.order_index=1 BEGIN SELECT RAISE(ABORT,'injected repair failure'); END;").execute(&pool).await.unwrap();
        assert!(repair_ownership(
            &pool,
            &parent.id,
            &directory.path().join("failed-repair.db"),
            "UTC"
        )
        .await
        .is_err());
        assert_eq!(
            SourcesRepo::get(&pool, &parent.id).await.unwrap().config,
            before.config
        );
        sqlx::query("DROP TRIGGER fail_repair")
            .execute(&pool)
            .await
            .unwrap();
        let backup = directory.path().join("before.db");
        let repaired = repair_ownership(&pool, &parent.id, &backup, "UTC")
            .await
            .unwrap();
        let parent_config: String = sqlx::query_scalar("SELECT config FROM workflows WHERE id=?")
            .bind(&parent.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        let parent_config: Value = serde_json::from_str(&parent_config).unwrap();
        assert_eq!(
            parent_config["__workflow_spec"],
            repaired.config["__workflow_spec"]
        );
        let state: (String, bool, String, String) =
            sqlx::query_as("SELECT notes,favorited,status,source_id FROM jobs")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            state,
            (
                "Keep note".into(),
                true,
                "interviewing".into(),
                parent.id.clone()
            )
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM job_status_events")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM blocked_postings")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert!(backup.is_file());
        assert_eq!(repaired.name, before.name);
        assert_eq!(repaired.filters, before.filters);
        assert_eq!(repaired.next_run_at, before.next_run_at);
        assert_eq!(repaired.identifier, before.identifier);
        assert_eq!(repaired.id, before.id);
        let children: Vec<String> =
            sqlx::query_scalar("SELECT id FROM sources WHERE workflow_id=?")
                .bind(&parent.id)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(children.len(), 2);
        for entry in repaired.config["__workflow_spec"]["sources"]
            .as_array()
            .unwrap()
        {
            assert!(children.contains(&entry["id"].as_str().unwrap().to_string()));
        }
        pool.close().await;
        let pool = crate::db::init_db(&path).await.unwrap();
        let unused = directory.path().join("unused.db");
        repair_ownership(&pool, &parent.id, &unused, "UTC")
            .await
            .unwrap();
        assert!(!unused.exists());
        let snapshot = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=ro", backup.display()))
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sources")
                .fetch_one(&snapshot)
                .await
                .unwrap(),
            1
        );
        snapshot.close().await;
        pool.close().await;
    }
}
