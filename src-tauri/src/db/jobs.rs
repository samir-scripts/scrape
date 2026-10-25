use crate::error::{AppError, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use scrape_core::enrich::{ParsedLocation, ENRICHMENT_VERSION};
use scrape_core::{
    now_iso, ClearanceLevel, JobStatus, RoleType, SalaryInterval, SalarySource, TriState,
    WorkAuthRequirement, WorkplaceType,
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
    #[ts(optional = nullable)]
    pub posting_identity: Option<String>,
    #[ts(optional = nullable)]
    pub removed_at: Option<String>,
    pub source_id: Option<String>,
    pub external_id: Option<String>,
    pub title: String,
    pub company: String,
    pub role_type: RoleType,
    pub location: Option<String>,
    pub url: String,
    pub date_posted: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub provider_updated_at: Option<String>,
    pub date_added: String,
    pub last_seen_at: String,
    pub status: JobStatus,
    pub status_changed_at: String,
    pub notes: Option<String>,
    pub favorited: bool,
    pub favorited_at: Option<String>,

    #[ts(optional = nullable)]
    pub compensation: Option<Vec<scrape_core::enrich::CompensationRange>>,
    // Explicitly named derived annual analytics; native filtering uses compensation.
    #[ts(optional = nullable, type = "number | null")]
    pub salary_annual_min: Option<i64>,
    #[ts(optional = nullable, type = "number | null")]
    pub salary_annual_max: Option<i64>,
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
    pub work_auth: WorkAuthRequirement,
    pub work_auth_evidence: Option<String>,
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
    #[ts(optional = nullable)]
    pub description: Option<scrape_core::enrich::JobDescription>,
    pub company_icon_url: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub company_logo: Option<scrape_core::branding::CompanyLogo>,
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
    StatusChangedAtDesc,
    StatusChangedAtAsc,
    DatePostedDesc,
    DatePostedAsc,
    CompanyAsc,
    CompanyDesc,
    TitleAsc,
    TitleDesc,
    StatusAsc,
    StatusDesc,
    SalaryDesc,
    SalaryAsc,
    ExperienceDesc,
    ExperienceAsc,
    FavoritedDesc,
    FavoritedAsc,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JobFilter {
    #[ts(optional = nullable)]
    pub matching: Option<scrape_core::matching::PostingRules>,
    pub statuses: Option<Vec<JobStatus>>,
    pub role_types: Option<Vec<RoleType>>,
    pub search: Option<String>,
    pub source_id: Option<String>,
    pub sort: Option<JobSort>,
    #[ts(type = "number | null")]
    pub limit: Option<i64>,
    #[ts(type = "number | null")]
    pub offset: Option<i64>,
    pub favorited_only: Option<bool>,

    // Salary bounds
    #[ts(type = "number | null")]
    pub salary_min: Option<f64>,
    #[ts(type = "number | null")]
    pub salary_max: Option<f64>,
    pub salary_currency: Option<String>,
    pub salary_include_unknown: Option<bool>,
    pub salary_interval: Option<SalaryInterval>,

    // Experience bounds
    #[ts(type = "number | null")]
    pub experience_min: Option<i64>,
    #[ts(type = "number | null")]
    pub experience_max: Option<i64>,
    pub experience_include_unknown: Option<bool>,
    #[serde(default)]
    pub experience_match_mode: scrape_core::matching::ExperienceMatchMode,

    // New Grad filter
    pub new_grad: Option<String>, // "any" | "new_grad_only" | "exclude_new_grad"

    // Eligibility tri-state filters
    pub sponsorship: Option<Vec<TriState>>,
    pub work_auth: Option<Vec<WorkAuthRequirement>>,
    pub clearance: Option<Vec<TriState>>,

    // Location filters
    pub countries: Option<Vec<String>>,
    pub regions: Option<Vec<String>>,
    pub cities: Option<Vec<String>>,
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
    #[serde(default)]
    pub location_data_complete: bool,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub provider_updated_at: Option<String>,

    #[serde(default)]
    pub compensation: Vec<scrape_core::enrich::CompensationRange>,
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
    pub work_auth: WorkAuthRequirement,
    pub work_auth_evidence: Option<String>,
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
    #[ts(optional = nullable)]
    pub description: Option<scrape_core::enrich::JobDescription>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub description_source: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub description_format: Option<scrape_core::enrich::DescriptionFormat>,
    pub company_icon_url: Option<String>,
    #[serde(default)]
    #[ts(optional = nullable)]
    pub company_logo: Option<scrape_core::branding::LogoEvidence>,
    #[serde(default)]
    pub locations: Vec<ParsedLocation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub struct UpsertStats {
    #[ts(type = "number")]
    pub blocked: i64,
    pub total: i64,
    pub inserted: i64,
    pub updated: i64,
}

pub struct JobsRepo;

impl JobsRepo {
    pub async fn get(pool: &SqlitePool, id: &str) -> Result<Job> {
        let r = sqlx::query(
            r#"
            SELECT id, dedupe_key, posting_identity, removed_at, compensation_json, source_id, external_id, title, company,
                   role_type, location, url, date_posted, provider_updated_at, date_added,
                   last_seen_at, status, status_changed_at, notes,
                   salary_min, salary_max, salary_currency, salary_interval,
                   salary_raw, salary_source, workplace_type, sponsorship,
                   sponsorship_evidence, clearance, clearance_level,
                   clearance_evidence, experience_min, experience_max,
                   experience_raw, is_new_grad, experience_evidence,
                   description_text, description_json, description_source, company_icon_url, enrichment_version, favorited, favorited_at,
                   (SELECT logo_json FROM employer_branding WHERE key=jobs.company_branding_key) AS logo_json,
                   (SELECT last_attempt_json FROM employer_branding WHERE key=jobs.company_branding_key) AS logo_attempt,
                   work_auth, work_auth_evidence
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

        let work_auth: WorkAuthRequirement = match r
            .try_get::<String, _>("work_auth")
            .unwrap_or_else(|_| "unknown".to_string())
            .as_str()
        {
            "citizen_only" => WorkAuthRequirement::CitizenOnly,
            "us_person" => WorkAuthRequirement::UsPerson,
            "open" => WorkAuthRequirement::Open,
            _ => WorkAuthRequirement::Unknown,
        };
        let work_auth_evidence: Option<String> = r.try_get("work_auth_evidence").unwrap_or(None);

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

        let favorited_int: i64 = r.try_get("favorited").unwrap_or(0);
        let favorited_at: Option<String> = r
            .try_get::<Option<String>, _>("favorited_at")
            .unwrap_or(None)
            .filter(|s| !s.trim().is_empty());

        Ok(Job {
            id: r.get("id"),
            dedupe_key: r.get("dedupe_key"),
            posting_identity: r.get("posting_identity"),
            removed_at: r.get("removed_at"),
            source_id: r.get("source_id"),
            external_id: r.get("external_id"),
            title: r.get("title"),
            company: r.get("company"),
            role_type,
            location: r.get("location"),
            url: r.get("url"),
            provider_updated_at: r.get("provider_updated_at"),
            date_posted: r.get("date_posted"),
            date_added: r.get("date_added"),
            last_seen_at: r.get("last_seen_at"),
            status,
            status_changed_at: r.get("status_changed_at"),
            notes: r.get("notes"),
            favorited: favorited_int == 1,
            favorited_at,
            compensation: Some(serde_json::from_str(
                &r.get::<String, _>("compensation_json"),
            )?),
            salary_annual_min: r.get("salary_min"),
            salary_annual_max: r.get("salary_max"),
            salary_min: r.get("salary_min"),
            salary_max: r.get("salary_max"),
            salary_currency: r.get("salary_currency"),
            salary_interval,
            salary_raw: r.get("salary_raw"),
            salary_source,
            workplace_type,
            sponsorship,
            sponsorship_evidence: r.get("sponsorship_evidence"),
            work_auth,
            work_auth_evidence,
            clearance,
            clearance_level,
            clearance_evidence: r.get("clearance_evidence"),
            experience_min: r.get("experience_min"),
            experience_max: r.get("experience_max"),
            experience_raw: r.get("experience_raw"),
            is_new_grad,
            experience_evidence: r.get("experience_evidence"),
            description_text: r.get("description_text"),
            description: r
                .try_get::<Option<String>, _>("description_json")?
                .map(|json| serde_json::from_str(&json))
                .transpose()?,
            company_icon_url: r.try_get("company_icon_url").unwrap_or(None),
            company_logo: super::branding::from_storage(
                r.try_get("logo_json")?,
                r.try_get("logo_attempt")?,
            )?,
            enrichment_version: r.get("enrichment_version"),
            locations,
        })
    }

    pub async fn list(pool: &SqlitePool, filter: JobFilter) -> Result<Page<Job>> {
        Self::list_at(pool, filter, chrono::Utc::now()).await
    }
    pub async fn list_at(
        pool: &SqlitePool,
        filter: JobFilter,
        started: chrono::DateTime<chrono::Utc>,
    ) -> Result<Page<Job>> {
        super::compensation::backfill(pool).await?;
        let limit = filter.limit.unwrap_or(50).clamp(1, 200);
        let offset = filter.offset.unwrap_or(0).max(0);

        use scrape_core::enrich::salary::CompensationSqlValue as Bind;
        let mut conditions = vec!["removed_at IS NULL".to_string()];
        let mut bindings = Vec::new();
        fn choices<T: Serialize>(
            column: &str,
            options: &[T],
            conditions: &mut Vec<String>,
            bindings: &mut Vec<Bind>,
        ) -> Result<()> {
            if options.is_empty() {
                return Ok(());
            }
            conditions.push(format!(
                "{column} IN ({})",
                vec!["?"; options.len()].join(",")
            ));
            for option in options {
                bindings.push(Bind::Text(
                    serde_json::to_value(option)?.as_str().unwrap().into(),
                ));
            }
            Ok(())
        }
        choices(
            "status",
            filter.statuses.as_deref().unwrap_or(&[]),
            &mut conditions,
            &mut bindings,
        )?;
        if let Some(source) = &filter.source_id {
            conditions.push("(source_id = ? OR workflow_id = ?)".into());
            bindings.push(Bind::Text(source.clone()));
            bindings.push(Bind::Text(source.clone()));
        }
        if filter.favorited_only == Some(true) {
            conditions.push("favorited = 1".into());
        }
        if let Some(search) = filter.search.as_ref().filter(|s| !s.trim().is_empty()) {
            conditions.push(r"(title LIKE ? ESCAPE '\' OR company LIKE ? ESCAPE '\' OR location LIKE ? ESCAPE '\' OR description_text LIKE ? ESCAPE '\' OR notes LIKE ? ESCAPE '\')".into());
            let escaped = search
                .trim()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            for _ in 0..5 {
                bindings.push(Bind::Text(format!("%{escaped}%")));
            }
        }
        let amount = |value: Option<f64>| -> Result<Option<i64>> {
            value
                .map(|v| {
                    if !v.is_finite()
                        || !(0.0..=1_000_000_000.0).contains(&v)
                        || (v * 100.0 - (v * 100.0).round()).abs() > 0.00001
                    {
                        Err(AppError::Validation(
                            "Salary amounts must be nonnegative with at most two decimal places"
                                .into(),
                        ))
                    } else {
                        Ok((v * 100.0).round() as i64)
                    }
                })
                .transpose()
        };
        let pay = scrape_core::enrich::CompensationFilter {
            min_minor: amount(filter.salary_min)?,
            max_minor: amount(filter.salary_max)?,
            currency: filter
                .salary_currency
                .as_ref()
                .map(|c| c.trim().to_ascii_uppercase())
                .filter(|c| !c.is_empty() && c != "ALL"),
            interval: filter.salary_interval,
            include_unknown: filter.salary_include_unknown.unwrap_or(false),
            match_mode: scrape_core::enrich::CompensationMatchMode::RangeOverlap,
        };
        pay.validate().map_err(AppError::Validation)?;

        let compensation = (pay.min_minor.is_some()
            || pay.max_minor.is_some()
            || pay.currency.is_some()
            || pay.interval.is_some())
        .then_some(pay);
        let new_grad = match filter.new_grad.as_deref() {
            None | Some("any") => None,
            Some("only" | "new_grad_only") => Some("only".to_string()),
            Some("exclude" | "exclude_new_grad") => Some("exclude".to_string()),
            Some(other) => {
                return Err(AppError::Validation(format!(
                    "Invalid new-grad filter: {other}"
                )))
            }
        };
        let workplaces = match filter.remote.as_deref() {
            None | Some("all" | "any") => vec![],
            Some("remote" | "remote_only") => vec![WorkplaceType::Remote],
            Some("hybrid" | "hybrid_only") => vec![WorkplaceType::Hybrid],
            Some("onsite" | "onsite_only") => vec![WorkplaceType::Onsite],
            Some("exclude_remote") => vec![
                WorkplaceType::Hybrid,
                WorkplaceType::Onsite,
                WorkplaceType::Unknown,
            ],
            Some(other) => {
                return Err(AppError::Validation(format!(
                    "Invalid workplace filter: {other}"
                )))
            }
        };
        let rules = scrape_core::matching::PostingRules {
            target_roles: vec![],
            companies: vec![],
            keywords: None,
            posting_date: None,
            compensation,
            role_types: filter.role_types.clone().unwrap_or_default(),
            workplaces,
            experience_min: filter.experience_min,
            experience_max: filter.experience_max,
            experience_include_unknown: filter.experience_include_unknown.unwrap_or(false),
            experience_match_mode: filter.experience_match_mode,
            new_grad,
            sponsorship: filter.sponsorship.clone().unwrap_or_default(),
            work_auth: filter.work_auth.clone().unwrap_or_default(),
            clearance: filter.clearance.clone().unwrap_or_default(),
            countries: filter.countries.clone().unwrap_or_default(),
            regions: filter.regions.clone().unwrap_or_default(),
            cities: filter.cities.clone().unwrap_or_default(),
            location_include_unknown: filter.location_include_unknown.unwrap_or(false),
        };
        for rules in [&rules].into_iter().chain(filter.matching.as_ref()) {
            rules.validate().map_err(AppError::Validation)?;
            if let Some(date) = &rules.posting_date {
                let dates: Vec<String> = sqlx::query_scalar("SELECT DISTINCT date_posted FROM jobs WHERE date_posted IS NOT NULL AND date_posted NOT IN (SELECT date_text FROM posting_date_cache WHERE timezone = ?)")
                    .bind(&date.timezone).fetch_all(pool).await?;
                for value in dates {
                    if let Some(instant) =
                        scrape_core::posting_date::posting_instant(&value, &date.timezone)
                    {
                        sqlx::query("INSERT OR IGNORE INTO posting_date_cache (date_text, timezone, instant) VALUES (?, ?, ?)")
                            .bind(value).bind(&date.timezone).bind(instant.to_rfc3339()).execute(pool).await?;
                    }
                }
            }
            let (clauses, values) = rules.sqlite_predicates_at(started);
            conditions.extend(clauses);
            bindings.extend(values);
        }

        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let order_by = match filter.sort.unwrap_or(JobSort::DateAddedDesc) {
            JobSort::DateAddedDesc => "date_added DESC",
            JobSort::DateAddedAsc => "date_added ASC",
            JobSort::StatusChangedAtDesc => "julianday(status_changed_at) DESC, id ASC",
            JobSort::StatusChangedAtAsc => "julianday(status_changed_at) ASC, id ASC",
            JobSort::DatePostedDesc => "date_posted DESC",
            JobSort::DatePostedAsc => "date_posted ASC",
            JobSort::CompanyAsc => "company ASC, date_added DESC",
            JobSort::CompanyDesc => "company DESC, date_added DESC",
            JobSort::TitleAsc => "title ASC, date_added DESC",
            JobSort::TitleDesc => "title DESC, date_added DESC",
            JobSort::StatusAsc => "status ASC, date_added DESC",
            JobSort::StatusDesc => "status DESC, date_added DESC",
            JobSort::SalaryDesc => "salary_max DESC, date_added DESC",
            JobSort::SalaryAsc => "salary_min ASC, date_added DESC",
            JobSort::ExperienceDesc => "experience_min DESC, date_added DESC",
            JobSort::ExperienceAsc => "experience_min ASC, date_added DESC",
            JobSort::FavoritedDesc => "favorited DESC, date_added DESC",
            JobSort::FavoritedAsc => "favorited ASC, date_added DESC",
        };

        // Query total count
        let count_sql = format!("SELECT COUNT(*) as count FROM jobs {}", where_clause);
        let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql);

        for value in &bindings {
            count_query = match value {
                Bind::Integer(value) => count_query.bind(*value),
                Bind::Text(value) => count_query.bind(value),
            };
        }
        let (total,) = count_query.fetch_one(pool).await?;

        // Query rows
        let select_sql = format!(
            r#"
            SELECT id, dedupe_key, posting_identity, removed_at, compensation_json, source_id, external_id, title, company,
                   role_type, location, url, date_posted, provider_updated_at, date_added,
                   last_seen_at, status, status_changed_at, notes,
                   salary_min, salary_max, salary_currency, salary_interval,
                   salary_raw, salary_source, workplace_type, sponsorship,
                   sponsorship_evidence, clearance, clearance_level,
                   clearance_evidence, experience_min, experience_max,
                   experience_raw, is_new_grad, experience_evidence,
                   description_text, description_json, description_source, enrichment_version, favorited, favorited_at,
                   company_icon_url, work_auth, work_auth_evidence,
                   (SELECT logo_json FROM employer_branding WHERE key=jobs.company_branding_key) AS logo_json,
                   (SELECT last_attempt_json FROM employer_branding WHERE key=jobs.company_branding_key) AS logo_attempt
            FROM jobs
            {}
            ORDER BY {}, id ASC
            LIMIT ? OFFSET ?
            "#,
            where_clause, order_by
        );

        let mut data_query = sqlx::query(&select_sql);
        for value in &bindings {
            data_query = match value {
                Bind::Integer(value) => data_query.bind(*value),
                Bind::Text(value) => data_query.bind(value),
            };
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

    pub async fn toggle_favorite(pool: &SqlitePool, id: &str) -> Result<Job> {
        use sqlx::Row;
        let current = sqlx::query("SELECT favorited FROM jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Job {} not found", id)))?;

        let current_fav: i64 = current.get("favorited");
        let new_fav = if current_fav == 1 { 0 } else { 1 };
        let now = now_iso();
        let favorited_at = if new_fav == 1 { Some(now) } else { None };

        sqlx::query(
            r#"
            UPDATE jobs
            SET favorited = ?1, favorited_at = ?2
            WHERE id = ?3
            "#,
        )
        .bind(new_fav)
        .bind(&favorited_at)
        .bind(id)
        .execute(pool)
        .await?;

        Self::get(pool, id).await
    }

    pub async fn set_favorite(pool: &SqlitePool, id: &str, favorited: bool) -> Result<Job> {
        let fav_int = if favorited { 1 } else { 0 };
        let now = now_iso();
        let favorited_at = if favorited { Some(now) } else { None };

        let rows = sqlx::query(
            r#"
            UPDATE jobs
            SET favorited = ?1, favorited_at = ?2
            WHERE id = ?3
            "#,
        )
        .bind(fav_int)
        .bind(&favorited_at)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(AppError::NotFound(format!("Job {} not found", id)));
        }

        Self::get(pool, id).await
    }

    /// Compatibility entry point: ordinary removal does not block discovery.
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool> {
        match super::postings::remove(pool, id, false).await {
            Ok(result) => Ok(result.copies > 0),
            Err(AppError::NotFound(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub async fn prune_expired_by_source(
        pool: &SqlitePool,
        source_id: &str,
        retention_days: i64,
        policy: &str,
    ) -> Result<u64> {
        Self::prune_expired_by_source_at(pool, source_id, retention_days, policy, Utc::now()).await
    }

    pub async fn prune_expired_by_source_at(
        pool: &SqlitePool,
        source_id: &str,
        retention_days: i64,
        policy: &str,
        now: DateTime<Utc>,
    ) -> Result<u64> {
        if retention_days <= 0 {
            return Ok(0); // 0 = keep forever
        }
        let age = Duration::try_days(retention_days).ok_or_else(|| {
            AppError::Validation("Retention days exceed the supported date range.".into())
        })?;
        let cutoff = now
            .checked_sub_signed(age)
            .ok_or_else(|| {
                AppError::Validation("Retention cutoff exceeds the supported date range.".into())
            })?
            .to_rfc3339_opts(SecondsFormat::Millis, true);

        // Legacy "all" is intentionally as safe as "untracked_only".
        // Unknown policies fail closed instead of deleting anything.
        if !matches!(policy, "all" | "untracked_only") {
            return Err(AppError::Validation(format!(
                "Unknown retention policy: {policy}"
            )));
        }
        let query_str = r#"
            DELETE FROM jobs
            WHERE (workflow_id = ?1 OR (workflow_id IS NULL AND source_id = ?1))
              AND removed_at IS NULL
              AND status IN ('scraped', 'viewed', 'ignored')
              AND favorited = 0
              AND (notes IS NULL OR trim(notes, char(9) || char(10) || char(13) || ' ') = '')
              AND julianday(date_added) <= julianday(?2)
            "#;

        let result = sqlx::query(query_str)
            .bind(source_id)
            .bind(&cutoff)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
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

        sqlx::query("UPDATE jobs SET removed_at = removed_at WHERE 0")
            .execute(&mut *tx)
            .await?;
        for job in batch {
            if let Some(evidence) = &job.company_logo {
                evidence.validate().map_err(AppError::Validation)?;
            }
            stats.total += 1;

            let identity = super::postings::identity_in_transaction(
                &mut tx,
                job.source_id.as_deref(),
                job.external_id.as_deref(),
                &job.url,
            )
            .await?;
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM blocked_postings WHERE identity = ? OR (identity LIKE 'legacy:%' AND legacy_key = ?))")
                .bind(&identity).bind(&job.dedupe_key).fetch_one(&mut *tx).await?;
            if blocked {
                stats.blocked += 1;
                continue;
            }

            use sqlx::Row;
            // A workflow can be repointed at another board. A provider ID reused
            // there must not overwrite annotations belonging to the old posting.
            let existing = sqlx::query("SELECT id, role_type, dedupe_key FROM jobs WHERE (posting_identity = ? AND source_id = ?) OR (dedupe_key = ? AND (posting_identity IS NULL OR posting_identity = ?)) ORDER BY CASE WHEN dedupe_key = ? THEN 0 ELSE 1 END, id LIMIT 1")
                .bind(&identity).bind(&job.source_id).bind(&job.dedupe_key).bind(&identity).bind(&job.dedupe_key).fetch_optional(&mut *tx).await?;
            let dedupe_key = if let Some(row) = &existing {
                row.get::<String, _>("dedupe_key")
            } else {
                let occupied: bool =
                    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE dedupe_key=?)")
                        .bind(&job.dedupe_key)
                        .fetch_one(&mut *tx)
                        .await?;
                if occupied {
                    format!("{}:posting:{identity}", job.dedupe_key)
                } else {
                    job.dedupe_key.clone()
                }
            };

            let role_type_str = match serde_json::to_value(job.role_type)? {
                serde_json::Value::String(s) => s,
                _ => unreachable!(),
            };

            let salary_int_str = job.salary_interval.map(|si| match si {
                SalaryInterval::Year => "year",
                SalaryInterval::Month => "month",
                SalaryInterval::Week => "week",
                SalaryInterval::Day => "day",
                SalaryInterval::Hour => "hour",
            });

            let salary_src_str = job.salary_source.map(|ss| match ss {
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

            let clr_lvl_str = job.clearance_level.map(|cl| match cl {
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

            let work_auth_str = match job.work_auth {
                WorkAuthRequirement::CitizenOnly => "citizen_only",
                WorkAuthRequirement::UsPerson => "us_person",
                WorkAuthRequirement::Open => "open",
                WorkAuthRequirement::Unknown => "unknown",
            };

            match existing {
                Some(ex) => {
                    stats.updated += 1;
                    let ex_id: String = ex.get("id");
                    let ex_role_type: String = ex.get("role_type");

                    let should_update_role = job.role_type != RoleType::Unknown;
                    let new_role_type = if should_update_role {
                        role_type_str
                    } else {
                        ex_role_type
                    };

                    sqlx::query(
                        r#"
                        UPDATE jobs
                        SET workflow_id = COALESCE(workflow_id, (SELECT id FROM workflows WHERE id = (SELECT workflow_id FROM sources WHERE id = jobs.source_id) OR id = jobs.source_id)),
                            title = ?1, company = ?2, role_type = ?3,
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
                            description_text = COALESCE(?25, description_text),
                            description_json = COALESCE(?26, description_json),
                            description_source = COALESCE(?27, description_source),
                            description_format = COALESCE(?28, description_format),
                            company_icon_url = COALESCE(?29, company_icon_url),
                            work_auth = CASE WHEN ?30 != 'unknown' THEN ?30 ELSE work_auth END,
                            work_auth_evidence = COALESCE(?31, work_auth_evidence)
                        WHERE id = ?32
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
                    .bind(job.description.as_ref().map(serde_json::to_string).transpose()?)
                    .bind(&job.description_source)
                    .bind(job.description_format.map(|format| match format {
                        scrape_core::enrich::DescriptionFormat::Html => "html",
                        scrape_core::enrich::DescriptionFormat::PlainText => "plain_text",
                        scrape_core::enrich::DescriptionFormat::Mixed => "mixed",
                    }))
                    .bind(&job.company_icon_url)
                    .bind(work_auth_str)
                    .bind(&job.work_auth_evidence)
                    .bind(&ex_id)
                    .execute(&mut *tx)
                    .await?;

                    // Replace locations child table entries
                    if job.location_data_complete {
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
                            id, dedupe_key, source_id, workflow_id, external_id, title,
                            company, role_type, location, url, date_posted,
                            date_added, last_seen_at, status, status_changed_at,
                            salary_min, salary_max, salary_currency, salary_interval,
                            salary_raw, salary_source, workplace_type, sponsorship,
                            sponsorship_evidence, clearance, clearance_level,
                            clearance_evidence, experience_min, experience_max,
                            experience_raw, is_new_grad, experience_evidence,
                            description_text, description_json, description_source, description_format, enrichment_version, company_icon_url,
                            work_auth, work_auth_evidence
                        )
                        VALUES (
                            ?1, ?2, ?3, (SELECT id FROM workflows WHERE id = (SELECT workflow_id FROM sources WHERE id = ?3) OR id = ?3), ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                            ?11, ?12, 'scraped', ?13, ?14, ?15, ?16, ?17,
                            ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                            ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33,
                            ?34, ?35, ?36, ?37, ?38
                        )
                        "#,
                    )
                    .bind(&id)
                    .bind(&dedupe_key)
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
                    .bind(job.description.as_ref().map(serde_json::to_string).transpose()?)
                    .bind(&job.description_source)
                    .bind(job.description_format.map(|format| match format {
                        scrape_core::enrich::DescriptionFormat::Html => "html",
                        scrape_core::enrich::DescriptionFormat::PlainText => "plain_text",
                        scrape_core::enrich::DescriptionFormat::Mixed => "mixed",
                    }))
                    .bind(ENRICHMENT_VERSION)
                    .bind(&job.company_icon_url)
                    .bind(work_auth_str)
                    .bind(&job.work_auth_evidence)
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

                    // Insert locations only when the source exposed complete
                    // eligibility data; an empty vector then means known empty.
                    if job.location_data_complete {
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
            sqlx::query(
                "UPDATE jobs SET posting_identity = ?, provider_updated_at = ?, compensation_json = ?, compensation_migrated = 1, removed_at = NULL, source_name_snapshot = COALESCE(source_name_snapshot, (SELECT name FROM sources WHERE id = jobs.source_id)), company_branding_key = ? WHERE dedupe_key = ?",
            )
            .bind(&identity)
            .bind(&job.provider_updated_at)
            .bind(serde_json::to_string(&job.compensation)?)
            .bind(job.source_id.as_deref().map(|id|scrape_core::branding::relationship_key(id,&job.company)))
            .bind(&dedupe_key)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(stats)
    }

    /// Finish saved-data repairs before scheduling starts. Each committed batch is
    /// resumable; one consistent backup precedes all mutations in this invocation.
    pub async fn reclassify_jobs(pool: &SqlitePool) -> Result<i64> {
        use sqlx::Row;
        let stale: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM jobs WHERE enrichment_version < ?)")
                .bind(ENRICHMENT_VERSION)
                .fetch_one(pool)
                .await?;
        let links = super::branding::pending_job_links(pool).await?;
        if !stale && links.is_empty() {
            return Ok(0);
        }

        let database = sqlx::query("PRAGMA database_list").fetch_one(pool).await?;
        let database_file: String = database.get("file");
        if !database_file.is_empty() {
            let database_path = std::path::PathBuf::from(database_file);
            let backup_path = database_path.with_file_name(format!(
                "{}.pre-description-repair-{}.db",
                database_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                Uuid::now_v7()
            ));
            super::backup_to(pool, &backup_path).await?;
        }

        if !links.is_empty() {
            let mut tx = pool.begin().await?;
            for (source_id, company, key) in links {
                super::branding::attach_jobs(&mut tx, &source_id, &company, &key).await?;
            }
            tx.commit().await?;
        }

        let mut updated = 0i64;
        loop {
            let mut tx = pool.begin().await?;
            let rows = sqlx::query(
                r#"
                SELECT id, title, description_text, description_source, description_format,
                       enrichment_version
                FROM jobs
                WHERE enrichment_version < ?1
                ORDER BY id
                LIMIT 200
                "#,
            )
            .bind(ENRICHMENT_VERSION)
            .fetch_all(&mut *tx)
            .await?;
            if rows.is_empty() {
                tx.rollback().await?;
                break;
            }
            let batch_len = rows.len() as i64;
            for r in rows {
                let id: String = r.get("id");
                let title: String = r.get("title");
                let stored_desc: Option<String> = r.get("description_text");
                let version: i64 = r.get("enrichment_version");

                // Version 3 already repaired descriptions and unrelated enrichment.
                // Missing descriptions, including older versions, need title-only
                // experience classification without filling unrelated metadata.
                if version >= 3 || stored_desc.is_none() {
                    let exp = scrape_core::enrich::extract_experience(
                        &title,
                        stored_desc.as_deref().unwrap_or_default(),
                    );
                    let ng_str = match exp.is_new_grad {
                        TriState::Yes => "yes",
                        TriState::No => "no",
                        TriState::Unknown => "unknown",
                    };
                    sqlx::query(
                        "UPDATE jobs SET experience_min=?1,experience_max=?2,experience_raw=?3,is_new_grad=?4,experience_evidence=?5,enrichment_version=?6 WHERE id=?7",
                    )
                    .bind(exp.min_years)
                    .bind(exp.max_years)
                    .bind(exp.raw)
                    .bind(ng_str)
                    .bind(exp.evidence)
                    .bind(ENRICHMENT_VERSION)
                    .bind(&id)
                    .execute(&mut *tx)
                    .await?;
                    continue;
                }

                // Preserve the historical pre-v3 description/enrichment repair.
                // Employment type is never inferred here: its original ATS hint
                // is unavailable, so an existing known value must survive.
                let stored_desc = stored_desc.as_deref().unwrap_or_default();
                let source: Option<String> = r.get("description_source");
                let stored_format: Option<String> = r.get("description_format");
                let repaired_input = source.as_deref().unwrap_or(stored_desc);
                let repaired = match stored_format.as_deref() {
                    Some("plain_text") => scrape_core::enrich::normalize_description_with_format(
                        repaired_input,
                        scrape_core::enrich::DescriptionFormat::PlainText,
                        32768,
                    ),
                    Some("mixed") => scrape_core::enrich::normalize_description_with_format(
                        repaired_input,
                        scrape_core::enrich::DescriptionFormat::Mixed,
                        32768,
                    ),
                    _ => scrape_core::enrich::normalize_description(repaired_input, 32768),
                };
                let desc = repaired.plain_text.as_str();
                let spon = scrape_core::enrich::classify_sponsorship(desc);
                let clr = scrape_core::enrich::classify_clearance(desc);
                let wa = scrape_core::enrich::classify_work_auth(desc);
                let exp = scrape_core::enrich::extract_experience(&title, desc);
                let sal = scrape_core::enrich::Salary::from_text(desc);
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
                let wa_str = match wa.value {
                    WorkAuthRequirement::CitizenOnly => "citizen_only",
                    WorkAuthRequirement::UsPerson => "us_person",
                    WorkAuthRequirement::Open => "open",
                    WorkAuthRequirement::Unknown => "unknown",
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
                let compensation = sal.as_ref().map(|s| s.ranges.clone()).unwrap_or_default();
                let (sal_min, sal_max, sal_curr, sal_int, sal_raw, sal_src) = if let Some(s) = sal {
                    let int_str =
                        s.ranges
                            .first()
                            .and_then(|range| range.interval)
                            .map(|interval| match interval {
                                SalaryInterval::Year => "year",
                                SalaryInterval::Month => "month",
                                SalaryInterval::Week => "week",
                                SalaryInterval::Day => "day",
                                SalaryInterval::Hour => "hour",
                            });
                    let src_str = match s.source {
                        SalarySource::Structured => "structured",
                        SalarySource::Text => "text",
                    };
                    (
                        s.min,
                        s.max,
                        Some(s.currency),
                        int_str,
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
                        work_auth = ?17, work_auth_evidence = ?18,
                        enrichment_version = ?19,
                        compensation_json = CASE WHEN compensation_json = '[]' THEN ?21 ELSE compensation_json END,
                        description_text = ?22, description_json = ?23,
                        compensation_migrated = 1
                    WHERE id = ?20
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
                .bind(wa_str)
                .bind(wa.evidence)
                .bind(ENRICHMENT_VERSION)
                .bind(&id)
                .bind(serde_json::to_string(&compensation)?)
                .bind(desc)
                .bind(serde_json::to_string(&repaired)?)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            updated += batch_len;
            tokio::task::yield_now().await;
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
    async fn stage_transitions_are_atomic_noops_do_not_add_history_and_state_survives_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stage-fixture.db");
        let pool = crate::db::init_db(&path).await.unwrap();
        sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited) VALUES ('stage-job','stage-job','Stage fixture','Fixture Co','full_time','https://fixture.example/job','2026-01-01','2026-01-01','applied','2026-01-01','Keep note',1)")
            .execute(&pool).await.unwrap();
        JobsRepo::update_status(&pool, "stage-job", JobStatus::Applied)
            .await
            .unwrap();
        assert!(JobsRepo::get_status_events(&pool, "stage-job")
            .await
            .unwrap()
            .is_empty());
        JobsRepo::update_status(&pool, "stage-job", JobStatus::Interviewing)
            .await
            .unwrap();
        JobsRepo::update_status(&pool, "stage-job", JobStatus::Interviewing)
            .await
            .unwrap();
        assert_eq!(
            JobsRepo::get_status_events(&pool, "stage-job")
                .await
                .unwrap()
                .len(),
            1
        );
        // Simulate a real history-persistence failure after the UPDATE. The whole
        // transition must roll back, including status_changed_at and annotations.
        sqlx::query("CREATE TRIGGER reject_history BEFORE INSERT ON job_status_events BEGIN SELECT RAISE(ABORT, 'fixture disk failure'); END")
            .execute(&pool).await.unwrap();
        let before = JobsRepo::get(&pool, "stage-job").await.unwrap();
        assert!(
            JobsRepo::update_status(&pool, "stage-job", JobStatus::Offer)
                .await
                .is_err()
        );
        let after = JobsRepo::get(&pool, "stage-job").await.unwrap();
        assert_eq!(after.status, before.status);
        assert_eq!(after.status_changed_at, before.status_changed_at);
        assert_eq!(
            JobsRepo::get_status_events(&pool, "stage-job")
                .await
                .unwrap()
                .len(),
            1
        );
        sqlx::query("DROP TRIGGER reject_history")
            .execute(&pool)
            .await
            .unwrap();
        JobsRepo::update_status(&pool, "stage-job", JobStatus::Offer)
            .await
            .unwrap();
        pool.close().await;
        let reopened = crate::db::init_db(&path).await.unwrap();
        let saved = JobsRepo::get(&reopened, "stage-job").await.unwrap();
        assert_eq!(saved.status, JobStatus::Offer);
        assert_eq!(saved.notes.as_deref(), Some("Keep note"));
        assert!(saved.favorited);
        assert_eq!(
            JobsRepo::get_status_events(&reopened, "stage-job")
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn repointing_a_workflow_never_transfers_saved_state_to_a_reused_provider_id() {
        let pool = init_memory_db().await.unwrap();
        let source = crate::db::sources::SourcesRepo::create(
            &pool,
            crate::db::sources::CreateSourceInput {
                kind: scrape_core::SourceKind::Greenhouse,
                name: "Scope fixture".into(),
                identifier: "acme".into(),
                config: None,
                enabled: Some(true),
                cadence: Some(scrape_core::SourceCadence::Manual),
                cadence_cron: None,
                retention_days: Some(30),
                retention_policy: None,
                filters: None,
            },
        )
        .await
        .unwrap();
        let posting = |title: &str, board: &str, role: &str| {
            crate::services::ingestion::scraped_job_from_event(serde_json::from_value(serde_json::json!({"type":"job","source_id":source.id,"external_id":"42","title":title,"company":board,"role_type":role,"url":format!("https://boards.greenhouse.io/{board}/jobs/42")})).unwrap()).unwrap()
        };
        let acme = posting("Acme Engineer", "acme", "full_time");
        JobsRepo::upsert_scraped(&pool, std::slice::from_ref(&acme))
            .await
            .unwrap();
        let original = JobsRepo::list(&pool, JobFilter::default())
            .await
            .unwrap()
            .items
            .remove(0);
        JobsRepo::update_notes(&pool, &original.id, "Acme referral".into())
            .await
            .unwrap();
        JobsRepo::update_status(&pool, &original.id, JobStatus::Applied)
            .await
            .unwrap();
        JobsRepo::set_favorite(&pool, &original.id, true)
            .await
            .unwrap();
        sqlx::query("UPDATE sources SET identifier='other' WHERE id=?")
            .bind(&source.id)
            .execute(&pool)
            .await
            .unwrap();
        let other = posting("Other Contractor", "other", "contract");
        assert_eq!(
            JobsRepo::upsert_scraped(&pool, std::slice::from_ref(&other))
                .await
                .unwrap()
                .inserted,
            1
        );
        let saved = JobsRepo::get(&pool, &original.id).await.unwrap();
        assert_eq!(saved.title, "Acme Engineer");
        assert_eq!(saved.notes.as_deref(), Some("Acme referral"));
        assert_eq!(saved.status, JobStatus::Applied);
        assert!(saved.favorited);
        let page = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(page.total, 2);
        assert!(page.items.iter().any(|job| job.title == "Other Contractor"
            && job.notes.is_none()
            && job.status == JobStatus::Scraped));
        assert_eq!(
            JobsRepo::upsert_scraped(&pool, &[other])
                .await
                .unwrap()
                .updated,
            1
        );
        sqlx::query("UPDATE sources SET identifier='acme' WHERE id=?")
            .bind(&source.id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            JobsRepo::upsert_scraped(&pool, &[acme])
                .await
                .unwrap()
                .updated,
            1
        );
        assert_eq!(
            JobsRepo::list(&pool, JobFilter::default())
                .await
                .unwrap()
                .total,
            2
        );
        let refreshed = posting("Acme Contract Engineer", "acme", "contract");
        JobsRepo::upsert_scraped(&pool, &[refreshed]).await.unwrap();
        let saved = JobsRepo::get(&pool, &original.id).await.unwrap();
        assert_eq!(saved.role_type, RoleType::Contract);
        assert_eq!(saved.notes.as_deref(), Some("Acme referral"));
    }

    #[tokio::test]
    async fn test_upsert_and_enrichment_query() {
        let pool = init_memory_db().await.unwrap();

        let batch = vec![
            ScrapedJob {
                compensation: scrape_core::enrich::Salary::from_structured(
                    Some(180_000.0),
                    Some(240_000.0),
                    Some("USD"),
                    Some("YEAR"),
                    Some("Fixture provider annual range"),
                )
                .unwrap()
                .ranges,
                dedupe_key: "greenhouse:stripe:101".to_string(),
                source_id: None,
                external_id: Some("101".to_string()),
                title: "Senior Software Engineer".to_string(),
                company: "Stripe".to_string(),
                role_type: RoleType::FullTime,
                location: Some("San Francisco, CA".to_string()),
                url: "https://stripe.com/jobs/101".to_string(),
                provider_updated_at: None,
                location_data_complete: true,
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
                work_auth: WorkAuthRequirement::UsPerson,
                work_auth_evidence: Some(
                    "Must be a US Person (US Citizen or Green Card holder)".to_string(),
                ),
                clearance: TriState::Unknown,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(5),
                experience_max: None,
                experience_raw: Some("5+ years".to_string()),
                is_new_grad: TriState::No,
                experience_evidence: Some("5+ years experience required".to_string()),
                description_text: Some(
                    "5+ years experience required. No visa sponsorship.".to_string(),
                ),
                description: None,
                description_source: None,
                description_format: None,
                company_icon_url: None,
                company_logo: None,
                locations: vec![ParsedLocation {
                    raw: "San Francisco, CA".to_string(),
                    city: Some("San Francisco".to_string()),
                    region_code: Some("US-CA".to_string()),
                    country_code: Some("US".to_string()),
                    is_remote: false,
                }],
            },
            ScrapedJob {
                compensation: scrape_core::enrich::Salary::from_structured(
                    Some(130_000.0),
                    Some(160_000.0),
                    Some("USD"),
                    Some("YEAR"),
                    Some("Fixture provider annual range"),
                )
                .unwrap()
                .ranges,
                dedupe_key: "lever:figma:202".to_string(),
                source_id: None,
                external_id: Some("202".to_string()),
                title: "Software Engineer - New Grad 2026".to_string(),
                company: "Figma".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Remote".to_string()),
                url: "https://figma.com/jobs/202".to_string(),
                provider_updated_at: None,
                location_data_complete: true,
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
                work_auth: WorkAuthRequirement::Open,
                work_auth_evidence: Some("Open to all candidates authorized to work".to_string()),
                clearance: TriState::No,
                clearance_level: None,
                clearance_evidence: Some("No clearance required".to_string()),
                experience_min: Some(0),
                experience_max: Some(1),
                experience_raw: Some("0-1 years".to_string()),
                is_new_grad: TriState::Yes,
                experience_evidence: Some("New Grad 2026".to_string()),
                description_text: Some("New Grad role. Visa sponsorship available.".to_string()),
                description: None,
                description_source: None,
                description_format: None,
                company_icon_url: None,
                company_logo: None,
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

        let jobs = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(jobs.total, 2);
        let j1 = jobs.items.iter().find(|j| j.company == "Stripe").unwrap();
        assert_eq!(j1.work_auth, WorkAuthRequirement::UsPerson);
        assert!(j1.work_auth_evidence.is_some());
        let j2 = jobs.items.iter().find(|j| j.company == "Figma").unwrap();
        assert_eq!(j2.work_auth, WorkAuthRequirement::Open);

        // Test list with salary overlap filter: [150k, 200k] should match BOTH
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                salary_min: Some(150_000.0),
                salary_max: Some(200_000.0),
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
                matching: None,
                salary_min: Some(200_000.0),
                salary_max: Some(250_000.0),
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
                matching: None,
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
                matching: None,
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
                matching: None,
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
                matching: None,
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
                matching: None,
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
                matching: None,
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
            compensation: vec![],
            dedupe_key: "lever:stripe:eng-1".to_string(),
            source_id: None,
            external_id: Some("eng-1".to_string()),
            title: "Staff Data Engineer".to_string(),
            company: "Stripe".to_string(),
            role_type: RoleType::FullTime,
            location: Some("San Francisco, CA".to_string()),
            url: "https://stripe.com/jobs/eng-1".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
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
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::No,
            experience_evidence: None,
            description_text: Some("Initial posting text".to_string()),
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![ParsedLocation {
                raw: "San Francisco, CA".to_string(),
                city: Some("San Francisco".to_string()),
                region_code: Some("US-CA".to_string()),
                country_code: Some("US".to_string()),
                is_remote: false,
            }],
        };

        let result = JobsRepo::upsert_scraped(&pool, &[initial_job])
            .await
            .unwrap();
        assert_eq!(result.inserted, 1);

        // Fetch inserted job
        let jobs = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(jobs.total, 1);
        let job_id = jobs.items[0].id.clone();
        assert_eq!(jobs.items[0].status, JobStatus::Scraped);
        assert_eq!(jobs.items[0].notes, None);

        // 2. User triages job: moves to 'Applied' and adds custom notes
        JobsRepo::update_status(&pool, &job_id, JobStatus::Applied)
            .await
            .unwrap();
        JobsRepo::update_notes(&pool, &job_id, Some("Referred by Jane Doe"))
            .await
            .unwrap();

        // Verify user triage is recorded
        let triaged_job = JobsRepo::get(&pool, &job_id).await.unwrap();
        assert_eq!(triaged_job.status, JobStatus::Applied);
        assert_eq!(triaged_job.notes.as_deref(), Some("Referred by Jane Doe"));

        // 3. Re-scrape same job: updated salary and changed location
        let updated_job = ScrapedJob {
            compensation: vec![],
            dedupe_key: "lever:stripe:eng-1".to_string(),
            source_id: None,
            external_id: Some("eng-1".to_string()),
            title: "Staff Data Engineer".to_string(),
            company: "Stripe".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Seattle, WA".to_string()),
            url: "https://stripe.com/jobs/eng-1".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
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
            work_auth: WorkAuthRequirement::UsPerson,
            work_auth_evidence: Some("US Person required".to_string()),
            clearance: TriState::No,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: Some(8),
            experience_max: None,
            experience_raw: Some("8+ years".to_string()),
            is_new_grad: TriState::No,
            experience_evidence: None,
            description_text: Some("Updated description text".to_string()),
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![ParsedLocation {
                raw: "Seattle, WA".to_string(),
                city: Some("Seattle".to_string()),
                region_code: Some("US-WA".to_string()),
                country_code: Some("US".to_string()),
                is_remote: false,
            }],
        };

        // A partial refresh that reports only Remote does not mean known
        // eligible geography became empty; retain the last explicit location.
        let mut incomplete_job = updated_job.clone();
        incomplete_job.location_data_complete = false;
        incomplete_job.location = Some("Remote".into());
        incomplete_job.workplace_type = WorkplaceType::Remote;
        incomplete_job.locations = vec![ParsedLocation {
            raw: "Remote".into(),
            city: None,
            region_code: None,
            country_code: None,
            is_remote: true,
        }];
        JobsRepo::upsert_scraped(&pool, &[incomplete_job])
            .await
            .unwrap();
        let after_partial = JobsRepo::get(&pool, &job_id).await.unwrap();
        assert_eq!(after_partial.locations.len(), 1);
        assert_eq!(
            after_partial.locations[0].city.as_deref(),
            Some("San Francisco")
        );

        let result2 = JobsRepo::upsert_scraped(&pool, std::slice::from_ref(&updated_job))
            .await
            .unwrap();
        assert_eq!(result2.inserted, 0);
        assert_eq!(result2.updated, 1);

        // 4. Assert user state is strictly PRESERVED, while enrichment data is UPDATED
        let after_rescrape = JobsRepo::get(&pool, &job_id).await.unwrap();
        assert_eq!(after_rescrape.status, JobStatus::Applied); // Preserved!
        assert_eq!(
            after_rescrape.notes.as_deref(),
            Some("Referred by Jane Doe")
        ); // Preserved!
        assert_eq!(after_rescrape.salary_min, Some(210_000)); // Updated!
        assert_eq!(after_rescrape.salary_max, Some(275_000)); // Updated!
        assert_eq!(after_rescrape.workplace_type, WorkplaceType::Hybrid); // Updated!
        assert_eq!(after_rescrape.sponsorship, TriState::Yes); // Updated!
        assert_eq!(after_rescrape.experience_min, Some(8)); // Updated!

        // Assert job_locations child table replaced old locations without orphans
        assert_eq!(after_rescrape.locations.len(), 1);
        assert_eq!(after_rescrape.locations[0].city.as_deref(), Some("Seattle"));
        assert_eq!(
            after_rescrape.locations[0].region_code.as_deref(),
            Some("US-WA")
        );

        // An explicitly complete empty result is different from an
        // incomplete refresh and can clear stale eligible locations.
        let mut complete_empty = updated_job.clone();
        complete_empty.location_data_complete = true;
        complete_empty.locations.clear();
        JobsRepo::upsert_scraped(&pool, &[complete_empty])
            .await
            .unwrap();
        assert!(JobsRepo::get(&pool, &job_id)
            .await
            .unwrap()
            .locations
            .is_empty());
    }

    #[tokio::test]
    async fn test_reclassify_jobs_extracts_enrichment_from_description() {
        let pool = init_memory_db().await.unwrap();

        // Insert job with no structured enrichment, only plain description text
        let raw_job = ScrapedJob {
                compensation: vec![],
            dedupe_key: "gh:unclassified:01".to_string(),
            source_id: None,
            external_id: Some("01".to_string()),
            title: "Security Analyst".to_string(),
            company: "DefenseCo".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Washington, DC".to_string()),
            url: "https://example.com/01".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
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
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: Some(
                "<h2>Requirements</h2><p>Annual compensation: $140,000 to $170,000. \
                 Requires 4+ years of relevant experience. \
                 Must hold an active Secret security clearance. \
                 Must be a U.S. Person (US citizen or lawful permanent resident / green card holder) under ITAR. \
                 Unfortunately, we cannot sponsor work visas.</p>"
                    .to_string(),
            ),
            description: None,
            description_source: None,
                description_format: None,
            company_icon_url: None,
            company_logo: None,
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
        sqlx::query("UPDATE jobs SET notes='keep me', favorited=1, status='applied' WHERE id = ?")
            .bind(&initial.items[0].id)
            .execute(&pool)
            .await
            .unwrap();

        // Run reclassification engine
        let updated_count = JobsRepo::reclassify_jobs(&pool).await.unwrap();
        assert_eq!(updated_count, 1);
        assert_eq!(JobsRepo::reclassify_jobs(&pool).await.unwrap(), 0);

        // Verify natural language enrichment populated columns
        let reclassified = JobsRepo::get(&pool, &initial.items[0].id).await.unwrap();
        assert_eq!(reclassified.salary_min, Some(140_000));
        assert_eq!(reclassified.salary_max, Some(170_000));
        assert_eq!(reclassified.experience_min, Some(4));
        assert_eq!(reclassified.clearance, TriState::Yes);
        assert_eq!(reclassified.sponsorship, TriState::No);
        assert_eq!(reclassified.work_auth, WorkAuthRequirement::UsPerson);
        assert!(reclassified.clearance_evidence.is_some());
        assert!(reclassified.sponsorship_evidence.is_some());
        assert!(reclassified.work_auth_evidence.is_some());
        assert_eq!(reclassified.notes.as_deref(), Some("keep me"));
        assert!(reclassified.favorited);
        assert_eq!(reclassified.status, JobStatus::Applied);
    }

    async fn seed_experience_repair_rows(pool: &SqlitePool, count: i64) {
        sqlx::query("INSERT INTO workflows(id,name,enabled,cadence,next_run_at,filters,config,created_at) VALUES ('repair-workflow','Protected workflow',0,'weekly','2030-01-01','{\"experience_min\":3}','{\"protected\":true}','2026-01-01')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,enabled,cadence,next_run_at,created_at) VALUES ('repair-source','repair-workflow','greenhouse','Protected source','repair-fixture',0,'weekly','2030-01-01','2026-01-01')")
            .execute(pool).await.unwrap();
        let description = scrape_core::enrich::normalize_description(
            "Required: 5 years of professional experience. Salary: $175,000 per year.",
            32768,
        );
        sqlx::query(
            r#"
            WITH RECURSIVE sequence(n) AS (
                VALUES(0) UNION ALL SELECT n+1 FROM sequence WHERE n+1 < ?1
            )
            INSERT INTO jobs(
                id,dedupe_key,posting_identity,source_id,workflow_id,external_id,
                title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,
                notes,favorited,favorited_at,salary_min,salary_max,salary_currency,
                salary_interval,salary_raw,salary_source,compensation_json,compensation_migrated,
                sponsorship,sponsorship_evidence,clearance,clearance_level,clearance_evidence,
                work_auth,work_auth_evidence,workplace_type,description_text,
                description_source,description_format,description_json,
                experience_min,experience_max,experience_raw,is_new_grad,experience_evidence,
                enrichment_version,company_branding_key,company_icon_url
            )
            SELECT printf('repair-%03d',n),printf('dedupe-%03d',n),printf('identity-%03d',n),
                'repair-source','repair-workflow',printf('%d',n),
                'SOFTWARE ENGINEER III','Acme','contract','https://fixture.test/job/'||n,
                '2026-01-01','2026-01-02','applied','2026-01-03',
                'Protected note',1,'2026-01-04',125000,150000,'CAD',
                'year','Protected salary','structured','[]',1,
                'no','Protected sponsorship','yes','secret','Protected clearance',
                'us_person','Protected work auth','hybrid',
                CASE WHEN n % 3 = 0 THEN NULL ELSE ?2 END,
                '<p>Old source: 1 year of professional experience.</p>','html',?3,
                0,1,'Old experience','yes','Old evidence',3,
                'pinned-branding-key','https://fixture.test/protected-logo.png'
            FROM sequence
            "#,
        )
        .bind(count)
        .bind(&description.plain_text)
        .bind(serde_json::to_string(&description).unwrap())
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO job_status_events(job_id,from_status,to_status,changed_at) VALUES ('repair-000','viewed','applied','2026-01-03')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO job_locations(job_id,raw,city,country_code,is_remote) VALUES ('repair-000','Toronto, Canada','Toronto','CA',0)")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO deleted_jobs(dedupe_key,deleted_at) VALUES ('protected-deleted','2026-01-01')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO blocked_postings(identity,title,company,blocked_at,legacy_key) VALUES ('protected-block','Blocked','Acme','2026-01-01','protected-deleted')")
            .execute(pool).await.unwrap();
        sqlx::query("INSERT INTO posting_legacy_blocks(legacy_key,identity) VALUES ('protected-deleted','protected-block')")
            .execute(pool).await.unwrap();
    }

    // Snapshot every persisted column, including future additions; only the
    // classifier's explicitly owned fields may change on version-3 rows.
    async fn repair_state_snapshot(pool: &SqlitePool) -> Vec<String> {
        let mut snapshots = Vec::new();
        for table in [
            "jobs",
            "job_status_events",
            "job_locations",
            "workflows",
            "sources",
            "deleted_jobs",
            "blocked_postings",
            "posting_legacy_blocks",
        ] {
            let columns: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                    .bind(table)
                    .fetch_all(pool)
                    .await
                    .unwrap();
            let columns = columns
                .into_iter()
                .filter(|name| {
                    table != "jobs"
                        || ![
                            "experience_min",
                            "experience_max",
                            "experience_raw",
                            "is_new_grad",
                            "experience_evidence",
                            "enrichment_version",
                        ]
                        .contains(&name.as_str())
                })
                .map(|name| format!("\"{}\"", name.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",");
            let query = format!(
                "SELECT json_group_array(json_array({columns})) FROM (SELECT * FROM {table} ORDER BY rowid)"
            );
            snapshots.push(sqlx::query_scalar(&query).fetch_one(pool).await.unwrap());
        }
        snapshots
    }

    fn description_repair_backups(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("pre-description-repair")
            })
            .collect()
    }

    #[tokio::test]
    async fn reclassify_drains_all_batches_including_title_only_and_preserves_protected_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("experience.db");
        let pool = crate::db::init_db(&path).await.unwrap();
        seed_experience_repair_rows(&pool, 405).await;
        sqlx::query("INSERT INTO jobs(id,dedupe_key,posting_identity,title,company,role_type,url,date_added,last_seen_at,status_changed_at,enrichment_version,sponsorship,sponsorship_evidence,salary_min,compensation_migrated) VALUES ('legacy-null','legacy-null','legacy:legacy-null','SOFTWARE ENGINEER III','Acme','internship','https://fixture.test/null','2026-01-01','2026-01-01','2026-01-01',0,'yes','Protected old hint',88000,1)")
            .execute(&pool).await.unwrap();
        let before = repair_state_snapshot(&pool).await;

        assert_eq!(JobsRepo::reclassify_jobs(&pool).await.unwrap(), 406);
        assert_eq!(repair_state_snapshot(&pool).await, before);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE enrichment_version < ?")
                .bind(ENRICHMENT_VERSION)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE description_text IS NULL AND experience_min IS NULL AND experience_max IS NULL AND is_new_grad='no'")
                .fetch_one(&pool).await.unwrap(),
            136
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE description_text IS NOT NULL AND experience_min=5 AND is_new_grad='no'")
                .fetch_one(&pool).await.unwrap(),
            270
        );
        let backups = description_repair_backups(directory.path());
        assert_eq!(backups.len(), 1);
        let backup = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(&backups[0])
                    .read_only(true),
            )
            .await
            .unwrap();
        assert_eq!(repair_state_snapshot(&backup).await, before);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE enrichment_version < ?")
                .bind(ENRICHMENT_VERSION)
                .fetch_one(&backup)
                .await
                .unwrap(),
            406
        );
        backup.close().await;
        pool.close().await;
        let reopened = crate::db::init_db(&path).await.unwrap();
        assert_eq!(JobsRepo::reclassify_jobs(&reopened).await.unwrap(), 0);
        assert_eq!(description_repair_backups(directory.path()).len(), 1);
        assert_eq!(repair_state_snapshot(&reopened).await, before);
    }

    #[tokio::test]
    async fn reclassify_failed_batch_rolls_back_and_restart_resumes_after_committed_batches() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("retry.db");
        let pool = crate::db::init_db(&path).await.unwrap();
        seed_experience_repair_rows(&pool, 405).await;
        let before = repair_state_snapshot(&pool).await;
        sqlx::query("CREATE TRIGGER reject_repair BEFORE UPDATE OF enrichment_version ON jobs WHEN NEW.id='repair-250' BEGIN SELECT RAISE(ABORT, 'authored repair write failure'); END")
            .execute(&pool).await.unwrap();
        assert!(JobsRepo::reclassify_jobs(&pool).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE enrichment_version = ?")
                .bind(ENRICHMENT_VERSION)
                .fetch_one(&pool)
                .await
                .unwrap(),
            200
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE id >= 'repair-200' AND enrichment_version=3 AND experience_min=0 AND is_new_grad='yes'")
                .fetch_one(&pool).await.unwrap(),
            205
        );
        assert_eq!(repair_state_snapshot(&pool).await, before);
        assert_eq!(description_repair_backups(directory.path()).len(), 1);
        sqlx::query("DROP TRIGGER reject_repair")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let reopened = crate::db::init_db(&path).await.unwrap();
        assert_eq!(JobsRepo::reclassify_jobs(&reopened).await.unwrap(), 205);
        assert_eq!(JobsRepo::reclassify_jobs(&reopened).await.unwrap(), 0);
        assert_eq!(description_repair_backups(directory.path()).len(), 2);
        assert_eq!(repair_state_snapshot(&reopened).await, before);
    }

    #[tokio::test]
    async fn reclassify_backup_failure_prevents_any_saved_data_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("original");
        let moved = directory.path().join("moved");
        std::fs::create_dir(&original).unwrap();
        let pool = crate::db::init_db(&original.join("backup-failure.db"))
            .await
            .unwrap();
        seed_experience_repair_rows(&pool, 3).await;
        let before = repair_state_snapshot(&pool).await;
        // The already-open database remains readable, but its original parent
        // path no longer exists, making the backup destination unwritable.
        std::fs::rename(&original, &moved).unwrap();
        let result = JobsRepo::reclassify_jobs(&pool).await;
        std::fs::rename(&moved, &original).unwrap();
        assert!(result.is_err());
        assert_eq!(repair_state_snapshot(&pool).await, before);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE enrichment_version=3")
                .fetch_one(&pool)
                .await
                .unwrap(),
            3
        );
        assert!(description_repair_backups(&original).is_empty());
        assert_eq!(JobsRepo::reclassify_jobs(&pool).await.unwrap(), 3);
        assert_eq!(description_repair_backups(&original).len(), 1);
    }

    #[tokio::test]
    async fn test_multi_filter_combination_query() {
        let pool = init_memory_db().await.unwrap();

        // Insert jobs across varied attribute permutations
        let jobs = vec![
            ScrapedJob {
                compensation: scrape_core::enrich::Salary::from_structured(
                    Some(160_000.0),
                    Some(200_000.0),
                    Some("USD"),
                    Some("YEAR"),
                    Some("Fixture provider annual range"),
                )
                .unwrap()
                .ranges,
                dedupe_key: "t1".to_string(),
                source_id: None,
                external_id: Some("1".to_string()),
                title: "Frontend Engineer".to_string(),
                company: "Vercel".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Remote".to_string()),
                url: "https://vercel.com/1".to_string(),
                provider_updated_at: None,
                location_data_complete: true,
                date_posted: Some("2026-10-01".to_string()),
                salary_min: Some(160_000),
                salary_max: Some(200_000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(SalaryInterval::Year),
                salary_raw: None,
                salary_source: Some(SalarySource::Structured),
                workplace_type: WorkplaceType::Remote,
                sponsorship: TriState::Yes,
                sponsorship_evidence: None,
                work_auth: WorkAuthRequirement::Open,
                work_auth_evidence: None,
                clearance: TriState::No,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(2),
                experience_max: Some(5),
                experience_raw: None,
                is_new_grad: TriState::No,
                experience_evidence: None,
                description_text: None,
                description: None,
                description_source: None,
                description_format: None,
                company_icon_url: None,
                company_logo: None,
                locations: vec![ParsedLocation {
                    raw: "Remote, US".to_string(),
                    city: None,
                    region_code: None,
                    country_code: Some("US".to_string()),
                    is_remote: true,
                }],
            },
            ScrapedJob {
                compensation: scrape_core::enrich::Salary::from_structured(
                    Some(130_000.0),
                    Some(170_000.0),
                    Some("USD"),
                    Some("YEAR"),
                    Some("Fixture provider annual range"),
                )
                .unwrap()
                .ranges,
                dedupe_key: "t2".to_string(),
                source_id: None,
                external_id: Some("2".to_string()),
                title: "Security Engineer".to_string(),
                company: "Lockheed".to_string(),
                role_type: RoleType::FullTime,
                location: Some("Arlington, VA".to_string()),
                url: "https://lockheed.com/2".to_string(),
                provider_updated_at: None,
                location_data_complete: true,
                date_posted: Some("2026-10-02".to_string()),
                salary_min: Some(130_000),
                salary_max: Some(170_000),
                salary_currency: Some("USD".to_string()),
                salary_interval: Some(SalaryInterval::Year),
                salary_raw: None,
                salary_source: Some(SalarySource::Structured),
                workplace_type: WorkplaceType::Onsite,
                sponsorship: TriState::No,
                sponsorship_evidence: None,
                work_auth: WorkAuthRequirement::CitizenOnly,
                work_auth_evidence: None,
                clearance: TriState::Yes,
                clearance_level: None,
                clearance_evidence: None,
                experience_min: Some(5),
                experience_max: Some(8),
                experience_raw: None,
                is_new_grad: TriState::No,
                experience_evidence: None,
                description_text: None,
                description: None,
                description_source: None,
                description_format: None,
                company_icon_url: None,
                company_logo: None,
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
            matching: None,
            salary_min: Some(150_000.0),
            salary_max: Some(220_000.0),
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

        // Test work_auth filter explicitly
        let wa_filter = JobFilter {
            matching: None,
            work_auth: Some(vec![WorkAuthRequirement::CitizenOnly]),
            ..Default::default()
        };
        let wa_result = JobsRepo::list(&pool, wa_filter).await.unwrap();
        assert_eq!(wa_result.total, 1);
        assert_eq!(wa_result.items[0].company, "Lockheed");

        // Mutate one dimension (e.g. require clearance) and ensure empty result without SQL syntax errors
        let non_matching_filter = JobFilter {
            matching: None,
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
            compensation: vec![],
            dedupe_key: "k1".to_string(),
            source_id: None,
            external_id: None,
            title: "Engineer CA".to_string(),
            company: "Tech Corp".to_string(),
            role_type: RoleType::FullTime,
            location: Some("San Francisco, CA".to_string()),
            url: "https://example.com/ca".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
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
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![ParsedLocation {
                raw: "San Francisco, CA".to_string(),
                city: Some("San Francisco".to_string()),
                region_code: Some("US-CA".to_string()),
                country_code: Some("US".to_string()),
                is_remote: false,
            }],
        };

        let job_on = ScrapedJob {
            compensation: vec![],
            dedupe_key: "k2".to_string(),
            source_id: None,
            external_id: None,
            title: "Engineer ON".to_string(),
            company: "Maple Tech".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Toronto, ON".to_string()),
            url: "https://example.com/on".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
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
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![ParsedLocation {
                raw: "Toronto, ON".to_string(),
                city: Some("Toronto".to_string()),
                region_code: Some("CA-ON".to_string()),
                country_code: Some("CA".to_string()),
                is_remote: false,
            }],
        };

        JobsRepo::upsert_scraped(&pool, &[job_ca, job_on])
            .await
            .unwrap();

        // 1. Filter by country US
        let us_filter = JobFilter {
            matching: None,
            countries: Some(vec!["US".to_string()]),
            location_include_unknown: Some(false),
            ..Default::default()
        };
        let us_res = JobsRepo::list(&pool, us_filter).await.unwrap();
        assert_eq!(us_res.total, 1);
        assert_eq!(us_res.items[0].company, "Tech Corp");

        // 2. Filter by region code suffix "CA" (should match "US-CA")
        let ca_suffix_filter = JobFilter {
            matching: None,
            regions: Some(vec!["CA".to_string()]),
            location_include_unknown: Some(false),
            ..Default::default()
        };
        let ca_res = JobsRepo::list(&pool, ca_suffix_filter).await.unwrap();
        assert_eq!(ca_res.total, 1);
        assert_eq!(ca_res.items[0].company, "Tech Corp");

        // 3. Filter by full ISO region code "CA-ON"
        let on_filter = JobFilter {
            matching: None,
            regions: Some(vec!["CA-ON".to_string()]),
            location_include_unknown: Some(false),
            ..Default::default()
        };
        let on_res = JobsRepo::list(&pool, on_filter).await.unwrap();
        assert_eq!(on_res.total, 1);
        assert_eq!(on_res.items[0].company, "Maple Tech");
    }

    #[tokio::test]
    async fn test_toggle_and_set_favorite() {
        let pool = init_memory_db().await.unwrap();

        let job = ScrapedJob {
            compensation: vec![],
            dedupe_key: "fav:1".to_string(),
            source_id: None,
            external_id: Some("1".to_string()),
            title: "Staff Rust Engineer".to_string(),
            company: "Acme Corp".to_string(),
            role_type: RoleType::FullTime,
            location: Some("Remote".to_string()),
            url: "https://example.com/fav/1".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
            date_posted: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Remote,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![],
        };

        JobsRepo::upsert_scraped(&pool, &[job]).await.unwrap();

        let list = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        let job_id = &list.items[0].id;
        assert!(!list.items[0].favorited);
        assert_eq!(list.items[0].favorited_at, None);

        // Toggle on
        let toggled_on = JobsRepo::toggle_favorite(&pool, job_id).await.unwrap();
        assert!(toggled_on.favorited);
        assert!(toggled_on.favorited_at.is_some());

        // Toggle off
        let toggled_off = JobsRepo::toggle_favorite(&pool, job_id).await.unwrap();
        assert!(!toggled_off.favorited);
        assert_eq!(toggled_off.favorited_at, None);

        // Set favorite explicitly
        let set_on = JobsRepo::set_favorite(&pool, job_id, true).await.unwrap();
        assert!(set_on.favorited);
        assert!(set_on.favorited_at.is_some());
    }

    #[tokio::test]
    async fn test_filter_favorited_only() {
        let pool = init_memory_db().await.unwrap();

        let job_a = ScrapedJob {
            compensation: vec![],
            dedupe_key: "fav:a".to_string(),
            source_id: None,
            external_id: Some("a".to_string()),
            title: "Job A".to_string(),
            company: "Alpha".to_string(),
            role_type: RoleType::FullTime,
            location: None,
            url: "https://example.com/a".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
            date_posted: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Remote,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![],
        };

        let job_b = ScrapedJob {
            compensation: vec![],
            dedupe_key: "fav:b".to_string(),
            source_id: None,
            external_id: Some("b".to_string()),
            title: "Job B".to_string(),
            company: "Beta".to_string(),
            role_type: RoleType::FullTime,
            location: None,
            url: "https://example.com/b".to_string(),
            provider_updated_at: None,
            location_data_complete: true,
            date_posted: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: WorkplaceType::Remote,
            sponsorship: TriState::Unknown,
            sponsorship_evidence: None,
            work_auth: WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            clearance: TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: TriState::Unknown,
            experience_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
            locations: vec![],
        };

        JobsRepo::upsert_scraped(&pool, &[job_a, job_b])
            .await
            .unwrap();

        let list = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(list.total, 2);
        let id_a = list
            .items
            .iter()
            .find(|j| j.company == "Alpha")
            .unwrap()
            .id
            .clone();

        // Favorite Alpha only
        JobsRepo::set_favorite(&pool, &id_a, true).await.unwrap();

        // Filter favorited only
        let fav_res = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                favorited_only: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(fav_res.total, 1);
        assert_eq!(fav_res.items[0].company, "Alpha");
    }

    #[tokio::test]
    async fn test_retention_prune_protects_favorited_notes_and_tracked() {
        use crate::db::sources::{CreateSourceInput, SourcesRepo};
        use scrape_core::SourceKind;

        let pool = init_memory_db().await.unwrap();

        let source = SourcesRepo::create(
            &pool,
            CreateSourceInput {
                kind: SourceKind::Greenhouse,
                name: "Retention Test".to_string(),
                identifier: "retention-test".to_string(),
                config: None,
                enabled: Some(true),
                cadence: None,
                cadence_cron: None,
                retention_days: Some(30),
                retention_policy: Some("untracked_only".to_string()),
                filters: None,
            },
        )
        .await
        .unwrap();

        // Create 4 jobs directly in SQL with date_added = 60 days ago
        let old_date = "2000-01-01T00:00:00Z";

        // Job 1: Untracked, unfavorited, no notes (WILL BE PRUNED)
        sqlx::query(
            r#"
            INSERT INTO jobs (id, dedupe_key, source_id, title, company, role_type, url, date_added, last_seen_at, status, status_changed_at, favorited)
            VALUES ('j1', 'key1', ?1, 'Job 1', 'Co 1', 'full_time', 'url1', ?2, ?2, 'scraped', ?2, 0)
            "#
        )
        .bind(&source.id)
        .bind(old_date)
        .execute(&pool)
        .await
        .unwrap();

        // Job 2: Untracked, FAVORITED = 1 (PROTECTED)
        sqlx::query(
            r#"
            INSERT INTO jobs (id, dedupe_key, source_id, title, company, role_type, url, date_added, last_seen_at, status, status_changed_at, favorited)
            VALUES ('j2', 'key2', ?1, 'Job 2', 'Co 2', 'full_time', 'url2', ?2, ?2, 'scraped', ?2, 1)
            "#
        )
        .bind(&source.id)
        .bind(old_date)
        .execute(&pool)
        .await
        .unwrap();

        // Job 3: Untracked, has NOTES (PROTECTED)
        sqlx::query(
            r#"
            INSERT INTO jobs (id, dedupe_key, source_id, title, company, role_type, url, date_added, last_seen_at, status, status_changed_at, notes, favorited)
            VALUES ('j3', 'key3', ?1, 'Job 3', 'Co 3', 'full_time', 'url3', ?2, ?2, 'scraped', ?2, 'Jane referral', 0)
            "#
        )
        .bind(&source.id)
        .bind(old_date)
        .execute(&pool)
        .await
        .unwrap();

        // Job 4: TRACKED ('applied') (PROTECTED)
        sqlx::query(
            r#"
            INSERT INTO jobs (id, dedupe_key, source_id, title, company, role_type, url, date_added, last_seen_at, status, status_changed_at, favorited)
            VALUES ('j4', 'key4', ?1, 'Job 4', 'Co 4', 'full_time', 'url4', ?2, ?2, 'applied', ?2, 0)
            "#
        )
        .bind(&source.id)
        .bind(old_date)
        .execute(&pool)
        .await
        .unwrap();

        // Run retention pruning
        let pruned = JobsRepo::prune_expired_by_source(&pool, &source.id, 30, "untracked_only")
            .await
            .unwrap();

        // Exactly 1 job (Job 1) should be pruned
        assert_eq!(pruned, 1);

        // Verify remaining jobs
        assert!(
            JobsRepo::get(&pool, "j1").await.is_err(),
            "Job 1 should be deleted"
        );
        assert!(
            JobsRepo::get(&pool, "j2").await.is_ok(),
            "Job 2 (favorited) must be preserved"
        );
        assert!(
            JobsRepo::get(&pool, "j3").await.is_ok(),
            "Job 3 (notes) must be preserved"
        );
        assert!(
            JobsRepo::get(&pool, "j4").await.is_ok(),
            "Job 4 (applied) must be preserved"
        );
    }

    #[tokio::test]
    async fn retention_uses_one_fixed_discovery_age_boundary_and_preserves_protected_rows() {
        use crate::db::sources::{CreateSourceInput, SourcesRepo};
        use chrono::{TimeZone, Utc};
        use scrape_core::SourceKind;

        let pool = init_memory_db().await.unwrap();
        let source = SourcesRepo::create(
            &pool,
            CreateSourceInput {
                kind: SourceKind::Greenhouse,
                name: "Fixed clock retention".into(),
                identifier: "fixed-clock".into(),
                config: None,
                enabled: Some(false),
                cadence: None,
                cadence_cron: None,
                retention_days: Some(1),
                retention_policy: Some("untracked_only".into()),
                filters: None,
            },
        )
        .await
        .unwrap();
        let now = Utc.with_ymd_and_hms(2026, 10, 15, 12, 0, 0).unwrap();
        let at_boundary = "2026-10-14T12:00:00.000Z";
        for (id, added, seen, status, notes, favorite, removed) in [
            (
                "old",
                "2026-10-14T11:59:59.000Z",
                "2026-10-15T12:00:00Z",
                "scraped",
                None,
                0,
                None,
            ),
            (
                "boundary",
                at_boundary,
                at_boundary,
                "viewed",
                None,
                0,
                None,
            ),
            (
                "fresh",
                "2026-10-14T12:00:01.000Z",
                at_boundary,
                "scraped",
                None,
                0,
                None,
            ),
            (
                "noted",
                at_boundary,
                at_boundary,
                "scraped",
                Some("keep"),
                0,
                None,
            ),
            (
                "favorite",
                at_boundary,
                at_boundary,
                "viewed",
                None,
                1,
                None,
            ),
            (
                "tracked",
                at_boundary,
                at_boundary,
                "applied",
                None,
                0,
                None,
            ),
            (
                "removed",
                at_boundary,
                at_boundary,
                "scraped",
                None,
                0,
                Some("2026-10-15T11:00:00Z"),
            ),
        ] {
            sqlx::query("INSERT INTO jobs (id,dedupe_key,source_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,removed_at) VALUES (?1,?1,?2,'Role','Org','https://example.test/job',?3,?4,?5,?4,?6,?7,?8)")
                .bind(id).bind(&source.id).bind(added).bind(seen).bind(status).bind(notes).bind(favorite).bind(removed)
                .execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO deleted_jobs (dedupe_key, deleted_at) VALUES ('blocked-identity', '2026-10-14T12:00:00Z')")
            .execute(&pool).await.unwrap();

        let deleted =
            JobsRepo::prune_expired_by_source_at(&pool, &source.id, 1, "untracked_only", now)
                .await
                .unwrap();
        assert_eq!(
            deleted, 2,
            "expired date_added rows include the exact cutoff despite recent last_seen"
        );
        let remaining: Vec<String> = sqlx::query_scalar("SELECT id FROM jobs ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(
            remaining,
            ["favorite", "fresh", "noted", "removed", "tracked"]
        );
        let blocks: i64 = sqlx::query_scalar("SELECT count(*) FROM deleted_jobs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(blocks, 1);
    }

    #[tokio::test]
    async fn searches_and_counts_more_than_two_thousand_jobs_and_treats_search_as_literal() {
        let pool = init_memory_db().await.unwrap();
        sqlx::query("WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<2105) INSERT INTO jobs (id, dedupe_key, title, company, url, date_added, last_seen_at, status_changed_at, compensation_migrated) SELECT printf('job-%04d',x), printf('key-%d',x), CASE WHEN x=2050 THEN 'Rare 100% Engineer' ELSE 'Engineer' END, 'Co', 'https://example.com/job', '2020-01-01', '2020-01-01', '2020-01-01', 1 FROM n")
            .execute(&pool).await.unwrap();
        let page = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                offset: Some(2000),
                limit: Some(50),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 2105);
        assert_eq!(page.items.len(), 50);
        let result = JobsRepo::list(
            &pool,
            JobFilter {
                matching: None,
                search: Some("100%".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].id, "job-2050");
        sqlx::query("UPDATE jobs SET notes='Needle in saved notes' WHERE id='job-2105'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            JobsRepo::list(
                &pool,
                JobFilter {
                    matching: None,
                    search: Some("Needle".into()),
                    ..Default::default()
                }
            )
            .await
            .unwrap()
            .total,
            1
        );
    }

    #[tokio::test]
    async fn currency_filter_binds_quotes_and_excludes_unknown_currency() {
        let pool = init_memory_db().await.unwrap();
        for (id, currency) in [
            ("usd", Some("USD")),
            ("unknown", None),
            ("quoted", Some("X' OR 1=1 --")),
        ] {
            sqlx::query("INSERT INTO jobs (id, dedupe_key, title, company, url, date_added, last_seen_at, status_changed_at, salary_currency) VALUES (?1, ?1, 'Engineer', 'Co', 'https://example.com/job', '2000-01-01', '2000-01-01', '2000-01-01', ?2)")
                .bind(id).bind(currency).execute(&pool).await.unwrap();
            let range = scrape_core::enrich::CompensationRange {
                min_minor: None,
                max_minor: None,
                currency: currency.map(str::to_string),
                interval: None,
                raw: "Fixture currency evidence".into(),
                source: SalarySource::Structured,
                derived: false,
                location: None,
            };
            sqlx::query(
                "UPDATE jobs SET compensation_json = ?, compensation_migrated = 1 WHERE id = ?",
            )
            .bind(serde_json::to_string(&vec![range]).unwrap())
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        }
        for (currency, expected) in [(" usd ", "usd"), ("X' OR 1=1 --", "quoted")] {
            let page = JobsRepo::list(
                &pool,
                JobFilter {
                    matching: None,
                    salary_currency: Some(currency.into()),
                    salary_include_unknown: Some(true),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(page.total, 1);
            assert_eq!(page.items[0].id, expected);
        }
    }

    #[tokio::test]
    async fn every_retention_policy_protects_all_tracked_states_and_annotations() {
        for policy in ["all", "untracked_only"] {
            let pool = init_memory_db().await.unwrap();
            sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at) VALUES ('source', 'greenhouse', 'Test', 'test', '2000-01-01')")
                .execute(&pool).await.unwrap();
            for (id, status, notes, favorite) in [
                ("applied", "applied", "", false),
                ("interview", "interviewing", "", false),
                ("offer", "offer", "", false),
                ("rejected", "rejected", "", false),
                ("notes", "scraped", "  Referral  ", false),
                ("favorite", "viewed", "", true),
                ("eligible", "scraped", " \t\n", false),
            ] {
                sqlx::query("INSERT INTO jobs (id, dedupe_key, source_id, title, company, url, date_added, last_seen_at, status, status_changed_at, notes, favorited) VALUES (?1, ?1, 'source', 'Engineer', 'Co', 'https://example.com/job', '2000-01-01T02:00:00+02:00', '2000-01-01', ?2, '2000-01-01', ?3, ?4)")
                    .bind(id).bind(status).bind(notes).bind(favorite).execute(&pool).await.unwrap();
            }
            sqlx::query("INSERT INTO job_status_events (job_id, to_status, changed_at) VALUES ('applied', 'applied', '2000-01-01')")
                .execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO deleted_jobs (dedupe_key, deleted_at) VALUES ('blocked', '2000-01-01')")
                .execute(&pool).await.unwrap();
            assert_eq!(
                JobsRepo::prune_expired_by_source(&pool, "source", 30, policy)
                    .await
                    .unwrap(),
                1
            );
            let ids: Vec<String> = sqlx::query_scalar("SELECT id FROM jobs ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert_eq!(
                ids,
                [
                    "applied",
                    "favorite",
                    "interview",
                    "notes",
                    "offer",
                    "rejected"
                ]
            );
            for table in ["job_status_events", "deleted_jobs"] {
                let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(count, 1);
            }
        }
    }

    #[tokio::test]
    async fn removal_rediscovery_and_explicit_blocking() {
        use scrape_core::RoleType;

        let pool = init_memory_db().await.unwrap();

        let mut scraped = vec![ScrapedJob {
            compensation: vec![],
            source_id: None,
            external_id: Some("99999".into()),
            dedupe_key: "gh:testco:99999".into(),
            title: "Staff Engineer".into(),
            company: "TestCo".into(),
            role_type: RoleType::FullTime,
            location: Some("Remote".into()),
            locations: vec![],
            url: "https://example.com/job/99999".into(),
            provider_updated_at: None,
            location_data_complete: true,
            date_posted: None,
            salary_min: None,
            salary_max: None,
            salary_currency: None,
            salary_interval: None,
            salary_raw: None,
            salary_source: None,
            workplace_type: scrape_core::WorkplaceType::Remote,
            sponsorship: scrape_core::TriState::Unknown,
            sponsorship_evidence: None,
            clearance: scrape_core::TriState::Unknown,
            clearance_level: None,
            clearance_evidence: None,
            experience_min: None,
            experience_max: None,
            experience_raw: None,
            is_new_grad: scrape_core::TriState::Unknown,
            experience_evidence: None,
            work_auth: scrape_core::WorkAuthRequirement::Unknown,
            work_auth_evidence: None,
            description_text: None,
            description: None,
            description_source: None,
            description_format: None,
            company_icon_url: None,
            company_logo: None,
        }];

        sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at) VALUES ('original-workflow', 'greenhouse', 'TestCo', 'testco', '2026-01-01')").execute(&pool).await.unwrap();
        scraped[0].source_id = Some("original-workflow".into());
        scraped[0].dedupe_key = "original-workflow:99999".into();
        // 1. Initial upsert
        let stats = JobsRepo::upsert_scraped(&pool, &scraped).await.unwrap();
        assert_eq!(stats.inserted, 1);

        let jobs = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(jobs.items.len(), 1);
        let job_id = &jobs.items[0].id;

        // Ordinary removal retains saved state and permits later ingestion.
        let removed = super::super::postings::remove(&pool, job_id, false)
            .await
            .unwrap();
        assert_eq!(removed.copies, 1);
        assert!(JobsRepo::get(&pool, job_id)
            .await
            .unwrap()
            .removed_at
            .is_some());
        assert_eq!(
            JobsRepo::list(&pool, JobFilter::default())
                .await
                .unwrap()
                .total,
            0
        );
        let rediscovered = JobsRepo::upsert_scraped(&pool, &scraped).await.unwrap();
        assert_eq!(rediscovered.updated, 1);
        assert!(JobsRepo::get(&pool, job_id)
            .await
            .unwrap()
            .removed_at
            .is_none());
        // Explicit blocking suppresses in-flight/later discovery until restoration.
        super::super::postings::remove(&pool, job_id, true)
            .await
            .unwrap();
        let blocked = JobsRepo::upsert_scraped(&pool, &scraped).await.unwrap();
        assert_eq!(blocked.updated + blocked.inserted, 0);
        // Recreating a workflow and changing provider destination must not bypass the block.
        sqlx::query("DELETE FROM sources WHERE id='original-workflow'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at) VALUES ('recreated-workflow', 'greenhouse', 'TestCo', 'testco', '2026-01-01')").execute(&pool).await.unwrap();
        scraped[0].source_id = Some("recreated-workflow".into());
        scraped[0].dedupe_key = "recreated-workflow:99999".into();
        scraped[0].url = "https://careers.example.com/?gh_jid=99999".into();
        let recreated = JobsRepo::upsert_scraped(&pool, &scraped).await.unwrap();
        assert_eq!(recreated.updated + recreated.inserted, 0);
        super::super::postings::restore(&pool, &removed.identity)
            .await
            .unwrap();
        assert!(JobsRepo::get(&pool, job_id)
            .await
            .unwrap()
            .removed_at
            .is_none());
        assert_eq!(
            JobsRepo::get_status_events(&pool, job_id)
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
