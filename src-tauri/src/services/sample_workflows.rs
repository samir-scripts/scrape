//! Shared seed service behind the desktop commands and isolated integration tests.
//! Ownership/trust repair is Plan 12 section 2; preserve existing seed behavior here.
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
    SourcesRepo::create(pool, SourcesRepo::default_sample_workflow_input()).await
}

pub async fn seed_all(pool: &SqlitePool) -> Result<Vec<Source>> {
    let mut created = Vec::new();
    let existing = SourcesRepo::list(pool).await?;
    for input in SourcesRepo::sample_workflows_suite() {
        if !existing
            .iter()
            .any(|s| s.identifier == input.identifier && s.kind == input.kind)
        {
            created.push(SourcesRepo::create(pool, input).await?);
        }
    }
    Ok(created)
}
