//! Execution preparation and durable, credential-redacted run evidence.
//! Snapshots describe observed state; they never enable sources or repair data.
use crate::db::{
    jobs::{JobsRepo, ScrapedJob, UpsertStats},
    runs::{LogLevel, RunCounts, RunsRepo},
    sources::Source,
};
use crate::error::Result;
use scrape_core::{WorkerConfig, WorkerLimits, WorkerSource, PROTOCOL_VERSION};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

/// The same preparation path is used by manual, scheduled and Run all execution.
pub fn prepare(source: &Source, timezone: &str, started_at: &str) -> Result<WorkerConfig> {
    let spec = source
        .config
        .get("__workflow_spec")
        .map(|v| serde_json::from_value::<super::workflow::WorkflowSpec>(v.clone()))
        .transpose()?;
    let (sources, cap) = if let Some(spec) = spec {
        let inputs = super::workflow::workflow_source_inputs(&spec, timezone)?;
        let sources = spec
            .sources
            .iter()
            .zip(inputs)
            .filter(|(s, _)| s.enabled)
            .map(|(s, input)| WorkerSource {
                id: if spec.sources.len() == 1 {
                    source.id.clone()
                } else {
                    s.id.clone()
                },
                kind: input.kind,
                name: s
                    .employer_hint
                    .clone()
                    .or_else(|| spec.metadata.company.clone())
                    .unwrap_or_else(|| s.label.clone()),
                identifier: input.identifier,
                config: input.config.unwrap_or(Value::Null),
            })
            .collect();
        (
            sources,
            Some(
                spec.max_jobs_per_run
                    .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN),
            ),
        )
    } else {
        (
            vec![WorkerSource {
                id: source.id.clone(),
                kind: source.kind,
                name: source
                    .config
                    .get("__company")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown company")
                    .into(),
                identifier: source.identifier.clone(),
                config: source.config.clone(),
            }],
            None,
        )
    };
    Ok(WorkerConfig {
        version: PROTOCOL_VERSION,
        sources,
        filters: super::runner::resolve_worker_filters(&source.filters, Default::default())?,
        run_started_at: Some(started_at.into()),
        limits: WorkerLimits::default(),
        max_jobs_per_run: cap,
        starting_source_index: Some(
            source
                .config
                .get("__starting_source_index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
        ),
        ..Default::default()
    })
}

fn secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace(['-', '_'], "");
    [
        "password",
        "passwd",
        "secret",
        "token",
        "apikey",
        "authorization",
        "credential",
        "cookie",
        "storagestate",
        "sessionmaterial",
    ]
    .iter()
    .any(|word| key.contains(word))
        || matches!(
            key.as_str(),
            "headers" | "invalidoriginalconfig" | "invalidoriginalfilters"
        )
}

/// Only diagnostic copies are sanitized. The actual worker config is untouched.
/// Executable bodies/paths/arguments may contain arbitrary secrets and are omitted.
pub fn redact(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if secret_key(key) {
                    *value = Value::String("[REDACTED]".into());
                } else if key == "script" {
                    if let Some(script) = value.as_object_mut() {
                        for (key, value) in script {
                            if key != "runtime" && key != "timeout_s" {
                                *value = match value {
                                    Value::Array(_) => json!(["[REDACTED]"]),
                                    _ => Value::String("[REDACTED]".into()),
                                };
                            }
                        }
                    } else {
                        *value = Value::String("[REDACTED]".into());
                    }
                } else {
                    redact(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact(value);
            }
        }
        Value::String(text) => {
            if let Ok(mut url) = url::Url::parse(text) {
                if matches!(url.scheme(), "http" | "https") {
                    let _ = url.set_username("");
                    let _ = url.set_password(None);
                    let pairs: Vec<_> = url
                        .query_pairs()
                        .map(|(key, value)| {
                            let value = if secret_key(&key) {
                                "[REDACTED]".into()
                            } else {
                                value.into_owned()
                            };
                            (key.into_owned(), value)
                        })
                        .collect();
                    if url.query().is_some() {
                        url.query_pairs_mut().clear().extend_pairs(pairs);
                    }
                    url.set_fragment(None);
                    *text = url.into();
                }
            }
        }
        _ => {}
    }
}

pub fn safe_config(config: &WorkerConfig) -> Result<WorkerConfig> {
    let mut value = serde_json::to_value(config)?;
    redact(&mut value);
    // Suppressed identities can expose saved destinations; export their count in context.
    value["blocked_postings"] = json!([]);
    Ok(serde_json::from_value(value)?)
}

async fn stored_workflow(pool: &SqlitePool, id: &str) -> Result<Option<Value>> {
    let Some(row) =
        sqlx::query("SELECT enabled, needs_review, config, filters FROM workflows WHERE id=?")
            .bind(id)
            .fetch_optional(pool)
            .await?
    else {
        return Ok(None);
    };
    let config: String = row.get("config");
    let filters: String = row.get("filters");
    Ok(Some(json!({
        "enabled": row.get::<i64, _>("enabled") == 1,
        "needs_review": row.get::<i64, _>("needs_review") == 1,
        "stored_spec": serde_json::from_str::<Value>(&config)?,
        "stored_filters": serde_json::from_str::<Value>(&filters)?,
    })))
}

/// Record before trust/runtime checks so failed preflight retains useful evidence.
/// The JSON log is per workflow, retaining all contexts in a Run all invocation.
pub async fn record_context(
    pool: &SqlitePool,
    run_id: &str,
    workflow_id: &str,
    config: &WorkerConfig,
) -> Result<()> {
    RunsRepo::record_config(pool, run_id, config).await?;
    let mut sources = Vec::new();
    let mut workflows = std::collections::BTreeMap::new();
    if let Some(workflow) = stored_workflow(pool, workflow_id).await? {
        workflows.insert(workflow_id.to_string(), workflow);
    }
    for source in &config.sources {
        let row = sqlx::query("SELECT workflow_id, enabled, needs_review, script_trusted, script_sha256, config, filters FROM sources WHERE id=?")
            .bind(&source.id).fetch_optional(pool).await?;
        if let Some(row) = row {
            let owner: Option<String> = row.get("workflow_id");
            if let Some(id) = &owner {
                if !workflows.contains_key(id) {
                    if let Some(workflow) = stored_workflow(pool, id).await? {
                        workflows.insert(id.clone(), workflow);
                    }
                }
            }
            let stored_config: String = row.get("config");
            let stored_filters: String = row.get("filters");
            sources.push(json!({"source_id":source.id,"workflow_id":owner,"source_row_exists":true,"enabled":row.get::<i64,_>("enabled") == 1,"needs_review":row.get::<i64,_>("needs_review") == 1,"script_trusted":row.get::<i64,_>("script_trusted") == 1,"script_sha256":row.get::<Option<String>,_>("script_sha256"),"stored_config":serde_json::from_str::<Value>(&stored_config)?,"stored_filters":serde_json::from_str::<Value>(&stored_filters)?}));
        } else {
            sources.push(json!({"source_id":source.id,"source_row_exists":false,"issue":"source_row_missing","recovery":"Open and save the workflow to review source ownership before running again."}));
        }
    }
    let missing = sources.iter().any(|s| s["source_row_exists"] == false);
    let mut context = json!({"diagnostic_version":1,"workflow_id":workflow_id,"seed_origin":null,"stage":"execution_preparation","requested_protocol_version":config.version,"blocked_identity_count":config.blocked_postings.len(),"effective_config":safe_config(config)?,"sources":sources,"workflows":workflows});
    redact(&mut context);
    RunsRepo::add_log(
        pool,
        run_id,
        if missing {
            LogLevel::Warn
        } else {
            LogLevel::Info
        },
        Some(workflow_id),
        &format!(
            "Workflow execution context: {}",
            serde_json::to_string(&context)?
        ),
    )
    .await
}

pub async fn preflight(
    pool: &SqlitePool,
    run_id: &str,
    workflow_id: &str,
    config: &WorkerConfig,
) -> Result<()> {
    record_context(pool, run_id, workflow_id, config).await?;
    if let Err(error) = super::script_trust::validate_execution(pool, &config.sources).await {
        RunsRepo::add_log(
            pool,
            run_id,
            LogLevel::Error,
            Some(workflow_id),
            &format!(
                "Execution preflight failed before discovery: {}",
                scrape_core::extraction::redact_script_stderr(error.to_string().as_bytes())
            ),
        )
        .await?;
        return Err(error);
    }
    Ok(())
}

/// A failed transaction must retain its preceding counters and explicit failure stage.
pub async fn persist_batch(
    pool: &SqlitePool,
    run_id: &str,
    batch: &[ScrapedJob],
    counts: &mut RunCounts,
) -> Result<UpsertStats> {
    match JobsRepo::upsert_scraped(pool, batch).await {
        Ok(stats) => {
            counts.inserted += stats.inserted;
            counts.updated += stats.updated;
            counts.blocked += stats.blocked;
            RunsRepo::record_counts(pool, run_id, counts).await?;
            Ok(stats)
        }
        Err(error) => {
            counts.failed += batch.len() as i64;
            RunsRepo::record_counts(pool, run_id, counts).await?;
            let message = format!("Persistence failed for a batch of {} accepted jobs; this batch was not committed. Earlier committed records remain saved. {}", batch.len(), error);
            RunsRepo::add_log(pool, run_id, LogLevel::Error, None, &message).await?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{init_memory_db, runs::TriggerType, sources::SourcesRepo};
    use crate::services::{ingestion::scraped_job_from_event, sample_workflows};

    #[tokio::test]
    async fn real_seed_service_context_retains_each_workflow_and_observed_ownership() {
        let pool = init_memory_db().await.unwrap();
        let sources = sample_workflows::seed_all(&pool).await.unwrap();
        assert_eq!(sources.len(), 6);
        assert!(sample_workflows::seed_all(&pool).await.unwrap().is_empty());
        let run = RunsRepo::start(&pool, TriggerType::Manual).await.unwrap();
        for source in &sources {
            let config = prepare(source, "UTC", &run.started_at).unwrap();
            record_context(&pool, &run.id, &source.id, &config)
                .await
                .unwrap();
        }
        let logs = RunsRepo::get_logs(&pool, &run.id).await.unwrap();
        assert_eq!(logs.len(), 6, "Run all must keep each workflow's context");
        for (source, log) in sources.iter().zip(logs) {
            let context: Value = serde_json::from_str(
                log.message
                    .strip_prefix("Workflow execution context: ")
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(context["workflow_id"], source.id);
            assert_eq!(
                context["workflows"][&source.id]["stored_filters"],
                source.filters
            );
            assert_eq!(context["requested_protocol_version"], PROTOCOL_VERSION);
            let config = prepare(source, "UTC", &run.started_at).unwrap();
            for observed in context["sources"].as_array().unwrap() {
                let id = observed["source_id"].as_str().unwrap();
                let row = SourcesRepo::get(&pool, id).await;
                assert_eq!(observed["source_row_exists"], row.is_ok());
                if row.is_err() {
                    assert_eq!(observed["issue"], "source_row_missing");
                }
            }
            assert_eq!(
                context["effective_config"]["filters"],
                serde_json::to_value(&config.filters).unwrap()
            );
        }
    }

    #[tokio::test]
    async fn failed_script_preflight_keeps_context_and_explicit_pre_discovery_error() {
        let pool = init_memory_db().await.unwrap();
        let source = sample_workflows::seed_all(&pool)
            .await
            .unwrap()
            .pop()
            .unwrap();
        let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
            .await
            .unwrap();
        let config = prepare(&source, "UTC", &run.started_at).unwrap();
        let error = preflight(&pool, &run.id, &source.id, &config)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("trust"));
        let saved = RunsRepo::get(&pool, &run.id).await.unwrap();
        assert!(saved.effective_config.is_some());
        let logs = RunsRepo::get_logs(&pool, &run.id).await.unwrap();
        assert_eq!(logs.len(), 2);
        assert!(logs[1]
            .message
            .starts_with("Execution preflight failed before discovery:"));
        assert_eq!(logs[1].level, LogLevel::Error);
        let snapshot = &logs[0].message;
        assert!(snapshot.contains("stored_spec"));
        assert!(!snapshot.contains("netflix_extractor.js"));
        assert!(!snapshot.contains("amazon_extractor.js"));
        assert!(
            !SourcesRepo::get(&pool, &source.id)
                .await
                .unwrap()
                .script_trusted
        );
    }

    #[tokio::test]
    async fn context_keeps_legacy_filters_separate_from_resolved_rules() {
        let pool = init_memory_db().await.unwrap();
        let source = sample_workflows::seed_default(&pool).await.unwrap();
        let stored = json!({
            "locations": ["Remote", "Austin"],
            "include_keywords": ["Software"],
            "query": "title LIKE '%Engineer%' OR title LIKE '%Intern%'",
            "matching": {"keywords": {"any": ["Engineer"], "fields": ["title"]}},
        });
        for table in ["sources", "workflows"] {
            sqlx::query(&format!("UPDATE {table} SET filters=? WHERE id=?"))
                .bind(stored.to_string())
                .bind(&source.id)
                .execute(&pool)
                .await
                .unwrap();
        }
        let source = SourcesRepo::get(&pool, &source.id).await.unwrap();
        let run = RunsRepo::start(&pool, TriggerType::Manual).await.unwrap();
        let config = prepare(&source, "UTC", &run.started_at).unwrap();
        record_context(&pool, &run.id, &source.id, &config)
            .await
            .unwrap();
        let log = RunsRepo::get_logs(&pool, &run.id).await.unwrap().remove(0);
        let snapshot: Value = serde_json::from_str(
            log.message
                .strip_prefix("Workflow execution context: ")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(snapshot["workflows"][&source.id]["stored_filters"], stored);
        assert_eq!(
            snapshot["effective_config"]["filters"],
            serde_json::to_value(&config.filters).unwrap()
        );
        assert_eq!(
            SourcesRepo::get(&pool, &source.id).await.unwrap().filters,
            stored
        );
        // History lookup must find the context in a Run all run as well.
        assert_eq!(
            RunsRepo::list_by_source(&pool, &source.id, 10)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    fn candidate(source_id: &str, id: &str) -> ScrapedJob {
        scraped_job_from_event(serde_json::from_value(json!({"type":"job","source_id":source_id,"external_id":id,"title":"Authored diagnostic fixture","company":"Authored","url":format!("https://fixture.invalid/jobs/{id}"),"role_type":"unknown"})).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn failed_batch_keeps_prior_commits_and_persists_failure_stage_and_counts() {
        let pool = init_memory_db().await.unwrap();
        let source = sample_workflows::seed_default(&pool).await.unwrap();
        let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
            .await
            .unwrap();
        let mut counts = RunCounts::default();
        persist_batch(
            &pool,
            &run.id,
            &[candidate(&source.id, "first")],
            &mut counts,
        )
        .await
        .unwrap();
        let batch = [
            candidate(&source.id, "second"),
            candidate("missing-source", "third"),
        ];
        let error = persist_batch(&pool, &run.id, &batch, &mut counts)
            .await
            .unwrap_err();
        assert!(matches!(error, crate::error::AppError::Database(_)));
        assert!(error.to_string().contains("FOREIGN KEY"));
        let saved = RunsRepo::get(&pool, &run.id).await.unwrap().counts.unwrap();
        assert_eq!((saved.inserted, saved.updated, saved.failed), (1, 0, 2));
        let rows = JobsRepo::list(&pool, Default::default()).await.unwrap();
        assert_eq!(
            rows.total, 1,
            "failing batch must roll back its otherwise-valid first job"
        );
        assert_eq!(rows.items[0].external_id.as_deref(), Some("first"));
        let logs = RunsRepo::get_logs(&pool, &run.id).await.unwrap();
        assert!(logs[0]
            .message
            .starts_with("Persistence failed for a batch of 2 accepted jobs"));
        assert!(logs[0]
            .message
            .contains("Earlier committed records remain saved"));
    }
}
