use crate::error::Result;
use scrape_core::enrich::salary::legacy_compensation;
use scrape_core::{SalaryInterval, SalarySource};
use sqlx::SqlitePool;

pub async fn backfill(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    type LegacyPay = (
        String,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<SalaryInterval>,
        Option<String>,
        Option<SalarySource>,
    );
    let rows: Vec<LegacyPay> = sqlx::query_as("SELECT id, salary_min, salary_max, salary_currency, salary_interval, salary_raw, salary_source FROM jobs WHERE compensation_migrated = 0")
        .fetch_all(&mut *tx).await?;
    for (id, min, max, currency, interval, raw, source) in rows {
        let ranges = legacy_compensation(
            min,
            max,
            currency.as_deref(),
            interval,
            raw.as_deref(),
            source,
        );
        sqlx::query(
            "UPDATE jobs SET compensation_json = ?, compensation_migrated = 1 WHERE id = ?",
        )
        .bind(serde_json::to_string(&ranges)?)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::jobs::{JobFilter, JobsRepo};
    use scrape_core::enrich::{CompensationFilter, Salary};
    #[tokio::test]
    async fn worker_predicate_and_repository_agree_on_native_ranges() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let mut vectors = Vec::new();
        for (id, min, max, currency, interval) in [
            (
                "overlap",
                Some(45.25),
                Some(55.75),
                Some("USD"),
                Some("hour"),
            ),
            ("upper", None, Some(50.0), Some("USD"), Some("hour")),
            ("lower", Some(60.0), None, Some("USD"), Some("hour")),
            ("low", Some(40.0), Some(49.99), Some("USD"), Some("hour")),
            (
                "annual",
                Some(100000.0),
                Some(150000.0),
                Some("USD"),
                Some("year"),
            ),
            ("eur", Some(45.0), Some(55.0), Some("EUR"), Some("hour")),
            (
                "currency_unknown",
                Some(45.0),
                Some(55.0),
                None,
                Some("hour"),
            ),
            ("unit_unknown", Some(45.0), Some(55.0), Some("USD"), None),
        ] {
            let ranges = Salary::from_structured(min, max, currency, interval, None)
                .unwrap()
                .ranges;
            sqlx::query("INSERT INTO jobs (id, dedupe_key, title, company, url, date_added, last_seen_at, status_changed_at, compensation_json, compensation_migrated) VALUES (?1, ?1, 'Engineer', 'Co', 'https://example.com/job', '2020-01-01', '2020-01-01', '2020-01-01', ?2, 1)")
                .bind(id).bind(serde_json::to_string(&ranges).unwrap()).execute(&pool).await.unwrap();
            vectors.push((id, ranges));
        }
        for (min, max, currency, interval) in [
            (
                Some(50.0),
                Some(60.0),
                Some("USD"),
                Some(SalaryInterval::Hour),
            ),
            (Some(55.75), None, Some("USD"), Some(SalaryInterval::Hour)),
            (None, Some(45.25), Some("USD"), Some(SalaryInterval::Hour)),
            (
                Some(120000.0),
                Some(160000.0),
                Some("USD"),
                Some(SalaryInterval::Year),
            ),
        ] {
            let worker = CompensationFilter {
                min_minor: min.map(|v| (v * 100.0_f64).round() as i64),
                max_minor: max.map(|v| (v * 100.0_f64).round() as i64),
                currency: currency.map(str::to_string),
                interval,
                include_unknown: false,
            };
            let mut expected: Vec<_> = vectors
                .iter()
                .filter(|(_, ranges)| worker.matches(ranges))
                .map(|(id, _)| id.to_string())
                .collect();
            expected.sort();
            let page = JobsRepo::list(
                &pool,
                JobFilter {
                    matching: None,
                    salary_min: min,
                    salary_max: max,
                    salary_currency: currency.map(str::to_string),
                    salary_interval: interval,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            let mut actual: Vec<_> = page.items.iter().map(|job| job.id.clone()).collect();
            actual.sort();
            assert_eq!(actual, expected);
            assert_eq!(page.total as usize, expected.len());
        }
        assert!(JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                salary_min: Some(60.0),
                salary_max: Some(50.0),
                ..Default::default()
            }
        )
        .await
        .is_err());
    }
    #[test]
    fn known_annualization_can_be_reversed_but_uncertain_units_stay_unknown() {
        let known = legacy_compensation(
            Some(104000),
            Some(124800),
            Some("USD"),
            Some(SalaryInterval::Hour),
            None,
            Some(SalarySource::Structured),
        );
        assert_eq!(known[0].min_minor, Some(5000));
        assert!(known[0].derived);
        assert!(legacy_compensation(Some(50), Some(60), Some("USD"), None, None, None).is_empty());
    }
}

#[cfg(test)]
mod rule_parity_tests {
    use crate::db::{
        init_memory_db,
        jobs::{JobFilter, JobsRepo},
    };
    use chrono::{DateTime, Utc};
    use scrape_core::{
        matching::{KeywordRules, PostingRules},
        posting_date::PostingDateFilter,
        WorkerEvent,
    };
    #[tokio::test]
    async fn enriched_worker_and_repository_rules_share_date_keywords_role_workplace_and_eligibility_semantics(
    ) {
        let pool = init_memory_db().await.unwrap();
        let clock = DateTime::parse_from_rfc3339("2026-10-14T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let mut events = Vec::new();
        for (id, workplace, grad, country, region, posted, min, max) in [
            (
                "a",
                "remote",
                "yes",
                "US",
                "US-CA",
                "2026-10-13",
                Some(0),
                Some(2),
            ),
            (
                "b",
                "hybrid",
                "no",
                "US",
                "US-NY",
                "2026-10-14T03:59:59Z",
                Some(2),
                None,
            ),
            (
                "c",
                "unknown",
                "unknown",
                "GB",
                "GB-LND",
                "2026-10-14T04:00:00Z",
                None,
                Some(3),
            ),
        ] {
            let location = serde_json::json!({"raw":country,"city":null,"country_code":country,"region_code":region,"is_remote":workplace=="remote"});
            let event: WorkerEvent = serde_json::from_value(serde_json::json!({
                "type":"job", "source_id":"fixture", "external_id":id, "title":"Rust Engineer", "company":"Co", "role_type":"full_time", "url":"https://example.com/job", "location":country,
                "workplace_type":workplace, "is_new_grad":grad, "date_posted":posted,
                "description_text":"Distributed systems", "locations":[location.clone()],
                "sponsorship":{"value":"yes","evidence":null}, "work_auth":{"value":"open","evidence":null}, "clearance":{"value":"no","level":null,"evidence":null},
                "experience":{"min_years":min,"max_years":max,"raw":null,"is_new_grad":grad,"evidence":null}
            })).unwrap();
            sqlx::query("INSERT INTO jobs (id,dedupe_key,title,company,role_type,url,location,date_posted,date_added,last_seen_at,status_changed_at,workplace_type,is_new_grad,sponsorship,work_auth,clearance,experience_min,experience_max,description_text,compensation_migrated) VALUES (?1,?1,'Rust Engineer','Co','full_time','https://example.com/job',?2,?3,'2026-01-01','2026-01-01','2026-01-01',?4,?5,'yes','open','no',?6,?7,'Distributed systems',1)")
                .bind(id).bind(country).bind(posted).bind(workplace).bind(grad).bind(min).bind(max).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO job_locations (job_id,raw,city,region_code,country_code,is_remote) VALUES (?, ?, NULL, ?, ?, ?)")
                .bind(id).bind(country).bind(region).bind(country).bind(workplace=="remote").execute(&pool).await.unwrap();
            events.push((id, event));
        }
        let rules: Vec<PostingRules> = vec![
            PostingRules {
                keywords: Some(KeywordRules {
                    any: vec!["Rust".into()],
                    all: vec!["Engineer".into()],
                    exclude: vec!["Senior".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
            PostingRules {
                experience_min: Some(2),
                experience_max: Some(3),
                ..Default::default()
            },
            PostingRules {
                experience_min: Some(2),
                experience_max: Some(3),
                experience_match_mode: scrape_core::matching::ExperienceMatchMode::RequiredMinimum,
                ..Default::default()
            },
            PostingRules {
                experience_min: Some(2),
                experience_max: Some(3),
                experience_match_mode: scrape_core::matching::ExperienceMatchMode::RequiredMinimum,
                experience_include_unknown: true,
                ..Default::default()
            },
            PostingRules {
                new_grad: Some("exclude".into()),
                ..Default::default()
            },
            PostingRules {
                workplaces: vec![scrape_core::WorkplaceType::Remote],
                ..Default::default()
            },
            PostingRules {
                countries: vec!["US".into()],
                regions: vec!["CA".into()],
                ..Default::default()
            },
            PostingRules {
                sponsorship: vec![scrape_core::TriState::Yes],
                clearance: vec![scrape_core::TriState::No],
                work_auth: vec![scrape_core::WorkAuthRequirement::Open],
                role_types: vec![scrape_core::RoleType::FullTime],
                ..Default::default()
            },
            PostingRules {
                posting_date: Some(PostingDateFilter {
                    last_hours: None,
                    from: Some("2026-10-13".into()),
                    through: Some("2026-10-13".into()),
                    timezone: "America/New_York".into(),
                    include_unknown: false,
                }),
                ..Default::default()
            },
            PostingRules {
                posting_date: Some(PostingDateFilter {
                    last_hours: Some(24),
                    from: None,
                    through: None,
                    timezone: "America/New_York".into(),
                    include_unknown: false,
                }),
                ..Default::default()
            },
        ];
        for rules in rules {
            let mut expected: Vec<_> = events
                .iter()
                .filter(|(_, event)| rules.matches_event_at(event, clock))
                .map(|(id, _)| id.to_string())
                .collect();
            expected.sort();
            if rules.experience_min == Some(2) {
                let authored: Vec<String> = match rules.experience_match_mode {
                    scrape_core::matching::ExperienceMatchMode::AdvertisedRangeOverlap => {
                        vec!["a", "b", "c"]
                    }
                    scrape_core::matching::ExperienceMatchMode::RequiredMinimum
                        if rules.experience_include_unknown =>
                    {
                        vec!["b", "c"]
                    }
                    scrape_core::matching::ExperienceMatchMode::RequiredMinimum => vec!["b"],
                }
                .into_iter()
                .map(str::to_string)
                .collect();
                assert_eq!(expected, authored);
            }
            let page = JobsRepo::list_at(
                &pool,
                JobFilter {
                    matching: Some(rules),
                    ..Default::default()
                },
                clock,
            )
            .await
            .unwrap();
            let mut actual: Vec<_> = page.items.iter().map(|job| job.id.clone()).collect();
            actual.sort();
            assert_eq!(actual, expected);
            assert_eq!(page.total as usize, expected.len());
        }
    }
}
