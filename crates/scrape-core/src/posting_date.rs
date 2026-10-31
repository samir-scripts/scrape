use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub fn first_local_instant(timezone: Tz, local: NaiveDateTime) -> Option<DateTime<Utc>> {
    for second in 0..=172800 {
        match timezone.from_local_datetime(&(local + Duration::seconds(second))) {
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
    // A named window resolved against each run's start, exclusive with explicit ranges.
    #[serde(default)]
    pub preset: Option<String>,
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
    /// Creates a persisted window in an IANA timezone, validating it at `run_started`.
    ///
    /// Supported presets are `past_12h`, `today`, `past_24h`, `past_48h`, `past_week` (168 hours),
    /// `past_month` (720 hours), and `any` (unrestricted). Each run resolves relative
    /// windows anew; `today` begins at the first valid local instant of that date.
    /// Use `from` and `through` without a preset for a custom absolute range.
    pub fn from_preset(
        preset: &str,
        timezone: &str,
        run_started: DateTime<Utc>,
    ) -> Result<Self, String> {
        let filter = Self {
            preset: Some(preset.into()),
            last_hours: None,
            from: None,
            through: None,
            timezone: timezone.into(),
            include_unknown: false,
        };
        filter.bounds(run_started)?;
        Ok(filter)
    }

    pub fn bounds(&self, run_started: DateTime<Utc>) -> Result<DateBounds, String> {
        let timezone: Tz = self
            .timezone
            .parse()
            .map_err(|_| "Posting-date timezone must be an IANA timezone")?;
        if let Some(preset) = self.preset.as_deref() {
            if self.last_hours.is_some() || self.from.is_some() || self.through.is_some() {
                return Err(
                    "Posting-date presets and explicit date ranges are mutually exclusive".into(),
                );
            }
            let hours = match preset {
                "past_12h" => 12,
                "past_24h" => 24,
                "past_48h" => 48,
                "past_week" => 168,
                "past_month" => 720,
                "today" => {
                    let midnight = run_started
                        .with_timezone(&timezone)
                        .date_naive()
                        .and_hms_opt(0, 0, 0)
                        .ok_or("Invalid posting-date start")?;
                    let start = first_local_instant(timezone, midnight)
                        .ok_or("Invalid posting-date start")?;
                    return Ok((Some(start), Some(run_started + Duration::milliseconds(1))));
                }
                "any" => return Ok((None, None)),
                _ => return Err("Invalid posting-date preset".into()),
            };
            return Ok((
                run_started.checked_sub_signed(Duration::hours(hours)),
                Some(run_started + Duration::milliseconds(1)),
            ));
        }
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
                        timezone,
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
        let Ok((from, through)) = self.bounds(started) else {
            return false;
        };
        let Some(value) = posted else {
            return self.include_unknown;
        };
        if self.preset.as_deref() == Some("past_48h") {
            if let Ok(day) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
                let Some(timezone) = self.timezone.parse::<Tz>().ok() else {
                    return false;
                };
                let Some(start) = day
                    .and_hms_opt(0, 0, 0)
                    .and_then(|local| first_local_instant(timezone, local))
                else {
                    return false;
                };
                let Some(end) = day
                    .succ_opt()
                    .and_then(|next| next.and_hms_opt(0, 0, 0))
                    .and_then(|local| first_local_instant(timezone, local))
                else {
                    return false;
                };
                return from.is_none_or(|bound| start >= bound)
                    && through.is_none_or(|bound| end <= bound);
            }
        }
        let Some(posted) = posting_instant(value, &self.timezone) else {
            return self.include_unknown;
        };
        from.is_none_or(|start| posted >= start) && through.is_none_or(|end| posted < end)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn instant(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn rolling_presets_use_canonical_windows_and_include_run_start() {
        let started = instant("2026-10-14T12:00:00Z");
        for (preset, hours, inside_hours, outside_hours) in [
            ("past_12h", 12, 6, 13),
            ("past_24h", 24, 23, 25),
            ("past_48h", 48, 47, 49),
            ("past_week", 168, 5 * 24, 8 * 24),
            ("past_month", 720, 20 * 24, 35 * 24),
        ] {
            let filter = PostingDateFilter::from_preset(preset, "UTC", started).unwrap();
            let lower = started - Duration::hours(hours);
            assert_eq!(filter.preset.as_deref(), Some(preset));
            assert_eq!(filter.last_hours, None);
            assert_eq!(
                filter.bounds(started).unwrap(),
                (Some(lower), Some(started + Duration::milliseconds(1)))
            );
            assert!(filter.matches(
                Some(&(started - Duration::hours(inside_hours)).to_rfc3339()),
                started
            ));
            assert!(!filter.matches(
                Some(&(started - Duration::hours(outside_hours)).to_rfc3339()),
                started
            ));
            assert!(filter.matches(Some(&lower.to_rfc3339()), started));
            assert!(!filter.matches(
                Some(&(lower - Duration::milliseconds(1)).to_rfc3339()),
                started
            ));
            assert!(filter.matches(Some(&started.to_rfc3339()), started));
            assert!(!filter.matches(
                Some(&(started + Duration::milliseconds(1)).to_rfc3339()),
                started
            ));
        }
    }
    #[test]
    fn forty_eight_hour_window_requires_whole_local_day_to_be_recent() {
        let started = instant("2026-03-09T16:00:00Z");
        let filter =
            PostingDateFilter::from_preset("past_48h", "America/New_York", started).unwrap();
        // The 48-hour cutoff falls within March 7 and the run occurs during March 9.
        assert!(!filter.matches(Some("2026-03-07"), started));
        assert!(filter.matches(Some("2026-03-08"), started));
        assert!(!filter.matches(Some("2026-03-09"), started));
        assert!(filter.matches(Some("2026-03-07T17:00:00Z"), started));
        assert!(!filter.matches(Some("2026-03-07T15:59:59Z"), started));
        // Existing windows retain their historical date-only interpretation.
        let previous =
            PostingDateFilter::from_preset("past_week", "America/New_York", started).unwrap();
        assert!(previous.matches(Some("2026-03-09"), started));
    }

    #[test]
    fn today_is_persisted_and_rolls_to_each_runs_local_midnight() {
        let first_run = instant("2026-10-14T12:00:00Z");
        let next_run = instant("2026-10-15T12:00:00Z");
        let filter =
            PostingDateFilter::from_preset("today", "America/New_York", first_run).unwrap();
        let encoded = serde_json::to_string(&filter).unwrap();
        let filter: PostingDateFilter = serde_json::from_str(&encoded).unwrap();
        assert_eq!(filter.from, None);
        assert_eq!(filter.through, None);
        for (started, midnight, yesterday, one_am) in [
            (
                first_run,
                "2026-10-14T04:00:00Z",
                "2026-10-14T03:59:00Z",
                "2026-10-14T05:00:00Z",
            ),
            (
                next_run,
                "2026-10-15T04:00:00Z",
                "2026-10-15T03:59:00Z",
                "2026-10-15T05:00:00Z",
            ),
        ] {
            assert_eq!(
                filter.bounds(started).unwrap(),
                (
                    Some(instant(midnight)),
                    Some(started + Duration::milliseconds(1))
                )
            );
            assert!(filter.matches(Some(midnight), started));
            assert!(filter.matches(Some(one_am), started));
            assert!(!filter.matches(Some(yesterday), started));
            assert!(filter.matches(Some(&started.to_rfc3339()), started));
            assert!(!filter.matches(
                Some(&(started + Duration::milliseconds(1)).to_rfc3339()),
                started
            ));
        }
        assert!(!filter.matches(Some("2026-10-14"), next_run));
    }

    #[test]
    fn today_observes_dst_transitions_and_nonexistent_midnight() {
        for (timezone, started, midnight) in [
            (
                "America/New_York",
                "2026-03-08T16:00:00Z",
                "2026-03-08T05:00:00Z",
            ),
            (
                "America/New_York",
                "2026-11-01T17:00:00Z",
                "2026-11-01T04:00:00Z",
            ),
            (
                "America/Sao_Paulo",
                "2018-11-04T12:00:00Z",
                "2018-11-04T03:00:00Z",
            ),
            (
                "America/Havana",
                "2020-11-01T12:00:00Z",
                "2020-11-01T04:00:00Z",
            ),
        ] {
            let started = instant(started);
            let filter = PostingDateFilter::from_preset("today", timezone, started).unwrap();
            assert_eq!(filter.bounds(started).unwrap().0, Some(instant(midnight)));
            assert!(filter.matches(Some(midnight), started));
            assert!(!filter.matches(
                Some(&(instant(midnight) - Duration::seconds(1)).to_rfc3339()),
                started
            ));
        }
    }

    #[test]
    fn presets_validate_names_timezones_and_exclusive_configuration() {
        let started = instant("2026-10-14T12:00:00Z");
        for preset in ["", "custom", "past_7d", "Today", "today "] {
            assert!(PostingDateFilter::from_preset(preset, "UTC", started).is_err());
        }
        for preset in ["today", "past_week", "any"] {
            assert!(PostingDateFilter::from_preset(preset, "not-a-timezone", started).is_err());
            let filter = PostingDateFilter::from_preset(preset, "UTC", started).unwrap();
            let conflicting = [
                PostingDateFilter {
                    last_hours: Some(12),
                    ..filter.clone()
                },
                PostingDateFilter {
                    from: Some("2026-10-14".into()),
                    ..filter.clone()
                },
                PostingDateFilter {
                    through: Some("2026-10-14".into()),
                    ..filter
                },
            ];
            for mut filter in conflicting {
                filter.include_unknown = true;
                assert!(filter.bounds(started).is_err());
                assert!(!filter.matches(None, started));
                assert!(!filter.matches(Some("2026-10-14"), started));
            }
        }
    }

    #[test]
    fn any_is_unrestricted_and_unknown_dates_follow_explicit_policy() {
        let started = instant("2026-10-14T12:00:00Z");
        let any = PostingDateFilter::from_preset("any", "UTC", started).unwrap();
        assert_eq!(any.bounds(started).unwrap(), (None, None));
        assert!(any.matches(Some("2000-01-01"), started));
        assert!(any.matches(Some("2030-01-01"), started));
        for preset in [
            "any",
            "today",
            "past_12h",
            "past_24h",
            "past_48h",
            "past_week",
            "past_month",
        ] {
            let mut filter = PostingDateFilter::from_preset(preset, "UTC", started).unwrap();
            for posted in [None, Some("not-a-date")] {
                assert!(!filter.matches(posted, started));
            }
            filter.include_unknown = true;
            for posted in [None, Some("not-a-date")] {
                assert!(filter.matches(posted, started));
            }
        }
    }

    #[test]
    fn existing_serialized_filters_default_to_no_preset() {
        let filter: PostingDateFilter =
            serde_json::from_str(r#"{"last_hours":24,"timezone":"UTC","include_unknown":false}"#)
                .unwrap();
        assert_eq!(filter.preset, None);
        let started = instant("2026-10-14T12:00:00Z");
        assert_eq!(
            filter.bounds(started).unwrap(),
            (
                Some(instant("2026-10-13T12:00:00Z")),
                Some(started + Duration::milliseconds(1))
            )
        );
    }

    #[test]
    fn absolute_dates_follow_workflow_timezone_and_relative_age_uses_captured_start() {
        let started = DateTime::parse_from_rfc3339("2026-10-14T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let filter = PostingDateFilter {
            preset: None,
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
