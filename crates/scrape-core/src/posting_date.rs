use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub fn first_local_instant(timezone: Tz, local: NaiveDateTime) -> Option<DateTime<Utc>> {
    for minute in 0..=2880 {
        match timezone.from_local_datetime(&(local + Duration::minutes(minute))) {
            LocalResult::Single(dt) => return Some(dt.with_timezone(&Utc)),
            LocalResult::Ambiguous(a, b) => return Some(a.min(b).with_timezone(&Utc)),
            LocalResult::None => {}
        }
    }
    None
}
pub fn posting_instant(value: &str, timezone: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }
    let day = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    first_local_instant(timezone.parse().ok()?, day.and_hms_opt(0, 0, 0)?)
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct PostingDateFilter {
    #[serde(default)]
    pub last_hours: Option<u32>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub through: Option<String>,
    pub timezone: String,
    #[serde(default)]
    pub include_unknown: bool,
}
pub type DateBounds = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);
impl PostingDateFilter {
    pub fn bounds(&self, run_started: DateTime<Utc>) -> Result<DateBounds, String> {
        let _: Tz = self
            .timezone
            .parse()
            .map_err(|_| "Posting-date timezone must be an IANA timezone")?;
        if let Some(hours) = self.last_hours {
            if hours == 0 || self.from.is_some() || self.through.is_some() {
                return Err(
                    "Posting-date age and absolute date ranges are mutually exclusive".into(),
                );
            }
            return Ok((
                run_started.checked_sub_signed(Duration::hours(i64::from(hours))),
                Some(run_started + Duration::milliseconds(1)),
            ));
        }
        let from = self
            .from
            .as_deref()
            .map(|s| posting_instant(s, &self.timezone).ok_or("Invalid posting-date start"))
            .transpose()?;
        let through = self
            .through
            .as_deref()
            .map(|s| {
                if let Ok(day) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    first_local_instant(
                        self.timezone.parse().unwrap(),
                        day.succ_opt()
                            .ok_or("Invalid posting-date end")?
                            .and_hms_opt(0, 0, 0)
                            .unwrap(),
                    )
                    .ok_or("Invalid posting-date end")
                } else {
                    posting_instant(s, &self.timezone)
                        .map(|dt| dt + Duration::milliseconds(1))
                        .ok_or("Invalid posting-date end")
                }
            })
            .transpose()?;
        if matches!((from, through), (Some(start), Some(end)) if start >= end) {
            return Err("Posting-date start must precede end".into());
        }
        Ok((from, through))
    }
    pub fn matches(&self, posted: Option<&str>, started: DateTime<Utc>) -> bool {
        let Some(posted) = posted.and_then(|value| posting_instant(value, &self.timezone)) else {
            return self.include_unknown;
        };
        let Ok((from, through)) = self.bounds(started) else {
            return false;
        };
        from.is_none_or(|start| posted >= start) && through.is_none_or(|end| posted < end)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absolute_dates_follow_workflow_timezone_and_relative_age_uses_captured_start() {
        let started = DateTime::parse_from_rfc3339("2026-10-14T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let filter = PostingDateFilter {
            last_hours: None,
            from: Some("2026-10-13".into()),
            through: Some("2026-10-13".into()),
            timezone: "America/New_York".into(),
            include_unknown: false,
        };
        assert!(filter.matches(Some("2026-10-14T03:59:59Z"), started));
        assert!(!filter.matches(Some("2026-10-14T04:00:00Z"), started));
        assert!(filter.matches(Some("2026-10-13"), started));
        assert!(!filter.matches(None, started));
        let age = PostingDateFilter {
            last_hours: Some(24),
            from: None,
            through: None,
            ..filter
        };
        assert!(age.matches(Some("2026-10-13T12:00:00Z"), started));
        assert!(!age.matches(Some("2026-10-13T11:59:59Z"), started));
    }
}
