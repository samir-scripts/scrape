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
        for (min, max, currency, interval, expected) in [
            (
                Some(50.0),
                Some(60.0),
                Some("USD"),
                Some(SalaryInterval::Hour),
                vec!["lower", "overlap", "upper"],
            ),
            (
                Some(55.75),
                None,
                Some("USD"),
                Some(SalaryInterval::Hour),
                vec!["lower", "overlap"],
            ),
            (
                None,
                Some(45.25),
                Some("USD"),
                Some(SalaryInterval::Hour),
                vec!["low", "overlap", "upper"],
            ),
            (
                Some(120000.0),
                Some(160000.0),
                Some("USD"),
                Some(SalaryInterval::Year),
                vec!["annual"],
            ),
        ] {
            let worker = CompensationFilter {
                min_minor: min.map(|v| (v * 100.0_f64).round() as i64),
                max_minor: max.map(|v| (v * 100.0_f64).round() as i64),
                currency: currency.map(str::to_string),
                interval,
                include_unknown: false,
                match_mode: scrape_core::enrich::CompensationMatchMode::RangeOverlap,
            };
            let mut worker_ids: Vec<_> = vectors
                .iter()
                .filter(|(_, ranges)| worker.matches(ranges))
                .map(|(id, _)| *id)
                .collect();
            worker_ids.sort_unstable();
            assert_eq!(worker_ids, expected);
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
    async fn advertised_annual_usd_remote_pay_has_literal_worker_and_sql_identities() {
        use scrape_core::enrich::{CompensationFilter, CompensationMatchMode, Salary};
        use scrape_core::{SalaryInterval, WorkplaceType};

        let pool = init_memory_db().await.unwrap();
        let clock = DateTime::parse_from_rfc3339("2026-10-23T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let rules = PostingRules {
            workplaces: vec![WorkplaceType::Remote],
            compensation: Some(CompensationFilter {
                min_minor: Some(6000001),
                max_minor: None,
                currency: Some("USD".into()),
                interval: Some(SalaryInterval::Year),
                include_unknown: false,
                match_mode: CompensationMatchMode::AdvertisedAmount,
            }),
            ..Default::default()
        };
        let expected = ["cent_above", "lower_high", "range", "upper_high"];
        let mut events = Vec::new();
        for (id, min, max, currency, interval, workplace) in [
            (
                "range",
                Some(50000.0),
                Some(80000.0),
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "cent_above",
                Some(60000.01),
                Some(60000.01),
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "upper_high",
                None,
                Some(80000.0),
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "lower_high",
                Some(70000.0),
                None,
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "exact",
                Some(60000.0),
                Some(60000.0),
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "below",
                Some(40000.0),
                Some(60000.0),
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "lower_low",
                Some(50000.0),
                None,
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "lower_exact",
                Some(60000.0),
                None,
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "unknown_amount",
                None,
                None,
                Some("USD"),
                Some("year"),
                "remote",
            ),
            (
                "unknown_currency",
                Some(100000.0),
                None,
                None,
                Some("year"),
                "remote",
            ),
            (
                "unknown_interval",
                Some(100000.0),
                None,
                Some("USD"),
                None,
                "remote",
            ),
            (
                "hour",
                Some(100000.0),
                None,
                Some("USD"),
                Some("hour"),
                "remote",
            ),
            (
                "month",
                Some(100000.0),
                None,
                Some("USD"),
                Some("month"),
                "remote",
            ),
            (
                "eur",
                Some(100000.0),
                None,
                Some("EUR"),
                Some("year"),
                "remote",
            ),
            (
                "hybrid",
                Some(100000.0),
                None,
                Some("USD"),
                Some("year"),
                "hybrid",
            ),
            (
                "onsite",
                Some(100000.0),
                None,
                Some("USD"),
                Some("year"),
                "onsite",
            ),
            (
                "unknown_workplace",
                Some(100000.0),
                None,
                Some("USD"),
                Some("year"),
                "unknown",
            ),
            (
                "multiple_ranges",
                Some(40000.0),
                Some(50000.0),
                Some("USD"),
                Some("year"),
                "remote",
            ),
            ("no_salary", None, None, None, None, "remote"),
        ] {
            let mut salary = Salary::from_structured(min, max, currency, interval, None);
            if id == "multiple_ranges" {
                let pay = salary.as_mut().unwrap();
                pay.ranges.extend(
                    Salary::from_text(
                        "Salary: EUR 80,000–90,000 per year; USD 80,000–90,000 per month.",
                    )
                    .unwrap()
                    .ranges,
                );
            }
            let ranges = salary.as_ref().map(|s| s.ranges.as_slice()).unwrap_or(&[]);
            sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_added,last_seen_at,status_changed_at,workplace_type,compensation_json,compensation_migrated) VALUES (?1,?1,'Engineer','Co',?2,'2026-10-23','2026-10-23','2026-10-23',?3,?4,1)")
                .bind(id)
                .bind(format!("https://fixture.test/{id}"))
                .bind(workplace)
                .bind(serde_json::to_string(ranges).unwrap())
                .execute(&pool).await.unwrap();
            let event: WorkerEvent = serde_json::from_value(serde_json::json!({
                "type":"job", "source_id":"fixture", "external_id":id,
                "title":"Engineer", "company":"Co", "url":format!("https://fixture.test/{id}"),
                "location":"", "role_type":"unknown", "workplace_type":workplace,
                "salary":salary
            }))
            .unwrap();
            events.push((id, event));
        }
        for (min_minor, max_minor, include_unknown, expected) in [
            (Some(6000001), None, false, expected.as_slice()),
            (Some(6000001), None, true, expected.as_slice()),
            (
                None,
                Some(6000000),
                false,
                &[
                    "below",
                    "exact",
                    "lower_exact",
                    "lower_low",
                    "multiple_ranges",
                    "range",
                ][..],
            ),
            (
                Some(6000001),
                Some(7000000),
                false,
                &["cent_above", "lower_high", "range"][..],
            ),
        ] {
            let mut rules = rules.clone();
            rules.compensation.as_mut().unwrap().include_unknown = include_unknown;
            rules.compensation.as_mut().unwrap().min_minor = min_minor;
            rules.compensation.as_mut().unwrap().max_minor = max_minor;
            let filters = scrape_core::WorkerFilters {
                matching: Some(rules.clone()),
                ..Default::default()
            };
            let evaluator = scrape_core::matching::MatchingEvaluator::new(&filters).unwrap();
            let mut worker_ids: Vec<_> = events
                .iter()
                .filter(|(_, event)| evaluator.rejection_reasons(event, clock).is_empty())
                .map(|(id, _)| *id)
                .collect();
            worker_ids.sort_unstable();
            assert_eq!(worker_ids, expected);
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
            let mut repository_ids: Vec<_> = page.items.iter().map(|job| job.id.as_str()).collect();
            repository_ids.sort_unstable();
            assert_eq!(repository_ids, expected);
            assert_eq!(page.total as usize, expected.len());
        }
    }

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
                    preset: None,
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
                    preset: None,
                    last_hours: Some(24),
                    from: None,
                    through: None,
                    timezone: "America/New_York".into(),
                    include_unknown: false,
                }),
                ..Default::default()
            },
            PostingRules {
                target_roles: vec!["Engineer".into()],
                companies: vec!["Co".into()],
                ..Default::default()
            },
            PostingRules {
                target_roles: vec!["Engineer".into()],
                companies: vec!["Google".into()],
                ..Default::default()
            },
            PostingRules {
                target_roles: vec!["Doctor".into()],
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

    #[tokio::test]
    async fn literal_role_company_choices_keep_worker_and_sql_parity() {
        let pool = init_memory_db().await.unwrap();
        let clock = DateTime::parse_from_rfc3339("2026-10-21T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let rows = [
            ("literal", "100%_ENGINEER\\ II", "A_B%\\ LLC"),
            ("wildcard-negative", "100xxEngineer", "AxBxx"),
            ("unicode", "ÉNGINEER", "Google"),
            ("unicode-negative", "éngineer", "Google"),
            ("wrong-company", "100%_ENGINEER\\ II", "Meta"),
            ("short-company", "ÉNGINEER", "Goo"),
        ];
        let rules = PostingRules {
            target_roles: vec![" 100%_Engineer\\ ".into(), "Éngineer".into()],
            companies: vec![" A_B%\\ ".into(), "Google".into()],
            ..Default::default()
        };
        for (id, title, company) in rows {
            sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_added,last_seen_at,status_changed_at) VALUES (?1,?1,?2,?3,?4,'2026-10-21','2026-10-21','2026-10-21')")
                .bind(id).bind(title).bind(company).bind(format!("https://fixture.test/{id}"))
                .execute(&pool).await.unwrap();
            let event: WorkerEvent = serde_json::from_value(serde_json::json!({
                "type":"job","source_id":"fixture","external_id":id,
                "title":title,"company":company,"url":format!("https://fixture.test/{id}"),
                "location":"","role_type":"unknown"
            }))
            .unwrap();
            assert_eq!(
                rules.matches_event_at(&event, clock),
                matches!(id, "literal" | "unicode"),
                "{id}"
            );
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
        let mut ids: Vec<_> = page.items.iter().map(|job| job.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, ["literal", "unicode"]);
        assert_eq!(page.total, 2);
    }
    #[tokio::test]
    async fn rolling_forty_eight_hour_window_keeps_day_precision_in_worker_and_sql() {
        let pool = init_memory_db().await.unwrap();
        let clock = DateTime::parse_from_rfc3339("2026-03-09T16:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let rules = PostingRules {
            posting_date: Some(
                scrape_core::posting_date::PostingDateFilter::from_preset(
                    "past_48h",
                    "America/New_York",
                    clock,
                )
                .unwrap(),
            ),
            ..Default::default()
        };
        for (id, posted, expected) in [
            ("cutoff-day", "2026-03-07", false),
            ("inside-day", "2026-03-08", true),
            ("current-day", "2026-03-09", false),
            ("inside-time", "2026-03-07T17:00:00Z", true),
            ("old-time", "2026-03-07T15:59:59Z", false),
        ] {
            sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_posted,date_added,last_seen_at,status_changed_at) VALUES (?1,?1,'Engineer','Fixture',?2,?3,'2026-03-09','2026-03-09','2026-03-09')")
                .bind(id).bind(format!("https://fixture.test/{id}")).bind(posted)
                .execute(&pool).await.unwrap();
            let event: WorkerEvent = serde_json::from_value(serde_json::json!({
                "type":"job","source_id":"fixture","external_id":id,
                "title":"Engineer","company":"Fixture","url":format!("https://fixture.test/{id}"),
                "location":"","role_type":"unknown","date_posted":posted
            })).unwrap();
            assert_eq!(rules.matches_event_at(&event, clock), expected, "{id}");
        }
        let page = JobsRepo::list_at(
            &pool,
            JobFilter { matching: Some(rules), ..Default::default() },
            clock,
        ).await.unwrap();
        let mut ids: Vec<_> = page.items.iter().map(|job| job.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, ["inside-day", "inside-time"]);
        assert_eq!(page.total, 2);
    }
}
