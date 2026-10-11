//! Employment-type inference from ATS hints and job titles.

use crate::RoleType;

impl RoleType {
    /// Infer the role type. An explicit `hint` from the source (e.g. Lever
    /// `commitment`, Ashby `employmentType`, JSON-LD `employmentType`) wins;
    /// otherwise fall back to keywords in the title; otherwise `Unknown`.
    pub fn infer(title: &str, hint: Option<&str>) -> RoleType {
        hint.and_then(Self::from_hint)
            .or_else(|| Self::from_title(title))
            .unwrap_or(RoleType::Unknown)
    }

    /// Map a free-form employment-type string ("FullTime", "Part-time",
    /// "INTERN", "Temporary", ...) to a role type.
    pub fn from_hint(hint: &str) -> Option<RoleType> {
        let squashed: String = hint
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if squashed.is_empty() {
            return None;
        }
        if squashed.contains("seasonal") {
            Some(RoleType::Seasonal)
        } else if squashed.contains("intern") || squashed.contains("coop") {
            Some(RoleType::Internship)
        } else if squashed.contains("parttime") {
            Some(RoleType::PartTime)
        } else if ["contract", "temporary", "temp", "freelance", "fixedterm"]
            .iter()
            .any(|k| squashed.contains(k))
        {
            Some(RoleType::Contract)
        } else if ["fulltime", "permanent", "regular"]
            .iter()
            .any(|k| squashed.contains(k))
        {
            Some(RoleType::FullTime)
        } else {
            None
        }
    }

    /// Word-boundary keyword matching on a job title. Deliberately does not
    /// treat "Internal Tools Engineer" as an internship.
    pub fn from_title(title: &str) -> Option<RoleType> {
        let lower = title.to_ascii_lowercase();
        let tokens: Vec<&str> = lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|t| !t.is_empty())
            .collect();
        let has = |w: &str| tokens.contains(&w);
        let has_pair = |a: &str, b: &str| tokens.windows(2).any(|p| p[0] == a && p[1] == b);

        if has("seasonal") {
            Some(RoleType::Seasonal)
        } else if has("intern")
            || has("interns")
            || has("internship")
            || has("coop")
            || has_pair("co", "op")
        {
            Some(RoleType::Internship)
        } else if has_pair("part", "time") || has("parttime") {
            Some(RoleType::PartTime)
        } else if has("contract")
            || has("contractor")
            || has("freelance")
            || has("temporary")
            || has("temp")
        {
            Some(RoleType::Contract)
        } else if has_pair("full", "time") || has("fulltime") {
            Some(RoleType::FullTime)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_wins_over_title() {
        assert_eq!(
            RoleType::infer("Software Engineer Intern", Some("FullTime")),
            RoleType::FullTime
        );
    }

    #[test]
    fn hints() {
        assert_eq!(RoleType::from_hint("FullTime"), Some(RoleType::FullTime));
        assert_eq!(RoleType::from_hint("Part-time"), Some(RoleType::PartTime));
        assert_eq!(RoleType::from_hint("INTERN"), Some(RoleType::Internship));
        assert_eq!(RoleType::from_hint("Temporary"), Some(RoleType::Contract));
        assert_eq!(RoleType::from_hint("Contractor"), Some(RoleType::Contract));
        assert_eq!(RoleType::from_hint("Seasonal"), Some(RoleType::Seasonal));
        assert_eq!(
            RoleType::from_hint("Seasonal / Temp"),
            Some(RoleType::Seasonal)
        );
        assert_eq!(RoleType::from_hint("Hybrid"), None);
        assert_eq!(RoleType::from_hint(""), None);
    }

    #[test]
    fn titles() {
        assert_eq!(
            RoleType::from_title("Summer 2027 Data Engineering Intern"),
            Some(RoleType::Internship)
        );
        assert_eq!(
            RoleType::from_title("Seasonal Warehouse Associate"),
            Some(RoleType::Seasonal)
        );
        assert_eq!(
            RoleType::from_title("Seasonal Support Specialist"),
            Some(RoleType::Seasonal)
        );
        assert_eq!(
            RoleType::from_title("Co-op, Platform Team"),
            Some(RoleType::Internship)
        );
        assert_eq!(RoleType::from_title("Internal Tools Engineer"), None);
        assert_eq!(
            RoleType::from_title("Backend Engineer (Contract)"),
            Some(RoleType::Contract)
        );
        assert_eq!(
            RoleType::from_title("Part-Time Data Analyst"),
            Some(RoleType::PartTime)
        );
        assert_eq!(
            RoleType::from_title("Engineer - Full Time"),
            Some(RoleType::FullTime)
        );
        assert_eq!(RoleType::from_title("Senior Software Engineer"), None);
    }

    #[test]
    fn falls_back_to_unknown() {
        assert_eq!(
            RoleType::infer("Senior Software Engineer", None),
            RoleType::Unknown
        );
    }
}
