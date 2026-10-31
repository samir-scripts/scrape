use super::branding::CompanyBranding;
use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, Utc};
use regex::Regex;
use scrape_core::enrich::{
    extract_experience, location as loc_enrich, normalize_description_with_format,
    DescriptionFormat,
};
use scrape_core::matching::{MatchingEvaluator, MatchingEvidence};
use scrape_core::protocol::{CoverageTermination, EnrichmentLimits};
use scrape_core::{
    RoleType, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, WorkplaceType,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::LazyLock;
use tokio::time::{timeout_at, Duration, Instant};
use url::Url;

static WORKDAY_LOCALE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z]{2}-[A-Z]{2}$").expect("valid regex"));

static WORKDAY_DAYS_AGO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"posted\s+(\d+)\s+days?\s+ago").expect("valid regex"));

static WORKDAY_MULTI_LOCATIONS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\d+\s+locations?$").expect("valid regex"));
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkdayEndpoint {
    pub origin: String,
    pub tenant: String,
    pub site: String,
}

impl WorkdayEndpoint {
    pub fn parse(raw: &str) -> Result<Self, ClientError> {
        let url = Url::parse(raw)?;
        let host = url
            .host_str()
            .ok_or_else(|| ClientError::Configuration("Missing host in Workday URL".into()))?;
        if !host.ends_with(".myworkdayjobs.com") {
            return Err(ClientError::Configuration(
                "Workday URL must end with .myworkdayjobs.com".into(),
            ));
        }
        let tenant = host
            .split('.')
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ClientError::Configuration("Invalid Workday tenant subdomain".into()))?
            .to_string();

        let mut segments = url
            .path_segments()
            .ok_or_else(|| ClientError::Configuration("Invalid Workday path".into()))?
            .filter(|s| !s.is_empty());

        let first = segments.next().ok_or_else(|| {
            ClientError::Configuration("Missing Workday site segment in URL".into())
        })?;

        let site = if WORKDAY_LOCALE_RE.is_match(first) {
            segments.next().ok_or_else(|| {
                ClientError::Configuration(
                    "Missing Workday site segment after locale in URL".into(),
                )
            })?
        } else {
            first
        }
        .to_string();

        let origin = format!("{}://{}", url.scheme(), host);
        Ok(Self {
            origin,
            tenant,
            site,
        })
    }

    pub fn list_url(&self) -> String {
        format!(
            "{}/wday/cxs/{}/{}/jobs",
            self.origin, self.tenant, self.site
        )
    }

    pub fn detail_url(&self, external_path: &str) -> String {
        let path = external_path.trim_start_matches("/job/");
        format!(
            "{}/wday/cxs/{}/{}/job/{}",
            self.origin, self.tenant, self.site, path
        )
    }

    pub fn apply_url(&self, external_path: &str) -> String {
        format!("{}/{}{}", self.origin, self.site, external_path)
    }
}

#[derive(Debug, Clone, Serialize)]
struct WorkdayListPayload {
    #[serde(rename = "appliedFacets")]
    applied_facets: serde_json::Value,
    limit: u32,
    offset: u64,
    #[serde(rename = "searchText")]
    search_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkdayListResponse {
    pub total: Option<u64>,
    #[serde(rename = "jobPostings", default)]
    pub job_postings: Vec<WorkdayJobPosting>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkdayJobPosting {
    pub title: Option<String>,
    #[serde(rename = "externalPath")]
    pub external_path: Option<String>,
    #[serde(rename = "locationsText")]
    pub locations_text: Option<String>,
    #[serde(rename = "postedOn")]
    pub posted_on: Option<String>,
    #[serde(rename = "bulletFields", default)]
    pub bullet_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkdayDetailResponse {
    #[serde(rename = "jobPostingInfo")]
    pub job_posting_info: Option<WorkdayJobPostingInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkdayJobPostingInfo {
    pub id: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "jobDescription")]
    pub job_description: Option<String>,
    pub location: Option<String>,
    #[serde(rename = "locationsText")]
    pub locations_text: Option<String>,
    #[serde(rename = "additionalLocations", default)]
    pub additional_locations: Vec<String>,
    #[serde(rename = "postedOn")]
    pub posted_on: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "timeType")]
    pub time_type: Option<String>,
    #[serde(rename = "jobReqId")]
    pub job_req_id: Option<String>,
    #[serde(rename = "externalUrl")]
    pub external_url: Option<String>,
    pub country: Option<WorkdayDescriptor>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkdayDescriptor {
    pub descriptor: Option<String>,
    pub id: Option<String>,
}

pub fn parse_workday_relative_date(text: &str, now: DateTime<Utc>) -> Option<String> {
    let lower = text.trim().to_ascii_lowercase();
    if lower.contains("today") {
        Some(now.format("%Y-%m-%d").to_string())
    } else if lower.contains("yesterday") {
        Some(
            (now - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string(),
        )
    } else if let Some(caps) = WORKDAY_DAYS_AGO_RE.captures(&lower) {
        let days: i64 = caps.get(1)?.as_str().parse().ok()?;
        Some(
            (now - chrono::Duration::days(days))
                .format("%Y-%m-%d")
                .to_string(),
        )
    } else {
        None
    }
}

pub fn workplace_from_locations(locations: &[&str]) -> WorkplaceType {
    for loc in locations {
        let lower = loc.to_ascii_lowercase();
        if lower.contains("remote") || lower.contains("virtual") || lower.contains("work from home")
        {
            return WorkplaceType::Remote;
        }
        if lower.contains("hybrid") {
            return WorkplaceType::Hybrid;
        }
    }
    if locations.iter().any(|loc| {
        let trimmed = loc.trim();
        !trimmed.is_empty() && !WORKDAY_MULTI_LOCATIONS_RE.is_match(trimmed)
    }) {
        WorkplaceType::Onsite
    } else {
        WorkplaceType::Unknown
    }
}

fn posting_to_event(
    source: &WorkerSource,
    endpoint: &WorkdayEndpoint,
    posting: WorkdayJobPosting,
    branding: &CompanyBranding,
    now: DateTime<Utc>,
) -> Option<WorkerEvent> {
    let title = posting.title.filter(|t| !t.trim().is_empty())?;
    let ext_path = posting.external_path.unwrap_or_default();
    let external_id = posting
        .bullet_fields
        .first()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| ext_path.trim_start_matches('/').replace('/', "_"));
    if external_id.is_empty() {
        return None;
    }

    let apply_url = endpoint.apply_url(&ext_path);
    let loc_text = posting.locations_text.unwrap_or_default();
    let date_posted = posting
        .posted_on
        .as_deref()
        .and_then(|p| parse_workday_relative_date(p, now));

    let role_type = RoleType::infer(&title, None);
    let workplace_type = workplace_from_locations(&[&loc_text]);

    let company = super::branding::employer_hint(source)
        .unwrap_or(&source.name)
        .to_string();

    let locations = if WORKDAY_MULTI_LOCATIONS_RE.is_match(loc_text.trim()) {
        Vec::new()
    } else {
        loc_enrich::parse(&loc_text)
    };
    let location_data_complete = loc_enrich::has_geographic_value(&locations);
    let experience = extract_experience(&title, "");
    let is_new_grad = experience.is_new_grad;

    let event = WorkerEvent::Job {
        discovery_provenance: None,
        source_id: source.id.clone(),
        external_id,
        title,
        url: apply_url,
        company,
        location: loc_text,
        salary: None,
        role_type,
        workplace_type,
        date_posted,
        provider_updated_at: None,
        locations,
        location_data_complete,
        sponsorship: None,
        clearance: None,
        work_auth: None,
        experience: Some(experience),
        is_new_grad,
        description_text: None,
        description: None,
        description_source: None,
        description_format: None,
        company_icon_url: None,
        company_logo: None,
    };

    Some(branding.apply(event))
}
/// Only detail evidence is merged: listing identity and destination remain authoritative.
fn merge_workday_detail(
    event: &mut WorkerEvent,
    detail: WorkdayDetailResponse,
    listing_requisition: Option<&str>,
) -> Result<bool, String> {
    let Some(info) = detail.job_posting_info else {
        return Err(
            "Workday detail has no jobPostingInfo; missing evidence remains unknown".into(),
        );
    };
    let WorkerEvent::Job {
        title,
        url,
        location,
        locations,
        location_data_complete,
        workplace_type,
        date_posted,
        salary,
        sponsorship,
        clearance,
        work_auth,
        role_type,
        experience,
        is_new_grad,
        description_text,
        description,
        description_source,
        description_format,
        ..
    } = event
    else {
        return Ok(false);
    };
    if listing_requisition
        .zip(info.job_req_id.as_deref())
        .is_some_and(|(listing, detail)| listing != detail)
    {
        return Err(
            "Workday detail requisition identity differs from listing; detail rejected".into(),
        );
    }
    if let Some(detail_url) = info.external_url.as_deref() {
        let same_posting = Url::parse(url)
            .ok()
            .zip(Url::parse(detail_url).ok())
            .is_some_and(|(listing, detail)| {
                listing.origin() == detail.origin()
                    && listing.path().split_once("/job/").map(|(_, path)| path)
                        == detail.path().split_once("/job/").map(|(_, path)| path)
                    && listing.path().contains("/job/")
            });
        if !same_posting {
            return Err("Workday detail destination differs from listing; detail rejected".into());
        }
    }
    *role_type = RoleType::infer(title, info.time_type.as_deref());
    if date_posted.is_none() {
        *date_posted = info
            .start_date
            .as_deref()
            .filter(|d| !d.trim().is_empty())
            .map(str::to_owned);
    }
    let mut detail_loc_strings: Vec<String> = Vec::new();
    for candidate_loc in info
        .location
        .iter()
        .chain(info.locations_text.iter())
        .chain(info.additional_locations.iter())
    {
        let trimmed = candidate_loc.trim();
        if !trimmed.is_empty()
            && !WORKDAY_MULTI_LOCATIONS_RE.is_match(trimmed)
            && !detail_loc_strings
                .iter()
                .any(|existing| existing == trimmed)
        {
            detail_loc_strings.push(trimmed.to_string());
        }
    }
    if !detail_loc_strings.is_empty() {
        if location.trim().is_empty() || WORKDAY_MULTI_LOCATIONS_RE.is_match(location.trim()) {
            *location = detail_loc_strings.join("; ");
        }
        let country_hint = info
            .country
            .as_ref()
            .and_then(|c| c.id.as_deref().or(c.descriptor.as_deref()));
        let parsed_detail: Vec<_> = detail_loc_strings
            .iter()
            .flat_map(|label| loc_enrich::parse_with_country(label, country_hint))
            .collect();
        if locations.is_empty() && !parsed_detail.is_empty() {
            *locations = parsed_detail;
        } else {
            for loc in parsed_detail {
                if !locations.contains(&loc) {
                    locations.push(loc);
                }
            }
        }
    }
    let detail_loc_refs: Vec<&str> = detail_loc_strings.iter().map(String::as_str).collect();
    if let Some(raw) = info.job_description.filter(|raw| !raw.trim().is_empty()) {
        let normalized = normalize_description_with_format(&raw, DescriptionFormat::Html, 32768);
        let classified = extract_experience(title, &normalized.plain_text);
        *is_new_grad = classified.is_new_grad;
        *experience = Some(classified);
        let section_locations = loc_enrich::from_description_sections(&normalized);
        *location_data_complete =
            loc_enrich::has_geographic_value(locations) || section_locations.section_found;
        locations.extend(section_locations.locations);
        if *workplace_type == WorkplaceType::Unknown {
            let from_desc = scrape_core::enrich::classify_workplace(
                None,
                location,
                Some(&normalized.plain_text),
            );
            *workplace_type = if from_desc != WorkplaceType::Unknown {
                from_desc
            } else if !detail_loc_refs.is_empty() {
                workplace_from_locations(&detail_loc_refs)
            } else {
                WorkplaceType::Unknown
            };
        }
        if salary.is_none() {
            *salary = scrape_core::enrich::Salary::from_text(&normalized.plain_text);
        }
        *sponsorship = Some(scrape_core::enrich::classify_sponsorship(
            &normalized.plain_text,
        ));
        *clearance = Some(scrape_core::enrich::classify_clearance(
            &normalized.plain_text,
        ));
        *work_auth = Some(scrape_core::enrich::classify_work_auth(
            &normalized.plain_text,
        ));
        *description_text = Some(normalized.plain_text.clone());
        *description = Some(normalized);
        *description_source = Some(raw);
        *description_format = Some(DescriptionFormat::Html);
    } else if !detail_loc_refs.is_empty() {
        if *workplace_type == WorkplaceType::Unknown {
            *workplace_type = workplace_from_locations(&detail_loc_refs);
        }
        *location_data_complete = loc_enrich::has_geographic_value(locations);
    }
    Ok(true)
}

pub async fn scrape_workday(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    scrape_workday_with(client, source, &mut |event| {
        events.push(event);
        true
    })
    .await?;
    Ok(events)
}

pub async fn scrape_workday_with(
    client: &HttpClient,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    let endpoint = WorkdayEndpoint::parse(&source.identifier)?;
    let now = Utc::now();
    let branding = CompanyBranding::fetch_source(
        client,
        &source.identifier,
        super::branding::employer_hint(source).unwrap_or(""),
        source,
    )
    .await;

    if let Some(warning) = branding.warning(&source.id) {
        if !sink(warning) {
            return Ok(());
        }
    }

    let list_url = endpoint.list_url();
    let mut offset = 0u64;
    let limit = 20u32;
    let mut total: Option<u64> = None;

    loop {
        let payload = WorkdayListPayload {
            applied_facets: serde_json::json!({}),
            limit,
            offset,
            search_text: String::new(),
        };

        let response: WorkdayListResponse = client.post_json(&list_url, &payload).await?;
        if total.is_none() {
            total = response.total;
            if let Some(t) = total {
                if !sink(WorkerEvent::SourceInfo {
                    source_id: source.id.clone(),
                    provider_total: Some(t),
                }) {
                    return Ok(());
                }
            }
        }

        let count = response.job_postings.len();
        if count == 0 {
            break;
        }

        for posting in response.job_postings {
            if let Some(event) = posting_to_event(source, &endpoint, posting, &branding, now) {
                if !sink(event) {
                    return Ok(());
                }
            }
        }

        offset += count as u64;
        if let Some(t) = total {
            if offset >= t {
                break;
            }
        }
    }

    Ok(())
}

pub(crate) enum WorkdayStep {
    Event(Box<(WorkerEvent, Option<MatchingEvidence>)>),
    Yield,
    Exhausted,
}

pub(crate) struct WorkdaySession {
    endpoint: WorkdayEndpoint,
    source: WorkerSource,
    offset: u64,
    limit: u32,
    total: Option<u64>,
    exhausted: bool,
    candidates: VecDeque<WorkdayJobPosting>,
    branding: CompanyBranding,
    branding_emitted: bool,
    pending: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
    detail_budget: Duration,
    source_deadline: Instant,
    workflow_deadline: Instant,
    pages_visited: u32,
    max_pages: u32,
    detail_limit: u32,
    detail_requests: u32,
    detail_bound_reported: bool,
    seen_details: std::collections::HashSet<String>,
}

impl WorkdaySession {
    pub async fn new(
        client: &HttpClient,
        source: &WorkerSource,
        limits: &WorkerLimits,
        enrichment: &EnrichmentLimits,
        workflow_deadline: Instant,
    ) -> Result<Self, ClientError> {
        let endpoint = WorkdayEndpoint::parse(&source.identifier)?;
        let overrides =
            scrape_core::extraction::limits(&source.config).map_err(ClientError::Configuration)?;
        let max_pages = (source
            .config
            .get("max_pages")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(scrape_core::search_source::DEFAULT_MAX_PAGES as u64)
            as u32)
            .min(limits.max_pages_per_source);
        let duration_s = overrides
            .max_source_duration_s
            .min(limits.max_source_duration_s);
        let source_deadline =
            workflow_deadline.min(Instant::now() + Duration::from_secs(duration_s));
        let branding = CompanyBranding::fetch_source(
            client,
            &source.identifier,
            super::branding::employer_hint(source).unwrap_or(""),
            source,
        )
        .await;

        let mut session = Self {
            endpoint,
            source: source.clone(),
            offset: 0,
            limit: 20,
            total: None,
            exhausted: false,
            candidates: VecDeque::new(),
            branding,
            branding_emitted: false,
            pending: VecDeque::new(),
            detail_budget: Duration::from_millis(enrichment.timeout_ms),
            source_deadline,
            workflow_deadline,
            pages_visited: 0,
            max_pages,
            detail_limit: enrichment.max_requests,
            detail_requests: 0,
            detail_bound_reported: false,
            seen_details: std::collections::HashSet::new(),
        };

        session.fetch_next_page(client).await?;
        Ok(session)
    }

    pub fn total(&self) -> Option<u64> {
        self.total
    }

    pub fn detail_requests(&self) -> u32 {
        self.detail_requests
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted && self.candidates.is_empty() && self.pending.is_empty()
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    async fn fetch_next_page(&mut self, client: &HttpClient) -> Result<(), ClientError> {
        if self.exhausted {
            return Ok(());
        }
        if let Some(t) = self.total {
            if self.offset >= t {
                self.exhausted = true;
                return Ok(());
            }
        }

        let list_url = self.endpoint.list_url();
        let payload = WorkdayListPayload {
            applied_facets: serde_json::json!({}),
            limit: self.limit,
            offset: self.offset,
            search_text: String::new(),
        };

        let response: WorkdayListResponse = timeout_at(
            self.source_deadline.min(self.workflow_deadline),
            client.post_json(&list_url, &payload),
        )
        .await
        .map_err(|_| {
            ClientError::TimeLimit("Workday listing reached workflow time limit".into())
        })??;
        self.pages_visited += 1;
        if self.total.is_none() {
            self.total = response.total;
        }

        let count = response.job_postings.len();
        if count == 0 {
            self.exhausted = true;
            return Ok(());
        }

        self.offset += count as u64;
        self.candidates.extend(response.job_postings);

        if let Some(t) = self.total {
            if self.offset >= t {
                self.exhausted = true;
            }
        }

        Ok(())
    }

    pub async fn next(
        &mut self,
        client: &HttpClient,
        filters: &WorkerFilters,
        started: DateTime<Utc>,
    ) -> Result<WorkdayStep, ClientError> {
        if let Some(pending) = self.pending.pop_front() {
            return Ok(WorkdayStep::Event(Box::new(pending)));
        }

        if !self.branding_emitted {
            self.branding_emitted = true;
            if let Some(warning) = self.branding.warning(&self.source.id) {
                return Ok(WorkdayStep::Event(Box::new((warning, None))));
            }
        }

        if Instant::now() >= self.source_deadline {
            self.exhausted = true;
            self.candidates.clear();
            return Ok(WorkdayStep::Event(Box::new((
                WorkerEvent::Warning {
                    source_id: self.source.id.clone(),
                    message: "Workday source/workflow time limit reached; remaining board pages were not visited".into(),
                    eligibility_incomplete: true,
                    termination: Some(CoverageTermination::TimeLimit),
                },
                None,
            ))));
        }

        if self.candidates.is_empty() {
            if self.exhausted {
                return Ok(WorkdayStep::Exhausted);
            }
            if self.pages_visited >= self.max_pages {
                self.exhausted = true;
                return Ok(WorkdayStep::Event(Box::new((
                    WorkerEvent::Warning {
                        source_id: self.source.id.clone(),
                        message:
                            "Workday page limit reached; remaining board pages were not visited"
                                .into(),
                        eligibility_incomplete: true,
                        termination: Some(CoverageTermination::PageLimit),
                    },
                    None,
                ))));
            }
            self.fetch_next_page(client).await?;
            if self.candidates.is_empty() {
                return Ok(WorkdayStep::Exhausted);
            }
            return Ok(WorkdayStep::Yield);
        }

        let posting = self.candidates.pop_front().unwrap();
        let listing_requisition = posting
            .bullet_fields
            .first()
            .filter(|id| !id.trim().is_empty())
            .cloned();
        let detail_url = posting
            .external_path
            .as_deref()
            .map(|path| self.endpoint.detail_url(path));
        let Some(mut event) = posting_to_event(
            &self.source,
            &self.endpoint,
            posting,
            &self.branding,
            started,
        ) else {
            return Ok(WorkdayStep::Yield);
        };

        let matcher = MatchingEvaluator::new(filters).map_err(ClientError::Configuration)?;
        let mut evidence = MatchingEvidence::from_event(&event, false);
        // Workday's authoritative timeType is only available from the detail response.
        evidence
            .fields
            .retain(|field| *field != scrape_core::FilterField::RoleType);
        if !matcher
            .assess(&event, started, &evidence)
            .rejected
            .is_empty()
        {
            return Ok(WorkdayStep::Event(Box::new((event, Some(evidence)))));
        }
        let mut warning = None;
        let mut detail_received = false;
        if let Some(url) = detail_url.filter(|url| self.seen_details.insert(url.clone())) {
            let bound = if self.detail_budget.is_zero() || Instant::now() >= self.source_deadline {
                Some(CoverageTermination::TimeLimit)
            } else if self.detail_requests >= self.detail_limit {
                Some(CoverageTermination::RequestLimit)
            } else {
                None
            };
            if let Some(bound) = bound {
                if !self.detail_bound_reported {
                    self.detail_bound_reported = true;
                    warning = Some(("Workday detail enrichment reached its request/time bound; missing evidence remains unknown".into(), Some(bound)));
                }
            } else {
                self.detail_requests += 1;
                let call_deadline = self
                    .source_deadline
                    .min(self.workflow_deadline)
                    .min(Instant::now() + self.detail_budget);
                let call_started = Instant::now();
                let detail_result = timeout_at(
                    call_deadline,
                    client.fetch_json::<WorkdayDetailResponse>(&url),
                )
                .await;
                self.detail_budget = self.detail_budget.saturating_sub(call_started.elapsed());
                match detail_result {
                    Ok(Ok(detail)) => match merge_workday_detail(
                        &mut event,
                        detail,
                        listing_requisition.as_deref(),
                    ) {
                        Ok(received) => {
                            detail_received = received;
                            if matches!(
                                &event,
                                WorkerEvent::Job {
                                    description_text: None,
                                    ..
                                }
                            ) {
                                warning = Some(("Workday detail has no description; missing evidence remains unknown".into(), None));
                            }
                        }
                        Err(message) => warning = Some((message, None)),
                    },
                    Ok(Err(error)) => {
                        warning = Some((format!("Workday detail {url} unavailable: {error}"), None))
                    }
                    Err(_) => {
                        self.detail_budget = Duration::ZERO;
                        self.detail_bound_reported = true;
                        warning = Some((format!("Workday detail {url} reached its time limit; missing evidence remains unknown"), Some(CoverageTermination::TimeLimit)));
                    }
                }
            }
        }
        let evidence = MatchingEvidence::from_event(&event, true);
        let assessment = matcher.assess(&event, started, &evidence);
        let incomplete = assessment.rejected.is_empty() && !assessment.unresolved.is_empty();
        if detail_received && incomplete {
            warning = Some(("Workday detail did not supply required eligibility evidence; missing fields remain unknown".into(), None));
        }
        if let Some((message, termination)) = warning {
            self.pending.push_back((event, Some(evidence)));
            return Ok(WorkdayStep::Event(Box::new((
                WorkerEvent::Warning {
                    source_id: self.source.id.clone(),
                    message,
                    eligibility_incomplete: incomplete,
                    termination,
                },
                None,
            ))));
        }
        Ok(WorkdayStep::Event(Box::new((event, Some(evidence)))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_workday_endpoint_urls() {
        let ep =
            WorkdayEndpoint::parse("https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite")
                .unwrap();
        assert_eq!(ep.origin, "https://nvidia.wd5.myworkdayjobs.com");
        assert_eq!(ep.tenant, "nvidia");
        assert_eq!(ep.site, "NVIDIAExternalCareerSite");
        assert_eq!(
            ep.list_url(),
            "https://nvidia.wd5.myworkdayjobs.com/wday/cxs/nvidia/NVIDIAExternalCareerSite/jobs"
        );
        assert_eq!(
            ep.detail_url("/job/US-CA/SWE_123"),
            "https://nvidia.wd5.myworkdayjobs.com/wday/cxs/nvidia/NVIDIAExternalCareerSite/job/US-CA/SWE_123"
        );
        assert_eq!(
            ep.apply_url("/job/US-CA/SWE_123"),
            "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite/job/US-CA/SWE_123"
        );

        let ep_locale = WorkdayEndpoint::parse(
            "https://nvidia.wd5.myworkdayjobs.com/en-US/NVIDIAExternalCareerSite",
        )
        .unwrap();
        assert_eq!(ep_locale.site, "NVIDIAExternalCareerSite");

        let ep_unclustered =
            WorkdayEndpoint::parse("https://target.myworkdayjobs.com/targetcareers").unwrap();
        assert_eq!(ep_unclustered.tenant, "target");
        assert_eq!(ep_unclustered.site, "targetcareers");

        assert!(WorkdayEndpoint::parse("https://google.com/jobs").is_err());
        assert!(WorkdayEndpoint::parse("https://target.myworkdayjobs.com").is_err());
    }

    #[test]
    fn parses_relative_dates() {
        let now = DateTime::parse_from_rfc3339("2026-10-20T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            parse_workday_relative_date("Posted Today", now),
            Some("2026-10-20".into())
        );
        assert_eq!(
            parse_workday_relative_date("Posted Yesterday", now),
            Some("2026-10-19".into())
        );
        assert_eq!(
            parse_workday_relative_date("Posted 5 Days Ago", now),
            Some("2026-10-15".into())
        );
        assert_eq!(
            parse_workday_relative_date("Posted 30+ Days Ago", now),
            None
        );
    }

    #[test]
    fn maps_time_type_to_role_type() {
        assert_eq!(
            RoleType::infer("Engineer", Some("Full time")),
            RoleType::FullTime
        );
        assert_eq!(
            RoleType::infer("Engineer", Some("Part time")),
            RoleType::PartTime
        );
        assert_eq!(
            RoleType::infer("Engineer", Some("Internship")),
            RoleType::Internship
        );
        assert_eq!(
            RoleType::infer("Software Engineering Intern", None),
            RoleType::Internship
        );
    }

    #[test]
    fn maps_locations_to_workplace() {
        assert_eq!(
            workplace_from_locations(&["Remote, US"]),
            WorkplaceType::Remote
        );
        assert_eq!(
            workplace_from_locations(&["Santa Clara, CA", "US Remote"]),
            WorkplaceType::Remote
        );
        assert_eq!(
            workplace_from_locations(&["Hybrid - Austin, TX"]),
            WorkplaceType::Hybrid
        );
        assert_eq!(
            workplace_from_locations(&["New York, NY"]),
            WorkplaceType::Onsite
        );
        assert_eq!(workplace_from_locations(&[]), WorkplaceType::Unknown);
    }
}
