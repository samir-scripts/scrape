use crate::error::{AppError, Result};
use scrape_core::{now_iso, JobStatus, RoleType};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct Job {
    pub id: String,
    pub dedupe_key: String,
    pub source_id: Option<String>,
    pub external_id: Option<String>,
    pub title: String,
    pub company: String,
    pub role_type: RoleType,
    pub location: Option<String>,
    pub url: String,
    pub date_posted: Option<String>,
    pub date_added: String,
    pub last_seen_at: String,
    pub status: JobStatus,
    pub status_changed_at: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct JobStatusEvent {
    pub id: i64,
    pub job_id: String,
    pub from_status: Option<JobStatus>,
    pub to_status: JobStatus,
    pub changed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum JobSort {
    DateAddedDesc,
    DateAddedAsc,
    DatePostedDesc,
    DatePostedAsc,
    CompanyAsc,
    TitleAsc,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JobFilter {
    pub statuses: Option<Vec<JobStatus>>,
    pub role_types: Option<Vec<RoleType>>,
    pub search: Option<String>,
    pub source_id: Option<String>,
    pub sort: Option<JobSort>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScrapedJob {
    pub dedupe_key: String,
    pub source_id: Option<String>,
    pub external_id: Option<String>,
    pub title: String,
    pub company: String,
    pub role_type: RoleType,
    pub location: Option<String>,
    pub url: String,
    pub date_posted: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct UpsertStats {
    pub total: i64,
    pub inserted: i64,
    pub updated: i64,
}

pub struct JobsRepo;

impl JobsRepo {
    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Job> {
        use sqlx::Row;
        let r = sqlx::query(
            r#"
            SELECT id, dedupe_key, source_id, external_id, title, company,
                   role_type, location, url, date_posted, date_added,
                   last_seen_at, status, status_changed_at, notes
            FROM jobs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {} not found", id)))?;

        let role_type_str: String = r.get("role_type");
        let status_str: String = r.get("status");
        let role_type: RoleType = serde_json::from_value(serde_json::Value::String(role_type_str))?;
        let status: JobStatus = serde_json::from_value(serde_json::Value::String(status_str))?;

        Ok(Job {
            id: r.get("id"),
            dedupe_key: r.get("dedupe_key"),
            source_id: r.get("source_id"),
            external_id: r.get("external_id"),
            title: r.get("title"),
            company: r.get("company"),
            role_type,
            location: r.get("location"),
            url: r.get("url"),
            date_posted: r.get("date_posted"),
            date_added: r.get("date_added"),
            last_seen_at: r.get("last_seen_at"),
            status,
            status_changed_at: r.get("status_changed_at"),
            notes: r.get("notes"),
        })
    }

    pub async fn list(pool: &SqlitePool, filter: JobFilter) -> Result<Page<Job>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 200);
        let offset = filter.offset.unwrap_or(0).max(0);

        // Build dynamic SQL query
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(statuses) = filter.statuses {
            if !statuses.is_empty() {
                let placeholders: Vec<&str> = statuses.iter().map(|_| "?").collect();
                conditions.push(format!("status IN ({})", placeholders.join(",")));
                for s in statuses {
                    let s_str = serde_json::to_string(&s)?;
                    params.push(s_str.trim_matches('"').to_string());
                }
            }
        }

        if let Some(role_types) = filter.role_types {
            if !role_types.is_empty() {
                let placeholders: Vec<&str> = role_types.iter().map(|_| "?").collect();
                conditions.push(format!("role_type IN ({})", placeholders.join(",")));
                for r in role_types {
                    let r_str = serde_json::to_string(&r)?;
                    params.push(r_str.trim_matches('"').to_string());
                }
            }
        }

        if let Some(source_id) = filter.source_id {
            conditions.push("source_id = ?".to_string());
            params.push(source_id);
        }

        if let Some(search) = filter.search {
            let term = format!("%{}%", search.trim());
            conditions.push("(title LIKE ? OR company LIKE ? OR location LIKE ?)".to_string());
            params.push(term.clone());
            params.push(term.clone());
            params.push(term);
        }

        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let order_by = match filter.sort.unwrap_or(JobSort::DateAddedDesc) {
            JobSort::DateAddedDesc => "date_added DESC",
            JobSort::DateAddedAsc => "date_added ASC",
            JobSort::DatePostedDesc => "date_posted DESC",
            JobSort::DatePostedAsc => "date_posted ASC",
            JobSort::CompanyAsc => "company ASC, date_added DESC",
            JobSort::TitleAsc => "title ASC, date_added DESC",
        };

        // Query total count
        let count_sql = format!("SELECT COUNT(*) as count FROM jobs {}", where_clause);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);
        for p in &params {
            count_query = count_query.bind(p);
        }
        let (total,) = count_query.fetch_one(pool).await?;

        // Query rows
        let select_sql = format!(
            r#"
            SELECT id, dedupe_key, source_id, external_id, title, company,
                   role_type, location, url, date_posted, date_added,
                   last_seen_at, status, status_changed_at, notes
            FROM jobs
            {}
            ORDER BY {}
            LIMIT ? OFFSET ?
            "#,
            where_clause, order_by
        );

        let mut data_query = sqlx::query(&select_sql);
        for p in &params {
            data_query = data_query.bind(p);
        }
        data_query = data_query.bind(limit).bind(offset);

        let rows = data_query.fetch_all(pool).await?;
        let mut items = Vec::new();
        for r in rows {
            use sqlx::Row;
            let role_type_str: String = r.get("role_type");
            let status_str: String = r.get("status");
            let role_type: RoleType =
                serde_json::from_value(serde_json::Value::String(role_type_str))?;
            let status: JobStatus = serde_json::from_value(serde_json::Value::String(status_str))?;

            items.push(Job {
                id: r.get("id"),
                dedupe_key: r.get("dedupe_key"),
                source_id: r.get("source_id"),
                external_id: r.get("external_id"),
                title: r.get("title"),
                company: r.get("company"),
                role_type,
                location: r.get("location"),
                url: r.get("url"),
                date_posted: r.get("date_posted"),
                date_added: r.get("date_added"),
                last_seen_at: r.get("last_seen_at"),
                status,
                status_changed_at: r.get("status_changed_at"),
                notes: r.get("notes"),
            });
        }

        Ok(Page {
            items,
            total,
            limit,
            offset,
        })
    }

    pub async fn update_status(pool: &SqlitePool, id: &str, new_status: JobStatus) -> Result<Job> {
        use sqlx::Row;
        let mut tx = pool.begin().await?;

        let current = sqlx::query("SELECT status FROM jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Job {} not found", id)))?;

        let current_status_str: String = current.get("status");
        let current_status: JobStatus =
            serde_json::from_value(serde_json::Value::String(current_status_str.clone()))?;

        if current_status != new_status {
            let now = now_iso();
            let new_status_str = match serde_json::to_value(new_status)? {
                serde_json::Value::String(s) => s,
                _ => unreachable!(),
            };

            sqlx::query(
                r#"
                UPDATE jobs
                SET status = ?1, status_changed_at = ?2
                WHERE id = ?3
                "#,
            )
            .bind(&new_status_str)
            .bind(&now)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO job_status_events (job_id, from_status, to_status, changed_at)
                VALUES (?1, ?2, ?3, ?4)
                "#,
            )
            .bind(id)
            .bind(current_status_str)
            .bind(new_status_str)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Self::get(pool, id).await
    }

    pub async fn bulk_update_status(
        pool: &SqlitePool,
        ids: &[String],
        new_status: JobStatus,
    ) -> Result<u64> {
        let mut count = 0;
        for id in ids {
            if Self::update_status(pool, id, new_status).await.is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    pub async fn update_notes(pool: &SqlitePool, id: &str, notes: Option<&str>) -> Result<Job> {
        let rows = sqlx::query("UPDATE jobs SET notes = ?1 WHERE id = ?2")
            .bind(notes)
            .bind(id)
            .execute(pool)
            .await?
            .rows_affected();

        if rows == 0 {
            return Err(AppError::NotFound(format!("Job {} not found", id)));
        }

        Self::get(pool, id).await
    }

    pub async fn get_status_events(pool: &SqlitePool, job_id: &str) -> Result<Vec<JobStatusEvent>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, job_id, from_status, to_status, changed_at
            FROM job_status_events
            WHERE job_id = ?
            ORDER BY changed_at ASC
            "#,
        )
        .bind(job_id)
        .fetch_all(pool)
        .await?;

        let mut events = Vec::new();
        for r in rows {
            let from_status_str: Option<String> = r.get("from_status");
            let to_status_str: String = r.get("to_status");
            let from_status: Option<JobStatus> = match from_status_str {
                Some(fs) => Some(serde_json::from_value(serde_json::Value::String(fs))?),
                None => None,
            };
            let to_status: JobStatus =
                serde_json::from_value(serde_json::Value::String(to_status_str))?;

            events.push(JobStatusEvent {
                id: r.get("id"),
                job_id: r.get("job_id"),
                from_status,
                to_status,
                changed_at: r.get("changed_at"),
            });
        }

        Ok(events)
    }

    pub async fn upsert_scraped(pool: &SqlitePool, batch: &[ScrapedJob]) -> Result<UpsertStats> {
        let mut stats = UpsertStats::default();
        let now = now_iso();

        let mut tx = pool.begin().await?;

        for job in batch {
            stats.total += 1;

            use sqlx::Row;
            let existing = sqlx::query("SELECT id, role_type FROM jobs WHERE dedupe_key = ?")
                .bind(&job.dedupe_key)
                .fetch_optional(&mut *tx)
                .await?;

            let role_type_str = match serde_json::to_value(job.role_type)? {
                serde_json::Value::String(s) => s,
                _ => unreachable!(),
            };

            match existing {
                Some(ex) => {
                    stats.updated += 1;

                    let ex_id: String = ex.get("id");
                    let ex_role_type: String = ex.get("role_type");

                    // If existing role_type is unknown, replace with incoming role_type
                    let should_update_role = ex_role_type == "unknown";
                    let new_role_type = if should_update_role {
                        role_type_str
                    } else {
                        ex_role_type
                    };

                    sqlx::query(
                        r#"
                        UPDATE jobs
                        SET title = ?1, company = ?2, role_type = ?3,
                            location = ?4, url = ?5, date_posted = ?6,
                            last_seen_at = ?7
                        WHERE id = ?8
                        "#,
                    )
                    .bind(&job.title)
                    .bind(&job.company)
                    .bind(new_role_type)
                    .bind(&job.location)
                    .bind(&job.url)
                    .bind(&job.date_posted)
                    .bind(&now)
                    .bind(ex_id)
                    .execute(&mut *tx)
                    .await?;
                }
                None => {
                    stats.inserted += 1;
                    let id = Uuid::now_v7().to_string();

                    sqlx::query(
                        r#"
                        INSERT INTO jobs (
                            id, dedupe_key, source_id, external_id, title,
                            company, role_type, location, url, date_posted,
                            date_added, last_seen_at, status, status_changed_at
                        )
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'scraped', ?13)
                        "#,
                    )
                    .bind(&id)
                    .bind(&job.dedupe_key)
                    .bind(&job.source_id)
                    .bind(&job.external_id)
                    .bind(&job.title)
                    .bind(&job.company)
                    .bind(role_type_str)
                    .bind(&job.location)
                    .bind(&job.url)
                    .bind(&job.date_posted)
                    .bind(&now)
                    .bind(&now)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await?;

                    sqlx::query(
                        r#"
                        INSERT INTO job_status_events (job_id, from_status, to_status, changed_at)
                        VALUES (?1, NULL, 'scraped', ?2)
                        "#,
                    )
                    .bind(&id)
                    .bind(&now)
                    .execute(&mut *tx)
                    .await?;
                }
            }
        }

        tx.commit().await?;
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[tokio::test]
    async fn test_upsert_and_status_transitions() {
        let pool = init_memory_db().await.unwrap();

        let batch = vec![
            ScrapedJob {
                dedupe_key: "greenhouse:stripe:101".to_string(),
                source_id: None,
                external_id: Some("101".to_string()),
                title: "Data Engineer".to_string(),
                company: "Stripe".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Remote".to_string()),
                url: "https://stripe.com/jobs/101".to_string(),
                date_posted: Some("2026-10-01T00:00:00Z".to_string()),
            },
            ScrapedJob {
                dedupe_key: "lever:figma:202".to_string(),
                source_id: None,
                external_id: Some("202".to_string()),
                title: "Product Designer Intern".to_string(),
                company: "Figma".to_string(),
                role_type: RoleType::Internship,
                location: Some("San Francisco".to_string()),
                url: "https://figma.com/jobs/202".to_string(),
                date_posted: None,
            },
        ];

        let stats = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.inserted, 2);
        assert_eq!(stats.updated, 0);

        // Verify page listing
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                statuses: Some(vec![JobStatus::Scraped]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 2);

        let job = &page.items[0];
        assert_eq!(job.status, JobStatus::Scraped);

        // Update status to Applied
        let updated = JobsRepo::update_status(&pool, &job.id, JobStatus::Applied)
            .await
            .unwrap();
        assert_eq!(updated.status, JobStatus::Applied);

        // Check events log
        let events = JobsRepo::get_status_events(&pool, &job.id).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].to_status, JobStatus::Scraped);
        assert_eq!(events[1].from_status, Some(JobStatus::Scraped));
        assert_eq!(events[1].to_status, JobStatus::Applied);

        // Upsert again: should UPDATE without overwriting status
        let stats2 = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(stats2.inserted, 0);
        assert_eq!(stats2.updated, 2);

        let job_after = JobsRepo::get(&pool, &job.id).await.unwrap();
        assert_eq!(job_after.status, JobStatus::Applied); // status preserved!
    }

    #[tokio::test]
    async fn test_notes_and_search() {
        let pool = init_memory_db().await.unwrap();

        let batch = vec![ScrapedJob {
            dedupe_key: "greenhouse:stripe:101".to_string(),
            source_id: None,
            external_id: Some("101".to_string()),
            title: "Data Engineer".to_string(),
            company: "Stripe".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Remote".to_string()),
            url: "https://stripe.com/jobs/101".to_string(),
            date_posted: None,
        }];

        JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        let page = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        let job = &page.items[0];

        JobsRepo::update_notes(&pool, &job.id, Some("Applied via referral."))
            .await
            .unwrap();

        let fetched = JobsRepo::get(&pool, &job.id).await.unwrap();
        assert_eq!(fetched.notes, Some("Applied via referral.".to_string()));

        // Search test
        let search_res = JobsRepo::list(
            &pool,
            JobFilter {
                search: Some("referral".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(search_res.total, 0); // search matches title/company/location

        let search_title = JobsRepo::list(
            &pool,
            JobFilter {
                search: Some("Data".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(search_title.total, 1);
    }
}
