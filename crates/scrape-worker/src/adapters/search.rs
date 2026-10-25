//! Public cross-employer feeds. Matching is local, not provider-side search.

use super::search_cache::SearchCache;
use crate::client::{ClientError, HttpClient};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, normalize_description_with_format,
    DescriptionFormat, Salary,
};
use scrape_core::protocol::CoverageTermination;
use scrape_core::{RoleType, WorkerEvent, WorkerLimits, WorkerSource, WorkplaceType};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use tokio::time::{timeout_at, Duration, Instant};
use url::Url;

const MAX_URL_BYTES: usize = 8192;

#[derive(Deserialize)]
struct SearchResponse {
    // Required: a provider error or unrelated JSON object must not mean an empty board.
    data: Vec<Value>,
    #[serde(default)]
    links: Option<SearchLinks>,
}

#[derive(Deserialize)]
struct SearchLinks {
    next: Option<String>,
}

#[derive(Deserialize)]
struct SearchItem {
    slug: Option<String>,
    title: String,
    company_name: String,
    url: String,
    description: Option<String>,
    location: Option<String>,
    remote: Option<bool>,
    #[serde(default)]
    job_types: Vec<String>,
    // created_at is deliberately not a publication date. The official API documents
    // ordering by this field, not the employer's original publication time:
    // https://documenter.getpostman.com/view/18545278/UVJbJdKh
}

#[derive(Clone, Copy)]
enum Provider {
    Arbeitnow,
    Muse,
    Adzuna,
    Jobicy,
}

#[derive(Deserialize)]
struct MuseResponse {
    page: u64,
    page_count: u64,
    results: Vec<Value>,
}

#[derive(Deserialize)]
struct MuseName {
    name: String,
}

#[derive(Deserialize)]
struct MuseRefs {
    landing_page: String,
}

#[derive(Deserialize)]
struct MuseItem {
    id: u64,
    name: String,
    company: MuseName,
    refs: MuseRefs,
    contents: Option<String>,
    publication_date: Option<String>,
    #[serde(default)]
    locations: Vec<MuseName>,
}

struct Posting {
    external_id: Option<String>,
    title: String,
    company: String,
    url: String,
    description: Option<String>,
    location: String,
    locations: Vec<loc_enrich::ParsedLocation>,
    remote: Option<bool>,
    job_types: Vec<String>,
    date_posted: Option<String>,
    salary: Option<Salary>,
    salary_from_description: bool,
    prefer_structured_salary: bool,
}

impl SearchItem {
    fn posting(self) -> Posting {
        let location = self.location.unwrap_or_default();
        let locations = loc_enrich::parse(&location);
        Posting {
            external_id: self.slug,
            title: self.title,
            company: self.company_name,
            url: self.url,
            description: self.description,
            location,
            locations,
            remote: self.remote,
            job_types: self.job_types,
            date_posted: None,
            salary: None,
            salary_from_description: true,
            prefer_structured_salary: false,
        }
    }
}

impl MuseItem {
    fn posting(self) -> Result<Posting, String> {
        let date_posted = self
            .publication_date
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                chrono::DateTime::parse_from_rfc3339(&value)
                    .map(|_| value)
                    .map_err(|error| format!("Invalid Muse publication_date: {error}"))
            })
            .transpose()?;
        let locations = self
            .locations
            .iter()
            .flat_map(|location| loc_enrich::parse(&location.name))
            .collect();
        let mut location = String::new();
        for value in self.locations {
            if !location.is_empty() {
                location.push_str("; ");
            }
            location.push_str(&value.name);
        }
        Ok(Posting {
            external_id: Some(self.id.to_string()),
            title: self.name,
            company: self.company.name,
            url: self.refs.landing_page,
            description: self.contents,
            location,
            locations,
            remote: None,
            job_types: Vec::new(),
            date_posted,
            salary: None,
            salary_from_description: true,
            prefer_structured_salary: false,
        })
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("Search posting requires {key}"))
}

fn provider_id(value: &Value) -> Result<String, String> {
    match value.get("id") {
        Some(Value::String(id)) if !id.trim().is_empty() => Ok(id.clone()),
        Some(Value::Number(id)) if id.as_u64().is_some() => Ok(id.to_string()),
        _ => Err("Search posting requires a provider identity".into()),
    }
}

fn optional_amount(value: &Value, key: &str) -> Option<f64> {
    let value = value.get(key)?;
    let amount = value
        .as_f64()
        .or_else(|| value.as_str()?.parse::<f64>().ok())?;
    (amount.is_finite() && amount >= 0.0).then_some(amount)
}

fn jobicy_salary(value: &Value) -> Option<Salary> {
    // A malformed optional amount invalidates that pay range, not the posting.
    for key in ["salaryMin", "salaryMax"] {
        if value.get(key).is_some_and(|value| !value.is_null())
            && optional_amount(value, key).is_none()
        {
            return None;
        }
    }
    Salary::from_structured(
        optional_amount(value, "salaryMin"),
        optional_amount(value, "salaryMax"),
        value
            .get("salaryCurrency")
            .and_then(Value::as_str)
            .filter(|currency| {
                currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_alphabetic())
            }),
        value.get("salaryPeriod").and_then(Value::as_str),
        None,
    )
}

fn valid_jobicy_envelope(value: &Value) -> bool {
    let Some(jobs) = value.get("jobs").and_then(Value::as_array) else {
        return false;
    };
    matches!(
        (
            value.get("hasMore").and_then(Value::as_bool),
            value.get("nextCursor")
        ),
        (Some(false), Some(Value::Null))
    ) || matches!((value.get("hasMore").and_then(Value::as_bool), value.get("nextCursor")),
            (Some(true), Some(Value::String(cursor))) if !cursor.trim().is_empty() && cursor.len() <= MAX_URL_BYTES && !jobs.is_empty())
}

fn publication(raw: Option<&str>, jobicy: bool) -> Result<Option<String>, String> {
    let Some(raw) = raw.filter(|raw| !raw.trim().is_empty()) else {
        return Ok(None);
    };
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Ok(Some(
            date.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
        ));
    }
    if jobicy {
        // Jobicy's documented pubDate is UTC, without an offset.
        if let Ok(date) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
            return Ok(Some(
                date.and_utc()
                    .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true),
            ));
        }
    }
    Err("Invalid provider original publication date".into())
}

fn adzuna_posting(value: Value) -> Result<Posting, String> {
    let location = value
        .get("location")
        .ok_or("Adzuna posting requires location")?;
    let display = location
        .get("display_name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mut locations = loc_enrich::parse(&display);
    if let Some(area) = location.get("area").and_then(Value::as_array) {
        // area is ordered country -> region -> county -> locality; use only
        // actual response evidence, never the request's US collection or where.
        if area.len() >= 3 {
            if let (Some(country), Some(region), Some(city)) = (
                area.first().and_then(Value::as_str),
                area.get(1).and_then(Value::as_str),
                area.last().and_then(Value::as_str),
            ) {
                if let Some(parsed) = loc_enrich::from_address(city, region, country) {
                    locations.push(parsed);
                }
            }
        }
    }
    let predicted = match value.get("salary_is_predicted") {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value != "0",
        Some(Value::Number(value)) => value.as_u64() != Some(0),
        // Missing prediction provenance is not evidence of advertised pay.
        _ => true,
    };
    Ok(Posting {
        external_id: Some(provider_id(&value)?),
        title: required_string(&value, "title")?,
        company: required_string(
            value
                .get("company")
                .ok_or("Adzuna posting requires company")?,
            "display_name",
        )?,
        url: required_string(&value, "redirect_url")?,
        description: value
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_owned),
        location: display,
        locations,
        remote: None,
        job_types: Vec::new(),
        date_posted: publication(value.get("created").and_then(Value::as_str), false)?,
        // Public numeric fields establish neither explicit currency nor native interval.
        salary: if predicted {
            None
        } else {
            Salary::from_structured(
                optional_amount(&value, "salary_min"),
                optional_amount(&value, "salary_max"),
                None,
                None,
                None,
            )
        },
        salary_from_description: !predicted,
        prefer_structured_salary: false,
    })
}

fn jobicy_posting(value: Value) -> Result<Posting, String> {
    let location = value
        .get("jobGeo")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let job_types = match value.get("jobType") {
        Some(Value::Array(types)) => types
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(Value::String(value)) => vec![value.clone()],
        _ => Vec::new(),
    };
    Ok(Posting {
        external_id: Some(provider_id(&value)?),
        title: required_string(&value, "jobTitle")?,
        company: required_string(&value, "companyName")?,
        url: required_string(&value, "url")?,
        description: value
            .get("jobDescription")
            .and_then(Value::as_str)
            .map(str::to_owned),
        locations: loc_enrich::parse(&location),
        location,
        remote: Some(true),
        job_types,
        date_posted: publication(value.get("pubDate").and_then(Value::as_str), true)?,
        salary: jobicy_salary(&value),
        prefer_structured_salary: true,
        salary_from_description: true,
    })
}

fn replace_query(url: &Url, key: &str, value: &str) -> Url {
    let pairs: Vec<_> = url
        .query_pairs()
        .filter(|(name, _)| name != key)
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    let mut next = url.clone();
    next.set_query(None);
    next.query_pairs_mut()
        .extend_pairs(pairs)
        .append_pair(key, value);
    next
}
fn http_url(raw: &str) -> Result<Url, String> {
    if raw.len() > MAX_URL_BYTES {
        return Err("Search URL exceeds 8192 bytes".into());
    }
    let url = Url::parse(raw).map_err(|e| format!("Invalid search HTTP(S) URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Search URLs must be HTTP(S), with a host and without credentials".into());
    }
    Ok(url)
}

fn page_number(url: &Url, default: u64) -> Result<u64, String> {
    let mut pages = url.query_pairs().filter(|(key, _)| key == "page");
    let page = match pages.next() {
        Some((_, value)) => value
            .parse::<u64>()
            .ok()
            .ok_or("Search pagination requires a nonnegative integer page")?,
        None => default,
    };
    if pages.next().is_some() {
        return Err("Search pagination contains duplicate page parameters".into());
    }
    Ok(page)
}

pub(crate) enum SearchStep {
    Event(Box<WorkerEvent>),
    Yield,
    Exhausted,
}

pub(crate) struct SearchSession {
    source_id: String,
    provider: Provider,
    endpoint: Url,
    current: Url,
    next: Option<String>,
    candidates: std::vec::IntoIter<Value>,
    visited: HashSet<String>,
    pages: u32,
    max_pages: u32,
    deadline: Instant,
    exhausted: bool,
    cache: Option<SearchCache>,
    pending_error: Option<String>,
}

impl SearchSession {
    pub fn new(
        source: &WorkerSource,
        limits: &WorkerLimits,
        workflow_deadline: Instant,
    ) -> Result<Self, ClientError> {
        let identifier = scrape_core::search_source::identifier(&source.identifier)
            .map_err(ClientError::Configuration)?;
        let provider = match identifier {
            "arbeitnow" => Provider::Arbeitnow,
            "themuse" => Provider::Muse,
            "adzuna" => Provider::Adzuna,
            "jobicy" => Provider::Jobicy,
            _ => return Err(ClientError::Configuration("Unknown search provider".into())),
        };
        scrape_core::search_source::validate_runtime_config(&source.config)
            .map_err(ClientError::Configuration)?;
        let endpoint = scrape_core::search_source::endpoint(identifier, &source.config)
            .map_err(ClientError::Configuration)?;
        let mut endpoint = http_url(endpoint.as_str()).map_err(ClientError::Configuration)?;
        let cache = if matches!(provider, Provider::Jobicy) {
            endpoint.query_pairs_mut().append_pair("count", "100");
            let directory = source
                .config
                .get("__provider_cache_dir")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ClientError::Configuration(
                        "Jobicy setup required: application cache directory is missing".into(),
                    )
                })?;
            Some(SearchCache::new(directory)?)
        } else {
            None
        };
        if matches!(provider, Provider::Adzuna) {
            let credentials = source.config.get("__provider_credentials")
                .ok_or_else(|| ClientError::Configuration("Adzuna setup required: configure App ID and App key in Settings > System > Job providers.".into()))?;
            endpoint
                .query_pairs_mut()
                .append_pair("app_id", credentials["app_id"].as_str().unwrap())
                .append_pair("app_key", credentials["app_key"].as_str().unwrap())
                .append_pair("results_per_page", "50")
                .append_pair("sort_by", "date");
            for key in ["what_or", "where", "max_days_old"] {
                if let Some(value) = source.config.get(key) {
                    if let Some(value) = value.as_str() {
                        endpoint.query_pairs_mut().append_pair(key, value);
                    } else if let Some(value) = value.as_u64() {
                        endpoint
                            .query_pairs_mut()
                            .append_pair(key, &value.to_string());
                    }
                }
            }
        }
        let initial_page = page_number(
            &endpoint,
            match provider {
                Provider::Arbeitnow => 1,
                Provider::Muse => 0,
                Provider::Adzuna | Provider::Jobicy => 1,
            },
        )
        .map_err(ClientError::Configuration)?;
        if matches!(provider, Provider::Arbeitnow) && initial_page == 0 {
            return Err(ClientError::Configuration(
                "Arbeitnow pages start at 1".into(),
            ));
        }
        let overrides =
            scrape_core::extraction::limits(&source.config).map_err(ClientError::Configuration)?;
        let max_pages = source
            .config
            .get("max_pages")
            .and_then(Value::as_u64)
            .unwrap_or(scrape_core::search_source::DEFAULT_MAX_PAGES as u64)
            as u32;
        let duration = overrides
            .max_source_duration_s
            .min(limits.max_source_duration_s);
        let workflow_duration = overrides
            .max_workflow_duration_s
            .min(limits.max_workflow_duration_s);
        let now = Instant::now();
        Ok(Self {
            source_id: source.id.clone(),
            provider,
            current: endpoint.clone(),
            next: Some(endpoint.to_string()),
            endpoint,
            candidates: Vec::new().into_iter(),
            visited: HashSet::new(),
            pages: 0,
            max_pages: max_pages.min(limits.max_pages_per_source),
            deadline: workflow_deadline
                .min(now + Duration::from_secs(duration))
                .min(now + Duration::from_secs(workflow_duration)),
            exhausted: false,
            cache,
            pending_error: None,
        })
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted
            || (self.pages > 0
                && self.next.is_none()
                && self.candidates.len() == 0
                && self.pending_error.is_none())
    }

    fn stop(&mut self, termination: CoverageTermination, message: &str) -> SearchStep {
        self.exhausted = true;
        self.candidates = Vec::new().into_iter();
        SearchStep::Event(Box::new(WorkerEvent::Warning {
            source_id: self.source_id.clone(),
            message: message.into(),
            eligibility_incomplete: true,
            termination: Some(termination),
        }))
    }

    fn continuation(&self, raw: &str) -> Result<Url, ClientError> {
        if matches!(self.provider, Provider::Adzuna | Provider::Jobicy) {
            let url = Url::parse(raw)
                .map_err(|_| ClientError::Extraction("Invalid provider continuation".into()))?;
            // These continuations are constructed locally from validated metadata,
            // never accepted as provider URLs.
            if url.origin() != self.endpoint.origin() || self.visited.contains(url.as_str()) {
                return Err(ClientError::Extraction(
                    "Repeated provider pagination cursor".into(),
                ));
            }
            return Ok(url);
        }
        if raw.len() > MAX_URL_BYTES {
            return Err(ClientError::Extraction(
                "Search pagination URL exceeds 8192 bytes".into(),
            ));
        }
        let joined = self.current.join(raw)?;
        let url = http_url(joined.as_str()).map_err(ClientError::Extraction)?;
        // Both feeds paginate the same collection with ?page=. Never let feed
        // content turn this adapter into an arbitrary same-origin URL crawler.
        let unchanged_query = |url: &Url| {
            url.query_pairs()
                .filter(|(key, _)| key != "page")
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect::<Vec<_>>()
        };
        if url.origin() != self.endpoint.origin()
            || url.path() != self.endpoint.path()
            || url.fragment().is_some()
            || unchanged_query(&url) != unchanged_query(&self.endpoint)
            || page_number(&url, 1).map_err(ClientError::Extraction)?
                != page_number(
                    &self.current,
                    match self.provider {
                        Provider::Arbeitnow => 1,
                        Provider::Muse => 0,
                        Provider::Adzuna | Provider::Jobicy => 1,
                    },
                )
                .map_err(ClientError::Extraction)?
                .saturating_add(1)
            || self.visited.contains(url.as_str())
        {
            return Err(ClientError::Extraction(
                "Unsafe or cyclic search pagination continuation".into(),
            ));
        }
        Ok(url)
    }

    pub async fn next(&mut self, client: &HttpClient) -> Result<SearchStep, ClientError> {
        if self.is_exhausted() {
            return Ok(SearchStep::Exhausted);
        }
        if Instant::now() >= self.deadline {
            return Ok(self.stop(
                CoverageTermination::TimeLimit,
                "Search source/workflow time limit reached; feed coverage is incomplete",
            ));
        }
        if let Some(value) = self.candidates.next() {
            let posting = match self.provider {
                Provider::Arbeitnow => serde_json::from_value::<SearchItem>(value)
                    .map(SearchItem::posting)
                    .map_err(|error| format!("Invalid Arbeitnow posting: {error}")),
                Provider::Muse => serde_json::from_value::<MuseItem>(value)
                    .map_err(|error| format!("Invalid Muse posting: {error}"))
                    .and_then(MuseItem::posting),
                Provider::Adzuna => {
                    adzuna_posting(value).map_err(|_| "Invalid Adzuna posting".into())
                }
                Provider::Jobicy => jobicy_posting(value),
            };
            let event = posting
                .and_then(|posting| posting.event(&self.source_id))
                .unwrap_or_else(|message| WorkerEvent::Error {
                    source_id: self.source_id.clone(),
                    message,
                    retryable: false,
                    invalid_posting: Some(true),
                });
            return Ok(SearchStep::Event(Box::new(event)));
        }
        if let Some(message) = self.pending_error.take() {
            return Err(ClientError::Extraction(message));
        }
        if self.pages >= self.max_pages {
            return Ok(self.stop(
                CoverageTermination::PageLimit,
                "Search page limit reached; remaining feed pages were not visited",
            ));
        }
        let Some(raw) = self.next.take() else {
            return Ok(SearchStep::Exhausted);
        };
        let url = if self.pages == 0 {
            self.endpoint.clone()
        } else {
            self.continuation(&raw)?
        };
        self.visited.insert(url.to_string());
        let response = timeout_at(self.deadline, async {
            match self.provider {
                Provider::Adzuna => client.fetch_adzuna_json(&url).await,
                Provider::Jobicy => {
                    let cache = self.cache.as_ref().expect("Jobicy cache configured");
                    if let Some(value) = cache.read(&url).await? {
                        // Cache bytes must pass the same collection validation as live pages.
                        if valid_jobicy_envelope(&value) {
                            return Ok(value);
                        }
                    }
                    let mut value: Value = client.fetch_json(url.as_str()).await?;
                    if valid_jobicy_envelope(&value) {
                        value = cache.store(&url, value).await?;
                    }
                    Ok(value)
                }
                _ => {
                    let (body, actual) = client.fetch_text_at(url.as_str(), false).await?;
                    let actual = http_url(&actual).map_err(ClientError::Extraction)?;
                    if actual.origin() != self.endpoint.origin()
                        || actual.path() != self.endpoint.path()
                    {
                        return Err(ClientError::Extraction(
                            "Search feed redirected outside its configured collection".into(),
                        ));
                    }
                    Ok(serde_json::from_str(&body)?)
                }
            }
        })
        .await;
        let value = match response {
            Err(_) => return Ok(self.stop(CoverageTermination::TimeLimit,
                "Search source/workflow time limit reached during a feed request; coverage is incomplete")),
            Ok(result) => result?,
        };
        let (candidates, next) = match self.provider {
            Provider::Arbeitnow => {
                let response: SearchResponse = serde_json::from_value(value)?;
                (response.data, response.links.and_then(|links| links.next))
            }
            Provider::Muse => {
                let response: MuseResponse = serde_json::from_value(value)?;
                let requested_page = page_number(&url, 0).map_err(ClientError::Extraction)?;
                if response.page != requested_page
                    || (response.page_count > 0 && response.page >= response.page_count)
                    || (response.page_count == 0 && !response.results.is_empty())
                {
                    return Err(ClientError::Extraction(
                        "Invalid Muse page/page_count envelope".into(),
                    ));
                }
                let next = if response.page < response.page_count.saturating_sub(1) {
                    let mut next = url.clone();
                    let pairs: Vec<_> = url
                        .query_pairs()
                        .filter(|(key, _)| key != "page")
                        .map(|(key, value)| (key.into_owned(), value.into_owned()))
                        .collect();
                    next.set_query(None);
                    next.query_pairs_mut()
                        .extend_pairs(pairs)
                        .append_pair("page", &(response.page + 1).to_string());
                    Some(next.into())
                } else {
                    None
                };
                (response.results, next)
            }
            Provider::Adzuna => {
                let mut value = value;
                let candidates: Vec<Value> = serde_json::from_value(
                    value.get_mut("results").map(Value::take).ok_or_else(|| {
                        ClientError::Extraction(
                            "Adzuna response requires results collection".into(),
                        )
                    })?,
                )
                .map_err(|_| {
                    ClientError::Extraction("Adzuna response requires results collection".into())
                })?;
                let page = url
                    .path()
                    .rsplit('/')
                    .next()
                    .and_then(|page| page.parse::<u64>().ok())
                    .unwrap_or(1);
                let next = match value.get("count").and_then(Value::as_u64) {
                    Some(count) if count < candidates.len() as u64 => {
                        self.pending_error = Some("Invalid Adzuna count metadata".into());
                        None
                    }
                    Some(count) if !candidates.is_empty() && page.saturating_mul(50) < count => {
                        let mut next = url.clone();
                        next.set_path(&format!("/v1/api/jobs/us/search/{}", page + 1));
                        Some(next.into())
                    }
                    Some(_) => None,
                    None => {
                        self.pending_error = Some("Invalid Adzuna count metadata".into());
                        None
                    }
                };
                (candidates, next)
            }
            Provider::Jobicy => {
                let mut value = value;
                let candidates: Vec<Value> = serde_json::from_value(
                    value.get_mut("jobs").map(Value::take).ok_or_else(|| {
                        ClientError::Extraction("Jobicy response requires jobs collection".into())
                    })?,
                )
                .map_err(|_| {
                    ClientError::Extraction("Jobicy response requires jobs collection".into())
                })?;
                let next = match (
                    value.get("hasMore").and_then(Value::as_bool),
                    value.get("nextCursor"),
                ) {
                    (Some(true), Some(Value::String(cursor)))
                        if !cursor.trim().is_empty()
                            && cursor.len() <= MAX_URL_BYTES
                            && !candidates.is_empty() =>
                    {
                        let next = replace_query(&url, "cursor", cursor);
                        if self.visited.contains(next.as_str()) {
                            self.pending_error = Some("Repeated Jobicy pagination cursor".into());
                            None
                        } else {
                            Some(next.into())
                        }
                    }
                    (Some(false), Some(Value::Null)) => None,
                    _ => {
                        self.pending_error =
                            Some("Invalid Jobicy hasMore/nextCursor pagination metadata".into());
                        None
                    }
                };
                (candidates, next)
            }
        };
        self.pages += 1;
        self.current = url;
        self.next = next;
        self.candidates = candidates.into_iter();
        // A page request is a turn of its own, even if it contains no candidates.
        Ok(SearchStep::Yield)
    }
}

impl Posting {
    fn event(self, source_id: &str) -> Result<WorkerEvent, String> {
        if self.title.trim().is_empty() || self.company.trim().is_empty() {
            return Err("Search posting requires a nonempty title and employer name".into());
        }
        let destination = http_url(self.url.trim())?;
        let external_id = match self.external_id.filter(|id| !id.trim().is_empty()) {
            Some(id) => id,
            None => scrape_core::identity::normalized_detail_url(destination.as_str())?,
        };
        let location = self.location;
        let description_source = self.description.filter(|raw| !raw.trim().is_empty());
        let description = description_source
            .as_deref()
            .map(|raw| normalize_description_with_format(raw, DescriptionFormat::Html, 32768));
        let text = description
            .as_ref()
            .map(|d| d.plain_text.as_str())
            .unwrap_or("");
        let mut locations = self.locations;
        let sections = description
            .as_ref()
            .map(loc_enrich::from_description_sections);
        let location_data_complete = loc_enrich::has_geographic_value(&locations)
            || sections.as_ref().is_some_and(|e| e.section_found);
        if let Some(sections) = sections {
            locations.extend(sections.locations);
        }
        let workplace_type = if self.remote == Some(true) {
            WorkplaceType::Remote
        } else {
            classify_workplace(None, &location, Some(text))
        };
        let salary = if self.prefer_structured_salary {
            self.salary.or_else(|| {
                self.salary_from_description
                    .then(|| Salary::from_text(text))
                    .flatten()
            })
        } else if self.salary_from_description {
            Salary::from_text(text).or(self.salary)
        } else {
            self.salary
        };
        let sponsorship = (!text.is_empty()).then(|| classify_sponsorship(text));
        let clearance = (!text.is_empty()).then(|| classify_clearance(text));
        let work_auth = (!text.is_empty()).then(|| classify_work_auth(text));
        let experience = extract_experience(&self.title, text);
        let is_new_grad = experience.is_new_grad;
        let role_type = RoleType::infer(&self.title, self.job_types.first().map(String::as_str));
        let description_text = (!text.is_empty()).then(|| text.to_owned());
        let description_format = description_source.as_ref().map(|_| DescriptionFormat::Html);
        Ok(WorkerEvent::Job {
            source_id: source_id.into(),
            external_id,
            title: self.title,
            company: self.company,
            location,
            url: destination.into(),
            salary,
            role_type,
            workplace_type,
            date_posted: self.date_posted,
            provider_updated_at: None,
            locations,
            location_data_complete,
            sponsorship,
            clearance,
            work_auth,
            experience: Some(experience),
            is_new_grad,
            description_text,
            description,
            description_source,
            description_format,
            company_icon_url: None,
            company_logo: None,
        })
    }
}

pub async fn scrape_search_with(
    client: &HttpClient,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    let limits = WorkerLimits::default();
    let mut session = SearchSession::new(
        source,
        &limits,
        Instant::now() + Duration::from_secs(limits.max_workflow_duration_s),
    )?;
    loop {
        match session.next(client).await? {
            SearchStep::Event(event) => {
                if !sink(*event) {
                    break;
                }
            }
            SearchStep::Yield => {}
            SearchStep::Exhausted => break,
        }
    }
    Ok(())
}
