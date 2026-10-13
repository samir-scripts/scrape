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
