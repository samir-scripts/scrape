use crate::types::WorkplaceType;

/// Classifies physical workplace arrangement (Remote, Hybrid, Onsite/In-Office)
/// by evaluating explicit ATS fields, location strings, and job description texts.
pub fn classify_workplace(
    explicit_type: Option<&str>,
    location_str: &str,
    description: Option<&str>,
) -> WorkplaceType {
    // 1. Check explicit ATS workplace_type field if provided
    if let Some(exp) = explicit_type {
        let trimmed = exp.trim();
        if !trimmed.is_empty() {
            let upper = trimmed.to_uppercase();
            if upper.contains("HYBRID") {
                return WorkplaceType::Hybrid;
            }
            if upper.contains("IN-OFFICE")
                || upper.contains("IN OFFICE")
                || upper.contains("ON-SITE")
                || upper.contains("ONSITE")
                || upper.contains("IN-PERSON")
                || upper.contains("IN PERSON")
                || upper == "OFFICE"
            {
                return WorkplaceType::Onsite;
            }
            if upper.contains("REMOTE")
                || upper.contains("DISTRIBUTED")
                || upper.contains("TELECOMMUTE")
                || upper.contains("VIRTUAL")
                || upper.contains("WORK FROM HOME")
                || upper.contains("ANYWHERE")
            {
                return WorkplaceType::Remote;
            }
        }
    }

    let loc_upper = location_str.trim().to_uppercase();

    // 2. Check location string for explicit workplace arrangement indicators
    if !loc_upper.is_empty() {
        if loc_upper.contains("HYBRID") {
            return WorkplaceType::Hybrid;
        }
        if loc_upper.contains("IN-OFFICE")
            || loc_upper.contains("IN OFFICE")
            || loc_upper.contains("ON-SITE")
            || loc_upper.contains("ONSITE")
            || loc_upper.contains("IN-PERSON")
            || loc_upper.contains("IN PERSON")
        {
            return WorkplaceType::Onsite;
        }
        if (loc_upper.contains("REMOTE")
            || loc_upper.contains("DISTRIBUTED")
            || loc_upper.contains("TELECOMMUTE")
            || loc_upper.contains("VIRTUAL")
            || loc_upper.contains("ANYWHERE"))
            && !loc_upper.contains("NOT REMOTE")
            && !loc_upper.contains("NO REMOTE")
            && !loc_upper.contains("NOT CONSIDERING REMOTE")
        {
            return WorkplaceType::Remote;
        }
    }

    // 3. Check description text for strong signals
    if let Some(desc) = description {
        let desc_upper = desc.to_uppercase();

        // Strong In-Office / On-Site negation of remote or in-person mandate
        if desc_upper.contains("NOT CONSIDERING REMOTE")
            || desc_upper.contains("NO REMOTE")
            || desc_upper.contains("NOT REMOTE")
            || desc_upper.contains("NOT OPEN TO REMOTE")
            || desc_upper.contains("NOT AVAILABLE FOR REMOTE")
            || desc_upper.contains("IN-PERSON EXPECTATIONS")
            || desc_upper.contains("IN OFFICE 3-5 DAYS")
            || desc_upper.contains("IN OFFICE 4-5 DAYS")
            || desc_upper.contains("IN OFFICE 5 DAYS")
            || desc_upper.contains("100% ON-SITE")
            || desc_upper.contains("100% ONSITE")
            || desc_upper.contains("100% IN-OFFICE")
            || desc_upper.contains("MANDATORY IN-OFFICE")
            || desc_upper.contains("REQUIRED IN-OFFICE")
        {
            return WorkplaceType::Onsite;
        }

        // Hybrid signals in description
        if desc_upper.contains("HYBRID WORK")
            || desc_upper.contains("HYBRID MODEL")
            || desc_upper.contains("HYBRID SCHEDULE")
            || desc_upper.contains("HYBRID ROLE")
            || desc_upper.contains("HYBRID ENVIRONMENT")
            || desc_upper.contains("2-3 DAYS IN OFFICE")
            || desc_upper.contains("2-3 DAYS PER WEEK IN OFFICE")
            || desc_upper.contains("3 DAYS A WEEK IN OFFICE")
            || desc_upper.contains("3 DAYS PER WEEK IN OFFICE")
            || desc_upper.contains("DAYS IN OFFICE,")
            || desc_upper.contains("DAYS IN OFFICE AND")
        {
            return WorkplaceType::Hybrid;
        }

        // Remote signals in description (ensure not negated)
        if (desc_upper.contains("100% REMOTE")
            || desc_upper.contains("FULLY REMOTE")
            || desc_upper.contains("REMOTE-FIRST")
            || desc_upper.contains("WORK FROM ANYWHERE")
            || desc_upper.contains("REMOTE ELIGIBLE"))
            && !desc_upper.contains("NOT 100% REMOTE")
            && !desc_upper.contains("NOT FULLY REMOTE")
        {
            return WorkplaceType::Remote;
        }
    }

    // 4. Fallback based on physical location presence
    if !loc_upper.is_empty() && loc_upper != "UNKNOWN" && loc_upper != "N/A" {
        WorkplaceType::Onsite
    } else {
        WorkplaceType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_explicit_in_office() {
        assert_eq!(
            classify_workplace(Some("In-Office"), "Austin, TX", None),
            WorkplaceType::Onsite
        );
        assert_eq!(
            classify_workplace(Some("on-site"), "Austin, TX", None),
            WorkplaceType::Onsite
        );
        assert_eq!(
            classify_workplace(Some("In-Person"), "New York, NY", None),
            WorkplaceType::Onsite
        );
    }

    #[test]
    fn test_classify_explicit_hybrid() {
        assert_eq!(
            classify_workplace(Some("Hybrid"), "Austin, TX", None),
            WorkplaceType::Hybrid
        );
    }

    #[test]
    fn test_classify_explicit_remote_and_distributed() {
        assert_eq!(
            classify_workplace(Some("Remote"), "", None),
            WorkplaceType::Remote
        );
        assert_eq!(
            classify_workplace(Some("Distributed"), "USA", None),
            WorkplaceType::Remote
        );
    }

    #[test]
    fn test_classify_location_string_cues() {
        assert_eq!(
            classify_workplace(None, "In-Office", None),
            WorkplaceType::Onsite
        );
        assert_eq!(
            classify_workplace(None, "Austin, TX (In-Office)", None),
            WorkplaceType::Onsite
        );
        assert_eq!(
            classify_workplace(None, "London, UK (Hybrid)", None),
            WorkplaceType::Hybrid
        );
        assert_eq!(
            classify_workplace(None, "Remote - US", None),
            WorkplaceType::Remote
        );
    }

    #[test]
    fn test_classify_description_in_office_negation() {
        let desc = "In office 3-5 days a week in Austin, TX. We are not considering remote or part-time for either terms. This is a full-time (40hr/week internship) with in-person expectations.";
        assert_eq!(
            classify_workplace(None, "Austin, TX", Some(desc)),
            WorkplaceType::Onsite
        );
    }

    #[test]
    fn test_classify_description_hybrid() {
        let desc = "We operate on a hybrid schedule with 2-3 days in office per week.";
        assert_eq!(
            classify_workplace(None, "San Francisco, CA", Some(desc)),
            WorkplaceType::Hybrid
        );
    }

    #[test]
    fn test_classify_description_fully_remote() {
        let desc = "This is a fully remote position open to candidates anywhere in the US.";
        assert_eq!(
            classify_workplace(None, "Remote", Some(desc)),
            WorkplaceType::Remote
        );
    }

    #[test]
    fn test_classify_empty_unknown() {
        assert_eq!(classify_workplace(None, "", None), WorkplaceType::Unknown);
        assert_eq!(
            classify_workplace(None, "Unknown", None),
            WorkplaceType::Unknown
        );
    }
}
