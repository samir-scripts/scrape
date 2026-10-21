//! Public cross-employer feeds. Matching is local, not provider-side search.

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
        })
    }
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
            _ => return Err(ClientError::Configuration("Unknown search provider".into())),
        };
        scrape_core::search_source::validate_config(&source.config)
            .map_err(ClientError::Configuration)?;
        let endpoint = scrape_core::search_source::endpoint(identifier, &source.config)
            .map_err(ClientError::Configuration)?;
        let endpoint = http_url(endpoint.as_str()).map_err(ClientError::Configuration)?;
        let initial_page = page_number(
            &endpoint,
            match provider {
                Provider::Arbeitnow => 1,
                Provider::Muse => 0,
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
        })
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted || (self.pages > 0 && self.next.is_none() && self.candidates.len() == 0)
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
        let response = timeout_at(self.deadline, client.fetch_text_at(url.as_str(), false)).await;
        let (body, actual) = match response {
            Err(_) => return Ok(self.stop(CoverageTermination::TimeLimit,
                "Search source/workflow time limit reached during a feed request; coverage is incomplete")),
            Ok(result) => result?,
        };
        let actual = http_url(&actual).map_err(ClientError::Extraction)?;
        if actual.origin() != self.endpoint.origin() || actual.path() != self.endpoint.path() {
            return Err(ClientError::Extraction(
                "Search feed redirected outside its configured collection".into(),
            ));
        }
        let (candidates, next) = match self.provider {
            Provider::Arbeitnow => {
                let response: SearchResponse = serde_json::from_str(&body)?;
                (response.data, response.links.and_then(|links| links.next))
            }
            Provider::Muse => {
                let response: MuseResponse = serde_json::from_str(&body)?;
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
        let salary = Salary::from_text(text);
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
