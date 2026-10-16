use crate::db::settings::SettingsRepo;
use crate::error::Result;
use chrono::{Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use scrape_core::RoleType;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeSet, HashMap};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct Metrics {
    pub totals: MetricsTotals,
    pub series: Vec<MetricsSeriesBucket>,
    pub by_role_type: Vec<RoleTypeMetric>,
    pub salary_distribution: Vec<SalaryDistributionBucket>,
    pub streak: MetricsStreak,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct MetricsTotals {
    #[ts(type = "number")]
    pub scraped: i64,
    #[ts(type = "number")]
    pub applied: i64,
    #[ts(type = "number")]
    pub interviewing: i64,
    #[ts(type = "number")]
    pub offers: i64,
    #[ts(type = "number")]
    pub favorited: i64,
    pub response_rate: f64,
    #[ts(type = "number | null")]
    pub median_salary: Option<i64>,
    #[ts(type = "number | null")]
    pub avg_salary: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct MetricsSeriesBucket {
    pub bucket_start: String,
    pub label: String,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct RoleTypeMetric {
    pub role_type: RoleType,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct SalaryDistributionBucket {
    pub range_label: String,
    #[ts(type = "number")]
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub struct MetricsStreak {
    #[ts(type = "number")]
    pub current: i64,
    #[ts(type = "number")]
    pub longest: i64,
    pub last_applied_on: Option<String>,
}

pub struct MetricsRepo;

impl MetricsRepo {
    pub async fn get_metrics(
        pool: &SqlitePool,
        granularity: &str,
        periods: u32,
    ) -> Result<Metrics> {
        let settings = SettingsRepo::get_all(pool).await?;
        let tz: Tz = settings.system_timezone.parse().unwrap_or(chrono_tz::UTC);
        let week_starts_on_monday = settings.system_week_start.to_lowercase() == "monday";

        // 1. Totals
        let scraped: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jobs WHERE removed_at IS NULL")
            .fetch_one(pool)
            .await?;

        let applied: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT id) FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE status IN ('applied', 'interviewing', 'offer', 'rejected')
                OR id IN (SELECT job_id FROM job_status_events WHERE to_status = 'applied')",
        )
        .fetch_one(pool)
        .await?;

        let interviewing: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT id) FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE status IN ('interviewing', 'offer')
                OR id IN (SELECT job_id FROM job_status_events WHERE to_status = 'interviewing')",
        )
        .fetch_one(pool)
        .await?;

        let offers: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT id) FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE status = 'offer'
                OR id IN (SELECT job_id FROM job_status_events WHERE to_status = 'offer')",
        )
        .fetch_one(pool)
        .await?;

        let responded: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT id) FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE status IN ('interviewing', 'offer')
                OR id IN (SELECT job_id FROM job_status_events WHERE to_status IN ('interviewing', 'offer'))"
        )
        .fetch_one(pool)
        .await?;

        let response_rate = if applied > 0 {
            (responded as f64) / (applied as f64)
        } else {
            0.0
        };

        let favorited: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM jobs WHERE removed_at IS NULL AND favorited = 1",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        // 2. Salary calculations
        let salary_rows: Vec<(Option<i64>, Option<i64>)> = sqlx::query_as(
            "SELECT salary_min, salary_max FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE salary_min IS NOT NULL OR salary_max IS NOT NULL",
        )
        .fetch_all(pool)
        .await?;

        let mut representative_salaries: Vec<i64> = salary_rows
            .into_iter()
            .filter_map(|(min, max)| match (min, max) {
                (Some(mn), Some(mx)) => Some((mn + mx) / 2),
                (Some(mn), None) => Some(mn),
                (None, Some(mx)) => Some(mx),
                (None, None) => None,
            })
            .collect();

        let (median_salary, avg_salary) = if representative_salaries.is_empty() {
            (None, None)
        } else {
            representative_salaries.sort_unstable();
            let count = representative_salaries.len();
            let sum: i64 = representative_salaries.iter().sum();
            let avg = sum / (count as i64);

            let median = if count % 2 == 1 {
                representative_salaries[count / 2]
            } else {
                (representative_salaries[(count / 2) - 1] + representative_salaries[count / 2]) / 2
            };

            (Some(median), Some(avg))
        };

        // Salary distribution buckets
        let mut under_100k = 0i64;
        let mut between_100k_150k = 0i64;
        let mut between_150k_200k = 0i64;
        let mut over_200k = 0i64;

        for &sal in &representative_salaries {
            if sal < 100_000 {
                under_100k += 1;
            } else if sal < 150_000 {
                between_100k_150k += 1;
            } else if sal < 200_000 {
                between_150k_200k += 1;
            } else {
                over_200k += 1;
            }
        }

        let salary_distribution = vec![
            SalaryDistributionBucket {
                range_label: "<$100k".to_string(),
                count: under_100k,
            },
            SalaryDistributionBucket {
                range_label: "$100k-$150k".to_string(),
                count: between_100k_150k,
            },
            SalaryDistributionBucket {
                range_label: "$150k-$200k".to_string(),
                count: between_150k_200k,
            },
            SalaryDistributionBucket {
                range_label: ">$200k".to_string(),
                count: over_200k,
            },
        ];

        // 3. By Role Type (jobs ever applied)
        let role_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT role_type, COUNT(*) as count
             FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE status IN ('applied', 'interviewing', 'offer', 'rejected')
                OR id IN (SELECT job_id FROM job_status_events WHERE to_status = 'applied')
             GROUP BY role_type",
        )
        .fetch_all(pool)
        .await?;

        let mut role_counts: HashMap<String, i64> = role_rows.into_iter().collect();

        let all_roles = [
            RoleType::FullTime,
            RoleType::Contract,
            RoleType::Internship,
            RoleType::PartTime,
            RoleType::Seasonal,
            RoleType::Unknown,
        ];

        let by_role_type = all_roles
            .into_iter()
            .map(|rt| {
                let key = serde_json::to_string(&rt)
                    .unwrap()
                    .trim_matches('"')
                    .to_string();
                let count = role_counts.remove(&key).unwrap_or(0);
                RoleTypeMetric {
                    role_type: rt,
                    count,
                }
            })
            .collect();

        // 4. Applied dates for series & streaks
        let applied_dates_raw: Vec<String> = sqlx::query_scalar(
            "SELECT COALESCE(
                (SELECT MIN(changed_at) FROM job_status_events WHERE job_id = jobs.id AND to_status = 'applied'),
                status_changed_at,
                date_added
             ) as applied_at
             FROM (SELECT * FROM jobs WHERE removed_at IS NULL) jobs
             WHERE status IN ('applied', 'interviewing', 'offer', 'rejected')
                OR id IN (SELECT job_id FROM job_status_events WHERE to_status = 'applied')"
        )
        .fetch_all(pool)
        .await?;

        let local_applied_dates: Vec<NaiveDate> = applied_dates_raw
            .iter()
            .filter_map(|raw| parse_to_local_date(raw, &tz))
            .collect();

        let now_local = Utc::now().with_timezone(&tz);
        let today_local = now_local.date_naive();
        let periods_count = if periods == 0 { 12 } else { periods };

        let series = if granularity.eq_ignore_ascii_case("month") {
            generate_month_series(&local_applied_dates, today_local, periods_count)
        } else {
            generate_week_series(
                &local_applied_dates,
                today_local,
                periods_count,
                week_starts_on_monday,
            )
        };

        let streak = calculate_streak(&local_applied_dates, today_local);

        Ok(Metrics {
            totals: MetricsTotals {
                scraped,
                applied,
                interviewing,
                offers,
                favorited,
                response_rate,
                median_salary,
                avg_salary,
            },
            series,
            by_role_type,
            salary_distribution,
            streak,
        })
    }
}

fn parse_to_local_date(raw: &str, tz: &Tz) -> Option<NaiveDate> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(dt.with_timezone(tz).date_naive());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        let utc_dt = Utc.from_utc_datetime(&naive);
        return Some(utc_dt.with_timezone(tz).date_naive());
    }
    if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return Some(date);
    }
    None
}

fn generate_week_series(
    dates: &[NaiveDate],
    today: NaiveDate,
    periods: u32,
    week_starts_on_monday: bool,
) -> Vec<MetricsSeriesBucket> {
    let weekday = today.weekday();
    let days_from_start = if week_starts_on_monday {
        weekday.num_days_from_monday() as i64
    } else {
        weekday.num_days_from_sunday() as i64
    };

    let current_week_start = today - Duration::days(days_from_start);
    let mut buckets = Vec::with_capacity(periods as usize);

    for i in (0..periods).rev() {
        let bucket_start = current_week_start - Duration::weeks(i as i64);
        let bucket_end = bucket_start + Duration::days(7);

        let count = dates
            .iter()
            .filter(|&&d| d >= bucket_start && d < bucket_end)
            .count() as i64;

        let label = format!("W{}", bucket_start.iso_week().week());

        buckets.push(MetricsSeriesBucket {
            bucket_start: bucket_start.format("%Y-%m-%d").to_string(),
            label,
            count,
        });
    }

    buckets
}

fn generate_month_series(
    dates: &[NaiveDate],
    today: NaiveDate,
    periods: u32,
) -> Vec<MetricsSeriesBucket> {
    let mut buckets = Vec::with_capacity(periods as usize);

    let cur_year = today.year();
    let cur_month = today.month() as i32;

    for i in (0..periods).rev() {
        let total_months = (cur_year * 12 + cur_month - 1) - i as i32;
        let y = total_months / 12;
        let m = (total_months % 12) + 1;

        let bucket_start = NaiveDate::from_ymd_opt(y, m as u32, 1).unwrap_or(today);
        let next_total = total_months + 1;
        let next_y = next_total / 12;
        let next_m = (next_total % 12) + 1;
        let bucket_end = NaiveDate::from_ymd_opt(next_y, next_m as u32, 1)
            .unwrap_or(bucket_start + Duration::days(31));

        let count = dates
            .iter()
            .filter(|&&d| d >= bucket_start && d < bucket_end)
            .count() as i64;

        let label = bucket_start.format("%b").to_string().to_uppercase();

        buckets.push(MetricsSeriesBucket {
            bucket_start: bucket_start.format("%Y-%m-%d").to_string(),
            label,
            count,
        });
    }

    buckets
}

fn calculate_streak(dates: &[NaiveDate], today: NaiveDate) -> MetricsStreak {
    if dates.is_empty() {
        return MetricsStreak {
            current: 0,
            longest: 0,
            last_applied_on: None,
        };
    }

    let date_set: BTreeSet<NaiveDate> = dates.iter().copied().collect();
    let last_applied_on = date_set
        .iter()
        .max()
        .map(|d| d.format("%Y-%m-%d").to_string());

    // Calculate longest streak
    let mut longest = 0i64;
    let mut running = 0i64;
    let mut prev_date: Option<NaiveDate> = None;

    for &d in &date_set {
        if let Some(prev) = prev_date {
            if d == prev + Duration::days(1) {
                running += 1;
            } else {
                running = 1;
            }
        } else {
            running = 1;
        }
        if running > longest {
            longest = running;
        }
        prev_date = Some(d);
    }

    // Calculate current streak: ends today or yesterday
    let yesterday = today - Duration::days(1);
    let start_anchor = if date_set.contains(&today) {
        Some(today)
    } else if date_set.contains(&yesterday) {
        Some(yesterday)
    } else {
        None
    };

    let current = if let Some(mut anchor) = start_anchor {
        let mut cur = 0i64;
        while date_set.contains(&anchor) {
            cur += 1;
            anchor -= Duration::days(1);
        }
        cur
    } else {
        0
    };

    MetricsStreak {
        current,
        longest,
        last_applied_on,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;
    use crate::db::jobs::{JobsRepo, ScrapedJob};
    use scrape_core::JobStatus;

    #[tokio::test]
    async fn test_metrics_empty_database() {
        let pool = init_memory_db().await.unwrap();
        let metrics = MetricsRepo::get_metrics(&pool, "week", 12).await.unwrap();

        assert_eq!(metrics.totals.scraped, 0);
        assert_eq!(metrics.totals.applied, 0);
        assert_eq!(metrics.totals.interviewing, 0);
        assert_eq!(metrics.totals.offers, 0);
        assert_eq!(metrics.totals.response_rate, 0.0);
        assert_eq!(metrics.totals.favorited, 0);
        assert_eq!(metrics.totals.median_salary, None);
        assert_eq!(metrics.totals.avg_salary, None);
        assert_eq!(metrics.series.len(), 12);
        for b in &metrics.series {
            assert_eq!(b.count, 0);
        }
        assert_eq!(metrics.streak.current, 0);
        assert_eq!(metrics.streak.longest, 0);
        assert_eq!(metrics.streak.last_applied_on, None);
    }

    #[tokio::test]
    async fn test_streak_today_and_yesterday_anchor() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let yesterday = today - Duration::days(1);
        let day_before = yesterday - Duration::days(1);

        // Active today: 3 days streak
        let dates = vec![day_before, yesterday, today];
        let streak = calculate_streak(&dates, today);
        assert_eq!(streak.current, 3);
        assert_eq!(streak.longest, 3);
        assert_eq!(streak.last_applied_on, Some("2026-10-09".to_string()));

        // Active yesterday (not yet applied today): streak still preserved as 2
        let dates_yesterday = vec![day_before, yesterday];
        let streak_y = calculate_streak(&dates_yesterday, today);
        assert_eq!(streak_y.current, 2);
        assert_eq!(streak_y.longest, 2);

        // Gap 2 days ago: current is 0
        let dates_broken = vec![day_before - Duration::days(1), day_before];
        let streak_b = calculate_streak(&dates_broken, today);
        assert_eq!(streak_b.current, 0);
        assert_eq!(streak_b.longest, 2);
    }

    fn mock_scraped_job(
        id: &str,
        title: &str,
        company: &str,
        role_type: RoleType,
        salary: Option<(i64, i64)>,
    ) -> ScrapedJob {
        ScrapedJob {
            compensation: vec![],
            dedupe_key: format!("test:{}:{}", company.to_lowercase(), id),
            source_id: None,
            external_id: Some(id.to_string()),
            title: title.to_string(),
            company: company.to_string(),
            role_type,
            location: Some("Remote".to_string()),
            url: format!("https://example.com/{}", id),
            provider_updated_at: None,
            location_data_complete: true,
            date_posted: None,
            salary_min: salary.map(|(min, _)| min),
            salary_max: salary.map(|(_, max)| max),
            salary_currency: salary.map(|_| "USD".to_string()),
            salary_interval: salary.map(|_| scrape_core::SalaryInterval::Year),
            salary_raw: None,
            salary_source: None,
            workplace_type: scrape_core::WorkplaceType::Remote,
            sponsorship: scrape_core::TriState::Unknown,
            sponsorship_evidence: None,
            work_auth: scrape_core::WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: scrape_core::TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: scrape_core::TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![],
        }
    }

    #[tokio::test]
    async fn test_salary_distribution_and_medians() {
        let pool = init_memory_db().await.unwrap();

        let jobs = vec![
            mock_scraped_job(
                "j1",
                "Junior Dev",
                "Startup",
                RoleType::FullTime,
                Some((80_000, 90_000)),
            ),
            mock_scraped_job(
                "j2",
                "Mid Dev",
                "MidCorp",
                RoleType::FullTime,
                Some((120_000, 140_000)),
            ),
            mock_scraped_job(
                "j3",
                "Staff Dev",
                "BigTech",
                RoleType::FullTime,
                Some((210_000, 250_000)),
            ),
        ];

        JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();

        let metrics = MetricsRepo::get_metrics(&pool, "week", 12).await.unwrap();
        assert_eq!(metrics.totals.scraped, 3);
        // Representative salaries: 85k, 130k, 230k -> median = 130k, avg = 148333
        assert_eq!(metrics.totals.median_salary, Some(130000));
        assert_eq!(metrics.totals.avg_salary, Some(148333));

        let dist = metrics.salary_distribution;
        assert_eq!(dist[0].range_label, "<$100k");
        assert_eq!(dist[0].count, 1);
        assert_eq!(dist[1].range_label, "$100k-$150k");
        assert_eq!(dist[1].count, 1);
        assert_eq!(dist[2].range_label, "$150k-$200k");
        assert_eq!(dist[2].count, 0);
        assert_eq!(dist[3].range_label, ">$200k");
        assert_eq!(dist[3].count, 1);
    }

    #[tokio::test]
    async fn test_metrics_pipeline_conversions_and_multiple_flips() {
        let pool = init_memory_db().await.unwrap();

        let jobs = vec![
            mock_scraped_job("j1", "Engineer 1", "Alpha", RoleType::FullTime, None),
            mock_scraped_job("j2", "Engineer 2", "Beta", RoleType::Contract, None),
        ];

        JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();

        // Get job IDs
        let stored_jobs = JobsRepo::list(&pool, Default::default()).await.unwrap();
        let id1 = &stored_jobs.items[0].id;
        let id2 = &stored_jobs.items[1].id;

        // Transition job 1: Scraped -> Applied -> Interviewing -> Offer
        JobsRepo::update_status(&pool, id1, JobStatus::Applied)
            .await
            .unwrap();
        JobsRepo::update_status(&pool, id1, JobStatus::Interviewing)
            .await
            .unwrap();
        JobsRepo::update_status(&pool, id1, JobStatus::Offer)
            .await
            .unwrap();

        // Transition job 2: Scraped -> Applied
        JobsRepo::update_status(&pool, id2, JobStatus::Applied)
            .await
            .unwrap();

        let metrics = MetricsRepo::get_metrics(&pool, "week", 12).await.unwrap();
        assert_eq!(metrics.totals.scraped, 2);
        assert_eq!(metrics.totals.applied, 2);
        assert_eq!(metrics.totals.interviewing, 1);
        assert_eq!(metrics.totals.offers, 1);
        // Response rate: 1 job received interviewing/offer out of 2 applied = 50%
        assert_eq!(metrics.totals.response_rate, 0.5);

        // Job 1 should count once for its role type
        let ft = metrics
            .by_role_type
            .iter()
            .find(|r| r.role_type == RoleType::FullTime)
            .unwrap();
        let ct = metrics
            .by_role_type
            .iter()
            .find(|r| r.role_type == RoleType::Contract)
            .unwrap();
        assert_eq!(ft.count + ct.count, 2);
    }
}
