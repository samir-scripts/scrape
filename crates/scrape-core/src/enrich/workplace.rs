use crate::types::WorkplaceType;

/// Classifies physical workplace arrangement (Remote, Hybrid, Onsite/In-Office)
/// by evaluating explicit ATS fields, location strings, and job description texts.
pub fn classify_workplace(
    explicit_type: Option<&str>,
    location_str: &str,
    description: Option<&str>,
) -> WorkplaceType {
    use crate::geography::normalize;
    let patterns = &*PATTERNS;
    let hint = |value: &str| {
        let value = normalize(value);
        if patterns.hybrid.is_match(&value) && !patterns.not_hybrid.is_match(&value) {
            Some(WorkplaceType::Hybrid)
        } else if ((patterns.onsite.is_match(&value) || value == "OFFICE")
            && !patterns.not_onsite.is_match(&value))
            || patterns.not_remote.is_match(&value)
        {
            Some(WorkplaceType::Onsite)
        } else if patterns.remote.is_match(&value) {
            Some(WorkplaceType::Remote)
        } else {
            None
        }
    };
    if let Some(result) = explicit_type.and_then(hint) {
        return result;
    }
    if let Some(result) = hint(location_str) {
        return result;
    }
    if let Some(description) = description {
        let description = normalize(description);
        if patterns.description_hybrid.is_match(&description)
            && !patterns.not_hybrid.is_match(&description)
        {
            return WorkplaceType::Hybrid;
        }
        if patterns.not_remote.is_match(&description)
            || (patterns.description_onsite.is_match(&description)
                && !patterns.not_onsite.is_match(&description))
        {
            return WorkplaceType::Onsite;
        }
        if patterns.description_remote.is_match(&description) {
            return WorkplaceType::Remote;
        }
    }
    let location = normalize(location_str);
    if location.is_empty()
        || matches!(location.as_str(), "UNKNOWN" | "N A")
        || patterns.not_onsite.is_match(&location)
        || patterns.not_hybrid.is_match(&location)
    {
        WorkplaceType::Unknown
    } else {
        WorkplaceType::Onsite
    }
}

struct Patterns {
    onsite: regex::Regex,
    remote: regex::Regex,
    hybrid: regex::Regex,
    not_remote: regex::Regex,
    not_onsite: regex::Regex,
    not_hybrid: regex::Regex,
    description_remote: regex::Regex,
    description_hybrid: regex::Regex,
    description_onsite: regex::Regex,
}
static PATTERNS: std::sync::LazyLock<Patterns> = std::sync::LazyLock::new(|| {
    let re = |value| regex::Regex::new(value).expect("valid workplace pattern");
    Patterns {
        onsite: re(r"\b(?:ON SITE|ONSITE|IN OFFICE|IN PERSON|OFFICE BASED)\b"),
        remote: re(
            r"\b(?:REMOTE|DISTRIBUTED|TELECOMMUTE|TELECOMMUTING|VIRTUAL|WORK FROM HOME|ANYWHERE|WORLDWIDE)\b",
        ),
        hybrid: re(r"\bHYBRID\b"),
        not_remote: re(
            r"\b(?:NOT(?: OPEN TO| AVAILABLE FOR| CONSIDERING| ELIGIBLE FOR)?|NO) (?:FULLY |100 )?(?:REMOTE|WORK FROM HOME)\b",
        ),
        not_onsite: re(r"\b(?:NOT|NO) (?:ON SITE|ONSITE|IN OFFICE|IN PERSON)\b"),
        not_hybrid: re(r"\b(?:NOT|NO) HYBRID\b"),
        description_remote: re(
            r"\b(?:100 REMOTE|FULLY REMOTE|REMOTE FIRST|WORK FROM ANYWHERE|WORK FROM HOME|REMOTE ELIGIBLE)\b",
        ),
        description_hybrid: re(
            r"\bHYBRID (?:WORK|MODEL|SCHEDULE|ROLE|ENVIRONMENT)\b|\b(?:2 3|3|2) DAYS(?: A WEEK| PER WEEK)? IN (?:THE )?OFFICE\b|\bDAYS IN OFFICE (?:AND|THEN)\b",
        ),
        description_onsite: re(
            r"\b(?:MANDATORY|REQUIRED|100) (?:ON SITE|ONSITE|IN OFFICE|IN PERSON)\b|\bIN PERSON EXPECTATIONS\b|\bIN OFFICE (?:3 5|4 5|5) DAYS\b",
        ),
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_negations_and_unrelated_words_remain_distinct() {
        for alias in [
            "in person",
            "IN-PERSON",
            "in_office",
            "on-site",
            "OnSite",
            "office based",
        ] {
            assert_eq!(
                classify_workplace(Some(alias), "", None),
                WorkplaceType::Onsite,
                "{alias}"
            );
            assert_eq!(
                classify_workplace(None, &format!("US ({alias})"), None),
                WorkplaceType::Onsite,
                "{alias}"
            );
        }
        assert_eq!(
            classify_workplace(
                Some("REMOTE"),
                "US",
                Some("In-person expectations elsewhere")
            ),
            WorkplaceType::Remote
        );
        assert_eq!(
            classify_workplace(None, "Remoteville, US", None),
            WorkplaceType::Onsite
        );
        assert_eq!(
            classify_workplace(
                None,
                "",
                Some("We build hybrid cloud and remotely operated sensors")
            ),
            WorkplaceType::Unknown
        );
        assert_eq!(
            classify_workplace(None, "", Some("This is not fully-remote")),
            WorkplaceType::Onsite
        );
        assert_eq!(
            classify_workplace(None, "", Some("Hybrid work, not fully remote")),
            WorkplaceType::Hybrid
        );
        assert_eq!(
            classify_workplace(None, "not onsite", None),
            WorkplaceType::Unknown
        );
        assert_eq!(
            classify_workplace(Some("work_from_home"), "", None),
            WorkplaceType::Remote
        );
    }

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
