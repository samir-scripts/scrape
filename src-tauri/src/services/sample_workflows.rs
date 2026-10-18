//! Shared seed service behind the desktop commands and isolated integration tests.
//! Every template uses the canonical atomic workflow persistence path.
use crate::db::sources::{Source, SourcesRepo};
use crate::error::Result;
use sqlx::SqlitePool;

pub async fn seed_default(pool: &SqlitePool) -> Result<Source> {
    if let Some(existing) = SourcesRepo::list(pool)
        .await?
        .into_iter()
        .find(|s| s.identifier == "cloudflare" && s.kind == scrape_core::SourceKind::Greenhouse)
    {
        return Ok(existing);
    }
    seed_input(pool, SourcesRepo::default_sample_workflow_input()).await
}

#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SeedWorkflowReport {
    pub created: Vec<Source>,
    pub already_present: Vec<Source>,
    pub needs_review: Vec<String>,
}

pub async fn seed_all(pool: &SqlitePool) -> Result<Vec<Source>> {
    Ok(seed_all_report(pool).await?.created)
}

pub async fn seed_all_report(pool: &SqlitePool) -> Result<SeedWorkflowReport> {
    let mut report = SeedWorkflowReport {
        created: vec![],
        already_present: vec![],
        needs_review: vec![],
    };
    let existing = SourcesRepo::list(pool).await?;
    for input in SourcesRepo::sample_workflows_suite() {
        let spec = crate::services::workflow::validate(
            input.config.as_ref().unwrap()["__workflow_spec"].clone(),
            "UTC",
        )?;
        let canonical = crate::services::workflow::source_input(&spec, "UTC")?;
        if let Some(source) = existing.iter().find(|s| {
            (s.identifier == canonical.identifier || s.identifier == input.identifier)
                && s.kind == input.kind
        }) {
            if source.needs_review {
                report.needs_review.push(source.id.clone());
            }
            report.already_present.push(source.clone());
        } else {
            let source = seed_input(pool, input).await?;
            if source.needs_review {
                report.needs_review.push(source.id.clone());
            }
            report.created.push(source);
        }
    }
    Ok(report)
}

async fn seed_input(
    pool: &SqlitePool,
    input: crate::db::sources::CreateSourceInput,
) -> Result<Source> {
    let timezone = crate::db::settings::SettingsRepo::get_value::<String>(pool, "system.timezone")
        .await?
        .unwrap_or_else(|| "UTC".into());
    let spec = crate::services::workflow::validate(
        input.config.as_ref().unwrap()["__workflow_spec"].clone(),
        &timezone,
    )?;
    crate::services::workflow::persist(pool, spec, None, true, &timezone).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn seeds_materialize_owned_children_without_independent_schedules() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let seeded = seed_all(&pool).await.unwrap();
        assert_eq!(seeded.len(), 6);
        assert_eq!(SourcesRepo::list(&pool).await.unwrap().len(), 6);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sources")
                .fetch_one(&pool)
                .await
                .unwrap(),
            8
        );
        for parent in seeded {
            let spec: crate::services::workflow::WorkflowSpec =
                serde_json::from_value(parent.config["__workflow_spec"].clone()).unwrap();
            for entry in &spec.sources {
                let owner: String =
                    sqlx::query_scalar("SELECT workflow_id FROM sources WHERE id=?")
                        .bind(&entry.id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                assert_eq!(owner, parent.id);
                let source = SourcesRepo::get(&pool, &entry.id).await.unwrap();
                if entry.id != parent.id {
                    assert_eq!(source.cadence, scrape_core::SourceCadence::Manual);
                    assert!(source.next_run_at.is_none());
                }
                if entry.extractor.script.is_some() {
                    assert!(source.needs_review && !source.enabled && !source.script_trusted);
                    assert!(source.script_sha256.is_some());
                }
            }
        }
        let report = seed_all_report(&pool).await.unwrap();
        assert!(report.created.is_empty());
        assert_eq!(report.already_present.len(), 6);
        assert_eq!(report.needs_review.len(), 1);
        assert!(seed_all(&pool).await.unwrap().is_empty());
    }
}
