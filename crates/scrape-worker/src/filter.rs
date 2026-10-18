use scrape_core::{FilterExpr, JobFilterContext, RoleType, WorkerFilters};

pub struct FilterMatcher<'a> {
    pub filters: &'a WorkerFilters,
    expr: FilterExpr,
}

impl<'a> FilterMatcher<'a> {
    pub fn new(filters: &'a WorkerFilters) -> Result<Self, String> {
        let expr = filters.to_expr()?;
        Ok(Self { filters, expr })
    }

    pub fn matches(&self, title: &str, location: &str, role_type: RoleType) -> bool {
        let ctx = JobFilterContext {
            title,
            location,
            description: "",
            company: "",
            role_type,
        };
        self.matches_context(&ctx)
    }

    pub fn matches_context(&self, ctx: &JobFilterContext) -> bool {
        self.expr.eval(ctx)
    }
}

/// Bounded callers use this same explanation for both execution coordinators.
pub(crate) fn unresolved_warning(
    event: &scrape_core::WorkerEvent,
    filters: &WorkerFilters,
    reasons: &[scrape_core::matching::RejectionReason],
) -> Option<scrape_core::WorkerEvent> {
    let scrape_core::WorkerEvent::Job {
        source_id,
        title,
        location,
        locations,
        ..
    } = event
    else {
        return None;
    };
    let labels = reasons
        .iter()
        .map(|reason| {
            serde_json::to_value(reason)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    let geography = if reasons.contains(&scrape_core::matching::RejectionReason::Geography) {
        let countries = filters
            .matching
            .as_ref()
            .map(|rules| {
                let mut values = rules
                    .countries
                    .iter()
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ");
                if rules.countries.len() > 10 {
                    values.push_str(", …");
                }
                values
            })
            .unwrap_or_default();
        let raw = if location.trim().is_empty() {
            locations
                .iter()
                .take(3)
                .map(|value| value.raw.chars().take(100).collect::<String>())
                .collect::<Vec<_>>()
                .join("; ")
        } else {
            location.chars().take(300).collect()
        };
        format!(
            " Requested countries: {}. Extracted location evidence: {}.",
            if countries.is_empty() {
                "unconstrained"
            } else {
                &countries
            },
            if raw.is_empty() { "unavailable" } else { &raw }
        )
    } else {
        String::new()
    };
    Some(scrape_core::WorkerEvent::Warning{source_id:source_id.clone(), message:format!("Required matching evidence is unresolved for '{}': {labels}.{geography} This posting was not accepted.",title.chars().take(160).collect::<String>()),eligibility_incomplete:true,termination:None})
}

/// One source's acceptance budget, shared by every adapter and execution route.
pub struct PostingGate<'a> {
    matcher: scrape_core::matching::MatchingEvaluator<'a>,
    filters: &'a WorkerFilters,
    source: &'a scrape_core::WorkerSource,
    blocked: &'a std::collections::HashSet<String>,
    seen: std::collections::HashSet<String>,
    started: chrono::DateTime<chrono::Utc>,
    pub summary: scrape_core::protocol::SourceScanStats,
    pub accepted: u32,
}
impl<'a> PostingGate<'a> {
    pub fn new(
        source: &'a scrape_core::WorkerSource,
        filters: &'a WorkerFilters,
        blocked: &'a std::collections::HashSet<String>,
        started: chrono::DateTime<chrono::Utc>,
        preview: bool,
    ) -> Result<Self, String> {
        let limit = source.max_jobs_per_run()?;
        Ok(Self {
            matcher: scrape_core::matching::MatchingEvaluator::new(filters)?,
            filters,
            source,
            blocked,
            seen: Default::default(),
            started,
            accepted: 0,
            summary: scrape_core::protocol::SourceScanStats {
                source_id: source.id.clone(),
                limit: if preview {
                    limit.min(scrape_core::protocol::PREVIEW_MAX_JOBS)
                } else {
                    limit
                },
                ..Default::default()
            },
        })
    }
    pub fn full(&self) -> bool {
        self.accepted >= self.summary.limit
    }
    pub fn accept(&mut self, event: scrape_core::WorkerEvent) -> Option<scrape_core::WorkerEvent> {
        self.accept_with_evidence(event, None)
    }
    pub fn accept_with_evidence(
        &mut self,
        event: scrape_core::WorkerEvent,
        evidence: Option<scrape_core::matching::MatchingEvidence>,
    ) -> Option<scrape_core::WorkerEvent> {
        use scrape_core::WorkerEvent;
        if let WorkerEvent::SourceInfo { provider_total, .. } = &event {
            self.summary.provider_total = *provider_total;
            return None;
        }
        if let WorkerEvent::Warning {
            termination,
            eligibility_incomplete,
            ..
        } = &event
        {
            if !eligibility_incomplete {
                self.summary.enrichment_warnings += 1;
            }
            if let Some(termination) = termination {
                self.summary.termination = *termination;
            }
        }
        if self.full() {
            return None;
        }
        if let WorkerEvent::Error {
            invalid_posting: Some(true),
            ..
        } = &event
        {
            self.summary.examined += 1;
            self.summary.invalid += 1;
        }
        if let WorkerEvent::Job {
            external_id,
            title,
            url,
            ..
        } = &event
        {
            self.summary.examined += 1;
            let identity = match scrape_core::PostingIdentity::for_posting(
                Some(self.source.kind),
                Some(&self.source.identifier),
                Some(external_id),
                url,
            ) {
                Ok(identity) => identity.key,
                Err(message) => {
                    self.summary.invalid += 1;
                    return Some(WorkerEvent::Error {
                        source_id: self.source.id.clone(),
                        message,
                        retryable: false,
                        invalid_posting: Some(true),
                    });
                }
            };
            if !self.seen.insert(identity.clone()) {
                self.summary.duplicates += 1;
                return None;
            }
            let assessment = evidence
                .as_ref()
                .map(|evidence| self.matcher.assess(&event, self.started, evidence));
            let reasons = assessment
                .as_ref()
                .map(|assessment| assessment.rejected.clone())
                .unwrap_or_else(|| self.matcher.rejection_reasons(&event, self.started));
            if let Some(primary) = reasons.first().copied() {
                self.summary.rejected += 1;
                if let Some(count) = self
                    .summary
                    .rejections
                    .iter_mut()
                    .find(|count| count.reason == primary)
                {
                    count.count += 1;
                } else {
                    self.summary
                        .rejections
                        .push(scrape_core::protocol::RejectionCount {
                            reason: primary,
                            count: 1,
                        });
                }
                if self.summary.rejection_examples.len() < 5 {
                    self.summary
                        .rejection_examples
                        .push(scrape_core::protocol::RejectionExample {
                            title: title.clone(),
                            primary,
                            overlapping: reasons,
                        });
                }
                return None;
            }
            if let Some(assessment) = assessment
                .as_ref()
                .filter(|value| !value.unresolved.is_empty())
            {
                self.summary.unresolved += 1;
                // Required missing evidence is separate from optional enrichment
                // failures. Bound examples without losing truthful source totals.
                if self.summary.unresolved <= 5 {
                    return unresolved_warning(&event, self.filters, &assessment.unresolved);
                }
                return None;
            }
            self.summary.matched += 1;
            if self.blocked.contains(&identity)
                || self
                    .blocked
                    .contains(&format!("{}:{}", self.source.id, external_id))
            {
                self.summary.blocked += 1;
                return None;
            }
            self.accepted += 1;
            self.summary.accepted = u64::from(self.accepted);
            self.summary.limit_reached = self.full();
            if self.full() {
                self.summary.termination = scrape_core::protocol::CoverageTermination::JobLimit;
            }
        }
        Some(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_query_is_an_error() {
        let filters = WorkerFilters {
            query: Some("title LIKE".into()),
            ..Default::default()
        };
        assert!(FilterMatcher::new(&filters).is_err());
    }

    #[test]
    fn test_filter_matcher() {
        let filters = WorkerFilters {
            include_keywords: vec!["Engineer".to_string(), "Developer".to_string()],
            exclude_keywords: vec!["Senior".to_string(), "Staff".to_string()],
            locations: vec!["Remote".to_string()],
            role_types: vec![RoleType::FullTime],
            query: None,
            policy: None,
            compensation: None,
            matching: None,
        };

        let matcher = FilterMatcher::new(&filters).unwrap();

        // Matches
        assert!(matcher.matches("Software Engineer", "Remote - US", RoleType::FullTime));

        // Excluded keyword
        assert!(!matcher.matches("Senior Software Engineer", "Remote", RoleType::FullTime));

        // Location mismatch
        assert!(!matcher.matches("Software Engineer", "New York, NY", RoleType::FullTime));

        // Role type mismatch
        assert!(!matcher.matches("Software Engineer", "Remote", RoleType::Contract));

        // Missing include keyword
        assert!(!matcher.matches("Product Manager", "Remote", RoleType::FullTime));
    }

    #[test]
    fn test_filter_matcher_with_sql_query() {
        let filters = WorkerFilters {
            include_keywords: vec![],
            exclude_keywords: vec![],
            locations: vec![],
            role_types: vec![],
            query: Some(
                "title LIKE '%Engineer%' AND (location LIKE '%Remote%' OR location LIKE '%Austin%') AND title NOT LIKE '%Senior%'"
                    .to_string(),
            ),
            policy: None,
            compensation: None,
            matching: None,
        };

        let matcher = FilterMatcher::new(&filters).unwrap();

        let ctx_match = JobFilterContext {
            title: "Software Engineer",
            location: "Austin, TX",
            description: "Distributed systems",
            company: "Cloudflare",
            role_type: RoleType::FullTime,
        };
        assert!(matcher.matches_context(&ctx_match));

        let ctx_senior = JobFilterContext {
            title: "Senior Software Engineer",
            location: "Austin, TX",
            description: "Distributed systems",
            company: "Cloudflare",
            role_type: RoleType::FullTime,
        };
        assert!(!matcher.matches_context(&ctx_senior));
    }
}

#[cfg(test)]
mod missing_evidence_diagnostics_tests {
    use super::*;
    use scrape_core::WorkerEvent;
    #[test]
    fn country_evidence_warnings_are_specific_bounded_and_not_optional_enrichment_failures() {
        let source = scrape_core::WorkerSource {
            id: "fixture".into(),
            kind: scrape_core::SourceKind::Selector,
            name: "Authored fixture".into(),
            identifier: "https://fixture.test/jobs".into(),
            config: serde_json::Value::Null,
        };
        let filters: WorkerFilters =
            serde_json::from_value(serde_json::json!({"matching":{"countries":["US"]}})).unwrap();
        let blocked = Default::default();
        let mut gate =
            PostingGate::new(&source, &filters, &blocked, chrono::Utc::now(), false).unwrap();
        let mut warnings = 0;
        for id in 0..7 {
            let event:WorkerEvent=serde_json::from_value(serde_json::json!({"type":"job","source_id":"fixture","external_id":id.to_string(),"title":"Engineer","company":"Fixture","url":format!("https://fixture.test/{id}"),"location":"Europe","role_type":"unknown","locations":[{"raw":"Europe","city":null,"region_code":null,"country_code":null,"is_remote":false}]})).unwrap();
            let evidence = scrape_core::matching::MatchingEvidence::from_event(&event, true);
            if let Some(WorkerEvent::Warning { message, .. }) =
                gate.accept_with_evidence(event, Some(evidence))
            {
                assert!(message.contains("Requested countries: US"));
                assert!(message.contains("Europe"));
                assert!(message.contains("geography"));
                warnings += 1;
            }
        }
        assert_eq!(warnings, 5);
        assert_eq!(gate.summary.unresolved, 7);
        assert_eq!(gate.summary.accepted, 0);
        assert_eq!(gate.summary.enrichment_warnings, 0);
    }
}
