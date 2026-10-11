use crate::types::{ClearanceLevel, TriState};
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

#[cfg(test)]
mod tests {
    use super::*;

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
