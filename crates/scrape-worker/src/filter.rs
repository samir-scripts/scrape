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

/// One source's acceptance budget, shared by every adapter and execution route.
pub struct PostingGate<'a> {
    matcher: scrape_core::matching::MatchingEvaluator<'a>,
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
        use scrape_core::WorkerEvent;
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
            let reasons = self.matcher.rejection_reasons(&event, self.started);
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
