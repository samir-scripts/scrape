use crate::db::runs::TriggerType;
use crate::db::settings::SettingsRepo;
use crate::db::sources::{Source, SourcesRepo};
use crate::error::{AppError, Result};
use crate::services::runner::ScrapeRunner;
use chrono::{DateTime, SecondsFormat, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashSet, VecDeque};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::Mutex;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CronValidationResult {
    pub ok: bool,
    pub next_runs: Vec<String>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SchedulerStatus {
    pub enabled: bool,
    pub next_run_at: Option<String>,
    pub running: bool,
    pub enabled_workflows: u32,
    pub schedules: Vec<ScheduledWorkflow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScheduledWorkflow {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub next_run_at: Option<String>,
}

// Read the same instants used by dispatch. No calendar calculation belongs in status.
fn status_for_sources(
    sources: &[Source],
    global: Option<DateTime<Utc>>,
    timezone: &str,
    enabled: bool,
    running: bool,
) -> SchedulerStatus {
    use scrape_core::SourceCadence;
    let mut schedules: Vec<_> = sources
        .iter()
        .filter(|s| s.enabled && !s.needs_review && s.cadence != SourceCadence::Manual)
        .map(|s| {
            let next = if s.cadence == SourceCadence::Global {
                global
            } else {
                s.next_run_at
                    .as_deref()
                    .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
                    .map(|v| v.with_timezone(&Utc))
            };
            ScheduledWorkflow {
                id: s.id.clone(),
                name: s.name.clone(),
                timezone: if s.cadence == SourceCadence::Global {
                    timezone.to_owned()
                } else {
                    s.config
                        .pointer("/__schedule/timezone")
                        .or_else(|| s.config.get("__cron_timezone"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(timezone)
                        .to_owned()
                },
                next_run_at: next.map(|v| v.to_rfc3339_opts(SecondsFormat::Millis, true)),
            }
        })
        .collect();
    schedules.sort_by(|a, b| a.next_run_at.cmp(&b.next_run_at).then(a.id.cmp(&b.id)));
    SchedulerStatus {
        enabled,
        running,
        enabled_workflows: sources.iter().filter(|s| s.enabled).count() as u32,
        next_run_at: if enabled {
            schedules.iter().filter_map(|s| s.next_run_at.clone()).min()
        } else {
            None
        },
        schedules,
    }
}

#[derive(Default)]
struct DispatchQueue {
    entries: VecDeque<(String, TriggerType)>,
    pending: HashSet<String>,
}
impl DispatchQueue {
    fn add(&mut self, id: String, trigger: TriggerType) {
        if self.pending.insert(id.clone()) {
            self.entries.push_back((id, trigger));
        }
    }
    #[cfg(test)]
    fn enqueue_due(&mut self, sources: &[Source], now: DateTime<Utc>, paused: bool) {
        if paused {
            return;
        }
        for source in sources {
            if is_due(source, now) {
                self.add(source.id.clone(), TriggerType::Schedule);
            }
        }
    }
    fn pop(&mut self) -> Option<(String, TriggerType)> {
        let entry = self.entries.pop_front()?;
        self.pending.remove(&entry.0);
        Some(entry)
    }
}
fn is_due(source: &Source, now: DateTime<Utc>) -> bool {
    source.enabled
        && !source.needs_review
        && !matches!(
            source.cadence,
            scrape_core::SourceCadence::Manual | scrape_core::SourceCadence::Global
        )
        && source
            .next_run_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|next| next.with_timezone(&Utc) <= now)
}

pub struct SchedulerService {
    is_paused: Arc<AtomicBool>,
    next_global: Arc<Mutex<Option<DateTime<Utc>>>>,
    pool: SqlitePool,
    runner: ScrapeRunner,
}
impl SchedulerService {
    pub async fn init(
        pool: SqlitePool,
        runner: ScrapeRunner,
        app: AppHandle,
    ) -> Result<Arc<Mutex<Self>>> {
        let enabled: bool = SettingsRepo::get_value(&pool, "scraper.enabled")
            .await?
            .unwrap_or(true);
        let cron: String = SettingsRepo::get_value(&pool, "scraper.cron")
            .await?
            .unwrap_or_else(|| "0 0 12 * * *".into());
        let timezone: String = SettingsRepo::get_value(&pool, "system.timezone")
            .await?
            .unwrap_or_else(|| "UTC".into());
        let saved_next: Option<String> =
            SettingsRepo::get_value(&pool, "scraper.next_run_at").await?;
        let next = saved_next
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|| {
                calculate_next_run(&cron, &timezone)
                    .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
                    .map(|dt| dt.with_timezone(&Utc))
            });
        let paused = Arc::new(AtomicBool::new(!enabled));
        let next_global = Arc::new(Mutex::new(next));
        let service = Arc::new(Mutex::new(Self {
            is_paused: paused.clone(),
            next_global: next_global.clone(),
            pool: pool.clone(),
            runner: runner.clone(),
        }));
        let legacy_startup: bool = SettingsRepo::get_value(&pool, "scraper.run_on_startup")
            .await?
            .unwrap_or(false);
        let startup = legacy_startup
            || SourcesRepo::list(&pool).await?.iter().any(|source| {
                source
                    .config
                    .get("__run_on_startup")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            });
        tokio::spawn(async move {
            let mut startup_at =
                (startup && enabled).then(|| Utc::now() + chrono::Duration::seconds(10));
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut queue = DispatchQueue::default();
            loop {
                interval.tick().await;
                let Ok(_permit) = runner.write_permit() else {
                    continue;
                };
                let now = Utc::now();
                if paused.load(Ordering::SeqCst) {
                    queue = DispatchQueue::default();
                    if startup_at.is_some_and(|at| at <= now) {
                        startup_at = None;
                    }
                    continue;
                }
                let sources = match SourcesRepo::list(&pool).await {
                    Ok(sources) => sources,
                    Err(error) => {
                        tracing::error!("Could not load scheduled workflows: {error}");
                        continue;
                    }
                };
                let mut global = next_global.lock().await;
                let global_due = global.is_some_and(|next| next <= now);
                let startup_due = startup_at.is_some_and(|at| at <= now);
                if global_due {
                    for source in sources.iter().filter(|s| {
                        s.enabled
                            && !s.needs_review
                            && s.cadence == scrape_core::SourceCadence::Global
                    }) {
                        queue.add(source.id.clone(), TriggerType::Schedule);
                    }
                    let cron: String = SettingsRepo::get_value(&pool, "scraper.cron")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "0 0 12 * * *".into());
                    let timezone: String = SettingsRepo::get_value(&pool, "system.timezone")
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "UTC".into());
                    *global = calculate_next_run_at(&cron, &timezone, now)
                        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
                        .map(|dt| dt.with_timezone(&Utc));
                    let _ = SettingsRepo::set_value(
                        &pool,
                        "scraper.next_run_at",
                        serde_json::json!(
                            global.map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
                        ),
                    )
                    .await;
                }
                if startup_due {
                    for source in sources.iter().filter(|s| {
                        s.enabled
                            && !s.needs_review
                            && s.config
                                .get("__run_on_startup")
                                .and_then(serde_json::Value::as_bool)
                                == Some(true)
                    }) {
                        queue.add(source.id.clone(), TriggerType::Startup);
                    }
                    startup_at = None;
                }
                for source in &sources {
                    if !is_due(source, now) {
                        continue;
                    }
                    let awaits_startup = startup_at.is_some()
                        && source
                            .config
                            .get("__run_on_startup")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true);
                    if !awaits_startup {
                        queue.add(source.id.clone(), TriggerType::Schedule);
                    }
                }
                drop(global);
                // One worker at a time. Failed overlap acquisition retains this
                // distinct pending trigger for the next dispatcher tick.
                while !paused.load(Ordering::SeqCst) && !runner.is_running() {
                    let Some((id, trigger)) = queue.pop() else {
                        break;
                    };
                    let source = match SourcesRepo::get(&pool, &id).await {
                        Ok(source) => source,
                        Err(_) => continue,
                    };
                    if !source.enabled
                        || source.needs_review
                        || source.cadence == scrape_core::SourceCadence::Manual
                    {
                        continue;
                    }
                    if source.cadence != scrape_core::SourceCadence::Global
                        && !is_due(&source, Utc::now())
                    {
                        continue;
                    }
                    match runner.trigger_single_source_as(&app, &id, trigger).await {
                        Err(AppError::AlreadyRunning) => {
                            queue.add(id, trigger);
                            break;
                        }
                        Err(error) => tracing::error!("Scheduled workflow failed: {error}"),
                        Ok(_) => {}
                    }
                }
            }
        });
        Ok(service)
    }
    pub async fn schedule_job(&mut self, cron: &str) -> Result<()> {
        let timezone: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await?
            .unwrap_or_else(|| "UTC".into());
        let next = calculate_next_run(cron, &timezone)
            .ok_or_else(|| AppError::Validation("Invalid schedule or timezone".into()))?;
        *self.next_global.lock().await = Some(
            DateTime::parse_from_rfc3339(&next)
                .unwrap()
                .with_timezone(&Utc),
        );
        SettingsRepo::set_value(&self.pool, "scraper.next_run_at", serde_json::json!(next)).await?;
        Ok(())
    }
    pub async fn reschedule(&mut self, cron: &str) -> Result<()> {
        self.schedule_job(cron).await
    }
    pub async fn set_paused(&mut self, paused: bool) -> Result<()> {
        SettingsRepo::set_value(&self.pool, "scraper.enabled", serde_json::json!(!paused)).await?;
        self.is_paused.store(paused, Ordering::SeqCst);
        Ok(())
    }
    pub async fn sync_after_cleanup(&mut self, paused: bool, reset: bool) -> Result<()> {
        self.is_paused.store(paused, Ordering::SeqCst);
        if reset {
            let settings = SettingsRepo::get_all(&self.pool).await?;
            self.reschedule(&settings.scraper_cron).await?;
        }
        Ok(())
    }
    pub async fn get_status(&self) -> Result<SchedulerStatus> {
        let sources = SourcesRepo::list(&self.pool).await?;
        let timezone: String = SettingsRepo::get_value(&self.pool, "system.timezone")
            .await?
            .unwrap_or_else(|| "UTC".into());
        Ok(status_for_sources(
            &sources,
            *self.next_global.lock().await,
            &timezone,
            !self.is_paused.load(Ordering::SeqCst),
            self.runner.is_running(),
        ))
    }
}

pub fn validate_cron_expression(expr: &str, tz_str: &str) -> CronValidationResult {
    let Ok(_) = Tz::from_str(tz_str) else {
        return CronValidationResult {
            ok: false,
            next_runs: vec![],
            error: Some("Invalid IANA timezone".into()),
        };
    };
    match Schedule::from_str(expr) {
        Ok(_) => {
            let mut after = chrono::Utc::now();
            let mut next_runs = Vec::new();
            for _ in 0..3 {
                let Some(next) = calculate_next_run_at(expr, tz_str, after) else {
                    break;
                };
                after = chrono::DateTime::parse_from_rfc3339(&next)
                    .expect("Calculated UTC instant")
                    .with_timezone(&chrono::Utc);
                next_runs.push(next);
            }
            CronValidationResult {
                ok: true,
                next_runs,
                error: None,
            }
        }
        Err(e) => CronValidationResult {
            ok: false,
            next_runs: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}

pub fn calculate_next_run(cron_expr: &str, tz_str: &str) -> Option<String> {
    calculate_next_run_at(cron_expr, tz_str, chrono::Utc::now())
}

pub fn calculate_next_run_at(
    cron_expr: &str,
    tz_str: &str,
    after: chrono::DateTime<chrono::Utc>,
) -> Option<String> {
    let tz = Tz::from_str(tz_str).ok()?;
    let schedule = Schedule::from_str(cron_expr).ok()?;
    // Evaluate calendar candidates without timezone gaps/duplicates, then apply
    // the shared local-time policy. A repeated instant runs only at its first occurrence.
    let calendar_start = after.with_timezone(&tz).naive_local().and_utc();
    schedule
        .after(&calendar_start)
        // At most two days of second-level candidates can precede the UTC
        // cursor during an exceptional local-clock rollback. Normal schedules
        // return on the first candidate.
        .take(172_801)
        .find_map(|candidate| {
            let instant =
                scrape_core::posting_date::first_local_instant(tz, candidate.naive_utc())?;
            (instant > after).then(|| instant.to_rfc3339_opts(SecondsFormat::Millis, true))
        })
}

pub fn calculate_next_run_for_source(
    source: &crate::db::sources::Source,
    tz_str: &str,
) -> Option<String> {
    super::calendar::next_for_source(
        source.cadence,
        source.cadence_cron.as_deref(),
        &source.config,
        source.enabled,
        tz_str,
        chrono::Utc::now(),
    )
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn reset_cleanup_replaces_the_in_memory_global_calendar() {
        use super::*;
        let pool = crate::db::init_memory_db().await.unwrap();
        SettingsRepo::set_value(&pool, "scraper.enabled", serde_json::json!(false))
            .await
            .unwrap();
        let previous = Utc::now() - chrono::Duration::days(1);
        let mut scheduler = SchedulerService {
            is_paused: Arc::new(AtomicBool::new(false)),
            next_global: Arc::new(Mutex::new(Some(previous))),
            runner: ScrapeRunner::new(pool.clone()),
            pool: pool.clone(),
        };
        scheduler.sync_after_cleanup(true, true).await.unwrap();
        assert!(scheduler.is_paused.load(Ordering::SeqCst));
        let next = scheduler.next_global.lock().await.unwrap();
        assert!(next > Utc::now());
        let persisted: String = SettingsRepo::get_value(&pool, "scraper.next_run_at")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            DateTime::parse_from_rfc3339(&persisted)
                .unwrap()
                .with_timezone(&Utc),
            next
        );
    }

    use super::*;

    #[test]
    fn cron_uses_the_same_gap_and_overlap_policy_as_calendar_workflows() {
        let parse = |value: &str| {
            chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .with_timezone(&chrono::Utc)
        };
        assert_eq!(
            calculate_next_run_at(
                "0 30 2 * * *",
                "America/New_York",
                parse("2026-03-08T06:00:00Z")
            ),
            Some("2026-03-08T07:00:00.000Z".into())
        );
        assert_eq!(
            calculate_next_run_at(
                "0 30 1 * * *",
                "America/New_York",
                parse("2026-11-01T04:00:00Z")
            ),
            Some("2026-11-01T05:30:00.000Z".into())
        );
        assert_eq!(
            calculate_next_run_at(
                "0 30 1 * * *",
                "America/New_York",
                parse("2026-11-01T06:15:00Z")
            ),
            Some("2026-11-02T06:30:00.000Z".into())
        );
    }

    #[test]
    fn cron_weekday_names_and_frequent_overlap_candidates_are_correct() {
        let parse = |value: &str| {
            chrono::DateTime::parse_from_rfc3339(value)
                .unwrap()
                .with_timezone(&chrono::Utc)
        };
        assert_eq!(
            calculate_next_run_at("0 0 9 * * MON-FRI", "UTC", parse("2026-10-18T08:00:00Z")),
            Some("2026-10-19T09:00:00.000Z".into())
        );
        assert_eq!(
            calculate_next_run_at("0 0 9 * * SAT,SUN", "UTC", parse("2026-10-19T08:00:00Z")),
            Some("2026-10-24T09:00:00.000Z".into())
        );
        assert_eq!(
            calculate_next_run_at(
                "* * * * * *",
                "America/New_York",
                parse("2026-11-01T06:15:00Z")
            ),
            Some("2026-11-01T07:00:00.000Z".into())
        );
    }

    #[test]
    fn test_validate_cron_valid() {
        let res = validate_cron_expression("0 0 12 * * *", "America/New_York");
        assert!(res.ok);
        assert_eq!(res.next_runs.len(), 3);
        assert!(res.error.is_none());

        // Verify clock schedule expressions
        for expr in &[
            "0 30 14 * * *",       // Daily at 14:30
            "0 30 14 * * 1-5",     // Weekdays numeric
            "0 30 14 * * MON-FRI", // Weekdays named
            "0 30 14 * * 6,7",     // Weekends numeric
            "0 30 14 * * SAT,SUN", // Weekends named
            "0 30 */12 * * *",     // Every 12 hours
            "0 30 */6 * * *",      // Every 6 hours
            "0 30 * * * *",        // Hourly
        ] {
            let res = validate_cron_expression(expr, "UTC");
            assert!(
                res.ok,
                "Expected expr '{}' to be valid, got error: {:?}",
                expr, res.error
            );
        }
    }

    #[test]
    fn test_validate_cron_invalid() {
        let res = validate_cron_expression("invalid cron", "UTC");
        assert!(!res.ok);
        assert!(res.next_runs.is_empty());
        assert!(res.error.is_some());
    }

    #[test]
    fn test_calculate_next_run() {
        let next_utc = calculate_next_run("0 0 12 * * *", "UTC");
        assert!(next_utc.is_some());

        let next_ny = calculate_next_run("0 0 12 * * *", "America/New_York");
        assert!(next_ny.is_some());

        let invalid = calculate_next_run("invalid-cron", "UTC");
        assert!(invalid.is_none());
    }

    #[test]
    fn test_calculate_next_run_for_source() {
        use scrape_core::SourceCadence;

        let mut source = crate::db::sources::Source {
            id: "src-1".to_string(),
            kind: scrape_core::SourceKind::Greenhouse,
            name: "Acme".to_string(),
            identifier: "acme".to_string(),
            config: serde_json::json!({}),
            enabled: true,
            last_run_at: None,
            last_status: None,
            last_error: None,
            created_at: "2026-10-10T00:00:00Z".to_string(),
            cadence: SourceCadence::Global,
            cadence_cron: None,
            next_run_at: None,
            last_run_duration_ms: None,
            retention_days: 30,
            retention_policy: "untracked_only".to_string(),
            filters: serde_json::json!({}),
            needs_review: false,
        };

        // Global cadence has no independent next run
        assert!(calculate_next_run_for_source(&source, "UTC").is_none());

        // Every 12h preset calculates next run
        source.cadence = SourceCadence::Every12h;
        assert!(calculate_next_run_for_source(&source, "UTC").is_some());

        // Custom Cron
        source.cadence = SourceCadence::Cron;
        source.cadence_cron = Some("0 30 9 * * *".to_string());
        assert!(calculate_next_run_for_source(&source, "UTC").is_some());
    }

    #[tokio::test]
    async fn status_uses_dispatch_instants_and_never_invents_automatic_work() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let clock = DateTime::parse_from_rfc3339("2026-10-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(status_for_sources(&[], Some(clock), "UTC", true, false)
            .next_run_at
            .is_none());
        for (id, cadence, enabled, next) in [
            ("manual", "manual", true, "2020-01-01T00:00:00Z"),
            ("disabled", "daily", false, "2020-01-01T00:00:00Z"),
            ("global", "global", true, "2020-01-01T00:00:00Z"),
            ("daily", "daily", true, "2026-10-15T08:00:00-04:00"),
        ] {
            sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at, cadence, enabled, next_run_at) VALUES (?1, 'greenhouse', ?1, ?1, '2020-01-01', ?2, ?3, ?4)")
                .bind(id).bind(cadence).bind(enabled).bind(next).execute(&pool).await.unwrap();
        }
        let mut sources = SourcesRepo::list(&pool).await.unwrap();
        let status = status_for_sources(&sources, Some(clock), "UTC", true, false);
        assert_eq!(status.enabled_workflows, 3);
        assert_eq!(status.schedules.len(), 2);
        assert_eq!(
            status.next_run_at.as_deref(),
            Some("2026-10-15T12:00:00.000Z")
        );
        assert!(status
            .schedules
            .iter()
            .all(|s| s.next_run_at == status.next_run_at));
        let daily = sources.iter_mut().find(|s| s.id == "daily").unwrap();
        daily.config = serde_json::json!({"__schedule": {"timezone": "America/New_York"}});
        assert!(is_due(daily, clock));
        assert!(!is_due(daily, clock - chrono::Duration::milliseconds(1)));
        let paused = status_for_sources(&sources, Some(clock), "UTC", false, true);
        assert!(paused.running && !paused.enabled);
        assert!(paused.next_run_at.is_none());
        assert_eq!(
            paused
                .schedules
                .iter()
                .find(|s| s.id == "daily")
                .unwrap()
                .timezone,
            "America/New_York"
        );
        sources.retain(|s| s.id == "manual");
        assert!(
            status_for_sources(&sources, Some(clock), "UTC", true, false)
                .schedules
                .is_empty()
        );
        sources[0].cadence = scrape_core::SourceCadence::Daily;
        sources[0].next_run_at = Some("bad timestamp".into());
        let invalid = status_for_sources(&sources, None, "UTC", true, false);
        assert_eq!(invalid.schedules.len(), 1);
        assert!(invalid.schedules[0].next_run_at.is_none());
    }

    #[tokio::test]
    async fn dispatcher_pause_manual_exclusion_and_duplicate_coalescing() {
        let pool = crate::db::init_memory_db().await.unwrap();
        for (id, cadence, enabled) in [
            ("due-a", "daily", true),
            ("due-b", "weekly", true),
            ("manual", "manual", true),
            ("disabled", "daily", false),
        ] {
            sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at, cadence, enabled, next_run_at) VALUES (?1, 'greenhouse', ?1, ?1, '2020-01-01', ?2, ?3, '2026-10-01T09:00:00Z')")
                .bind(id).bind(cadence).bind(enabled).execute(&pool).await.unwrap();
        }
        let sources = SourcesRepo::list(&pool).await.unwrap();
        let clock = DateTime::parse_from_rfc3339("2026-10-13T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut queue = DispatchQueue::default();
        queue.enqueue_due(&sources, clock, true);
        assert!(queue.entries.is_empty());
        queue.enqueue_due(&sources, clock, false);
        queue.enqueue_due(&sources, clock, false);
        assert_eq!(queue.entries.len(), 2);
        let ids: HashSet<_> = [queue.pop().unwrap().0, queue.pop().unwrap().0]
            .into_iter()
            .collect();
        assert_eq!(ids, ["due-a".into(), "due-b".into()].into_iter().collect());
        assert!(queue.pop().is_none());
    }
}
