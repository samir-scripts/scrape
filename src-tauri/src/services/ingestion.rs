use crate::db::jobs::ScrapedJob;
use crate::error::{AppError, Result};
use scrape_core::{TriState, WorkerEvent};

/// Defense at the persistence boundary against an overproducing sidecar.
pub struct RunPostingBudget(std::collections::HashMap<String, u32>);
impl RunPostingBudget {
    pub fn new(sources: &[scrape_core::WorkerSource]) -> Result<Self> {
        Ok(Self(
            sources
                .iter()
                .map(|source| {
                    Ok((
                        source.id.clone(),
                        source.max_jobs_per_run().map_err(AppError::Validation)?,
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }
    pub fn take(&mut self, source_id: &str) -> Result<()> {
        let remaining = self
            .0
            .get_mut(source_id)
            .ok_or_else(|| AppError::Validation(format!("Unknown workflow ID: {source_id}")))?;
        if *remaining == 0 {
            return Err(AppError::Validation(format!(
                "Worker exceeded max_jobs_per_run for workflow {source_id}"
            )));
        }
        *remaining -= 1;
        Ok(())
    }
}

/// The single ingestion mapper used by both the sidecar runner and boundary tests.
pub fn scraped_job_from_event(event: WorkerEvent) -> Result<ScrapedJob> {
    match event {
        WorkerEvent::Job {
            source_id,
            external_id,
            title,
            company,
            role_type,
            location,
            url,
            provider_updated_at,
            date_posted,
            salary,
            locations,
            location_data_complete,
            workplace_type,
            sponsorship,
            work_auth,
            clearance,
            experience,
            is_new_grad,
            description_text,
            description,
            description_source,
            description_format,
            company_icon_url,
            company_logo,
        } => {
            if title.trim().is_empty()
                || external_id.trim().is_empty()
                || source_id.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "Posting has a missing title or identifier".into(),
                ));
            }
            scrape_core::identity::normalized_detail_url(&url).map_err(AppError::Validation)?;
            let dedupe_key = format!("{}:{}", source_id, external_id);
            let loc_opt = if location.is_empty() {
                None
            } else {
                Some(location)
            };

            let compensation = salary
                .as_ref()
                .map(|s| s.ranges.clone())
                .unwrap_or_default();
            let (
                salary_min,
                salary_max,
                salary_currency,
                salary_interval,
                salary_raw,
                salary_source,
            ) = if let Some(s) = salary {
                (
                    s.min,
                    s.max,
                    Some(s.currency),
                    s.ranges.first().and_then(|range| range.interval),
                    Some(s.raw),
                    Some(s.source),
                )
            } else {
                (None, None, None, None, None, None)
            };

            let (sponsorship_val, sponsorship_evidence) = if let Some(sp) = sponsorship {
                (sp.value, sp.evidence)
            } else {
                (TriState::Unknown, None)
            };

            let (work_auth_val, work_auth_evidence) = if let Some(wa) = work_auth {
                (wa.value, wa.evidence)
            } else {
                (scrape_core::WorkAuthRequirement::Unknown, None)
            };

            let (clearance_val, clearance_level, clearance_evidence) = if let Some(cl) = clearance {
                (cl.value, cl.level, cl.evidence)
            } else {
                (TriState::Unknown, None, None)
            };

            let (experience_min, experience_max, experience_raw, experience_evidence) =
                if let Some(exp) = experience {
                    (exp.min_years, exp.max_years, exp.raw, exp.evidence)
                } else {
                    (None, None, None, None)
                };

            Ok(ScrapedJob {
                compensation,
                dedupe_key,
                source_id: Some(source_id),
                external_id: Some(external_id),
                title,
                company,
                role_type,
                location: loc_opt,
                url,
                provider_updated_at,
                date_posted,
                location_data_complete,
                salary_min,
                salary_max,
                salary_currency,
                salary_interval,
                salary_raw,
                salary_source,
                workplace_type,
                sponsorship: sponsorship_val,
                sponsorship_evidence,
                work_auth: work_auth_val,
                work_auth_evidence,
                clearance: clearance_val,
                clearance_level,
                clearance_evidence,
                experience_min,
                experience_max,
                experience_raw,
                is_new_grad,
                experience_evidence,
                description_text,
                description,
                description_source,
                description_format,
                company_icon_url,
                company_logo,
                locations,
            })
        }
        _ => Err(AppError::Validation("Expected a posting event".into())),
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn persistence_budget_rejects_surplus_and_unknown_sources_independently() {
        let source = |id: &str| scrape_core::WorkerSource {
            id: id.into(),
            kind: scrape_core::SourceKind::Lever,
            name: id.into(),
            identifier: id.into(),
            config: serde_json::json!({"__workflow_spec":{"max_jobs_per_run":2}}),
        };
        let mut budget = RunPostingBudget::new(&[source("one"), source("two")]).unwrap();
        assert!(budget.take("unknown").is_err());
        for id in ["one", "one", "two", "two"] {
            budget.take(id).unwrap();
        }
        assert!(budget
            .take("one")
            .unwrap_err()
            .to_string()
            .contains("exceeded max_jobs_per_run"));
        assert!(budget.take("two").is_err());
    }
}
