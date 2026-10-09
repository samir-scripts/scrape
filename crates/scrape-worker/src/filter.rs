use scrape_core::{RoleType, WorkerFilters};

pub struct FilterMatcher<'a> {
    filters: &'a WorkerFilters,
}

impl<'a> FilterMatcher<'a> {
    pub fn new(filters: &'a WorkerFilters) -> Self {
        Self { filters }
    }

    pub fn matches(&self, title: &str, location: &str, role_type: RoleType) -> bool {
        let title_lower = title.to_lowercase();
        let loc_lower = location.to_lowercase();

        // Include keywords: at least one must match if filter is set
        if !self.filters.include_keywords.is_empty() {
            let matched = self.filters.include_keywords.iter().any(|k| {
                let k_lower = k.to_lowercase();
                title_lower.contains(&k_lower) || loc_lower.contains(&k_lower)
            });
            if !matched {
                return false;
            }
        }

        // Exclude keywords: none must match
        if !self.filters.exclude_keywords.is_empty() {
            let excluded = self.filters.exclude_keywords.iter().any(|k| {
                let k_lower = k.to_lowercase();
                title_lower.contains(&k_lower) || loc_lower.contains(&k_lower)
            });
            if excluded {
                return false;
            }
        }

        // Locations: at least one must match if filter is set
        if !self.filters.locations.is_empty() {
            let matched = self.filters.locations.iter().any(|loc| {
                let l_lower = loc.to_lowercase();
                loc_lower.contains(&l_lower)
            });
            if !matched {
                return false;
            }
        }

        // Role types: must match if filter is set
        if !self.filters.role_types.is_empty() && !self.filters.role_types.contains(&role_type) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_matcher() {
        let filters = WorkerFilters {
            include_keywords: vec!["Engineer".to_string(), "Developer".to_string()],
            exclude_keywords: vec!["Senior".to_string(), "Staff".to_string()],
            locations: vec!["Remote".to_string()],
            role_types: vec![RoleType::FullTime],
        };

        let matcher = FilterMatcher::new(&filters);

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
}
