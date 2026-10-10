use crate::error::{AppError, Result};
use scrape_core::enrich::{ParsedLocation, ENRICHMENT_VERSION};
use scrape_core::{
    now_iso, ClearanceLevel, JobStatus, RoleType, SalaryInterval, SalarySource, TriState,
    WorkplaceType,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
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

    // Enrichment fields
    #[ts(type = "number | null")]
    pub salary_min: Option<i64>,
    #[ts(type = "number | null")]
    pub salary_max: Option<i64>,
    pub salary_currency: Option<String>,
    pub salary_interval: Option<SalaryInterval>,
    pub salary_raw: Option<String>,
    pub salary_source: Option<SalarySource>,
    pub workplace_type: WorkplaceType,
    pub sponsorship: TriState,
    pub sponsorship_evidence: Option<String>,
    pub clearance: TriState,
    pub clearance_level: Option<ClearanceLevel>,
    pub clearance_evidence: Option<String>,
    #[ts(type = "number | null")]
    pub experience_min: Option<i64>,
    #[ts(type = "number | null")]
    pub experience_max: Option<i64>,
    pub experience_raw: Option<String>,
    pub is_new_grad: TriState,
    pub experience_evidence: Option<String>,
    pub description_text: Option<String>,
    pub enrichment_version: i32,
    #[serde(default)]
    pub locations: Vec<ParsedLocation>,
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
    SalaryDesc,
    SalaryAsc,
    ExperienceDesc,
    ExperienceAsc,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JobFilter {
    pub statuses: Option<Vec<JobStatus>>,
    pub role_types: Option<Vec<RoleType>>,
    pub search: Option<String>,
    pub source_id: Option<String>,
    pub sort: Option<JobSort>,
    #[ts(type = "number | null")]
    pub limit: Option<i64>,
    #[ts(type = "number | null")]
    pub offset: Option<i64>,

    // Salary bounds
    #[ts(type = "number | null")]
    pub salary_min: Option<i64>,
    #[ts(type = "number | null")]
    pub salary_max: Option<i64>,
    pub salary_currency: Option<String>,
    pub salary_include_unknown: Option<bool>,

    // Experience bounds
    #[ts(type = "number | null")]
    pub experience_min: Option<i64>,
    #[ts(type = "number | null")]
    pub experience_max: Option<i64>,
    pub experience_include_unknown: Option<bool>,

    // New Grad filter
    pub new_grad: Option<String>, // "any" | "new_grad_only" | "exclude_new_grad"

    // Eligibility tri-state filters
    pub sponsorship: Option<Vec<TriState>>,
    pub clearance: Option<Vec<TriState>>,

    // Location filters
    pub countries: Option<Vec<String>>,
    pub regions: Option<Vec<String>>,
    pub remote: Option<String>, // "any" | "remote_only" | "exclude_remote"
    pub location_include_unknown: Option<bool>,
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
pub struct FilterImpactPreview {
    pub kept: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocationOptions {
    pub countries: Vec<String>,
    pub regions: Vec<String>,
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

    // Enrichment
    #[ts(type = "number | null")]
    pub salary_min: Option<i64>,
    #[ts(type = "number | null")]
    pub salary_max: Option<i64>,
    pub salary_currency: Option<String>,
    pub salary_interval: Option<SalaryInterval>,
    pub salary_raw: Option<String>,
    pub salary_source: Option<SalarySource>,
    pub workplace_type: WorkplaceType,
    pub sponsorship: TriState,
    pub sponsorship_evidence: Option<String>,
    pub clearance: TriState,
    pub clearance_level: Option<ClearanceLevel>,
    pub clearance_evidence: Option<String>,
    #[ts(type = "number | null")]
    pub experience_min: Option<i64>,
    #[ts(type = "number | null")]
    pub experience_max: Option<i64>,
    pub experience_raw: Option<String>,
    pub is_new_grad: TriState,
    pub experience_evidence: Option<String>,
    pub description_text: Option<String>,
    #[serde(default)]
    pub locations: Vec<ParsedLocation>,
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
        let r = sqlx::query(
            r#"
            SELECT id, dedupe_key, source_id, external_id, title, company,
                   role_type, location, url, date_posted, date_added,
                   last_seen_at, status, status_changed_at, notes,
                   salary_min, salary_max, salary_currency, salary_interval,
                   salary_raw, salary_source, workplace_type, sponsorship,
                   sponsorship_evidence, clearance, clearance_level,
                   clearance_evidence, experience_min, experience_max,
                   experience_raw, is_new_grad, experience_evidence,
                   description_text, enrichment_version
            FROM jobs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Job {} not found", id)))?;

        let locations = Self::get_locations(pool, id).await?;
        Self::map_row_to_job(r, locations)
    }

    pub async fn get_locations(pool: &SqlitePool, job_id: &str) -> Result<Vec<ParsedLocation>> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT raw, city, region_code, country_code, is_remote
            FROM job_locations
            WHERE job_id = ?
            "#,
        )
        .bind(job_id)
        .fetch_all(pool)
        .await?;

        let mut locs = Vec::new();
        for r in rows {
            let is_remote_int: i64 = r.get("is_remote");
            locs.push(ParsedLocation {
                raw: r.get("raw"),
                city: r.get("city"),
                region_code: r.get("region_code"),
                country_code: r.get("country_code"),
                is_remote: is_remote_int == 1,
            });
        }
        Ok(locs)
    }

    async fn load_locations_batch(
        pool: &SqlitePool,
        job_ids: &[String],
    ) -> Result<HashMap<String, Vec<ParsedLocation>>> {
        if job_ids.is_empty() {
            return Ok(HashMap::new());
        }
        use sqlx::Row;
        let placeholders: Vec<&str> = job_ids.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT job_id, raw, city, region_code, country_code, is_remote FROM job_locations WHERE job_id IN ({})",
            placeholders.join(",")
        );
        let mut query = sqlx::query(&sql);
        for id in job_ids {
            query = query.bind(id);
        }
        let rows = query.fetch_all(pool).await?;

        let mut map: HashMap<String, Vec<ParsedLocation>> = HashMap::new();
        for r in rows {
            let job_id: String = r.get("job_id");
            let is_remote_int: i64 = r.get("is_remote");
            let loc = ParsedLocation {
                raw: r.get("raw"),
                city: r.get("city"),
                region_code: r.get("region_code"),
                country_code: r.get("country_code"),
                is_remote: is_remote_int == 1,
            };
            map.entry(job_id).or_default().push(loc);
        }
        Ok(map)
    }

    fn map_row_to_job(r: sqlx::sqlite::SqliteRow, locations: Vec<ParsedLocation>) -> Result<Job> {
        use sqlx::Row;
        let role_type_str: String = r.get("role_type");
        let status_str: String = r.get("status");
        let role_type: RoleType = serde_json::from_value(serde_json::Value::String(role_type_str))?;
        let status: JobStatus = serde_json::from_value(serde_json::Value::String(status_str))?;

        let salary_interval: Option<SalaryInterval> = r
            .get::<Option<String>, _>("salary_interval")
            .and_then(|s| match s.as_str() {
                "year" => Some(SalaryInterval::Year),
                "month" => Some(SalaryInterval::Month),
                "week" => Some(SalaryInterval::Week),
                "day" => Some(SalaryInterval::Day),
                "hour" => Some(SalaryInterval::Hour),
                _ => None,
            });

        let salary_source: Option<SalarySource> = r
            .get::<Option<String>, _>("salary_source")
            .and_then(|s| match s.as_str() {
                "structured" => Some(SalarySource::Structured),
                "text" => Some(SalarySource::Text),
                _ => None,
            });

        let workplace_type: WorkplaceType = match r.get::<String, _>("workplace_type").as_str() {
            "remote" => WorkplaceType::Remote,
            "hybrid" => WorkplaceType::Hybrid,
            "onsite" => WorkplaceType::Onsite,
            _ => WorkplaceType::Unknown,
        };

        let sponsorship: TriState = match r.get::<String, _>("sponsorship").as_str() {
            "yes" => TriState::Yes,
            "no" => TriState::No,
            _ => TriState::Unknown,
        };

        let clearance: TriState = match r.get::<String, _>("clearance").as_str() {
            "yes" => TriState::Yes,
            "no" => TriState::No,
            _ => TriState::Unknown,
        };

        let clearance_level: Option<ClearanceLevel> = r
            .get::<Option<String>, _>("clearance_level")
            .and_then(|s| match s.as_str() {
                "public_trust" => Some(ClearanceLevel::PublicTrust),
                "confidential" => Some(ClearanceLevel::Confidential),
                "secret" => Some(ClearanceLevel::Secret),
                "top_secret" => Some(ClearanceLevel::TopSecret),
                "ts_sci" => Some(ClearanceLevel::TsSci),
                _ => None,
            });

        let is_new_grad: TriState = match r.get::<String, _>("is_new_grad").as_str() {
            "yes" => TriState::Yes,
            "no" => TriState::No,
            _ => TriState::Unknown,
        };

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
            salary_min: r.get("salary_min"),
            salary_max: r.get("salary_max"),
            salary_currency: r.get("salary_currency"),
            salary_interval,
            salary_raw: r.get("salary_raw"),
            salary_source,
            workplace_type,
            sponsorship,
            sponsorship_evidence: r.get("sponsorship_evidence"),
            clearance,
            clearance_level,
            clearance_evidence: r.get("clearance_evidence"),
            experience_min: r.get("experience_min"),
            experience_max: r.get("experience_max"),
            experience_raw: r.get("experience_raw"),
            is_new_grad,
            experience_evidence: r.get("experience_evidence"),
            description_text: r.get("description_text"),
            enrichment_version: r.get("enrichment_version"),
            locations,
        })
    }

    pub async fn list(pool: &SqlitePool, filter: JobFilter) -> Result<Page<Job>> {
        let limit = filter.limit.unwrap_or(50).clamp(1, 200);
        let offset = filter.offset.unwrap_or(0).max(0);

        let mut conditions = Vec::new();

        // Statuses
        if let Some(statuses) = &filter.statuses {
            if !statuses.is_empty() {
                let status_strs: Vec<String> = statuses
                    .iter()
                    .map(|s| serde_json::to_string(s).unwrap().trim_matches('"').to_string())
                    .collect();
                let placeholders = vec!["?"; status_strs.len()].join(",");
                conditions.push(format!("status IN ({})", placeholders));
            }
        }

        // Role types
        if let Some(role_types) = &filter.role_types {
            if !role_types.is_empty() {
                let role_strs: Vec<String> = role_types
                    .iter()
                    .map(|r| serde_json::to_string(r).unwrap().trim_matches('"').to_string())
                    .collect();
                let placeholders = vec!["?"; role_strs.len()].join(",");
                conditions.push(format!("role_type IN ({})", placeholders));
            }
        }

        // Source ID
        if filter.source_id.is_some() {
            conditions.push("source_id = ?".to_string());
        }

        // Search term
        if filter.search.is_some() {
            conditions.push("(title LIKE ? OR company LIKE ? OR location LIKE ?)".to_string());
        }

        // Salary range overlap: [lo, hi] overlaps if salary_max >= lo AND salary_min <= hi
        let sal_min = filter.salary_min;
        let sal_max = filter.salary_max;
        let sal_inc_unknown = filter.salary_include_unknown.unwrap_or(true);

        if sal_min.is_some() || sal_max.is_some() {
            let lo = sal_min.unwrap_or(0);
            let hi = sal_max.unwrap_or(i64::MAX);

            if sal_inc_unknown {
                conditions.push(format!(
                    "((salary_max IS NULL AND salary_min IS NULL) OR (salary_max >= {} AND salary_min <= {}))",
                    lo, hi
                ));
            } else {
                conditions.push(format!(
                    "(salary_max IS NOT NULL AND salary_min IS NOT NULL AND salary_max >= {} AND salary_min <= {})",
                    lo, hi
                ));
            }
        }

        // Salary currency
        if let Some(curr) = &filter.salary_currency {
            if !curr.trim().is_empty() {
                if sal_inc_unknown {
                    conditions.push(format!(
                        "(salary_currency = '{}' OR salary_currency IS NULL)",
                        curr.trim().to_uppercase()
                    ));
                } else {
                    conditions.push(format!(
                        "salary_currency = '{}'",
                        curr.trim().to_uppercase()
                    ));
                }
            }
        }

        // Experience (YOE) bounds: [min_yoe, max_yoe]
        let exp_min = filter.experience_min;
        let exp_max = filter.experience_max;
        let exp_inc_unknown = filter.experience_include_unknown.unwrap_or(true);

        if exp_min.is_some() || exp_max.is_some() {
            let min_y = exp_min.unwrap_or(0);
            let max_y = exp_max.unwrap_or(30);

            if exp_inc_unknown {
                conditions.push(format!(
                    "((experience_min IS NULL) OR (experience_min <= {} AND (experience_max IS NULL OR experience_max >= {})))",
                    max_y, min_y
                ));
            } else {
                conditions.push(format!(
                    "(experience_min IS NOT NULL AND (experience_min <= {} AND (experience_max IS NULL OR experience_max >= {})))",
                    max_y, min_y
                ));
            }
        }

        // New Grad filter: any | new_grad_only | exclude_new_grad
        if let Some(ng) = &filter.new_grad {
            match ng.as_str() {
                "new_grad_only" => {
                    conditions.push("(is_new_grad = 'yes' OR experience_min = 0)".to_string());
                }
                "exclude_new_grad" => {
                    conditions.push("(is_new_grad = 'no' OR experience_min >= 2)".to_string());
                }
                _ => {}
            }
        }

        // Sponsorship tri-state list
        if let Some(spon_list) = &filter.sponsorship {
            if !spon_list.is_empty() && spon_list.len() < 3 {
                let spon_strs: Vec<String> = spon_list
                    .iter()
                    .map(|s| serde_json::to_string(s).unwrap().trim_matches('"').to_string())
                    .collect();
                let joined = spon_strs
                    .iter()
                    .map(|s| format!("'{}'", s))
                    .collect::<Vec<_>>()
                    .join(",");
                conditions.push(format!("sponsorship IN ({})", joined));
            }
        }

        // Clearance tri-state list
        if let Some(clr_list) = &filter.clearance {
            if !clr_list.is_empty() && clr_list.len() < 3 {
                let clr_strs: Vec<String> = clr_list
                    .iter()
                    .map(|s| serde_json::to_string(s).unwrap().trim_matches('"').to_string())
                    .collect();
                let joined = clr_strs
                    .iter()
                    .map(|s| format!("'{}'", s))
                    .collect::<Vec<_>>()
                    .join(",");
                conditions.push(format!("clearance IN ({})", joined));
            }
        }

        // Remote policy
        if let Some(rem) = &filter.remote {
            match rem.as_str() {
                "remote" | "remote_only" => {
                    conditions.push("(workplace_type = 'remote' OR EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id = jobs.id AND jl.is_remote = 1))".to_string());
                }
                "exclude_remote" => {
                    conditions.push("(workplace_type != 'remote' AND NOT EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id = jobs.id AND jl.is_remote = 1))".to_string());
                }
                "hybrid" | "hybrid_only" => {
                    conditions.push("workplace_type = 'hybrid'".to_string());
                }
                "onsite" | "onsite_only" => {
                    conditions.push("workplace_type = 'onsite'".to_string());
                }
                _ => {}
            }
        }

        // Countries and Regions geo matching
        let has_countries = filter.countries.as_ref().map(|c| !c.is_empty()).unwrap_or(false);
        let has_regions = filter.regions.as_ref().map(|r| !r.is_empty()).unwrap_or(false);

        if has_countries || has_regions {
            let mut geo_sub = Vec::new();
            if let Some(c_list) = &filter.countries {
                if !c_list.is_empty() {
                    let formatted = c_list
                        .iter()
                        .map(|c| format!("'{}'", c.replace('\'', "''")))
                        .collect::<Vec<_>>()
                        .join(",");
                    geo_sub.push(format!("jl.country_code IN ({})", formatted));
                }
            }
            if let Some(r_list) = &filter.regions {
                if !r_list.is_empty() {
                    let mut region_conditions = Vec::new();
                    for r in r_list {
                        let clean = r.replace('\'', "''");
                        region_conditions.push(format!("jl.region_code = '{clean}' OR jl.region_code LIKE '%-{clean}'"));
                    }
                    geo_sub.push(format!("({})", region_conditions.join(" OR ")));
                }
            }

            let geo_clause = geo_sub.join(" OR ");
            if filter.location_include_unknown.unwrap_or(true) {
                conditions.push(format!(
                    "(EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id = jobs.id AND ({})) OR NOT EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id = jobs.id))",
                    geo_clause
                ));
            } else {
                conditions.push(format!(
                    "EXISTS (SELECT 1 FROM job_locations jl WHERE jl.job_id = jobs.id AND ({}))",
                    geo_clause
                ));
            }
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
            JobSort::SalaryDesc => "salary_max DESC, date_added DESC",
            JobSort::SalaryAsc => "salary_min ASC, date_added DESC",
            JobSort::ExperienceDesc => "experience_min DESC, date_added DESC",
            JobSort::ExperienceAsc => "experience_min ASC, date_added DESC",
        };

        // Query total count
        let count_sql = format!("SELECT COUNT(*) as count FROM jobs {}", where_clause);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);

        // Bind parameters in order: statuses, role_types, source_id, search
        if let Some(statuses) = &filter.statuses {
            for s in statuses {
                let s_str = serde_json::to_string(&s)?;
                count_query = count_query.bind(s_str.trim_matches('"').to_string());
            }
        }
        if let Some(role_types) = &filter.role_types {
            for r in role_types {
                let r_str = serde_json::to_string(&r)?;
                count_query = count_query.bind(r_str.trim_matches('"').to_string());
            }
        }
        if let Some(src_id) = &filter.source_id {
            count_query = count_query.bind(src_id.clone());
        }
        if let Some(search) = &filter.search {
            let term = format!("%{}%", search.trim());
            count_query = count_query
                .bind(term.clone())
                .bind(term.clone())
                .bind(term);
        }

        let (total,) = count_query.fetch_one(pool).await?;

        // Query rows
        let select_sql = format!(
            r#"
            SELECT id, dedupe_key, source_id, external_id, title, company,
                   role_type, location, url, date_posted, date_added,
                   last_seen_at, status, status_changed_at, notes,
                   salary_min, salary_max, salary_currency, salary_interval,
                   salary_raw, salary_source, workplace_type, sponsorship,
                   sponsorship_evidence, clearance, clearance_level,
                   clearance_evidence, experience_min, experience_max,
                   experience_raw, is_new_grad, experience_evidence,
                   description_text, enrichment_version
            FROM jobs
            {}
            ORDER BY {}
            LIMIT ? OFFSET ?
            "#,
            where_clause, order_by
        );

        let mut data_query = sqlx::query(&select_sql);
        if let Some(statuses) = &filter.statuses {
            for s in statuses {
                let s_str = serde_json::to_string(&s)?;
                data_query = data_query.bind(s_str.trim_matches('"').to_string());
            }
        }
        if let Some(role_types) = &filter.role_types {
            for r in role_types {
                let r_str = serde_json::to_string(&r)?;
                data_query = data_query.bind(r_str.trim_matches('"').to_string());
            }
        }
        if let Some(src_id) = &filter.source_id {
            data_query = data_query.bind(src_id.clone());
        }
        if let Some(search) = &filter.search {
            let term = format!("%{}%", search.trim());
            data_query = data_query
                .bind(term.clone())
                .bind(term.clone())
                .bind(term);
        }
        data_query = data_query.bind(limit).bind(offset);

        let rows = data_query.fetch_all(pool).await?;

        use sqlx::Row;
        let job_ids: Vec<String> = rows.iter().map(|r| r.get("id")).collect();
        let locations_map = Self::load_locations_batch(pool, &job_ids).await?;

        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.get("id");
            let locs = locations_map.get(&id).cloned().unwrap_or_default();
            items.push(Self::map_row_to_job(r, locs)?);
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

            let salary_int_str = job
                .salary_interval
                .map(|si| match si {
                    SalaryInterval::Year => "year",
                    SalaryInterval::Month => "month",
                    SalaryInterval::Week => "week",
                    SalaryInterval::Day => "day",
                    SalaryInterval::Hour => "hour",
                });

            let salary_src_str = job
                .salary_source
                .map(|ss| match ss {
                    SalarySource::Structured => "structured",
                    SalarySource::Text => "text",
                });

            let workplace_str = match job.workplace_type {
                WorkplaceType::Remote => "remote",
                WorkplaceType::Hybrid => "hybrid",
                WorkplaceType::Onsite => "onsite",
                WorkplaceType::Unknown => "unknown",
            };

            let spon_str = match job.sponsorship {
                TriState::Yes => "yes",
                TriState::No => "no",
                TriState::Unknown => "unknown",
            };

            let clr_str = match job.clearance {
                TriState::Yes => "yes",
                TriState::No => "no",
                TriState::Unknown => "unknown",
            };

            let clr_lvl_str = job
                .clearance_level
                .map(|cl| match cl {
                    ClearanceLevel::PublicTrust => "public_trust",
                    ClearanceLevel::Confidential => "confidential",
                    ClearanceLevel::Secret => "secret",
                    ClearanceLevel::TopSecret => "top_secret",
                    ClearanceLevel::TsSci => "ts_sci",
                });

            let new_grad_str = match job.is_new_grad {
                TriState::Yes => "yes",
                TriState::No => "no",
                TriState::Unknown => "unknown",
            };

            match existing {
                Some(ex) => {
                    stats.updated += 1;
                    let ex_id: String = ex.get("id");
                    let ex_role_type: String = ex.get("role_type");

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
                            last_seen_at = ?7,
                            salary_min = COALESCE(?8, salary_min),
                            salary_max = COALESCE(?9, salary_max),
                            salary_currency = COALESCE(?10, salary_currency),
                            salary_interval = COALESCE(?11, salary_interval),
                            salary_raw = COALESCE(?12, salary_raw),
                            salary_source = COALESCE(?13, salary_source),
                            workplace_type = CASE WHEN ?14 != 'unknown' THEN ?14 ELSE workplace_type END,
                            sponsorship = CASE WHEN ?15 != 'unknown' THEN ?15 ELSE sponsorship END,
                            sponsorship_evidence = COALESCE(?16, sponsorship_evidence),
                            clearance = CASE WHEN ?17 != 'unknown' THEN ?17 ELSE clearance END,
                            clearance_level = COALESCE(?18, clearance_level),
                            clearance_evidence = COALESCE(?19, clearance_evidence),
                            experience_min = COALESCE(?20, experience_min),
                            experience_max = COALESCE(?21, experience_max),
                            experience_raw = COALESCE(?22, experience_raw),
                            is_new_grad = CASE WHEN ?23 != 'unknown' THEN ?23 ELSE is_new_grad END,
                            experience_evidence = COALESCE(?24, experience_evidence),
                            description_text = COALESCE(?25, description_text)
                        WHERE id = ?26
                        "#,
                    )
                    .bind(&job.title)
                    .bind(&job.company)
                    .bind(new_role_type)
                    .bind(&job.location)
                    .bind(&job.url)
                    .bind(&job.date_posted)
                    .bind(&now)
                    .bind(job.salary_min)
                    .bind(job.salary_max)
                    .bind(&job.salary_currency)
                    .bind(salary_int_str)
                    .bind(&job.salary_raw)
                    .bind(salary_src_str)
                    .bind(workplace_str)
                    .bind(spon_str)
                    .bind(&job.sponsorship_evidence)
                    .bind(clr_str)
                    .bind(clr_lvl_str)
                    .bind(&job.clearance_evidence)
                    .bind(job.experience_min)
                    .bind(job.experience_max)
                    .bind(&job.experience_raw)
                    .bind(new_grad_str)
                    .bind(&job.experience_evidence)
                    .bind(&job.description_text)
                    .bind(&ex_id)
                    .execute(&mut *tx)
                    .await?;

                    // Replace locations child table entries
                    if !job.locations.is_empty() {
                        sqlx::query("DELETE FROM job_locations WHERE job_id = ?")
                            .bind(&ex_id)
                            .execute(&mut *tx)
                            .await?;

                        for loc in &job.locations {
                            sqlx::query(
                                r#"
                                INSERT OR IGNORE INTO job_locations (job_id, raw, city, region_code, country_code, is_remote)
                                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                                "#,
                            )
                            .bind(&ex_id)
                            .bind(&loc.raw)
                            .bind(&loc.city)
                            .bind(&loc.region_code)
                            .bind(&loc.country_code)
                            .bind(if loc.is_remote { 1 } else { 0 })
                            .execute(&mut *tx)
                            .await?;
                        }
                    }
                }
                None => {
                    stats.inserted += 1;
                    let id = Uuid::now_v7().to_string();

                    sqlx::query(
                        r#"
                        INSERT INTO jobs (
                            id, dedupe_key, source_id, external_id, title,
                            company, role_type, location, url, date_posted,
                            date_added, last_seen_at, status, status_changed_at,
                            salary_min, salary_max, salary_currency, salary_interval,
                            salary_raw, salary_source, workplace_type, sponsorship,
                            sponsorship_evidence, clearance, clearance_level,
                            clearance_evidence, experience_min, experience_max,
                            experience_raw, is_new_grad, experience_evidence,
                            description_text, enrichment_version
                        )
                        VALUES (
                            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                            ?11, ?12, 'scraped', ?13, ?14, ?15, ?16, ?17,
                            ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                            ?26, ?27, ?28, ?29, ?30, ?31, ?32
                        )
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
                    .bind(job.salary_min)
                    .bind(job.salary_max)
                    .bind(&job.salary_currency)
                    .bind(salary_int_str)
                    .bind(&job.salary_raw)
                    .bind(salary_src_str)
                    .bind(workplace_str)
                    .bind(spon_str)
                    .bind(&job.sponsorship_evidence)
                    .bind(clr_str)
                    .bind(clr_lvl_str)
                    .bind(&job.clearance_evidence)
                    .bind(job.experience_min)
                    .bind(job.experience_max)
                    .bind(&job.experience_raw)
                    .bind(new_grad_str)
                    .bind(&job.experience_evidence)
                    .bind(&job.description_text)
                    .bind(ENRICHMENT_VERSION)
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

                    // Insert locations
                    for loc in &job.locations {
                        sqlx::query(
                            r#"
                            INSERT OR IGNORE INTO job_locations (job_id, raw, city, region_code, country_code, is_remote)
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                            "#,
                        )
                        .bind(&id)
                        .bind(&loc.raw)
                        .bind(&loc.city)
                        .bind(&loc.region_code)
                        .bind(&loc.country_code)
                        .bind(if loc.is_remote { 1 } else { 0 })
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }

        tx.commit().await?;
        Ok(stats)
    }

    pub async fn reclassify_jobs(pool: &SqlitePool) -> Result<i64> {
        use sqlx::Row;
        let rows = sqlx::query(
            r#"
            SELECT id, title, description_text
            FROM jobs
            WHERE enrichment_version < ?1 AND description_text IS NOT NULL
            "#,
        )
        .bind(ENRICHMENT_VERSION)
        .fetch_all(pool)
        .await?;

        let mut updated = 0i64;
        for r in rows {
            let id: String = r.get("id");
            let title: String = r.get("title");
            let desc: String = r.get("description_text");

            let spon = scrape_core::enrich::classify_sponsorship(&desc);
            let clr = scrape_core::enrich::classify_clearance(&desc);
            let exp = scrape_core::enrich::extract_experience(&title, &desc);
            let sal = scrape_core::enrich::Salary::from_text(&desc);

            let clr_lvl_str = clr.level.map(|cl| match cl {
                ClearanceLevel::PublicTrust => "public_trust",
                ClearanceLevel::Confidential => "confidential",
                ClearanceLevel::Secret => "secret",
                ClearanceLevel::TopSecret => "top_secret",
                ClearanceLevel::TsSci => "ts_sci",
            });

            let spon_str = match spon.value {
                TriState::Yes => "yes",
                TriState::No => "no",
                TriState::Unknown => "unknown",
            };

            let clr_str = match clr.value {
                TriState::Yes => "yes",
                TriState::No => "no",
                TriState::Unknown => "unknown",
            };

            let ng_str = match exp.is_new_grad {
                TriState::Yes => "yes",
                TriState::No => "no",
                TriState::Unknown => "unknown",
            };

            let (sal_min, sal_max, sal_curr, sal_int, sal_raw, sal_src) = if let Some(s) = sal {
                let int_str = match s.interval {
                    SalaryInterval::Year => "year",
                    SalaryInterval::Month => "month",
                    SalaryInterval::Week => "week",
                    SalaryInterval::Day => "day",
                    SalaryInterval::Hour => "hour",
                };
                let src_str = match s.source {
                    SalarySource::Structured => "structured",
                    SalarySource::Text => "text",
                };
                (
                    s.min,
                    s.max,
                    Some(s.currency),
                    Some(int_str),
                    Some(s.raw),
                    Some(src_str),
                )
            } else {
                (None, None, None, None, None, None)
            };

            sqlx::query(
                r#"
                UPDATE jobs
                SET sponsorship = ?1, sponsorship_evidence = ?2,
                    clearance = ?3, clearance_level = ?4, clearance_evidence = ?5,
                    experience_min = ?6, experience_max = ?7, experience_raw = ?8,
                    is_new_grad = ?9, experience_evidence = ?10,
                    salary_min = COALESCE(salary_min, ?11),
                    salary_max = COALESCE(salary_max, ?12),
                    salary_currency = COALESCE(salary_currency, ?13),
                    salary_interval = COALESCE(salary_interval, ?14),
                    salary_raw = COALESCE(salary_raw, ?15),
                    salary_source = COALESCE(salary_source, ?16),
                    enrichment_version = ?17
                WHERE id = ?18
                "#,
            )
            .bind(spon_str)
            .bind(spon.evidence)
            .bind(clr_str)
            .bind(clr_lvl_str)
            .bind(clr.evidence)
            .bind(exp.min_years)
            .bind(exp.max_years)
            .bind(exp.raw)
            .bind(ng_str)
            .bind(exp.evidence)
            .bind(sal_min)
            .bind(sal_max)
            .bind(sal_curr)
            .bind(sal_int)
            .bind(sal_raw)
            .bind(sal_src)
            .bind(ENRICHMENT_VERSION)
            .bind(&id)
            .execute(pool)
            .await?;

            updated += 1;
        }

        Ok(updated)
    }

    pub async fn preview_filter_impact(
        pool: &SqlitePool,
        filter: JobFilter,
    ) -> Result<FilterImpactPreview> {
        let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM jobs")
            .fetch_one(pool)
            .await?;
        let page = Self::list(pool, filter).await?;
        Ok(FilterImpactPreview {
            kept: page.total,
            total,
        })
    }

    pub async fn list_location_options(pool: &SqlitePool) -> Result<LocationOptions> {
        use sqlx::Row;
        let c_rows = sqlx::query(
            "SELECT DISTINCT country_code FROM job_locations WHERE country_code IS NOT NULL ORDER BY country_code ASC",
        )
        .fetch_all(pool)
        .await?;

        let r_rows = sqlx::query(
            "SELECT DISTINCT region_code FROM job_locations WHERE region_code IS NOT NULL ORDER BY region_code ASC",
        )
        .fetch_all(pool)
        .await?;

        let countries = c_rows
            .into_iter()
            .filter_map(|r| r.get::<Option<String>, _>("country_code"))
            .collect();
        let regions = r_rows
            .into_iter()
            .filter_map(|r| r.get::<Option<String>, _>("region_code"))
            .collect();

        Ok(LocationOptions { countries, regions })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[tokio::test]
    async fn test_upsert_and_enrichment_query() {
        let pool = init_memory_db().await.unwrap();

        let batch = vec![
            ScrapedJob {
                dedupe_key: "greenhouse:stripe:101".to_string(),
                source_id: None,
                external_id: Some("101".to_string()),
                title: "Senior Software Engineer".to_string(),
                company: "Stripe".to_string(),
                role_type: RoleType::FullTime,
                location: Some("San Francisco, CA".to_string()),
                url: "https://stripe.com/jobs/101".to_string(),
                date_posted: Some("2026-10-01T00:00:00Z".to_string()),
                salary_min: Some(180_000),
                salary_max: Some(240_000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(SalaryInterval::Year),
                salary_raw: Some("$180k - $240k".to_string()),
                salary_source: Some(SalarySource::Structured),
                workplace_type: WorkplaceType::Hybrid,
                sponsorship: TriState::No,
                sponsorship_evidence: Some("No visa sponsorship".to_string()),
                clearance: TriState::Unknown,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(5),
                experience_max: None,
                experience_raw: Some("5+ years".to_string()),
                is_new_grad: TriState::No,
                experience_evidence: Some("5+ years experience required".to_string()),
                description_text: Some("5+ years experience required. No visa sponsorship.".to_string()),
                locations: vec![ParsedLocation {
                    raw: "San Francisco, CA".to_string(),
                    city: Some("San Francisco".to_string()),
                    region_code: Some("US-CA".to_string()),
                    country_code: Some("US".to_string()),
                    is_remote: false,
                }],
            },
            ScrapedJob {
                dedupe_key: "lever:figma:202".to_string(),
                source_id: None,
                external_id: Some("202".to_string()),
                title: "Software Engineer - New Grad 2026".to_string(),
                company: "Figma".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Remote".to_string()),
                url: "https://figma.com/jobs/202".to_string(),
                date_posted: None,
                salary_min: Some(130_000),
                salary_max: Some(160_000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(SalaryInterval::Year),
                salary_raw: Some("$130,000 - $160,000".to_string()),
                salary_source: Some(SalarySource::Structured),
                workplace_type: WorkplaceType::Remote,
                sponsorship: TriState::Yes,
                sponsorship_evidence: Some("Visa sponsorship available".to_string()),
                clearance: TriState::No,
                clearance_level: None,
                clearance_evidence: Some("No clearance required".to_string()),
                experience_min: Some(0),
                experience_max: Some(1),
                experience_raw: Some("0-1 years".to_string()),
                is_new_grad: TriState::Yes,
                experience_evidence: Some("New Grad 2026".to_string()),
                description_text: Some("New Grad role. Visa sponsorship available.".to_string()),
                locations: vec![ParsedLocation {
                    raw: "Remote".to_string(),
                    city: None,
                    region_code: None,
                    country_code: None,
                    is_remote: true,
                }],
            },
        ];

        let stats = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.inserted, 2);

        // Test list with salary overlap filter: [150k, 200k] should match BOTH
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                salary_min: Some(150_000),
                salary_max: Some(200_000),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 2);

        // Salary [200k, 250k] should only match Stripe
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                salary_min: Some(200_000),
                salary_max: Some(250_000),
                salary_include_unknown: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].company, "Stripe");

        // New grad filter: new_grad_only should only match Figma
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                new_grad: Some("new_grad_only".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].company, "Figma");

        // YOE filter: min 4 should only match Stripe
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                experience_min: Some(4),
                experience_max: Some(10),
                experience_include_unknown: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].company, "Stripe");

        // Sponsorship filter: Yes should only match Figma
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                sponsorship: Some(vec![TriState::Yes]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].company, "Figma");

        // Geo filter: US-CA should only match Stripe
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                regions: Some(vec!["US-CA".to_string()]),
                location_include_unknown: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].company, "Stripe");

        // Remote only should match Figma
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                remote: Some("remote_only".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].company, "Figma");

        // Test location options
        let loc_opts = JobsRepo::list_location_options(&pool).await.unwrap();
        assert_eq!(loc_opts.countries, vec!["US"]);
        assert_eq!(loc_opts.regions, vec!["US-CA"]);

        // Test impact preview
        let preview = JobsRepo::preview_filter_impact(
            &pool,
            JobFilter {
                new_grad: Some("new_grad_only".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(preview.total, 2);
        assert_eq!(preview.kept, 1);
    }

    #[tokio::test]
    async fn test_upsert_scraped_preserves_user_triage_state_and_updates_enrichment() {
        let pool = init_memory_db().await.unwrap();

        // 1. Initial scrape ingestion
        let initial_job = ScrapedJob {
            dedupe_key: "lever:stripe:eng-1".to_string(),
            source_id: None,
            external_id: Some("eng-1".to_string()),
            title: "Staff Data Engineer".to_string(),
            company: "Stripe".to_string(),
            role_type: RoleType::FullTime,
            location: Some("San Francisco, CA".to_string()),
            url: "https://stripe.com/jobs/eng-1".to_string(),
            date_posted: Some("2026-10-01T00:00:00Z".to_string()),
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Onsite,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::No,
            experience_evidence: None,
            description_text: Some("Initial posting text".to_string()),
            locations: vec![ParsedLocation {
                raw: "San Francisco, CA".to_string(),
                city: Some("San Francisco".to_string()),
                region_code: Some("US-CA".to_string()),
                country_code: Some("US".to_string()),
                is_remote: false,
            }],
        };

        let result = JobsRepo::upsert_scraped(&pool, &[initial_job]).await.unwrap();
        assert_eq!(result.inserted, 1);

        // Fetch inserted job
        let jobs = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(jobs.total, 1);
        let job_id = jobs.items[0].id.clone();
        assert_eq!(jobs.items[0].status, JobStatus::Scraped);
        assert_eq!(jobs.items[0].notes, None);

        // 2. User triages job: moves to 'Applied' and adds custom notes
        JobsRepo::update_status(&pool, &job_id, JobStatus::Applied).await.unwrap();
        JobsRepo::update_notes(&pool, &job_id, Some("Referred by Jane Doe")).await.unwrap();

        // Verify user triage is recorded
        let triaged_job = JobsRepo::get(&pool, &job_id).await.unwrap();
        assert_eq!(triaged_job.status, JobStatus::Applied);
        assert_eq!(triaged_job.notes.as_deref(), Some("Referred by Jane Doe"));

        // 3. Re-scrape same job: updated salary and changed location
        let updated_job = ScrapedJob {
            dedupe_key: "lever:stripe:eng-1".to_string(),
            source_id: None,
            external_id: Some("eng-1".to_string()),
            title: "Staff Data Engineer".to_string(),
            company: "Stripe".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Seattle, WA".to_string()),
            url: "https://stripe.com/jobs/eng-1".to_string(),
            date_posted: Some("2026-10-01T00:00:00Z".to_string()),
            salary_min: Some(210_000),
            salary_max: Some(275_000),
            salary_currency: Some("USD".to_string()),
            salary_interval: Some(SalaryInterval::Year),
            salary_raw: Some("$210K - $275K".to_string()),
            salary_source: Some(SalarySource::Structured),
            workplace_type: WorkplaceType::Hybrid,
            sponsorship: TriState::Yes,
            sponsorship_evidence: Some("Sponsors H1B".to_string()),
            clearance: TriState::No,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: Some(8),
            experience_max: None,
            experience_raw: Some("8+ years".to_string()),
            is_new_grad: TriState::No,
            experience_evidence: None,
            description_text: Some("Updated description text".to_string()),
            locations: vec![ParsedLocation {
                raw: "Seattle, WA".to_string(),
                city: Some("Seattle".to_string()),
                region_code: Some("US-WA".to_string()),
                country_code: Some("US".to_string()),
                is_remote: false,
            }],
        };

        let result2 = JobsRepo::upsert_scraped(&pool, &[updated_job]).await.unwrap();
        assert_eq!(result2.inserted, 0);
        assert_eq!(result2.updated, 1);

        // 4. Assert user state is strictly PRESERVED, while enrichment data is UPDATED
        let after_rescrape = JobsRepo::get(&pool, &job_id).await.unwrap();
        assert_eq!(after_rescrape.status, JobStatus::Applied); // Preserved!
        assert_eq!(after_rescrape.notes.as_deref(), Some("Referred by Jane Doe")); // Preserved!
        assert_eq!(after_rescrape.salary_min, Some(210_000)); // Updated!
        assert_eq!(after_rescrape.salary_max, Some(275_000)); // Updated!
        assert_eq!(after_rescrape.workplace_type, WorkplaceType::Hybrid); // Updated!
        assert_eq!(after_rescrape.sponsorship, TriState::Yes); // Updated!
        assert_eq!(after_rescrape.experience_min, Some(8)); // Updated!

        // Assert job_locations child table replaced old locations without orphans
        assert_eq!(after_rescrape.locations.len(), 1);
        assert_eq!(after_rescrape.locations[0].city.as_deref(), Some("Seattle"));
        assert_eq!(after_rescrape.locations[0].region_code.as_deref(), Some("US-WA"));
    }

    #[tokio::test]
    async fn test_reclassify_jobs_extracts_enrichment_from_description() {
        let pool = init_memory_db().await.unwrap();

        // Insert job with no structured enrichment, only plain description text
        let raw_job = ScrapedJob {
            dedupe_key: "gh:unclassified:01".to_string(),
            source_id: None,
            external_id: Some("01".to_string()),
            title: "Security Analyst".to_string(),
            company: "DefenseCo".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Washington, DC".to_string()),
            url: "https://example.com/01".to_string(),
            date_posted: Some("2026-10-05T00:00:00Z".to_string()),
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Unknown,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: Some(
                "Annual compensation: $140,000 to $170,000. \
                 Requires 4+ years of relevant experience. \
                 Must hold an active Secret security clearance. \
                 Unfortunately, we cannot sponsor work visas."
                    .to_string(),
            ),
            locations: vec![],
        };

        JobsRepo::upsert_scraped(&pool, &[raw_job]).await.unwrap();

        // Simulate pre-enrichment historic posting (version = 0)
        let initial = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(initial.items[0].salary_min, None);
        assert_eq!(initial.items[0].sponsorship, TriState::Unknown);
        assert_eq!(initial.items[0].clearance, TriState::Unknown);

        sqlx::query("UPDATE jobs SET enrichment_version = 0 WHERE id = ?")
            .bind(&initial.items[0].id)
            .execute(&pool)
            .await
            .unwrap();

        // Run reclassification engine
        let updated_count = JobsRepo::reclassify_jobs(&pool).await.unwrap();
        assert_eq!(updated_count, 1);

        // Verify natural language enrichment populated columns
        let reclassified = JobsRepo::get(&pool, &initial.items[0].id).await.unwrap();
        assert_eq!(reclassified.salary_min, Some(140_000));
        assert_eq!(reclassified.salary_max, Some(170_000));
        assert_eq!(reclassified.experience_min, Some(4));
        assert_eq!(reclassified.clearance, TriState::Yes);
        assert_eq!(reclassified.sponsorship, TriState::No);
        assert!(reclassified.clearance_evidence.is_some());
        assert!(reclassified.sponsorship_evidence.is_some());
    }

    #[tokio::test]
    async fn test_multi_filter_combination_query() {
        let pool = init_memory_db().await.unwrap();

        // Insert jobs across varied attribute permutations
        let jobs = vec![
            ScrapedJob {
                dedupe_key: "t1".to_string(),
                source_id: None,
                external_id: Some("1".to_string()),
                title: "Frontend Engineer".to_string(),
                company: "Vercel".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Remote".to_string()),
                url: "https://vercel.com/1".to_string(),
                date_posted: Some("2026-10-01".to_string()),
                salary_min: Some(160_000),
                salary_max: Some(200_000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(SalaryInterval::Year),
                salary_raw: None,
                salary_source: None,
                workplace_type: WorkplaceType::Remote,
                sponsorship: TriState::Yes,
                sponsorship_evidence: None,
                clearance: TriState::No,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(2),
                experience_max: Some(5),
                experience_raw: None,
                is_new_grad: TriState::No,
                experience_evidence: None,
                description_text: None,
                locations: vec![ParsedLocation {
                    raw: "Remote, US".to_string(),
                    city: None,
                    region_code: None,
                    country_code: Some("US".to_string()),
                    is_remote: true,
                }],
            },
            ScrapedJob {
                dedupe_key: "t2".to_string(),
                source_id: None,
                external_id: Some("2".to_string()),
                title: "Security Engineer".to_string(),
                company: "Lockheed".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Arlington, VA".to_string()),
                url: "https://lockheed.com/2".to_string(),
                date_posted: Some("2026-10-02".to_string()),
                salary_min: Some(130_000),
                salary_max: Some(170_000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(SalaryInterval::Year),
                salary_raw: None,
                salary_source: None,
                workplace_type: WorkplaceType::Onsite,
                sponsorship: TriState::No,
                sponsorship_evidence: None,
                clearance: TriState::Yes,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(5),
                experience_max: Some(8),
                experience_raw: None,
                is_new_grad: TriState::No,
                experience_evidence: None,
                description_text: None,
                locations: vec![ParsedLocation {
                    raw: "Arlington, VA, US".to_string(),
                    city: Some("Arlington".to_string()),
                    region_code: Some("US-VA".to_string()),
                    country_code: Some("US".to_string()),
                    is_remote: false,
                }],
            },
        ];

        JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();

        // Query with 5 filters combined simultaneously
        let combined_filter = JobFilter {
            salary_min: Some(150_000),
            salary_max: Some(220_000),
            salary_include_unknown: Some(false),
            experience_max: Some(6),
            experience_include_unknown: Some(false),
            sponsorship: Some(vec![TriState::Yes]),
            clearance: Some(vec![TriState::No]),
            remote: Some("remote".to_string()),
            search: Some("Frontend".to_string()),
            ..Default::default()
        };

        let result = JobsRepo::list(&pool, combined_filter).await.unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].company, "Vercel");

        // Mutate one dimension (e.g. require clearance) and ensure empty result without SQL syntax errors
        let non_matching_filter = JobFilter {
            clearance: Some(vec![TriState::Yes]),
            remote: Some("remote".to_string()),
            ..Default::default()
        };
        let result2 = JobsRepo::list(&pool, non_matching_filter).await.unwrap();
        assert_eq!(result2.total, 0);
    }

    #[tokio::test]
    async fn test_location_country_and_cascading_region_filter() {
        let pool = init_memory_db().await.unwrap();

        let job_ca = ScrapedJob {
            dedupe_key: "k1".to_string(),
            source_id: None,
            external_id: None,
            title: "Engineer CA".to_string(),
            company: "Tech Corp".to_string(),
            role_type: RoleType::FullTime,
            location: Some("San Francisco, CA".to_string()),
            url: "https://example.com/ca".to_string(),
            date_posted: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Hybrid,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            locations: vec![ParsedLocation {
                raw: "San Francisco, CA".to_string(),
                city: Some("San Francisco".to_string()),
                region_code: Some("US-CA".to_string()),
                country_code: Some("US".to_string()),
                is_remote: false,
            }],
        };

        let job_on = ScrapedJob {
            dedupe_key: "k2".to_string(),
            source_id: None,
            external_id: None,
            title: "Engineer ON".to_string(),
            company: "Maple Tech".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Toronto, ON".to_string()),
            url: "https://example.com/on".to_string(),
            date_posted: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Hybrid,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            locations: vec![ParsedLocation {
                raw: "Toronto, ON".to_string(),
                city: Some("Toronto".to_string()),
                region_code: Some("CA-ON".to_string()),
                country_code: Some("CA".to_string()),
                is_remote: false,
            }],
        };

        JobsRepo::upsert_scraped(&pool, &[job_ca, job_on]).await.unwrap();

        // 1. Filter by country US
        let us_filter = JobFilter {
            countries: Some(vec!["US".to_string()]),
            location_include_unknown: Some(false),
            ..Default::default()
        };
        let us_res = JobsRepo::list(&pool, us_filter).await.unwrap();
        assert_eq!(us_res.total, 1);
        assert_eq!(us_res.items[0].company, "Tech Corp");

        // 2. Filter by region code suffix "CA" (should match "US-CA")
        let ca_suffix_filter = JobFilter {
            regions: Some(vec!["CA".to_string()]),
            location_include_unknown: Some(false),
            ..Default::default()
        };
        let ca_res = JobsRepo::list(&pool, ca_suffix_filter).await.unwrap();
        assert_eq!(ca_res.total, 1);
        assert_eq!(ca_res.items[0].company, "Tech Corp");

        // 3. Filter by full ISO region code "CA-ON"
        let on_filter = JobFilter {
            regions: Some(vec!["CA-ON".to_string()]),
            location_include_unknown: Some(false),
            ..Default::default()
        };
        let on_res = JobsRepo::list(&pool, on_filter).await.unwrap();
        assert_eq!(on_res.total, 1);
        assert_eq!(on_res.items[0].company, "Maple Tech");
    }
}
