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
