use crate::types::{SalaryInterval, SalarySource};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Salary {
    #[ts(type = "number | null")]
    pub min: Option<i64>,
    #[ts(type = "number | null")]
    pub max: Option<i64>,
    pub currency: String,
    pub interval: SalaryInterval,
    pub raw: String,
    pub source: SalarySource,
}

static SALARY_RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[$€£]|USD|CAD|EUR|GBP)?\s*(\d{1,3}(?:,\d{3})+|\d{2,6})\s*(k|K)?\s*(?:-|–|—|to)\s*[$€£]?\s*(\d{1,3}(?:,\d{3})+|\d{2,6})\s*(k|K)?(?:\s*(USD|CAD|EUR|GBP))?(?:\s*(?:/|\bper\b|\ba\b)\s*(year|yr|annually|annual|hour|hr|hourly|month|mo|week|wk|day))?").unwrap()
});

static HOURLY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[$€£]|USD|CAD|EUR|GBP)?\s*(\d{2,3}(?:\.\d{1,2})?)\s*(?:-|–|—|to)\s*[$€£]?\s*(\d{2,3}(?:\.\d{1,2})?)\s*(?:/|\bper\b|\ba\b)\s*(hour|hr|hourly)").unwrap()
});

static SINGLE_SALARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[$€£]|USD|CAD|EUR|GBP)\s*(\d{1,3}(?:,\d{3})+|\d{2,6})\s*(k|K)?(?:\s*(?:/|\bper\b|\ba\b)\s*(year|yr|annually|annual|hour|hr|hourly|month|mo|week|wk|day))?").unwrap()
});

static CONTEXT_GUARD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(salary|compensation|pay|base pay|base salary|range|rate|hourly|annual|annually|per year|per hour|/yr|/hr)\b").unwrap()
});

static FALSE_POSITIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(401\s*\(k\)|series [a-z]|seed round|venture|stipend|bonus up to|equity|signing bonus|relocation)\b").unwrap()
});

impl Salary {
    pub fn annualize(amount: f64, interval: SalaryInterval) -> i64 {
        let annualized = match interval {
            SalaryInterval::Hour => amount * 2080.0,
            SalaryInterval::Day => amount * 260.0,
            SalaryInterval::Week => amount * 52.0,
            SalaryInterval::Month => amount * 12.0,
            SalaryInterval::Year => amount,
        };
        annualized.round() as i64
    }

    pub fn from_structured(
        min: Option<f64>,
        max: Option<f64>,
        currency: Option<&str>,
        interval_str: Option<&str>,
        raw_display: Option<&str>,
    ) -> Option<Self> {
        let curr = currency.unwrap_or("USD").trim().to_uppercase();
        let interval = match interval_str.map(|s| s.to_lowercase()).as_deref() {
            Some("hour") | Some("hr") | Some("hourly") => SalaryInterval::Hour,
            Some("day") | Some("daily") => SalaryInterval::Day,
            Some("week") | Some("weekly") => SalaryInterval::Week,
            Some("month") | Some("monthly") => SalaryInterval::Month,
            _ => SalaryInterval::Year,
        };

        let min_val = min.filter(|&m| m > 0.0);
        let max_val = max.filter(|&m| m > 0.0);

        if min_val.is_none() && max_val.is_none() {
            return None;
        }

        let raw = if let Some(r) = raw_display.filter(|r| !r.trim().is_empty()) {
            r.to_string()
        } else {
            match (min_val, max_val) {
                (Some(lo), Some(hi)) => format!("${:.0} - ${:.0}", lo, hi),
                (Some(lo), None) => format!("${:.0}+", lo),
                (None, Some(hi)) => format!("Up to ${:.0}", hi),
                (None, None) => unreachable!(),
            }
        };

        let annualized_min = min_val.map(|m| Self::annualize(m, interval));
        let annualized_max = max_val.map(|m| Self::annualize(m, interval));

        Some(Salary {
            min: annualized_min,
            max: annualized_max,
            currency: curr,
            interval,
            raw,
            source: SalarySource::Structured,
        })
    }

    pub fn from_text(text: &str) -> Option<Self> {
        if text.trim().is_empty() {
            return None;
        }

        // Must have salary-related context words nearby
        if !CONTEXT_GUARD_RE.is_match(text) {
            return None;
        }

        // Detect currency
        let currency = if text.contains('€') || text.contains("EUR") {
            "EUR".to_string()
        } else if text.contains('£') || text.contains("GBP") {
            "GBP".to_string()
        } else if text.contains("CAD") {
            "CAD".to_string()
        } else {
            "USD".to_string()
        };

        // 1. Try hourly range first
        if let Some(caps) = HOURLY_RE.captures(text) {
            let lo: f64 = caps[1].parse().ok()?;
            let hi: f64 = caps[2].parse().ok()?;
            if lo >= 10.0 && hi <= 500.0 && lo <= hi {
                let raw = caps[0].to_string();
                return Some(Salary {
                    min: Some(Self::annualize(lo, SalaryInterval::Hour)),
                    max: Some(Self::annualize(hi, SalaryInterval::Hour)),
                    currency,
                    interval: SalaryInterval::Hour,
                    raw,
                    source: SalarySource::Text,
                });
            }
        }

        // 2. Try salary range
        if let Some(caps) = SALARY_RANGE_RE.captures(text) {
            let matched_span = caps.get(0).unwrap().as_str();
            if FALSE_POSITIVE_RE.is_match(matched_span) {
                return None;
            }

            let lo_str = caps[1].replace(',', "");
            let hi_str = caps[3].replace(',', "");

            let mut lo: f64 = lo_str.parse().ok()?;
            let mut hi: f64 = hi_str.parse().ok()?;

            let is_lo_k = caps.get(2).is_some();
            let is_hi_k = caps.get(4).is_some();

            if is_lo_k || (is_hi_k && lo < 1000.0) {
                lo *= 1000.0;
            }
            if is_hi_k {
                hi *= 1000.0;
            }

            let interval = match caps.get(6).map(|m| m.as_str().to_lowercase()).as_deref() {
                Some("hour") | Some("hr") | Some("hourly") => SalaryInterval::Hour,
                Some("month") | Some("mo") => SalaryInterval::Month,
                Some("week") | Some("wk") => SalaryInterval::Week,
                Some("day") => SalaryInterval::Day,
                _ => SalaryInterval::Year,
            };

            // Range checks
            if interval == SalaryInterval::Year && (lo < 10000.0 || hi > 2_500_000.0 || lo > hi) {
                return None;
            }

            return Some(Salary {
                min: Some(Self::annualize(lo, interval)),
                max: Some(Self::annualize(hi, interval)),
                currency,
                interval,
                raw: matched_span.trim().to_string(),
                source: SalarySource::Text,
            });
        }

        // 3. Try single salary
        if let Some(caps) = SINGLE_SALARY_RE.captures(text) {
            let matched_span = caps.get(0).unwrap().as_str();
            if FALSE_POSITIVE_RE.is_match(matched_span) {
                return None;
            }

            let val_str = caps[1].replace(',', "");
            let mut val: f64 = val_str.parse().ok()?;

            if caps.get(2).is_some() {
                val *= 1000.0;
            }

            let interval = match caps.get(3).map(|m| m.as_str().to_lowercase()).as_deref() {
                Some("hour") | Some("hr") | Some("hourly") => SalaryInterval::Hour,
                Some("month") | Some("mo") => SalaryInterval::Month,
                Some("week") | Some("wk") => SalaryInterval::Week,
                Some("day") => SalaryInterval::Day,
                _ => SalaryInterval::Year,
            };

            if interval == SalaryInterval::Year && (val < 15000.0 || val > 2_500_000.0) {
                return None;
            }

            return Some(Salary {
                min: Some(Self::annualize(val, interval)),
                max: None,
                currency,
                interval,
                raw: matched_span.trim().to_string(),
                source: SalarySource::Text,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salary_structured() {
        let s = Salary::from_structured(Some(120000.0), Some(160000.0), Some("USD"), Some("year"), None).unwrap();
        assert_eq!(s.min, Some(120000));
        assert_eq!(s.max, Some(160000));
        assert_eq!(s.currency, "USD");
        assert_eq!(s.interval, SalaryInterval::Year);
    }

    #[test]
    fn test_salary_structured_hourly() {
        let s = Salary::from_structured(Some(50.0), Some(75.0), Some("USD"), Some("hour"), None).unwrap();
        assert_eq!(s.min, Some(104000));
        assert_eq!(s.max, Some(156000));
        assert_eq!(s.interval, SalaryInterval::Hour);
    }

    #[test]
    fn test_salary_from_text_ranges() {
        let test_cases = vec![
            ("Base salary range: $120,000 - $150,000 per year.", 120000, 150000),
            ("Expected compensation: $120k - $150k.", 120000, 150000),
            ("Salary: $140k – 180k /yr", 140000, 180000),
            ("The pay range is USD 130,000 to 175,000 annually.", 130000, 175000),
            ("Salary range: $100,000–$130,000.", 100000, 130000),
            ("Base compensation: $90k to $120k / year", 90000, 120000),
        ];

        for (text, expected_min, expected_max) in test_cases {
            let res = Salary::from_text(text).unwrap_or_else(|| panic!("Failed to parse: {}", text));
            assert_eq!(res.min, Some(expected_min), "Failed min on: {}", text);
            assert_eq!(res.max, Some(expected_max), "Failed max on: {}", text);
        }
    }

    #[test]
    fn test_salary_from_text_hourly() {
        let s = Salary::from_text("Hourly rate: $60 - $85 per hour depending on experience.").unwrap();
        assert_eq!(s.min, Some(124800));
        assert_eq!(s.max, Some(176800));
        assert_eq!(s.interval, SalaryInterval::Hour);
    }

    #[test]
    fn test_salary_from_text_currencies() {
        let gbp = Salary::from_text("Salary range: £60,000 - £80,000 per annum.").unwrap();
        assert_eq!(gbp.currency, "GBP");

        let eur = Salary::from_text("Base compensation: €70,000 - €95,000 / year.").unwrap();
        assert_eq!(eur.currency, "EUR");
    }

    #[test]
    fn test_salary_false_positives() {
        let false_positives = vec![
            "We recently raised a $5M Series A round.",
            "Benefits include a 401(k) matching plan up to 5%.",
            "Includes a $100 stipend for wellness.",
            "We have over 500,000 users worldwide.",
            "Looking for 5-10 years of experience.",
        ];

        for text in false_positives {
            assert!(Salary::from_text(text).is_none(), "Should reject: {}", text);
        }
    }
}
