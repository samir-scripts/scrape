use crate::error::{AppError, Result};
use chrono::{DateTime, Datelike, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct CalendarSchedule {
    pub time: String,
    /// ISO weekdays: Monday=1 through Sunday=7; empty means every day.
    pub weekdays: Vec<u8>,
    pub timezone: String,
}
impl CalendarSchedule {
    pub fn validate(&self) -> Result<()> {
        self.timezone.parse::<Tz>().map_err(|_| {
            AppError::Validation("schedule.timezone must be an IANA timezone".into())
        })?;
        NaiveTime::parse_from_str(&self.time, "%H:%M")
            .map_err(|_| AppError::Validation("schedule.time must be HH:MM".into()))?;
        if self.weekdays.iter().any(|day| !(1..=7).contains(day)) {
            return Err(AppError::Validation(
                "schedule.weekdays must contain ISO weekdays 1–7".into(),
            ));
        }
        Ok(())
    }
    pub fn next_after(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>> {
        self.validate()?;
        let tz = self.timezone.parse::<Tz>().unwrap();
        let time = NaiveTime::parse_from_str(&self.time, "%H:%M").unwrap();
        let start = after.with_timezone(&tz).date_naive();
        for offset in 0..16 {
            let date = start + Duration::days(offset);
            if !self.weekdays.is_empty()
                && !self
                    .weekdays
                    .contains(&(date.weekday().number_from_monday() as u8))
            {
                continue;
            }
            let local = date.and_time(time);
            if let Some(instant) = scrape_core::posting_date::first_local_instant(tz, local) {
                if instant > after {
                    return Ok(instant);
                }
            }
        }
        Err(AppError::Validation(
            "Schedule has no future occurrence".into(),
        ))
    }
}

pub fn next_for_source(
    cadence: scrape_core::SourceCadence,
    cron: Option<&str>,
    config: &serde_json::Value,
    enabled: bool,
    timezone: &str,
    after: DateTime<Utc>,
) -> Result<Option<String>> {
    use scrape_core::SourceCadence;
    let calendar = config
        .get("__schedule")
        .map(|value| {
            serde_json::from_value::<CalendarSchedule>(value.clone())
                .map_err(|error| AppError::Validation(format!("schedule: {error}")))
        })
        .transpose()?;
    if let Some(calendar) = &calendar {
        calendar.validate()?;
    }
    if !enabled || matches!(cadence, SourceCadence::Global | SourceCadence::Manual) {
        return Ok(None);
    }
    if matches!(cadence, SourceCadence::Daily | SourceCadence::Weekly) {
        let calendar = calendar.unwrap_or(CalendarSchedule {
            time: "09:00".into(),
            weekdays: if cadence == SourceCadence::Weekly {
                vec![1]
            } else {
                vec![]
            },
            timezone: timezone.into(),
        });
        return calendar
            .next_after(after)
            .map(|dt| Some(dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)));
    }
    if calendar.is_some() {
        return Err(AppError::Validation(
            "Calendar settings require daily or weekly cadence".into(),
        ));
    }
    let expression = if cadence == SourceCadence::Cron {
        cron
    } else {
        cadence.default_cron()
    }
    .ok_or_else(|| AppError::Validation("Custom schedules require a cron expression".into()))?;
    super::scheduler::calculate_next_run_at(
        expression,
        config
            .get("__cron_timezone")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(timezone),
        after,
    )
    .map(Some)
    .ok_or_else(|| {
        AppError::Validation("Schedule or timezone has no valid future occurrence".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }
    #[test]
    fn daylight_saving_gap_runs_at_first_valid_instant_and_overlap_runs_once() {
        let gap = CalendarSchedule {
            time: "02:30".into(),
            weekdays: vec![],
            timezone: "America/New_York".into(),
        };
        assert_eq!(
            gap.next_after(utc("2026-03-08T05:00:00Z")).unwrap(),
            utc("2026-03-08T07:00:00Z")
        );
        let repeated = CalendarSchedule {
            time: "01:30".into(),
            weekdays: vec![],
            timezone: "America/New_York".into(),
        };
        assert_eq!(
            repeated.next_after(utc("2026-11-01T04:00:00Z")).unwrap(),
            utc("2026-11-01T05:30:00Z")
        );
        assert_eq!(
            repeated.next_after(utc("2026-11-01T05:31:00Z")).unwrap(),
            utc("2026-11-02T06:30:00Z")
        );
    }
    #[test]
    fn weekdays_timezone_and_catchup_next_future_occurrence() {
        let schedule = CalendarSchedule {
            time: "09:00".into(),
            weekdays: vec![1, 3, 5],
            timezone: "Asia/Kolkata".into(),
        };
        assert_eq!(
            schedule.next_after(utc("2026-10-13T12:00:00Z")).unwrap(),
            utc("2026-10-14T03:30:00Z")
        );
        let invalid = CalendarSchedule {
            timezone: "Unrecognized/Zone".into(),
            ..schedule
        };
        assert!(invalid.validate().is_err());
    }
}
