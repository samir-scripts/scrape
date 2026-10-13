use crate::types::{SalaryInterval, SalarySource};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use ts_rs::TS;

/// Amounts in hundredths of the reported currency's major unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CompensationRange {
    #[ts(type = "number | null")]
    pub min_minor: Option<i64>,
    #[ts(type = "number | null")]
    pub max_minor: Option<i64>,
    pub currency: Option<String>,
    pub interval: Option<SalaryInterval>,
    pub raw: String,
    pub source: SalarySource,
    pub derived: bool,
    pub location: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Salary {
    // Compatibility analytics fields: derived annual major units, never native pay.
    #[ts(type = "number | null")]
    pub min: Option<i64>,
    #[ts(type = "number | null")]
    pub max: Option<i64>,
    pub currency: String,
    pub interval: SalaryInterval,
    pub raw: String,
    pub source: SalarySource,
    #[serde(default)]
    pub ranges: Vec<CompensationRange>,
}

static RANGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(?i)([$€£]|USD|CAD|EUR|GBP)?\s*(\d[\d,]*(?:\.\d{1,2})?)\s*(k)?\s*(?:-|–|—|to)\s*([$€£]|USD|CAD|EUR|GBP)?\s*(\d[\d,]*(?:\.\d{1,2})?)\s*(k)?(?:\s*(USD|CAD|EUR|GBP))?(?:\s*(?:/|per|a)\s*(year|yr|annum|annually|annual|hour|hr|hourly|month|mo|week|wk|day))?"
).unwrap()
});
static SINGLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
    r"(?i)([$€£]|USD|CAD|EUR|GBP)\s*(\d[\d,]*(?:\.\d{1,2})?)\s*(k)?(?:\s*(?:/|per|a)\s*(year|yr|annum|annually|annual|hour|hr|hourly|month|mo|week|wk|day))?"
).unwrap()
});
static CONTEXT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(salary|compensation|pay|rate|hourly|annual|annually)\b").unwrap()
});

fn parse_interval(value: Option<&str>) -> Option<SalaryInterval> {
    if let Some(value) = value {
        let words = value.to_ascii_lowercase();
        for (word, interval) in [
            ("hourly", SalaryInterval::Hour),
            ("hour", SalaryInterval::Hour),
            ("annual", SalaryInterval::Year),
            ("yearly", SalaryInterval::Year),
            ("year", SalaryInterval::Year),
            ("monthly", SalaryInterval::Month),
            ("weekly", SalaryInterval::Week),
        ] {
            if words
                .split(|c: char| !c.is_ascii_alphabetic())
                .any(|w| w == word)
            {
                return Some(interval);
            }
        }
    }
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("hour" | "hr" | "hourly") => Some(SalaryInterval::Hour),
        Some("day" | "daily") => Some(SalaryInterval::Day),
        Some("week" | "wk" | "weekly") => Some(SalaryInterval::Week),
        Some("month" | "mo" | "monthly") => Some(SalaryInterval::Month),
        Some("year" | "yr" | "annum" | "annual" | "annually" | "yearly") => {
            Some(SalaryInterval::Year)
        }
        _ => None,
    }
}
fn currency(symbol: Option<&str>) -> Option<String> {
    symbol.map(|symbol| match symbol.to_ascii_uppercase().as_str() {
        "$" => "USD".into(),
        "€" => "EUR".into(),
        "£" => "GBP".into(),
        other => other.into(),
    })
}
fn minor(value: f64) -> Option<i64> {
    (value.is_finite() && value >= 0.0 && value <= (i64::MAX / 100) as f64)
        .then(|| (value * 100.0).round() as i64)
}
impl Salary {
    pub fn annual_factor(interval: SalaryInterval) -> i64 {
        match interval {
            SalaryInterval::Hour => 2080,
            SalaryInterval::Day => 260,
            SalaryInterval::Week => 52,
            SalaryInterval::Month => 12,
            SalaryInterval::Year => 1,
        }
    }
    pub fn annualize(amount: f64, interval: SalaryInterval) -> i64 {
        (amount * Self::annual_factor(interval) as f64).round() as i64
    }
    pub fn from_ranges(ranges: Vec<CompensationRange>) -> Option<Self> {
        let first = ranges.first()?;
        // Analytics use one evidenced range, never an invented blend of ranges.
        let min = first.interval.and_then(|i| {
            first
                .min_minor
                .map(|v| Self::annualize(v as f64 / 100.0, i))
        });
        let max = first.interval.and_then(|i| {
            first
                .max_minor
                .map(|v| Self::annualize(v as f64 / 100.0, i))
        });
        Some(Self {
            min,
            max,
            currency: first.currency.clone().unwrap_or_default(),
            interval: first.interval.unwrap_or(SalaryInterval::Year),
            raw: first.raw.clone(),
            source: first.source,
            ranges,
        })
    }
    pub fn from_structured(
        min: Option<f64>,
        max: Option<f64>,
        currency_value: Option<&str>,
        interval_str: Option<&str>,
        raw_display: Option<&str>,
    ) -> Option<Self> {
        if min.is_some_and(|v| minor(v).is_none()) || max.is_some_and(|v| minor(v).is_none()) {
            return None;
        }
        let min_minor = min.and_then(minor);
        let max_minor = max.and_then(minor);
        if min_minor.is_none() && max_minor.is_none()
            || matches!((min_minor, max_minor), (Some(lo), Some(hi)) if lo > hi)
        {
            return None;
        }
        let interval = parse_interval(interval_str);
        let curr = currency_value
            .map(|v| v.trim().to_ascii_uppercase())
            .filter(|v| !v.is_empty());
        let raw = raw_display
            .filter(|r| !r.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                format!(
                    "{} {}–{} {}",
                    curr.as_deref().unwrap_or("Unknown currency"),
                    min.map(|v| format!("{v:.2}")).unwrap_or_default(),
                    max.map(|v| format!("{v:.2}")).unwrap_or_default(),
                    interval_str.unwrap_or("unknown interval")
                )
            });
        Self::from_ranges(vec![CompensationRange {
            min_minor,
            max_minor,
            currency: curr,
            interval,
            raw,
            source: SalarySource::Structured,
            derived: false,
            location: None,
        }])
    }
    pub fn from_text(text: &str) -> Option<Self> {
        if !CONTEXT_RE.is_match(text) {
            return None;
        }
        let mut ranges = Vec::new();
        let mut spans = Vec::new();
        for caps in RANGE_RE.captures_iter(text) {
            let span = caps.get(0)?;
            let amount = |number: &str, multiplier: bool| -> Option<i64> {
                minor(
                    number.replace(',', "").parse::<f64>().ok()?
                        * if multiplier { 1000.0 } else { 1.0 },
                )
            };
            let lo = amount(&caps[2], caps.get(3).is_some() || caps.get(6).is_some())?;
            let hi = amount(&caps[5], caps.get(6).is_some())?;
            if lo > hi {
                continue;
            }
            let curr = currency(
                caps.get(7)
                    .or(caps.get(1))
                    .or(caps.get(4))
                    .map(|m| m.as_str()),
            );
            let preceding = text[..span.start()]
                .chars()
                .rev()
                .take(40)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
            let interval = parse_interval(caps.get(8).map(|m| m.as_str()))
                .or_else(|| parse_interval(Some(&preceding)));
            if curr.is_none() && interval.is_none() {
                continue;
            }
            ranges.push(CompensationRange {
                min_minor: Some(lo),
                max_minor: Some(hi),
                currency: curr,
                interval,
                raw: span.as_str().trim().into(),
                source: SalarySource::Text,
                derived: false,
                location: None,
            });
            spans.push(span.range());
        }
        for caps in SINGLE_RE.captures_iter(text) {
            let span = caps.get(0)?;
            if spans
                .iter()
                .any(|covered| covered.start <= span.start() && span.start() < covered.end)
            {
                continue;
            }
            // Ignore benefit/bonus amounts; they are not base compensation ranges.
            let prefix = &text[..span.start()];
            let recent = prefix
                .chars()
                .rev()
                .take(40)
                .collect::<String>()
                .to_ascii_lowercase();
            let recent = recent.chars().rev().collect::<String>();
            if ["stipend", "bonus", "equity", "relocation"]
                .iter()
                .any(|word| recent.contains(word))
            {
                continue;
            }
            let value = minor(
                caps[2].replace(',', "").parse::<f64>().ok()?
                    * if caps.get(3).is_some() { 1000.0 } else { 1.0 },
            )?;
            let upper = recent.contains("up to");
            ranges.push(CompensationRange {
                min_minor: if upper { None } else { Some(value) },
                max_minor: if upper { Some(value) } else { None },
                currency: currency(Some(&caps[1])),
                interval: parse_interval(caps.get(4).map(|m| m.as_str())),
                raw: span.as_str().trim().into(),
                source: SalarySource::Text,
                derived: false,
                location: None,
            });
        }
        Self::from_ranges(ranges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salary_structured() {
        let s = Salary::from_structured(
            Some(120000.0),
            Some(160000.0),
            Some("USD"),
            Some("year"),
            None,
        )
        .unwrap();
        assert_eq!(s.min, Some(120000));
        assert_eq!(s.max, Some(160000));
        assert_eq!(s.currency, "USD");
        assert_eq!(s.interval, SalaryInterval::Year);
    }

    #[test]
    fn test_salary_structured_hourly() {
        let s = Salary::from_structured(Some(50.0), Some(75.0), Some("USD"), Some("hour"), None)
            .unwrap();
        assert_eq!(s.min, Some(104000));
        assert_eq!(s.max, Some(156000));
        assert_eq!(s.interval, SalaryInterval::Hour);
    }

    #[test]
    fn test_salary_from_text_ranges() {
        let test_cases = vec![
            (
                "Base salary range: $120,000 - $150,000 per year.",
                120000,
                150000,
            ),
            (
                "Expected compensation: $120k - $150k per year.",
                120000,
                150000,
            ),
            ("Salary: $140k – 180k /yr", 140000, 180000),
            (
                "The pay range is USD 130,000 to 175,000 per year.",
                130000,
                175000,
            ),
            ("Salary range: $100,000–$130,000 per year.", 100000, 130000),
            ("Base compensation: $90k to $120k / year", 90000, 120000),
        ];

        for (text, expected_min, expected_max) in test_cases {
            let res =
                Salary::from_text(text).unwrap_or_else(|| panic!("Failed to parse: {}", text));
            assert_eq!(res.min, Some(expected_min), "Failed min on: {}", text);
            assert_eq!(res.max, Some(expected_max), "Failed max on: {}", text);
        }
    }

    #[test]
    fn test_salary_from_text_hourly() {
        let s =
            Salary::from_text("Hourly rate: $60 - $85 per hour depending on experience.").unwrap();
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

/// Recover known backend annualized records without guessing from magnitude.
pub fn legacy_compensation(
    min: Option<i64>,
    max: Option<i64>,
    currency: Option<&str>,
    interval: Option<SalaryInterval>,
    raw: Option<&str>,
    source: Option<SalarySource>,
) -> Vec<CompensationRange> {
    if let Some(extracted) = raw.and_then(|text| Salary::from_text(&format!("Salary: {text}"))) {
        if extracted
            .ranges
            .iter()
            .all(|range| range.interval.is_some())
        {
            return extracted.ranges;
        }
    }
    match (interval, source) {
        (Some(interval), Some(source)) if min.is_some() || max.is_some() => {
            vec![CompensationRange {
                min_minor: min.map(|v| {
                    (v as f64 * 100.0 / Salary::annual_factor(interval) as f64).round() as i64
                }),
                max_minor: max.map(|v| {
                    (v as f64 * 100.0 / Salary::annual_factor(interval) as f64).round() as i64
                }),
                currency: currency.map(str::to_owned),
                interval: Some(interval),
                raw: raw.unwrap_or("").into(),
                source,
                derived: true,
                location: None,
            }]
        }
        _ => Vec::new(),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CompensationFilter {
    #[ts(type = "number | null")]
    pub min_minor: Option<i64>,
    #[ts(type = "number | null")]
    pub max_minor: Option<i64>,
    pub currency: Option<String>,
    pub interval: Option<SalaryInterval>,
    #[serde(default)]
    pub include_unknown: bool,
}
impl CompensationFilter {
    pub fn validate(&self) -> Result<(), String> {
        if self.min_minor.is_some_and(|v| v < 0)
            || self.max_minor.is_some_and(|v| v < 0)
            || matches!((self.min_minor, self.max_minor), (Some(lo), Some(hi)) if lo > hi)
        {
            return Err(
                "Compensation bounds must be nonnegative and minimum must not exceed maximum"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn matches(&self, ranges: &[CompensationRange]) -> bool {
        if self.min_minor.is_none()
            && self.max_minor.is_none()
            && self.currency.is_none()
            && self.interval.is_none()
        {
            return true;
        }
        if ranges.is_empty() {
            return self.include_unknown && self.interval.is_none() && self.currency.is_none();
        }
        ranges.iter().any(|r| {
            if self.interval.is_some() && self.interval != r.interval {
                return false;
            }
            if let Some(currency) = &self.currency {
                if r.currency
                    .as_ref()
                    .is_none_or(|c| !c.eq_ignore_ascii_case(currency))
                {
                    return false;
                }
            }
            if r.min_minor.is_none() && r.max_minor.is_none() {
                return self.include_unknown;
            }
            self.min_minor
                .is_none_or(|lo| r.max_minor.is_none_or(|hi| hi >= lo))
                && self
                    .max_minor
                    .is_none_or(|hi| r.min_minor.is_none_or(|lo| lo <= hi))
        })
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    #[test]
    fn native_cents_and_explicit_interval_are_preserved() {
        let salary =
            Salary::from_structured(Some(45.25), Some(55.75), Some("USD"), Some("hour"), None)
                .unwrap();
        assert_eq!(salary.ranges[0].min_minor, Some(4525));
        let filter = CompensationFilter {
            min_minor: Some(5000),
            max_minor: Some(6000),
            currency: Some("USD".into()),
            interval: Some(SalaryInterval::Hour),
            include_unknown: false,
        };
        assert!(filter.matches(&salary.ranges));
        assert!(filter.matches(
            &Salary::from_structured(None, Some(50.0), Some("USD"), Some("hour"), None)
                .unwrap()
                .ranges
        ));
        assert!(!filter.matches(
            &Salary::from_structured(Some(45.0), Some(55.0), Some("USD"), Some("year"), None)
                .unwrap()
                .ranges
        ));
        assert!(!filter.matches(
            &Salary::from_structured(Some(45.0), Some(55.0), None, Some("hour"), None)
                .unwrap()
                .ranges
        ));
        assert!(!filter.matches(&[]));
    }
    #[test]
    fn separate_currencies_and_unknown_units_are_not_blended() {
        let salary =
            Salary::from_text("Salary: USD 45.25–55.75 per hour; EUR 70,000–90,000 per year.")
                .unwrap();
        assert_eq!(salary.ranges.len(), 2);
        assert_eq!(salary.ranges[0].currency.as_deref(), Some("USD"));
        assert_eq!(salary.ranges[1].currency.as_deref(), Some("EUR"));
        let unknown = Salary::from_text("Salary: $120k–$150k").unwrap();
        assert_eq!(unknown.ranges[0].interval, None);
        assert_eq!(unknown.min, None);
        assert!(
            Salary::from_structured(Some(60.0), Some(50.0), Some("USD"), Some("hour"), None)
                .is_none()
        );
    }
}

#[derive(Debug, Clone)]
pub enum CompensationSqlValue {
    Integer(i64),
    Text(String),
}
impl CompensationFilter {
    /// Compile the same native-range model used by worker evaluation to SQLite.
    pub fn sqlite_predicate(&self) -> (String, Vec<CompensationSqlValue>) {
        let mut clauses = Vec::new();
        let mut values = Vec::new();
        if let Some(interval) = self.interval {
            clauses.push("json_extract(pay.value, '$.interval') = ?".to_string());
            values.push(CompensationSqlValue::Text(
                serde_json::to_value(interval)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .into(),
            ));
        }
        if let Some(currency) = &self.currency {
            clauses.push("upper(json_extract(pay.value, '$.currency')) = ?".to_string());
            values.push(CompensationSqlValue::Text(currency.to_ascii_uppercase()));
        }
        let known = "(json_extract(pay.value, '$.min_minor') IS NOT NULL OR json_extract(pay.value, '$.max_minor') IS NOT NULL)";
        let mut overlap = Vec::new();
        if let Some(min) = self.min_minor {
            overlap.push("(json_extract(pay.value, '$.max_minor') IS NULL OR json_extract(pay.value, '$.max_minor') >= ?)");
            values.push(CompensationSqlValue::Integer(min));
        }
        if let Some(max) = self.max_minor {
            overlap.push("(json_extract(pay.value, '$.min_minor') IS NULL OR json_extract(pay.value, '$.min_minor') <= ?)");
            values.push(CompensationSqlValue::Integer(max));
        }
        let overlap = if overlap.is_empty() {
            "1".to_string()
        } else {
            overlap.join(" AND ")
        };
        clauses.push(if self.include_unknown {
            format!("(NOT {known} OR ({overlap}))")
        } else {
            format!("({known} AND ({overlap}))")
        });
        let sql = format!(
            "EXISTS (SELECT 1 FROM json_each(compensation_json) pay WHERE {})",
            clauses.join(" AND ")
        );
        if self.include_unknown && self.interval.is_none() && self.currency.is_none() {
            (
                format!("(json_array_length(compensation_json) = 0 OR {sql})"),
                values,
            )
        } else {
            (sql, values)
        }
    }
}
