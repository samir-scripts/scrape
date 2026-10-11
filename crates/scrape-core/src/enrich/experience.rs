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

// Title heuristics
static TITLE_NEW_GRAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:new\s+grad(?:uate)?|university\s+grad(?:uate)?|campus\s+hire|early\s+career|class\s+of\s+202[4-7]|intern\s+to\s+hire|rotational\s+program|entry\s+level|associate\s+engineer|associate\s+developer)\b").unwrap()
});

static TITLE_SENIOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:senior|sr\.?|lead)\b").unwrap());

static TITLE_STAFF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:staff|principal|distinguished|architect|director)\b").unwrap()
});

// Non-entry level disqualifiers in text
static NOT_ENTRY_LEVEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:this\s+is\s+not\s+an\s+entry[- ]level\s+role|not\s+an\s+entry[- ]level\s+position|minimum\s+\d+\s+years\s+of\s+post[- ]grad(?:uate)?\s+experience)\b").unwrap()
});

// Text YOE patterns
static YOE_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+)\s*(?:-|–|—|to)\s*(\d+)\+?\s*(?:years?|yrs?)(?:\s+of)?(?:\s+(?:relevant|professional|software|industry|work|hands-on|practical|engineering|technical|post[- ]graduate|related|direct|prior|development|coding|full[- ]time)){0,3}\s*experience\b").unwrap()
});

static YOE_PLUS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+)\+\s*(?:years?|yrs?)(?:\s+of)?(?:\s+(?:relevant|professional|software|industry|work|hands-on|practical|engineering|technical|post[- ]graduate|related|direct|prior|development|coding|full[- ]time)){0,3}\s*experience\b").unwrap()
});

static YOE_MIN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:at\s+least|minimum\s+of|minimum)\s*(\d+)\s*(?:years?|yrs?)(?:\s+of)?(?:\s+(?:relevant|professional|software|industry|work|hands-on|practical|engineering|technical|post[- ]graduate|related|direct|prior|development|coding|full[- ]time)){0,3}\s*experience\b").unwrap()
});

static YOE_EXACT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+)\s*(?:years?|yrs?)(?:\s+of)?(?:\s+(?:relevant|professional|software|industry|work|hands-on|practical|engineering|technical|post[- ]graduate|related|direct|prior|development|coding|full[- ]time)){0,3}\s*experience\b").unwrap()
});

static YOE_ZERO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:0|no)\s*(?:years?|yrs?)(?:\s+of)?\s*experience\b|\bno\s+experience\s+required\b",
    )
    .unwrap()
});

// False positive guards in sentence context
static FALSE_POSITIVE_DEGREE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:\d+[- ]year|two[- ]year|three[- ]year|four[- ]year)\s*(?:degree|bachelor|program)\b").unwrap()
});

static FALSE_POSITIVE_COMPANY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:in\s+business|of\s+history|company\s+history|founded|combined\s+experience|serving\s+clients)\b").unwrap()
});

static FALSE_POSITIVE_ROADMAP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:in\s+the\s+next|next\s+\d+\s+years|over\s+the\s+next)\b").unwrap()
});

/// Split text into candidate sentences
fn split_sentences(text: &str) -> Vec<&str> {
    text.split(['.', '\n', '•', ';', '\r'])
        .map(|s| s.trim())
        .filter(|s| s.len() >= 6)
        .collect()
}

pub fn extract_experience(title: &str, text: &str) -> ExperienceClassification {
    let title_is_new_grad = TITLE_NEW_GRAD_RE.is_match(title);
    let title_is_senior = TITLE_SENIOR_RE.is_match(title);
    let title_is_staff = TITLE_STAFF_RE.is_match(title);

    let sentences = split_sentences(text);

    let mut found_min: Option<i64> = None;
    let mut found_max: Option<i64> = None;
    let mut found_raw: Option<String> = None;
    let mut found_evidence: Option<String> = None;
    let mut is_explicit_zero = false;
    let mut has_not_entry_level = false;

    // Scan sentences for experience requirements
    for sentence in &sentences {
        if NOT_ENTRY_LEVEL_RE.is_match(sentence) {
            has_not_entry_level = true;
        }

        // Check for false positive rejectors on the sentence
        if FALSE_POSITIVE_COMPANY_RE.is_match(sentence)
            || FALSE_POSITIVE_ROADMAP_RE.is_match(sentence)
        {
            continue;
        }

        // Check for zero / entry level
        if YOE_ZERO_RE.is_match(sentence) {
            is_explicit_zero = true;
            found_min = Some(0);
            found_max = Some(0);
            found_raw = Some("0 years".to_string());
            found_evidence = Some(sentence.to_string());
            break;
        }

        if !FALSE_POSITIVE_DEGREE_RE.is_match(sentence) {
            // Check Range e.g. 3-5 years of experience
            for caps in YOE_RANGE_RE.captures_iter(sentence) {
                if let (Ok(lo), Ok(hi)) = (caps[1].parse::<i64>(), caps[2].parse::<i64>()) {
                    if lo <= 30 && hi <= 30 && lo <= hi {
                        // Multi-tier lowest threshold
                        if found_min.is_none() || lo < found_min.unwrap() {
                            found_min = Some(lo);
                            found_max = Some(hi);
                            found_raw = Some(format!("{}-{} years", lo, hi));
                            found_evidence = Some(sentence.to_string());
                        }
                    }
                }
            }

            // Check Plus e.g. 5+ years of experience
            for caps in YOE_PLUS_RE.captures_iter(sentence) {
                if let Ok(lo) = caps[1].parse::<i64>() {
                    if lo <= 30 && (found_min.is_none() || lo < found_min.unwrap()) {
                        found_min = Some(lo);
                        found_max = None;
                        found_raw = Some(format!("{}+ years", lo));
                        found_evidence = Some(sentence.to_string());
                    }
                }
            }

            // Check Minimum e.g. minimum 3 years of experience
            for caps in YOE_MIN_RE.captures_iter(sentence) {
                if let Ok(lo) = caps[1].parse::<i64>() {
                    if lo <= 30 && (found_min.is_none() || lo < found_min.unwrap()) {
                        found_min = Some(lo);
                        found_max = None;
                        found_raw = Some(format!("{}+ years", lo));
                        found_evidence = Some(sentence.to_string());
                    }
                }
            }

            // Check Exact e.g. 3 years of software engineering experience
            for caps in YOE_EXACT_RE.captures_iter(sentence) {
                if let Ok(lo) = caps[1].parse::<i64>() {
                    if lo <= 30 && (found_min.is_none() || lo < found_min.unwrap()) {
                        found_min = Some(lo);
                        found_max = None;
                        found_raw = Some(format!("{}+ years", lo));
                        found_evidence = Some(sentence.to_string());
                    }
                }
            }
        }
    }

    // Apply title fallbacks if text had no experience numbers
    if found_min.is_none() {
        if title_is_new_grad {
            found_min = Some(0);
            found_max = Some(1);
            found_raw = Some("0-1 years".to_string());
            found_evidence = Some(format!("Title indicates early career: {}", title));
        } else if title_is_staff {
            found_min = Some(8);
            found_max = None;
            found_raw = Some("8+ years".to_string());
            found_evidence = Some(format!("Title indicates staff/principal level: {}", title));
        } else if title_is_senior {
            found_min = Some(5);
            found_max = None;
            found_raw = Some("5+ years".to_string());
            found_evidence = Some(format!("Title indicates senior level: {}", title));
        }
    }

    // Determine is_new_grad status
    let is_new_grad = if has_not_entry_level || title_is_senior || title_is_staff {
        TriState::No
    } else if title_is_new_grad || is_explicit_zero {
        TriState::Yes
    } else if let Some(min) = found_min {
        if min == 0 && found_max.unwrap_or(0) <= 1 {
            TriState::Yes
        } else if min >= 2 {
            TriState::No
        } else {
            TriState::Unknown
        }
    } else {
        TriState::Unknown
    };

    ExperienceClassification {
        min_years: found_min,
        max_years: found_max,
        raw: found_raw,
        is_new_grad,
        evidence: found_evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yoe_range_standard() {
        let text = "We require 3-5 years of experience in distributed systems.";
        let res = extract_experience("Software Engineer", text);
        assert_eq!(res.min_years, Some(3));
        assert_eq!(res.max_years, Some(5));
        assert_eq!(res.raw.as_deref(), Some("3-5 years"));
        assert_eq!(res.is_new_grad, TriState::No);
        assert!(res.evidence.unwrap().contains("3-5 years"));
    }

    #[test]
    fn test_yoe_range_en_dash() {
        let text = "Must have 2–4 years of software experience.";
        let res = extract_experience("Backend Dev", text);
        assert_eq!(res.min_years, Some(2));
        assert_eq!(res.max_years, Some(4));
        assert_eq!(res.raw.as_deref(), Some("2-4 years"));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_yoe_plus() {
        let text = "Applicants must possess 5+ years of relevant experience.";
        let res = extract_experience("Engineer", text);
        assert_eq!(res.min_years, Some(5));
        assert_eq!(res.max_years, None);
        assert_eq!(res.raw.as_deref(), Some("5+ years"));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_yoe_minimum() {
        let text = "Minimum of 4 years of professional experience required.";
        let res = extract_experience("Data Engineer", text);
        assert_eq!(res.min_years, Some(4));
        assert_eq!(res.max_years, None);
        assert_eq!(res.raw.as_deref(), Some("4+ years"));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_yoe_at_least() {
        let text = "Requires at least 7 years of industry experience.";
        let res = extract_experience("Software Engineer", text);
        assert_eq!(res.min_years, Some(7));
        assert_eq!(res.max_years, None);
        assert_eq!(res.raw.as_deref(), Some("7+ years"));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_yoe_exact() {
        let text = "Requires 3 years of software engineering experience.";
        let res = extract_experience("Software Engineer", text);
        assert_eq!(res.min_years, Some(3));
        assert_eq!(res.max_years, None);
        assert_eq!(res.raw.as_deref(), Some("3+ years"));
    }

    #[test]
    fn test_yoe_zero() {
        let text = "Requires 0 years of experience. We provide full training.";
        let res = extract_experience("Junior Developer", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(0));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_yoe_no_experience_required() {
        let text = "No experience required for this training role.";
        let res = extract_experience("Analyst", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(0));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_new_grad_title_heuristic() {
        let text = "Join our team and build high scale systems.";
        let res = extract_experience("Software Engineer - New Grad 2026", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(1));
        assert_eq!(res.is_new_grad, TriState::Yes);
        assert!(res.evidence.unwrap().contains("New Grad"));
    }

    #[test]
    fn test_university_grad_title() {
        let text = "Open to recent graduates.";
        let res = extract_experience("University Graduate Software Engineer", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(1));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_campus_hire_title() {
        let text = "Campus rotational program.";
        let res = extract_experience("Campus Hire - Tech Specialist", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(1));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_early_career_title() {
        let text = "Great opportunity for growth.";
        let res = extract_experience("Early Career Software Engineer", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(1));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_class_of_2025_title() {
        let text = "Starts summer 2025.";
        let res = extract_experience("Developer - Class of 2025", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(1));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_entry_level_title() {
        let text = "Learn modern software engineering.";
        let res = extract_experience("Entry Level Software Engineer", text);
        assert_eq!(res.min_years, Some(0));
        assert_eq!(res.max_years, Some(1));
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_senior_title_fallback() {
        let text = "Lead technical initiatives across multiple squads.";
        let res = extract_experience("Senior Software Engineer", text);
        assert_eq!(res.min_years, Some(5));
        assert_eq!(res.max_years, None);
        assert_eq!(res.raw.as_deref(), Some("5+ years"));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_staff_title_fallback() {
        let text = "Set architecture strategy for our cloud platform.";
        let res = extract_experience("Staff Software Engineer", text);
        assert_eq!(res.min_years, Some(8));
        assert_eq!(res.max_years, None);
        assert_eq!(res.raw.as_deref(), Some("8+ years"));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_principal_title_fallback() {
        let text = "Drive technical excellence across the enterprise.";
        let res = extract_experience("Principal Architect", text);
        assert_eq!(res.min_years, Some(8));
        assert_eq!(res.max_years, None);
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_senior_with_explicit_text_wins_over_title() {
        let text = "Requires 7+ years of experience in distributed backend architecture.";
        let res = extract_experience("Senior Software Engineer", text);
        assert_eq!(res.min_years, Some(7));
        assert_eq!(res.max_years, None);
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_false_positive_four_year_degree() {
        let text = "Requires a 4-year degree in Computer Science. Must be passionate.";
        let res = extract_experience("Software Engineer", text);
        // Degree length should not be treated as 4 years of experience
        assert_eq!(res.min_years, None);
        assert_eq!(res.is_new_grad, TriState::Unknown);
    }

    #[test]
    fn test_false_positive_two_year_degree() {
        let text = "Requires a 2-year degree or equivalent technical certification.";
        let res = extract_experience("Associate Developer", text);
        assert_eq!(res.min_years, Some(0)); // From associate developer title
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_false_positive_company_history() {
        let text = "Our company has over 25 years of history serving global clients.";
        let res = extract_experience("Software Engineer", text);
        assert_eq!(res.min_years, None);
    }

    #[test]
    fn test_false_positive_in_business() {
        let text = "We have been 15 years in business helping startups scale.";
        let res = extract_experience("Frontend Engineer", text);
        assert_eq!(res.min_years, None);
    }

    #[test]
    fn test_false_positive_roadmap() {
        let text = "We plan to double our engineering organization in the next 2-3 years.";
        let res = extract_experience("DevOps Engineer", text);
        assert_eq!(res.min_years, None);
    }

    #[test]
    fn test_false_positive_next_5_years() {
        let text = "Our mission for the next 5 years is to revolutionize AI compute.";
        let res = extract_experience("ML Engineer", text);
        assert_eq!(res.min_years, None);
    }

    #[test]
    fn test_multi_tier_lowest_threshold() {
        let text =
            "Requires a BS with 5+ years of experience, or an MS with 3+ years of experience.";
        let res = extract_experience("Backend Engineer", text);
        // Should pick the lowest threshold (3)
        assert_eq!(res.min_years, Some(3));
    }

    #[test]
    fn test_multi_sentence_picks_lowest() {
        let text = "5+ years of experience in Python. 2-4 years of experience in Kubernetes.";
        let res = extract_experience("Infrastructure Engineer", text);
        assert_eq!(res.min_years, Some(2));
        assert_eq!(res.max_years, Some(4));
    }

    #[test]
    fn test_disqualifier_not_entry_level() {
        let text = "This is not an entry-level role. Please only apply if qualified.";
        let res = extract_experience("Developer", text);
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_post_graduate_disqualifier() {
        let text = "Requires minimum 3 years of post-graduate experience.";
        let res = extract_experience("Software Engineer", text);
        assert_eq!(res.min_years, Some(3));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_lead_title() {
        let text = "Oversee sprint execution and mentorship.";
        let res = extract_experience("Lead QA Engineer", text);
        assert_eq!(res.min_years, Some(5));
        assert_eq!(res.is_new_grad, TriState::No);
    }

    #[test]
    fn test_unspecified_posting() {
        let text = "Great company culture, competitive perks, free lunch.";
        let res = extract_experience("Full Stack Developer", text);
        assert_eq!(res.min_years, None);
        assert_eq!(res.max_years, None);
        assert_eq!(res.is_new_grad, TriState::Unknown);
        assert_eq!(res.evidence, None);
    }

    #[test]
    fn test_yoe_range_with_plus() {
        let text = "Looking for 5-7+ years of relevant experience.";
        let res = extract_experience("Senior Developer", text);
        assert_eq!(res.min_years, Some(5));
        assert_eq!(res.max_years, Some(7));
    }

    #[test]
    fn test_rotational_program_title() {
        let text = "Rotate across 3 engineering teams in 2 years.";
        let res = extract_experience("Rotational Program Engineer", text);
        assert_eq!(res.is_new_grad, TriState::Yes);
    }

    #[test]
    fn test_intern_to_hire_title() {
        let text = "Direct pathway to full time.";
        let res = extract_experience("Intern to Hire - Data Analyst", text);
        assert_eq!(res.is_new_grad, TriState::Yes);
    }
}
