use crate::db::jobs::ScrapedJob;
use crate::error::{AppError, Result};
use scrape_core::{TriState, WorkerEvent};

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
                locations,
            })
        }
        _ => Err(AppError::Validation("Expected a posting event".into())),
    }
}
