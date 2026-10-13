use crate::types::{ClearanceLevel, TriState, WorkAuthRequirement};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClassifiedSponsorship {
    pub value: TriState,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClassifiedClearance {
    pub value: TriState,
    pub level: Option<ClearanceLevel>,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClassifiedWorkAuth {
    pub value: WorkAuthRequirement,
    pub evidence: Option<String>,
}

static SPONSORSHIP_NO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:unable|not able|cannot|can't|will not|won't|does not|do not)\s+(?:to\s+)?(?:provide\s+|offer\s+)?(?:visa\s+)?sponsor|\bwithout\s+(?:the\s+need\s+for\s+)?(?:current\s+or\s+future\s+)?(?:visa\s+)?sponsorship|\bno\s+(?:visa\s+)?sponsorship\b|\bmust\s+be\s+(?:a\s+)?u\.?s\.?\s+citizen|\bu\.?s\.?\s+citizenship\s+(?:is\s+)?required|\bgreen\s+card\s+holders?\s+only|\b(?:itar|ear)\s+compliance\b|\bu\.?s\.?\s+person\s+(?:status\s+)?required").unwrap()
});

static SPONSORSHIP_YES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:visa\s+)?sponsorship\s+(?:is\s+)?(?:available|provided|offered|supported)|\b(?:we\s+|will\s+)sponsor\b|\bh-?1b\s+(?:sponsorship|transfer)s?\s+(?:available|supported|welcome)|\bopen\s+to\s+sponsor(?:ing)?\b").unwrap()
});

static CLEARANCE_NO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bno\s+(?:security\s+)?clearance\s+(?:is\s+)?required|\bsecurity\s+clearance\s+(?:is\s+)?not\s+required").unwrap()
});

static CLEARANCE_YES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:active|current)?\s*(ts/sci|top\s+secret|secret|public\s+trust)\s*(?:security\s+)?clearance|\bsecurity\s+clearance\s+(?:is\s+)?required|\bmust\s+(?:be\s+able\s+to\s+)?(?:obtain|maintain|hold)\s+(?:a\s+)?(?:security\s+)?clearance|\bpolygraph\s+(?:examination\s+)?required").unwrap()
});

static WORK_AUTH_EXCLUDE_GC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:permanent\s+residents?|green\s+card\s+holders?)\s+(?:are\s+)?(?:not\s+eligible|cannot\s+be\s+considered)\b|\bonly\s+u\.?s\.?\s+citizens?\s+(?:will\s+be\s+considered|are\s+eligible)\b|\bu\.?s\.?\s+citizenship\s+only\b").unwrap()
});

static WORK_AUTH_US_PERSON_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:u\.?s\.?\s+person\s+(?:status|required|status\s+required)?|requires?\s+(?:a\s+)?u\.?s\.?\s+person|must\s+be\s+(?:a\s+)?u\.?s\.?\s+person)\b|\b(?:u\.?s\.?\s+citizen\s+or\s+(?:lawful\s+)?permanent\s+resident|u\.?s\.?\s+citizen\s+or\s+green\s+card\s+holder)\b|\b(?:green\s+card\s+or\s+u\.?s\.?\s+citizen|green\s+card\s+holders?\s+only)\b|\b(?:compliance\s+with\s+(?:the\s+)?(?:itar|ear)|(?:itar|ear)\s+(?:regulations?|compliance|export\s+controls?|requirements?|restrictions?))\b|\b(?:lawful\s+permanent\s+resident|green\s+card\s+holder)\s+(?:status\s+)?required\b").unwrap()
});

static WORK_AUTH_CITIZEN_ONLY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bmust\s+be\s+(?:a\s+)?u\.?s\.?\s+citizen\b|\bu\.?s\.?\s+citizenship\s+(?:is\s+)?required\b").unwrap()
});

static WORK_AUTH_OPEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:authorized\s+to\s+work\s+in\s+the\s+u\.?s\.?|authorization\s+to\s+work\s+in\s+the\s+united\s+states)\b").unwrap()
});

/// Split text into sentences for scoped context analysis
fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let len = chars.len();

    for i in 0..len {
        let (idx, c) = chars[i];
        let is_delimiter = if c == '\n' || c == '•' || c == ';' {
            true
        } else if c == '.' {
            let followed_by_space = i + 1 >= len || chars[i + 1].1.is_whitespace();
            let is_abbreviation = i >= 2 && chars[i - 2].1 == '.' && chars[i - 1].1.is_alphabetic();
            let prev_is_single_letter = i >= 1
                && chars[i - 1].1.is_alphabetic()
                && (i == 1 || !chars[i - 2].1.is_alphabetic());
            followed_by_space && !is_abbreviation && !prev_is_single_letter
        } else {
            false
        };

        if is_delimiter {
            let sentence = text[start..idx].trim();
            if sentence.len() >= 6 {
                sentences.push(sentence);
            }
            start = idx + c.len_utf8();
        }
    }

    if start < text.len() {
        let sentence = text[start..].trim();
        if sentence.len() >= 6 {
            sentences.push(sentence);
        }
    }

    sentences
}

pub fn classify_sponsorship(text: &str) -> ClassifiedSponsorship {
    let sentences = split_sentences(text);

    // Rule 1: "No" beats "Yes" in the same posting
    for sentence in &sentences {
        if sentence.to_lowercase().contains("will you require")
            || sentence
                .to_lowercase()
                .contains("do you now or in the future")
        {
            continue; // Ignore application questionnaire prompt
        }
        if SPONSORSHIP_NO_RE.is_match(sentence) {
            return ClassifiedSponsorship {
                value: TriState::No,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    // Rule 2: Check for "Yes"
    for sentence in &sentences {
        if SPONSORSHIP_YES_RE.is_match(sentence) {
            return ClassifiedSponsorship {
                value: TriState::Yes,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    ClassifiedSponsorship {
        value: TriState::Unknown,
        evidence: None,
    }
}

pub fn classify_clearance(text: &str) -> ClassifiedClearance {
    let sentences = split_sentences(text);

    // Rule 1: Check for explicit "No clearance required"
    for sentence in &sentences {
        if CLEARANCE_NO_RE.is_match(sentence) {
            return ClassifiedClearance {
                value: TriState::No,
                level: None,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    // Rule 2: Check for "Yes" clearance requirements
    for sentence in &sentences {
        if let Some(caps) = CLEARANCE_YES_RE.captures(sentence) {
            let level = caps.get(1).map(|m| {
                let lvl = m.as_str().to_lowercase();
                if lvl.contains("ts/sci") {
                    ClearanceLevel::TsSci
                } else if lvl.contains("top secret") {
                    ClearanceLevel::TopSecret
                } else if lvl.contains("secret") {
                    ClearanceLevel::Secret
                } else if lvl.contains("public trust") {
                    ClearanceLevel::PublicTrust
                } else {
                    ClearanceLevel::Secret
                }
            });

            return ClassifiedClearance {
                value: TriState::Yes,
                level,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    ClassifiedClearance {
        value: TriState::Unknown,
        level: None,
        evidence: None,
    }
}

pub fn classify_work_auth(text: &str) -> ClassifiedWorkAuth {
    let sentences = split_sentences(text);

    let filtered_sentences: Vec<&str> = sentences
        .into_iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            !lower.contains("are you a u.s. citizen")
                && !lower.contains("are you legally authorized")
                && !lower.contains("do you now or in the future")
                && !lower.contains("will you require")
        })
        .collect();

    // Rule 1: Explicit Green Card / Permanent Resident exclusions -> CitizenOnly
    for sentence in &filtered_sentences {
        if WORK_AUTH_EXCLUDE_GC_RE.is_match(sentence) {
            return ClassifiedWorkAuth {
                value: WorkAuthRequirement::CitizenOnly,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    // Rule 2: US Person (Citizen OR Green Card / Permanent Resident under ITAR/EAR) -> UsPerson
    for sentence in &filtered_sentences {
        if WORK_AUTH_US_PERSON_RE.is_match(sentence) {
            return ClassifiedWorkAuth {
                value: WorkAuthRequirement::UsPerson,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    // Rule 3: US Citizenship required (without Green Card allowance) -> CitizenOnly
    for sentence in &filtered_sentences {
        if WORK_AUTH_CITIZEN_ONLY_RE.is_match(sentence) {
            let lower = sentence.to_lowercase();
            if !lower.contains("green card") && !lower.contains("permanent resident") {
                return ClassifiedWorkAuth {
                    value: WorkAuthRequirement::CitizenOnly,
                    evidence: Some(sentence.to_string()),
                };
            }
        }
    }

    // Rule 4: Explicit Open authorization statement -> Open
    for sentence in &filtered_sentences {
        if WORK_AUTH_OPEN_RE.is_match(sentence) {
            return ClassifiedWorkAuth {
                value: WorkAuthRequirement::Open,
                evidence: Some(sentence.to_string()),
            };
        }
    }

    ClassifiedWorkAuth {
        value: WorkAuthRequirement::Unknown,
        evidence: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_auth_us_person() {
        let samples = vec![
            "To conform to U.S. Government export regulations, applicant must be a U.S. Person: U.S. Citizen or lawful Permanent Resident.",
            "Must be a U.S. Citizen or Green Card holder under ITAR regulations.",
            "Position requires U.S. person status.",
            "Green card holders only for this hardware program.",
            "Requires compliance with EAR export controls for permanent residents and citizens.",
        ];

        for s in samples {
            let res = classify_work_auth(s);
            assert_eq!(res.value, WorkAuthRequirement::UsPerson, "Failed on: {}", s);
            assert!(res.evidence.is_some());
        }
    }

    #[test]
    fn test_work_auth_citizen_only() {
        let samples = vec![
            "Must be a U.S. citizen due to federal defense contract requirements.",
            "Only U.S. citizens are eligible for this security role. Green card holders are not eligible.",
            "U.S. citizenship required for high-assurance cryptography work.",
        ];

        for s in samples {
            let res = classify_work_auth(s);
            assert_eq!(res.value, WorkAuthRequirement::CitizenOnly, "Failed on: {}", s);
            assert!(res.evidence.is_some());
        }
    }

    #[test]
    fn test_work_auth_open_and_unknown() {
        let open_text = "Candidates must be authorized to work in the U.S. upon hire.";
        let res_open = classify_work_auth(open_text);
        assert_eq!(res_open.value, WorkAuthRequirement::Open);
        assert!(res_open.evidence.is_some());

        let unknown_text = "Looking for a seasoned Rust backend developer.";
        let res_unknown = classify_work_auth(unknown_text);
        assert_eq!(res_unknown.value, WorkAuthRequirement::Unknown);
        assert!(res_unknown.evidence.is_none());
    }

    #[test]
    fn test_sponsorship_no() {
        let samples = vec![
            "We are unable to provide visa sponsorship for this position.",
            "Candidates must be able to work without sponsorship.",
            "Must be a U.S. citizen due to government contracts.",
            "Green card holders only for this team.",
            "No visa sponsorship available.",
        ];

        for s in samples {
            let res = classify_sponsorship(s);
            assert_eq!(res.value, TriState::No, "Failed on: {}", s);
            assert!(res.evidence.is_some());
        }
    }

    #[test]
    fn test_sponsorship_yes() {
        let samples = vec![
            "Visa sponsorship is available for qualified applicants.",
            "We sponsor H-1B transfers and international candidates.",
            "H-1B sponsorship welcome.",
            "Open to sponsoring candidates located globally.",
        ];

        for s in samples {
            let res = classify_sponsorship(s);
            assert_eq!(res.value, TriState::Yes, "Failed on: {}", s);
            assert!(res.evidence.is_some());
        }
    }

    #[test]
    fn test_sponsorship_priority_no_beats_yes() {
        let text = "We provide visa sponsorship for many engineering roles; however, for this defense unit we cannot sponsor.";
        let res = classify_sponsorship(text);
        assert_eq!(res.value, TriState::No);
    }

    #[test]
    fn test_clearance_yes() {
        let text = "Must possess an active Top Secret security clearance upon hire.";
        let res = classify_clearance(text);
        assert_eq!(res.value, TriState::Yes);
        assert_eq!(res.level, Some(ClearanceLevel::TopSecret));

        let text2 = "Requires active TS/SCI clearance with polygraph.";
        let res2 = classify_clearance(text2);
        assert_eq!(res2.value, TriState::Yes);
        assert_eq!(res2.level, Some(ClearanceLevel::TsSci));
    }

    #[test]
    fn test_clearance_no() {
        let text = "No security clearance is required for this civilian role.";
        let res = classify_clearance(text);
        assert_eq!(res.value, TriState::No);
    }

    #[test]
    fn test_clearance_false_positive() {
        let text = "Don't miss our annual summer clearance sale!";
        let res = classify_clearance(text);
        assert_eq!(res.value, TriState::Unknown);
    }
}
