use crate::types::TriState;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExperienceClassification {
    #[ts(type = "number | null")]
    pub min_years: Option<i64>,
    #[ts(type = "number | null")]
    pub max_years: Option<i64>,
    pub raw: Option<String>,
    pub is_new_grad: TriState,
    pub evidence: Option<String>,
}

// Keep work bounded even for direct callers that bypass description normalization.
const MAX_INPUT_BYTES: usize = 512 * 1024;
const MAX_TITLE_BYTES: usize = 4096;

static NEW_GRAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:(?:new|recent|university)[ -]+grad(?:uate)?s?|campus[ -]+hir(?:e|ing)|entry[- ]level)\b").unwrap()
});
static GRAD_ELIGIBILITY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:open\s+to|welcome|welcomes|hiring|seeking|eligible\s+for|suitable\s+for)\s+(?:(?:new|recent|university)[ -]+grad(?:uate)?s?|campus[ -]+hires?)\b|\b(?:new|recent|university)[ -]+grad(?:uate)?s?\s+(?:are\s+)?(?:welcome|eligible|encouraged|invited)\b|\b(?:an?\s+)?entry[- ]level\s+(?:role|position|opportunity)\b|\bcampus[ -]+hir(?:e|ing)\b").unwrap()
});
static NON_ENTRY_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:senior|sr\.?|staff|principal|lead|distinguished|architect|director)\b|\b(?:engineer|developer|swe)\s*(?:[-–—]\s*)?(?:level\s+)?(?:iii|iv|v|vi|vii|viii|ix|x|[3-9]|10)\b").unwrap()
});
static NOT_ENTRY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bnot\s+(?:an?\s+)?entry[- ]level\b|\b(?:new|recent|university)\s+graduates?\s+(?:are\s+)?not\s+eligible\b").unwrap()
});
static YEARS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?P<min>\d+)\s*(?:(?:-|–|—|to)\s*(?P<max>\d+))?\s*\+?\s*(?:years?|yrs?)\b")
        .unwrap()
});
static ZERO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bno\s+(?:(?:prior|professional|work|relevant)\s+)?experience\s+(?:is\s+)?(?:required|necessary|needed)\b").unwrap()
});
static GENERAL_EXPERIENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:of\s+)?(?:(?:relevant|professional|software|industry|work|hands[- ]on|practical|engineering|technical|post[- ]graduate|related|direct|prior|development|coding|full[- ]time|overall|total)\s+){0,5}experience\b").unwrap()
});
static OVERALL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:overall|total)\b").unwrap());
static DOMAIN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(?:in|with|using)\s+(?:(?:the|a|an)\s+)?(?:software(?:\s+(?:engineering|development))?|engineering|distributed\s+(?:systems|backend\s+architecture)|(?:backend|frontend|full[- ]stack|web|mobile)\s+development|data\s+engineering|(?:related|relevant)\s+field|similar\s+role)\b").unwrap()
});
static TOOL_SCOPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:in|with|using)\b").unwrap());
static REQUIRED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:required|require|requires|must|minimum|at\s+least|mandatory)\b").unwrap()
});
static PREFERRED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:preferred|preferably|desired|desirable|nice[- ]to[- ]have|bonus|a\s+plus)\b",
    )
    .unwrap()
});
static NEGATED_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:not\s+(?:required|necessary|needed)|no\s+(?:minimum|requirement)|do(?:es)?\s+not\s+require|don't\s+require|without\s+requiring)\b").unwrap()
});
static MAXIMUM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:at\s+most|up\s+to|no\s+more\s+than|maximum(?:\s+of)?)\s*$").unwrap()
});
static DEGREE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:bachelor(?:['’]s)?|master(?:['’]s)?|doctorate|ph\.?d\.?|bs|ms|bsc|msc)\b")
        .unwrap()
});
static EDUCATION_YEARS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:\+|\bwith|\band)\s*$").unwrap());
static OR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s*,?\s+\bor\b\s+").unwrap());
static AND_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(?:and|but)\b|,").unwrap());
static COMPANY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:(?:our\s+(?:company|team)|we)\s+(?:has|have|brings?|offers?)\b|combined\s+experience|company\s+history|in\s+business|serving\s+clients|in\s+the\s+next|over\s+the\s+next)\b").unwrap()
});

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    General,
    Required,
    Preferred,
}

#[derive(Clone, Copy)]
struct Requirement<'a> {
    min: Option<i64>,
    max: Option<i64>,
    required: bool,
    evidence: &'a str,
}

fn bounded(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn heading(text: &str) -> Option<Scope> {
    let label = text
        .trim()
        .trim_start_matches('#')
        .trim()
        .to_ascii_lowercase();
    match label.trim_end_matches(':').trim() {
        "required"
        | "requirements"
        | "required qualifications"
        | "minimum qualifications"
        | "basic qualifications"
        | "minimum requirements"
        | "what you bring"
        | "what you'll need"
        | "qualifications" => Some(Scope::Required),
        "preferred"
        | "preferred qualifications"
        | "desired qualifications"
        | "nice to have"
        | "bonus qualifications" => Some(Scope::Preferred),
        "responsibilities" | "about us" | "about the company" | "benefits" | "the role"
        | "what you'll do" => Some(Scope::General),
        _ => None,
    }
}

// Tool-specific tenure is not an overall professional-experience minimum.
fn general_experience(tail: &str) -> bool {
    if OVERALL_RE.is_match(tail) {
        return true;
    }
    let Some(experience) = GENERAL_EXPERIENCE_RE.find(tail) else {
        return false;
    };
    let after = &tail[experience.end()..];
    !TOOL_SCOPE_RE.is_match(after) || DOMAIN_RE.is_match(after)
}

fn requirements<'a>(clause: &'a str, scope: Scope) -> Vec<Requirement<'a>> {
    let mut result = Vec::new();
    if COMPANY_RE.is_match(clause) {
        return result;
    }
    let years: Vec<_> = YEARS_RE.captures_iter(clause).collect();
    for (index, caps) in years.iter().enumerate() {
        let matched = caps.get(0).unwrap();
        let preceding_end = index
            .checked_sub(1)
            .map_or(0, |previous| years[previous].get(0).unwrap().end());
        let following_start = years
            .get(index + 1)
            .map_or(clause.len(), |next| next.get(0).unwrap().start());
        let before = &clause[preceding_end..matched.start()];
        let after = &clause[matched.end()..following_start];
        let local_before = AND_RE.split(before).last().unwrap_or(before);
        let local_after = AND_RE.split(after).next().unwrap_or(after);
        let preferred = PREFERRED_RE.is_match(local_before) || PREFERRED_RE.is_match(local_after);
        let required = REQUIRED_RE.is_match(local_before) || REQUIRED_RE.is_match(local_after);
        if preferred
            || (scope == Scope::Preferred && !required)
            || NEGATED_RE.is_match(local_before)
            || NEGATED_RE.is_match(local_after)
            || local_before.trim_end().ends_with("no")
        {
            continue;
        }
        let education = DEGREE_RE.is_match(&clause[..matched.start()])
            && EDUCATION_YEARS_RE.is_match(local_before)
            && local_after.trim().trim_end_matches(',').is_empty();
        if !general_experience(local_after) && !education {
            continue;
        }
        let Ok(min) = caps["min"].parse::<i64>() else {
            continue;
        };
        let max = caps
            .name("max")
            .and_then(|value| value.as_str().parse::<i64>().ok());
        if min > 30 || max.is_some_and(|max| max > 30 || max < min) {
            continue;
        }
        let maximum_only = MAXIMUM_RE.is_match(local_before);
        if maximum_only && max.is_some() {
            continue;
        }
        result.push(Requirement {
            min: (!maximum_only).then_some(min),
            max: if maximum_only {
                Some(min)
            } else if min == 0 && max.is_none() {
                Some(0)
            } else {
                max
            },
            required: required || scope == Scope::Required,
            evidence: clause,
        });
    }
    if ZERO_RE.is_match(clause) && scope != Scope::Preferred && !PREFERRED_RE.is_match(clause) {
        // "No experience required with Python" is not job-wide eligibility.
        let zero = ZERO_RE.find(clause).unwrap();
        if !TOOL_SCOPE_RE.is_match(&clause[zero.end()..]) {
            result.push(Requirement {
                min: Some(0),
                max: Some(0),
                required: true,
                evidence: clause,
            });
        }
    }
    result
}

pub fn extract_experience(title: &str, text: &str) -> ExperienceClassification {
    let title = bounded(title, MAX_TITLE_BYTES);
    let bounded_text = bounded(text, MAX_INPUT_BYTES);
    let mut ambiguous = bounded_text.len() < text.len();
    let mut scope = Scope::General;
    let mut found: Vec<Requirement<'_>> = Vec::new();
    let mut ambiguous_evidence: Vec<&str> = Vec::new();
    let mut graduate_evidence = NEW_GRAD_RE.is_match(title).then_some(title);
    let mut non_entry_evidence = NON_ENTRY_TITLE_RE.is_match(title).then_some(title);

    for line in bounded_text.lines() {
        let mut line = line.trim();
        if let Some((label, rest)) = line.split_once(':') {
            if let Some(next_scope) = heading(label) {
                scope = next_scope;
                line = rest;
            }
        } else if let Some(next_scope) = heading(line) {
            scope = next_scope;
            continue;
        }
        for clause in line
            .split(['.', ';', '•'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if NOT_ENTRY_RE.is_match(clause) {
                non_entry_evidence = Some(clause);
            } else if scope != Scope::Preferred
                && GRAD_ELIGIBILITY_RE.is_match(clause)
                && !NEGATED_RE.is_match(clause)
            {
                graduate_evidence = Some(clause);
            }
            let branches: Vec<_> = OR_RE.split(clause).collect();
            if branches.len() > 1 && YEARS_RE.is_match(clause) {
                // Only explicit education/experience alternatives have a known
                // least qualifying branch. Never assume the applicant's degree.
                let mut alternatives = Vec::new();
                let mut resolved = true;
                for branch in &branches {
                    let options = requirements(branch, scope);
                    if !DEGREE_RE.is_match(branch) || options.is_empty() {
                        resolved = false;
                        break;
                    }
                    alternatives.push(*options.iter().max_by_key(|r| r.min).unwrap());
                }
                if resolved {
                    let mut least = *alternatives.iter().min_by_key(|r| r.min).unwrap();
                    least.evidence = clause;
                    found.push(least);
                } else if branches
                    .iter()
                    .any(|branch| !requirements(branch, scope).is_empty())
                {
                    ambiguous = true;
                    ambiguous_evidence.push(clause);
                }
                continue;
            }
            let candidates = requirements(clause, scope);
            if candidates.len() > 1
                && !AND_RE.is_match(clause)
                && !candidates.iter().all(|r| r.required)
            {
                ambiguous = true;
                ambiguous_evidence.push(clause);
            }
            found.extend(candidates);
        }
    }

    if found.len() > 1
        && !found.iter().all(|r| r.required)
        && found.iter().any(|r| {
            !r.required
                && r.min.is_some_and(|min| min > 0)
                && found
                    .iter()
                    .any(|other| other.min != r.min && other.evidence != r.evidence)
        })
    {
        ambiguous = true;
    }
    let decisive = if ambiguous {
        None
    } else {
        found.iter().max_by_key(|r| r.min).copied()
    };
    let min_years = decisive.and_then(|r| r.min);
    let max_years = decisive.and_then(|r| r.max);
    let is_new_grad = if non_entry_evidence.is_some() || min_years.is_some_and(|min| min >= 2) {
        TriState::No
    } else if ambiguous {
        TriState::Unknown
    } else if graduate_evidence.is_some()
        || (min_years == Some(0) && max_years.is_some_and(|max| max <= 1))
    {
        TriState::Yes
    } else {
        TriState::Unknown
    };
    let raw = decisive.map(|r| match (r.min, r.max) {
        (Some(0), Some(0)) => "0 years".to_owned(),
        (Some(min), Some(max)) => format!("{}-{} years", min, max),
        (Some(min), None) => format!("{}+ years", min),
        (None, Some(max)) => format!("up to {} years", max),
        (None, None) => unreachable!("requirements always contain at least one numeric bound"),
    });
    let mut evidence: Vec<&str> = if ambiguous {
        found
            .iter()
            .map(|r| r.evidence)
            .chain(ambiguous_evidence)
            .collect()
    } else {
        decisive.iter().map(|r| r.evidence).collect()
    };
    if let Some(non_entry) = non_entry_evidence {
        evidence.push(non_entry);
    } else if decisive.is_none() {
        if let Some(graduate) = graduate_evidence {
            evidence.push(graduate);
        }
    }
    evidence.dedup();
    ExperienceClassification {
        min_years,
        max_years,
        raw,
        is_new_grad,
        evidence: (!evidence.is_empty()).then(|| evidence.join("; ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_requirements_preserve_supported_bounds() {
        for (text, min, max) in [
            (
                "We require 3-5 years of experience in distributed systems.",
                3,
                Some(5),
            ),
            ("Must have 2–4 years of software experience.", 2, Some(4)),
            ("Requires 2—4 years of experience.", 2, Some(4)),
            ("Requires 3 to 5 years of work experience.", 3, Some(5)),
            ("Looking for 5-7+ years of relevant experience.", 5, Some(7)),
            (
                "Applicants must possess 5+ years of relevant experience.",
                5,
                None,
            ),
            (
                "Minimum of 4 years of professional experience required.",
                4,
                None,
            ),
            ("Requires at least 7 years of industry experience.", 7, None),
            (
                "Requires 3 years of software engineering experience.",
                3,
                None,
            ),
            (
                "Requires minimum 3 years of post-graduate experience.",
                3,
                None,
            ),
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!(
                (result.min_years, result.max_years),
                (Some(min), max),
                "{text}"
            );
            assert_eq!(result.is_new_grad, TriState::No, "{text}");
            assert!(result.evidence.as_deref().unwrap().contains("years"));
        }
    }

    #[test]
    fn numbered_senior_titles_never_invent_years() {
        for title in [
            "SOFTWARE ENGINEER III",
            "Software Engineer IV",
            "Developer V",
            "SWE VI",
            "Engineer VII",
            "Engineer VIII",
            "Engineer IX",
            "Engineer X",
            "Engineer 3",
            "Engineer 10",
            "Developer Level III",
            "Engineer - III",
            "Senior Software Engineer",
            "Sr. Engineer",
            "Staff Engineer",
            "Principal Architect",
            "Lead QA Engineer",
        ] {
            for text in ["", "Build reliable software."] {
                let result = extract_experience(title, text);
                assert_eq!(
                    (result.min_years, result.max_years, result.raw),
                    (None, None, None),
                    "{title}"
                );
                assert_eq!(result.is_new_grad, TriState::No, "{title}");
                assert!(result.evidence.unwrap().contains(title));
            }
        }
    }

    #[test]
    fn non_role_numbers_and_company_levels_are_not_seniority() {
        for title in [
            "Engineer I",
            "Engineer II",
            "Engineer 1",
            "Engineer 2",
            "Engineer L3",
            "Engineer IC4",
            "Software Engineer E5",
            "Engineer, Platform 3",
            "Engineer for Team III",
            "Engineer 30",
            "Associate Developer",
            "Early Career Software Engineer",
            "Developer - Class of 2025",
            "Rotational Program Engineer",
            "Intern to Hire - Data Analyst",
        ] {
            let result = extract_experience(title, "");
            assert_eq!(
                (result.min_years, result.max_years),
                (None, None),
                "{title}"
            );
            assert_eq!(result.is_new_grad, TriState::Unknown, "{title}");
        }
    }

    #[test]
    fn explicit_graduate_titles_do_not_fabricate_numeric_bounds() {
        for title in [
            "Software Engineer - New Grad 2026",
            "University Graduate Software Engineer",
            "Campus Hire - Tech Specialist",
            "Entry-Level Software Engineer",
            "Recent Graduate Developer",
        ] {
            let result = extract_experience(title, "");
            assert_eq!(
                (result.min_years, result.max_years, result.raw),
                (None, None, None),
                "{title}"
            );
            assert_eq!(result.is_new_grad, TriState::Yes, "{title}");
            assert_eq!(result.evidence.as_deref(), Some(title));
        }
    }

    #[test]
    fn explicit_graduate_eligibility_can_come_from_description() {
        for text in [
            "Open to recent graduates.",
            "New graduates are encouraged to apply.",
            "This is an entry-level role.",
            "University graduates are eligible.",
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!(result.min_years, None, "{text}");
            assert_eq!(result.is_new_grad, TriState::Yes, "{text}");
            assert!(result.evidence.is_some());
        }
        let result = extract_experience("Engineer", "You will mentor recent graduates.");
        assert_eq!(result.is_new_grad, TriState::Unknown);
    }

    #[test]
    fn required_general_experience_wins_over_preferred_tool_tenure_in_any_order() {
        for text in [
            "5 years of professional experience required; 1 year of Kubernetes preferred.",
            "1 year of Kubernetes preferred; 5 years of professional experience required.",
            "5 years of professional experience required, and 1 year of Kubernetes preferred.",
            "Required qualifications:\n5 years of professional experience\nPreferred qualifications:\n1 year of experience",
            "Preferred qualifications:\n1 year of experience\nBasic qualifications:\n5 years of professional experience",
        ] {
            let result = extract_experience("SOFTWARE ENGINEER III", text);
            assert_eq!((result.min_years, result.max_years), (Some(5), None), "{text}");
            assert_eq!(result.is_new_grad, TriState::No, "{text}");
            assert!(result.evidence.unwrap().contains("5 years"));
        }
    }

    #[test]
    fn local_preference_does_not_become_a_requirement() {
        for text in [
            "5 years of professional experience preferred.",
            "Desired qualifications:\n5 years of experience",
            "3 years of experience is a plus.",
            "Preferred: 5 years of work experience",
            "Required qualifications:\n5 years of professional experience preferred.",
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!((result.min_years, result.max_years), (None, None), "{text}");
            assert_eq!(result.is_new_grad, TriState::Unknown, "{text}");
        }
        let result = extract_experience(
            "Engineer",
            "Preferred qualifications:\nMust have 3 years of professional experience.",
        );
        assert_eq!(result.min_years, Some(3));
    }

    #[test]
    fn zero_does_not_short_circuit_mandatory_experience() {
        for text in [
            "No experience required. Must have 5 years of professional experience.",
            "Must have 5 years of professional experience. No experience required.",
            "0-1 years of experience. Minimum 5 years of professional experience required.",
        ] {
            let result = extract_experience("New Graduate Engineer", text);
            assert_eq!(
                (result.min_years, result.max_years),
                (Some(5), None),
                "{text}"
            );
            assert_eq!(result.is_new_grad, TriState::No, "{text}");
            assert!(result.evidence.unwrap().contains("5 years"));
        }
    }

    #[test]
    fn mandatory_two_years_overrides_graduate_language() {
        let result = extract_experience(
            "New Grad Engineer",
            "Recent graduates are welcome. At least 2 years of professional experience required.",
        );
        assert_eq!(result.min_years, Some(2));
        assert_eq!(result.is_new_grad, TriState::No);
        assert!(result.evidence.unwrap().contains("2 years"));
    }

    #[test]
    fn job_wide_zero_or_zero_to_one_is_positive_without_title_contradictions() {
        for text in [
            "Requires 0 years of experience.",
            "No experience required for this training role.",
            "No prior experience is necessary.",
            "0–1 years of professional experience.",
        ] {
            let result = extract_experience("Analyst", text);
            assert_eq!(result.min_years, Some(0), "{text}");
            assert_eq!(result.is_new_grad, TriState::Yes, "{text}");
            let senior = extract_experience("Engineer III", text);
            assert_eq!(senior.is_new_grad, TriState::No, "{text}");
            let evidence = senior.evidence.unwrap();
            assert!(evidence.contains("Engineer III"));
            assert!(evidence.contains("experience"));
        }
        for text in ["0-2 years of experience.", "1+ years of experience."] {
            assert_eq!(
                extract_experience("Engineer", text).is_new_grad,
                TriState::Unknown,
                "{text}"
            );
        }
    }

    #[test]
    fn conjunctions_take_strongest_general_minimum() {
        for text in [
            "5 years overall and 1 year Rust.",
            "3 years of work experience and 5 years of professional experience.",
            "Must have 3 years of work experience. At least 5 years of professional experience.",
            "Required qualifications:\n3 years of work experience\n5 years of professional experience",
            "5 years of professional experience required and 1 year of experience with Rust.",
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!(result.min_years, Some(5), "{text}");
            assert_eq!(result.is_new_grad, TriState::No, "{text}");
        }
    }

    #[test]
    fn education_or_uses_least_qualifying_branch_not_an_assumed_degree() {
        for text in [
            "Bachelor's + 3 years OR Master's + 1 year",
            "Master's + 1 year OR Bachelor's + 3 years",
            "Requires a BS with 5+ years of experience, or an MS with 3+ years of experience.",
            "Bachelor's and 3 years of professional experience OR Master's and 1 year of professional experience.",
        ] {
            let result = extract_experience("Engineer", text);
            let expected = if text.contains("5+") { 3 } else { 1 };
            assert_eq!(result.min_years, Some(expected), "{text}");
            assert!(result.evidence.unwrap().contains("years"));
        }
        let result = extract_experience("Engineer", "Bachelor's + 3 years OR Master's + 1 year");
        assert_eq!(result.is_new_grad, TriState::Unknown);
    }

    #[test]
    fn unresolved_alternatives_and_unrelated_thresholds_stay_unknown_with_evidence() {
        for text in [
            "5 years of professional experience or 1 year of Rust.",
            "Bachelor's + 3 years OR Master's",
            "5 years of professional experience. 2 years of work experience.",
            "2 years of work experience. 5 years of professional experience.",
        ] {
            let result = extract_experience("New Grad Engineer", text);
            assert_eq!(
                (result.min_years, result.max_years, result.raw),
                (None, None, None),
                "{text}"
            );
            assert_eq!(result.is_new_grad, TriState::Unknown, "{text}");
            let evidence = result.evidence.unwrap();
            assert!(evidence.contains("years"), "{text}");
        }
    }

    #[test]
    fn tool_specific_experience_does_not_supply_general_years_or_new_grad() {
        for text in [
            "5+ years of experience in Python. 2-4 years of experience in Kubernetes.",
            "1 year of experience with Rust required.",
            "0 years of experience with Kubernetes required.",
            "No experience required with Python.",
            "5 years of professional experience using Kubernetes.",
            "5 years Python experience required.",
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!((result.min_years, result.max_years), (None, None), "{text}");
            assert_eq!(result.is_new_grad, TriState::Unknown, "{text}");
        }
    }

    #[test]
    fn negated_numeric_requirements_do_not_supply_years() {
        for text in [
            "5 years of professional experience is not required.",
            "We do not require 3 years of experience.",
            "No minimum 5 years of experience.",
            "3 years of experience is not necessary.",
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!(result.min_years, None, "{text}");
            assert_eq!(result.is_new_grad, TriState::Unknown, "{text}");
        }
    }

    #[test]
    fn degree_duration_company_history_and_roadmap_are_not_experience() {
        for text in [
            "Requires a 4-year degree in Computer Science.",
            "Requires a 2-year degree or equivalent technical certification.",
            "Our company has over 25 years of professional experience.",
            "Our team has 20 years of combined experience.",
            "We have been 15 years in business helping startups scale.",
            "Our mission for the next 5 years is to revolutionize AI compute.",
        ] {
            let result = extract_experience("Associate Developer", text);
            assert_eq!(result.min_years, None, "{text}");
            assert_eq!(result.is_new_grad, TriState::Unknown, "{text}");
        }
        let result = extract_experience(
            "Engineer",
            "Requires a 4-year degree and 3 years of professional experience.",
        );
        assert_eq!(result.min_years, Some(3));
    }

    #[test]
    fn explicit_not_entry_level_overrides_graduate_title() {
        let result = extract_experience("New Grad Developer", "This is not an entry-level role.");
        assert_eq!(result.is_new_grad, TriState::No);
        assert_eq!(result.min_years, None);
        assert!(result.evidence.unwrap().contains("not an entry-level"));
    }

    #[test]
    fn empty_and_unspecified_postings_remain_unknown() {
        for text in ["", "Great company culture, competitive perks, free lunch."] {
            let result = extract_experience("Full Stack Developer", text);
            assert_eq!(
                (result.min_years, result.max_years, result.raw),
                (None, None, None)
            );
            assert_eq!(result.is_new_grad, TriState::Unknown);
            assert_eq!(result.evidence, None);
        }
    }

    #[test]
    fn excessive_and_inverted_numbers_do_not_become_requirements() {
        for text in [
            "300 years of experience required.",
            "5-3 years of experience required.",
            "999999999999999999999999 years of experience required.",
        ] {
            assert_eq!(
                extract_experience("Engineer", text).min_years,
                None,
                "{text}"
            );
        }
    }

    #[test]
    fn maximum_only_is_not_a_known_minimum() {
        for text in [
            "At most 5 years of professional experience.",
            "Up to 5 years of experience.",
            "No more than 5 years of work experience.",
        ] {
            let result = extract_experience("Engineer", text);
            assert_eq!(
                (result.min_years, result.max_years),
                (None, Some(5)),
                "{text}"
            );
            assert_eq!(result.is_new_grad, TriState::Unknown, "{text}");
            assert!(result.evidence.unwrap().contains("5 years"));
        }
    }

    #[test]
    fn truncated_input_cannot_prove_entry_eligibility() {
        let text = format!(
            "No experience required.\n{}Minimum 5 years of professional experience required.",
            "é".repeat(MAX_INPUT_BYTES)
        );
        let result = extract_experience("New Grad Engineer", &text);
        assert_eq!((result.min_years, result.max_years), (None, None));
        assert_eq!(result.is_new_grad, TriState::Unknown);
        assert!(result.evidence.unwrap().contains("No experience required"));
    }
}
