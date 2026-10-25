//! Recorded provider controls are frozen independently of production templates.
//! Changing the user-facing workflow catalog must never alter capture outcomes.
use super::workflow::{self, WorkflowSpec};
use crate::db::sources::Source;
use crate::error::{AppError, Result};
use sqlx::SqlitePool;

pub const REVISION: u32 = 1;

pub struct VerificationProfile {
    pub id: String,
    pub spec: WorkflowSpec,
}

pub fn profiles() -> Result<Vec<VerificationProfile>> {
    Ok(super::legacy_sample_workflows::templates()?
        .into_iter()
        .map(|template| VerificationProfile {
            id: template.id,
            spec: template.spec,
        })
        .collect())
}

pub fn profile(id: &str) -> Result<WorkflowSpec> {
    profiles()?
        .into_iter()
        .find(|profile| profile.id == id)
        .map(|profile| profile.spec)
        .ok_or_else(|| {
            AppError::Validation(format!("Unknown provider verification profile '{id}'"))
        })
}

/// Persist only the selected control through production identity/trust handling.
/// This does not seed user workflows, grant trust, enable scripts or start workers.
pub async fn save(pool: &SqlitePool, id: &str) -> Result<Source> {
    workflow::persist(pool, profile(id)?, None, false, "UTC").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_profiles_keep_original_controls_outside_production_catalog() {
        let profiles = profiles().unwrap();
        assert_eq!(profiles.len(), 12);
        let cloudflare = profile("cloudflare-systems-network").unwrap();
        let matching = cloudflare
            .filters
            .as_ref()
            .unwrap()
            .matching
            .as_ref()
            .unwrap();
        assert_eq!(
            matching.keywords.as_ref().unwrap().any,
            ["Software", "Engineer", "Developer", "Systems", "Intern"]
        );
        assert!(matching.target_roles.is_empty() && matching.countries.is_empty());
        let strict = profile("cloudflare-systems-network-restrictive").unwrap();
        assert!(!strict
            .filters
            .as_ref()
            .unwrap()
            .matching
            .as_ref()
            .unwrap()
            .countries
            .is_empty());
        let scripts = profile("streaming-cloud-script").unwrap();
        assert_eq!(scripts.metadata.enabled, Some(false));
        assert!(scripts
            .sources
            .iter()
            .all(|source| !source.enabled && source.extractor.script.is_some()));
        let production = super::super::template_catalog::catalog().unwrap();
        assert!(production
            .templates
            .iter()
            .all(|template| profiles.iter().all(|profile| profile.id != template.id)));
    }
}
