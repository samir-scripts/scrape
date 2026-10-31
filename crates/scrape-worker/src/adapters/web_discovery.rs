//! Bounded, public-only discovery across independent origins.
use super::{
    jsonld,
    selector::{self, SelectorCandidate},
    selector_session::SelectorStep,
};
use crate::{
    client::ClientError,
    public_web::{PublicWeb, PublicWebError},
};
use chrono::{DateTime, Utc};
use scrape_core::{
    matching::MatchingEvidence,
    protocol::{CoverageTermination, DiscoveryDateEvidence, DiscoveryProvenance},
    static_source::StaticSourceConfig,
    web_discovery::{canonical_seeds, WebDiscoveryConfig},
    WorkerEvent, WorkerLimits, WorkerSource,
};
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Duration,
};
use tokio::{
    task::JoinSet,
    time::{timeout_at, Instant},
};
use url::Url;

#[derive(Clone)]
struct Entry {
    url: Url,
    seed: String,
    referrer: Option<String>,
    depth: u32,
    sitemap: bool,
    title_lead: bool,
    listing: Option<ListingEvidence>,
}
type FetchResult = (Entry, Result<(Url, Vec<u8>, Vec<String>), PublicWebError>);

#[derive(Clone)]
struct ListingEvidence {
    detail_url: Url,
    identifier: Option<String>,
    title: String,
    date_posted: Option<String>,
    evidence: Option<DiscoveryDateEvidence>,
    indeterminate: bool,
}

struct StructuredPosting {
    url: Url,
    title: String,
    identifier: Option<String>,
    valid_through: Option<String>,
    date_posted: Option<String>,
}

#[derive(Default)]
struct PublicationEvidence {
    date: Option<DiscoveryDateEvidence>,
    identifier: Option<String>,
    indeterminate: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum EntryKind {
    Seed,
    JobDetail,
    CareerIndex,
    OfficialSite,
    DirectoryProfile,
    DirectoryPage,
    Sitemap,
}

fn entry_kind(entry: &Entry) -> EntryKind {
    if entry.depth == 0 && !entry.sitemap {
        return EntryKind::Seed;
    }
    if entry.sitemap {
        return EntryKind::Sitemap;
    }
    let path = entry.url.path().to_ascii_lowercase();
    if entry.url.host_str() == Some("ncnonprofits.org") {
        if path == "/members/member-directory" {
            return EntryKind::DirectoryPage;
        }
        return EntryKind::DirectoryProfile;
    }
    if entry.url.host_str() == Some("en.wikipedia.org") {
        return EntryKind::DirectoryProfile;
    }
    let segments: Vec<_> = path.split('/').filter(|s| !s.is_empty()).collect();
    if entry.title_lead
        || entry.listing.is_some()
        || segments
            .windows(2)
            .any(|pair| matches!(pair[0], "job" | "jobs") && !pair[1].is_empty())
    {
        return EntryKind::JobDetail;
    }
    if entry
        .referrer
        .as_ref()
        .and_then(|s| Url::parse(s).ok())
        .is_some_and(|url| origin(&url) != origin(&entry.url))
        && path == "/"
    {
        return EntryKind::OfficialSite;
    }
    EntryKind::CareerIndex
}

pub(crate) struct WebDiscoverySession {
    source: WorkerSource,
    primary_seed: String,
    config: WebDiscoveryConfig,
    extraction: StaticSourceConfig,
    transport: PublicWeb,
    deadline: Instant,
    started: DateTime<Utc>,
    timezone: Option<String>,
    target_roles: Vec<String>,
    page_limit: u32,
    pages: u32,
    sitemap_counts: HashMap<String, u32>,
    sitemap_entries: HashMap<String, u32>,
    queues: HashMap<String, VecDeque<Entry>>,
    round_robin: VecDeque<String>,
    in_flight: HashSet<String>,
    seen: HashSet<String>,
    ranks: HashMap<String, (EntryKind, u32, [u8; 32])>,
    dispatches: usize,
    directory_pages: u32,
    jobs_seen: HashSet<String>,
    pending: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
    tasks: JoinSet<FetchResult>,
    bounded: bool,
    exhausted: bool,
}

fn origin(url: &Url) -> String {
    url.origin().ascii_serialization()
}
fn valid(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
}
fn canonical(mut url: Url) -> Url {
    url.set_fragment(None);
    url
}
fn trace_url(url: &Url) -> String {
    let mut safe = url.clone();
    let _ = safe.set_username("");
    let _ = safe.set_password(None);
    safe.set_query(None);
    safe.set_fragment(None);
    safe.to_string()
}

impl WebDiscoverySession {
    pub fn new(
        source: &WorkerSource,
        limits: &WorkerLimits,
        workflow_deadline: Instant,
        root_transport: &PublicWeb,
        timezone: Option<&str>,
        target_roles: &[String],
        started: DateTime<Utc>,
    ) -> Result<Self, ClientError> {
        // The host carries scheduling/workflow metadata alongside adapter options.
        // Public workflow validation still rejects unknown user-supplied options.
        let adapter_config = match source.config.as_object() {
            Some(object) => serde_json::Value::Object(
                object
                    .iter()
                    .filter(|(key, _)| !key.starts_with("__"))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            ),
            None => source.config.clone(),
        };
        let config =
            WebDiscoveryConfig::parse(&adapter_config).map_err(ClientError::Configuration)?;
        let seeds =
            canonical_seeds(&source.identifier, &config).map_err(ClientError::Configuration)?;
        let primary_seed =
            canonical(Url::parse(source.identifier.trim()).map_err(ClientError::from)?).to_string();
        let extraction = StaticSourceConfig::parse(
            &serde_json::json!({"crawl":{"extraction":"public_html"}}),
            limits,
        )
        .map_err(ClientError::Configuration)?;
        let page_limit = config.max_pages.min(limits.max_pages_per_source);
        // Navigation pages and all HTTP requests have separate finite budgets.
        // The latter includes robots, redirects and sitemap requests.
        let transport = root_transport.for_source_with_origin_limit(
            (page_limit as usize).saturating_mul(8),
            config.max_domains as usize,
        );
        let mut session = Self {
            source: source.clone(),
            primary_seed,
            config,
            extraction,
            transport,
            started,
            target_roles: target_roles.to_vec(),
            timezone: timezone.map(str::to_owned),
            deadline: workflow_deadline
                .min(Instant::now() + Duration::from_secs(limits.max_source_duration_s)),
            page_limit,
            pages: 0,
            ranks: HashMap::new(),
            dispatches: 0,
            directory_pages: 0,
            sitemap_counts: HashMap::new(),
            sitemap_entries: HashMap::new(),
            queues: HashMap::new(),
            round_robin: VecDeque::new(),
            in_flight: HashSet::new(),
            seen: HashSet::new(),
            jobs_seen: HashSet::new(),
            pending: VecDeque::new(),
            tasks: JoinSet::new(),
            bounded: false,
            exhausted: false,
        };
        for seed in seeds {
            let url = Url::parse(&seed).map_err(ClientError::from)?;
            session.enqueue(Entry {
                url,
                seed,
                referrer: None,
                depth: 0,
                sitemap: false,
                title_lead: false,
                listing: None,
            });
        }
        Ok(session)
    }
    fn enqueue(&mut self, mut entry: Entry) {
        entry.url = canonical(entry.url);
        if !valid(&entry.url) || entry.url.as_str().len() > 4096 {
            return;
        }
        if entry.depth > self.config.max_depth {
            self.bounded = true;
            return;
        }
        let key = entry.url.to_string();
        if self.seen.contains(&key) {
            return;
        }
        let host = origin(&entry.url);
        let mut hash = Sha256::new();
        hash.update(self.started.date_naive().to_string());
        hash.update(key.as_bytes());
        let rank = (entry_kind(&entry), entry.depth, hash.finalize().into());
        if let Some(old) = self.ranks.get(&key).copied() {
            if let Some(existing) = self
                .queues
                .get_mut(&host)
                .and_then(|queue| queue.iter_mut().find(|queued| queued.url == entry.url))
            {
                let listing = merge_listing(
                    existing.listing.take(),
                    entry.listing.take(),
                    self.timezone.as_deref(),
                );
                if rank < old {
                    *existing = entry;
                    self.ranks.insert(key, rank);
                }
                existing.listing = listing;
            }
            return;
        }
        if entry.sitemap && self.sitemap_counts.get(&host).copied().unwrap_or(0) >= 5 {
            self.bounded = true;
            return;
        }
        let total = self.ranks.len();
        let origin_full = self
            .queues
            .get(&host)
            .is_some_and(|queue| queue.len() >= 32);
        let global_full = total >= self.page_limit as usize * 4;
        if origin_full || global_full {
            let mut victim: Option<(String, String, (EntryKind, u32, [u8; 32]))> = None;
            for (candidate_host, queue) in &self.queues {
                if (origin_full && candidate_host != &host) || (!origin_full && queue.len() <= 1) {
                    continue;
                }
                for candidate in queue {
                    if candidate.depth == 0 && candidate.url.as_str() == self.primary_seed {
                        continue;
                    }
                    let candidate_key = candidate.url.to_string();
                    let candidate_rank = self.ranks[&candidate_key];
                    if victim
                        .as_ref()
                        .is_none_or(|(_, _, prior)| candidate_rank > *prior)
                    {
                        victim = Some((candidate_host.clone(), candidate_key, candidate_rank));
                    }
                }
            }
            match victim {
                Some((victim_host, victim_key, victim_rank)) if rank < victim_rank => {
                    if let Some(queue) = self.queues.get_mut(&victim_host) {
                        queue.retain(|candidate| candidate.url.as_str() != victim_key);
                    }
                    self.ranks.remove(&victim_key);
                }
                _ => {
                    self.bounded = true;
                    return;
                }
            }
            self.bounded = true;
        }
        if entry.sitemap {
            *self.sitemap_counts.entry(host.clone()).or_default() += 1;
        }
        self.ranks.insert(key, rank);
        let queue = self.queues.entry(host.clone()).or_default();
        if queue.is_empty() && !self.round_robin.contains(&host) {
            self.round_robin.push_back(host);
        }
        queue.push_back(entry);
    }
    fn schedule(&mut self) {
        while self.tasks.len() < 4 && self.pages + (self.tasks.len() as u32) < self.page_limit {
            let available = self
                .round_robin
                .iter()
                .filter(|host| !self.in_flight.contains(*host))
                .filter_map(|host| {
                    let queue = self.queues.get(host)?;
                    let best = queue
                        .iter()
                        .min_by_key(|entry| self.ranks.get(entry.url.as_str()))?;
                    Some((host.clone(), self.transport.origin_admitted(&best.url)))
                })
                .collect::<Vec<_>>();
            if available.is_empty() {
                break;
            }
            let seed_host = available
                .iter()
                .find_map(|(host, _)| {
                    self.queues
                        .get(host)?
                        .iter()
                        .any(|entry| {
                            entry_kind(entry) == EntryKind::Seed
                                && entry.url.as_str() == self.primary_seed
                        })
                        .then(|| host.clone())
                })
                .or_else(|| {
                    available.iter().find_map(|(host, _)| {
                        self.queues
                            .get(host)?
                            .iter()
                            .any(|entry| entry_kind(entry) == EntryKind::Seed)
                            .then(|| host.clone())
                    })
                });
            let sitemap_host = if (self.dispatches + 1) % 8 == 0 {
                available.iter().find_map(|(host, _)| {
                    self.queues
                        .get(host)?
                        .iter()
                        .any(|entry| entry.sitemap)
                        .then(|| host.clone())
                })
            } else {
                None
            };
            let preferred_admitted = self.dispatches % 4 != 3;
            let selected_sitemap = seed_host.is_none() && sitemap_host.is_some();
            let host = seed_host
                .or(sitemap_host.clone())
                .or_else(|| {
                    available
                        .iter()
                        .find(|(_, admitted)| *admitted == preferred_admitted)
                        .map(|(host, _)| host.clone())
                })
                .or_else(|| available.first().map(|(host, _)| host.clone()))
                .unwrap();
            let queue = self.queues.get_mut(&host).unwrap();
            let index = queue
                .iter()
                .position(|entry| entry.depth == 0 && entry.url.as_str() == self.primary_seed)
                .or_else(|| {
                    queue
                        .iter()
                        .enumerate()
                        .filter(|(_, entry)| {
                            !selected_sitemap
                                || sitemap_host.as_ref() != Some(&host)
                                || entry.sitemap
                        })
                        .min_by_key(|(_, entry)| self.ranks.get(entry.url.as_str()))
                        .map(|(index, _)| index)
                })
                .unwrap();
            let entry = queue.remove(index).unwrap();
            if entry.url.host_str() == Some("ncnonprofits.org")
                && entry.url.path() == "/members/member-directory"
            {
                if self.directory_pages >= 4 {
                    self.ranks.remove(entry.url.as_str());
                    if queue.is_empty() {
                        self.round_robin.retain(|candidate| candidate != &host);
                    }
                    self.bounded = true;
                    continue;
                }
                self.directory_pages += 1;
            }
            if let Err(error) = self.transport.admit_origin(&entry.url) {
                queue.push_back(entry);
                self.bounded = true;
                if matches!(error, PublicWebError::OriginLimit(_)) {
                    self.round_robin.retain(|candidate| candidate != &host);
                    continue;
                }
                self.round_robin.retain(|candidate| candidate != &host);
                continue;
            }
            self.ranks.remove(entry.url.as_str());
            self.seen.insert(entry.url.to_string());
            self.round_robin.retain(|candidate| candidate != &host);
            if !queue.is_empty() {
                self.round_robin.push_back(host.clone());
            }
            self.dispatches += 1;
            tracing::debug!(seed = %Url::parse(&entry.seed).map(|u| trace_url(&u)).unwrap_or_default(),
                referrer = %entry.referrer.as_deref().and_then(|s| Url::parse(s).ok()).map(|u| trace_url(&u)).unwrap_or_default(),
                requested = %trace_url(&entry.url), depth = entry.depth, kind = ?entry_kind(&entry), "discovery dispatch");
            self.in_flight.insert(host);
            let client = self.transport.clone();
            self.tasks.spawn(async move {
                let result = if entry.sitemap {
                    client
                        .fetch_sitemap(entry.url.as_str())
                        .await
                        .map(|p| (p.url, p.body, Vec::new()))
                } else {
                    client
                        .fetch(entry.url.as_str())
                        .await
                        .map(|p| (p.url, p.body.into_bytes(), p.robots_sitemaps))
                };
                (entry, result)
            });
        }
    }
    pub async fn next(&mut self) -> Result<SelectorStep, ClientError> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(SelectorStep::Event(Box::new(event)));
        }
        if self.exhausted {
            return Ok(SelectorStep::Exhausted);
        }
        if Instant::now() >= self.deadline {
            self.tasks.abort_all();
            self.exhausted = true;
            return Ok(self.warning(
                "Discovery duration limit reached",
                Some(CoverageTermination::TimeLimit),
            ));
        }
        self.schedule();
        if self.tasks.is_empty() {
            self.exhausted = true;
            if self.bounded || !self.round_robin.is_empty() {
                return Ok(self.warning(
                    "Discovery page, origin, or request limit reached",
                    Some(CoverageTermination::PageLimit),
                ));
            }
            return Ok(SelectorStep::Exhausted);
        }
        let finished = timeout_at(self.deadline, self.tasks.join_next()).await;
        let (entry, result) = match finished {
            Ok(Some(Ok(value))) => value,
            Ok(Some(Err(error))) => {
                return Err(ClientError::Extraction(format!(
                    "Discovery fetch task failed: {error}"
                )))
            }
            Ok(None) => return Ok(SelectorStep::Exhausted),
            Err(_) => {
                self.tasks.abort_all();
                self.exhausted = true;
                return Ok(self.warning(
                    "Discovery duration limit reached",
                    Some(CoverageTermination::TimeLimit),
                ));
            }
        };
        self.in_flight.remove(&origin(&entry.url));
        self.pages += 1;
        match result {
            Ok((actual, bytes, sitemaps)) => {
                tracing::debug!(requested = %trace_url(&entry.url), final_url = %trace_url(&actual), "discovery fetched");
                self.seen.insert(actual.to_string());
                if actual != entry.url {
                    self.ranks.remove(actual.as_str());
                    if let Some(queue) = self.queues.get_mut(&origin(&actual)) {
                        queue.retain(|pending| pending.url != actual);
                    }
                }
                if entry.sitemap {
                    self.read_sitemap(&entry, &actual, &bytes);
                } else {
                    let html = String::from_utf8(bytes).map_err(|e| {
                        ClientError::Extraction(format!("Invalid HTML encoding: {e}"))
                    })?;
                    self.discover_sitemaps(&entry, &actual, sitemaps);
                    self.read_page(&entry, &actual, &html)?;
                }
            }
            Err(
                error @ (PublicWebError::BudgetExhausted | PublicWebError::GlobalBudgetExhausted),
            ) => {
                self.tasks.abort_all();
                self.exhausted = true;
                return Ok(self.warning(
                    &format!("Discovery fetch {} failed: {error}", entry.url),
                    Some(CoverageTermination::RequestLimit),
                ));
            }
            Err(error @ PublicWebError::OriginLimit(_)) => {
                self.bounded = true;
                return Ok(self.warning(
                    &format!("Discovery fetch {} failed: {error}", entry.url),
                    None,
                ));
            }
            Err(error) => {
                return Ok(self.warning(
                    &format!("Discovery fetch {} failed: {error}", entry.url),
                    None,
                ))
            }
        }
        if let Some(event) = self.pending.pop_front() {
            Ok(SelectorStep::Event(Box::new(event)))
        } else {
            Ok(SelectorStep::Yield)
        }
    }
    fn warning(&self, message: &str, termination: Option<CoverageTermination>) -> SelectorStep {
        SelectorStep::Event(Box::new((
            WorkerEvent::Warning {
                source_id: self.source.id.clone(),
                message: message.into(),
                eligibility_incomplete: true,
                termination,
            },
            None,
        )))
    }
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }
    pub fn pages_visited(&self) -> u32 {
        self.pages
    }
    pub fn origins_visited(&self) -> u32 {
        self.transport.origins_used() as u32
    }
    fn discover_sitemaps(&mut self, entry: &Entry, actual: &Url, mut maps: Vec<String>) {
        if matches!(
            actual.host_str(),
            Some("en.wikipedia.org" | "ncnonprofits.org")
        ) {
            return;
        }
        let host = origin(actual);
        if self.sitemap_counts.contains_key(&host) {
            return;
        }
        self.sitemap_counts.insert(host.clone(), 0);
        if maps.is_empty() {
            maps.push("/sitemap.xml".into());
        }
        for raw in maps.into_iter().take(5) {
            if let Ok(url) = actual.join(&raw) {
                if origin(&url) == host {
                    self.enqueue(Entry {
                        url,
                        seed: entry.seed.clone(),
                        referrer: Some(actual.to_string()),
                        depth: entry.depth,
                        sitemap: true,
                        title_lead: false,
                        listing: None,
                    });
                }
            }
        }
    }
    fn read_sitemap(&mut self, entry: &Entry, actual: &Url, bytes: &[u8]) {
        let host = origin(actual);
        let Ok(text) = std::str::from_utf8(bytes) else {
            self.pending.push_back((
                WorkerEvent::Warning {
                    source_id: self.source.id.clone(),
                    message: format!("Sitemap {actual} is not UTF-8"),
                    eligibility_incomplete: true,
                    termination: None,
                },
                None,
            ));
            return;
        };
        let mut urls = Vec::new();
        if text.trim_start().starts_with('<') {
            let mut reader = quick_xml::Reader::from_str(text);
            let mut in_loc = false;
            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::DocType(_)) => {
                        self.pending.push_back((
                            WorkerEvent::Warning {
                                source_id: self.source.id.clone(),
                                message: format!("Rejected sitemap DTD at {actual}"),
                                eligibility_incomplete: true,
                                termination: None,
                            },
                            None,
                        ));
                        return;
                    }
                    Ok(quick_xml::events::Event::Start(tag)) => {
                        in_loc = tag.local_name().as_ref() == b"loc"
                    }
                    Ok(quick_xml::events::Event::End(_)) => in_loc = false,
                    Ok(quick_xml::events::Event::Text(value)) if in_loc => match value.unescape() {
                        Ok(raw) => {
                            if urls.len() < 1001 {
                                urls.push(raw.into_owned());
                            } else {
                                self.bounded = true;
                            }
                        }
                        Err(error) => {
                            self.pending.push_back((
                                WorkerEvent::Warning {
                                    source_id: self.source.id.clone(),
                                    message: format!("Invalid sitemap entity at {actual}: {error}"),
                                    eligibility_incomplete: true,
                                    termination: None,
                                },
                                None,
                            ));
                            return;
                        }
                    },
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(error) => {
                        self.pending.push_back((
                            WorkerEvent::Warning {
                                source_id: self.source.id.clone(),
                                message: format!("Malformed sitemap {actual}: {error}"),
                                eligibility_incomplete: true,
                                termination: None,
                            },
                            None,
                        ));
                        return;
                    }
                    _ => {}
                }
            }
        } else {
            for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
                if urls.len() >= 1001 {
                    self.bounded = true;
                    break;
                }
                urls.push(line.to_owned());
            }
        }
        let index = text.contains("<sitemapindex") || text.contains(":sitemapindex");
        for raw in urls {
            let used = self.sitemap_entries.entry(host.clone()).or_default();
            if *used >= 1000 {
                self.bounded = true;
                break;
            }
            *used += 1;
            let Ok(url) = actual.join(&raw) else { continue };
            if origin(&url) != host {
                continue;
            }
            if index {
                if self.sitemap_counts.get(&host).copied().unwrap_or(0) < 5 {
                    self.enqueue(Entry {
                        url,
                        seed: entry.seed.clone(),
                        referrer: Some(actual.to_string()),
                        depth: entry.depth,
                        sitemap: true,
                        title_lead: false,
                        listing: None,
                    });
                }
            } else if !non_job_navigation(url.path())
                && !excluded_path(url.path())
                && relevant(url.path())
            {
                self.enqueue(Entry {
                    url,
                    seed: entry.seed.clone(),
                    referrer: Some(actual.to_string()),
                    depth: entry.depth.saturating_add(1),
                    sitemap: false,
                    title_lead: false,
                    listing: None,
                });
            }
        }
    }
    fn read_page(&mut self, entry: &Entry, actual: &Url, html: &str) -> Result<(), ClientError> {
        let document = Html::parse_document(html);
        let mut structured = Vec::new();
        jsonld::extract_detail_job_postings_with(
            html,
            actual.as_str(),
            &self.source,
            &mut |event| {
                structured.push(event);
                true
            },
        );
        let has_structured = structured
            .iter()
            .any(|event| matches!(event, WorkerEvent::Job { .. }));
        if has_structured {
            let single = structured
                .iter()
                .filter(|event| matches!(event, WorkerEvent::Job { .. }))
                .count()
                == 1;
            let metadata = structured_postings(&document, actual);
            'postings: for mut event in structured {
                let WorkerEvent::Job {
                    title,
                    url,
                    external_id,
                    date_posted,
                    description_source,
                    ..
                } = &mut event
                else {
                    continue;
                };
                let Ok(target) = actual.join(url.as_str()) else {
                    continue;
                };
                *url = canonical(target).to_string();
                let matches: Vec<_> = metadata
                    .iter()
                    .filter(|posting| {
                        posting.title == *title
                            && posting.url.as_str() == url.as_str()
                            && posting
                                .identifier
                                .as_ref()
                                .is_none_or(|id| id == external_id)
                    })
                    .collect();
                let posting = matches.first().copied();
                let block = posting_block(&document, title);
                if closed_posting(description_source.as_deref().unwrap_or(""))
                    || block.is_some_and(closed_block)
                    || (single && block.is_none() && closed_block(document.root_element()))
                {
                    continue;
                }
                let mut context = PublicationEvidence {
                    identifier: posting.and_then(|posting| posting.identifier.clone()),
                    ..PublicationEvidence::default()
                };
                for raw in matches
                    .iter()
                    .filter_map(|posting| posting.valid_through.as_deref())
                {
                    match expiry(raw, self.timezone.as_deref(), self.started) {
                        Some(true) => continue 'postings,
                        Some(false) => {}
                        None => {
                            self.date_warning(actual, "Posting expiry requires a configured posting-date timezone or an unambiguous ISO value");
                            continue 'postings;
                        }
                    }
                }
                let observed = Utc::now();
                for other in &matches {
                    match narrower_date(
                        date_posted.as_deref(),
                        observed,
                        other.date_posted.as_deref(),
                        observed,
                        self.timezone.as_deref(),
                    ) {
                        Ok(true) => *date_posted = other.date_posted.clone(),
                        Ok(false) => {}
                        Err(reason) => {
                            context.indeterminate = true;
                            self.date_warning(actual, reason);
                        }
                    }
                }
                let root = block.unwrap_or_else(|| document.root_element());
                let visible = visible_dates_in(root).unwrap_or_default();
                if visible.len() > 1 {
                    context.indeterminate = true;
                    self.date_warning(
                        actual,
                        "Conflicting publication dates; date is indeterminate",
                    );
                } else if let Some(raw) = visible.iter().next() {
                    let normalized = normalize_visible_date(raw);
                    match narrower_date(
                        date_posted.as_deref(),
                        observed,
                        normalized.as_deref(),
                        observed,
                        self.timezone.as_deref(),
                    ) {
                        Ok(true) => {
                            *date_posted = normalized;
                            context.date = Some(date_evidence(
                                raw,
                                actual,
                                "visible_publication",
                                visible_locator(root, raw),
                                observed,
                            ));
                        }
                        Ok(false) => {}
                        Err(reason) => {
                            context.indeterminate = true;
                            self.date_warning(actual, reason);
                        }
                    }
                }
                if context.indeterminate {
                    *date_posted = None;
                }
                self.accept(
                    event,
                    entry,
                    actual,
                    "json_ld",
                    "JobPosting.datePosted",
                    Some(context),
                );
            }
        } else {
            let block_selector =
                Selector::parse("article, section, [data-requisition-id], [data-job-id]")
                    .expect("valid selector");
            let heading_selector = Selector::parse("h1, h2, h3, h4").expect("valid selector");
            let apply_selector = Selector::parse("a[href]").expect("valid selector");
            let mut block_count = 0usize;
            for block in document.select(&block_selector) {
                let Some(heading) = block.select(&heading_selector).next() else {
                    continue;
                };
                let title = heading
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_owned();
                if title.len() < 3 || title.len() > 120 || closed_block(block) {
                    continue;
                }
                let requisition = block
                    .value()
                    .attr("data-requisition-id")
                    .or_else(|| block.value().attr("data-job-id"))
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_owned);
                let identifier = requisition.clone().or_else(|| {
                    block.select(&apply_selector).find_map(|anchor| {
                        let text = anchor
                            .text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_ascii_lowercase();
                        if !text.contains("apply") {
                            return None;
                        }
                        let url = actual.join(anchor.value().attr("href")?).ok()?;
                        valid(&url).then(|| url.to_string())
                    })
                });
                let Some(identifier) = identifier else {
                    continue;
                };
                let location_selector =
                    Selector::parse("[class*='location'], [itemprop='jobLocation']")
                        .expect("valid selector");
                let location = block
                    .select(&location_selector)
                    .next()
                    .map(|node| node.text().collect::<Vec<_>>().join(" ").trim().to_owned())
                    .unwrap_or_default();
                let posting = selector::SelectorPosting {
                    title,
                    url: actual.to_string(),
                    external_id: identifier,
                    location,
                    salary_text: None,
                    description_source: Some(block.html()),
                    description_format: Some(scrape_core::enrich::DescriptionFormat::Html),
                    date_posted: visible_date_in(block),
                    company_logo: None,
                    detail_url: None,
                };
                let mut event = posting.event(&self.source);
                if let WorkerEvent::Job { company, .. } = &mut event {
                    *company =
                        explicit_company(&document).unwrap_or_else(|| "Unknown company".into());
                }
                block_count += 1;
                self.accept(
                    event,
                    entry,
                    actual,
                    "visible_publication",
                    "job block publication label",
                    Some(PublicationEvidence {
                        identifier: requisition,
                        ..PublicationEvidence::default()
                    }),
                );
            }
            let mut candidates = Vec::new();
            let mut emitted_detail = false;
            selector::extract_heuristic_candidates(
                &document,
                actual,
                actual.as_str(),
                &self.source,
                &self.extraction,
                &mut |candidate| {
                    candidates.push(candidate);
                    true
                },
            )?;
            for candidate in candidates {
                if let SelectorCandidate::Posting(posting) = candidate {
                    if block_count > 0 {
                        continue;
                    }
                    if posting.url != actual.as_str() {
                        continue;
                    }
                    let heading = Selector::parse("h1").expect("valid selector");
                    let is_detail_heading = document.select(&heading).any(|node| {
                        node.text().collect::<Vec<_>>().join(" ").trim() == posting.title.trim()
                    });
                    if !is_detail_heading {
                        continue;
                    }
                    let block = posting_block(&document, &posting.title)
                        .unwrap_or_else(|| document.root_element());
                    if closed_block(block) {
                        continue;
                    }
                    let mut event = posting.event(&self.source);
                    selector::hydrate_public_html_job(&mut event, html, actual, &self.source)?;
                    if let WorkerEvent::Job {
                        company,
                        date_posted,
                        ..
                    } = &mut event
                    {
                        *company =
                            explicit_company(&document).unwrap_or_else(|| "Unknown company".into());
                        *date_posted = visible_date_in(block);
                    }
                    emitted_detail = true;
                    self.accept(
                        event,
                        entry,
                        actual,
                        "visible_publication",
                        "explicit publication label",
                        None,
                    );
                }
            }
            if block_count == 0 && !emitted_detail {
                let main_selector = Selector::parse("main").expect("valid selector");
                let title_selector = Selector::parse("h1").expect("valid selector");
                let headings = Selector::parse("h2, h3, h4").expect("valid selector");
                let email = Selector::parse("a[href^='mailto:']").expect("valid selector");
                if let Some(main) = document.select(&main_selector).next() {
                    let labels: Vec<_> = main
                        .select(&headings)
                        .map(|node| {
                            node.text()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .to_ascii_lowercase()
                        })
                        .collect();
                    let has_apply = main.select(&email).any(|node| {
                        node.text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_ascii_lowercase()
                            .contains("apply")
                    });
                    let has_responsibilities =
                        labels.iter().any(|label| label.contains("responsibilit"));
                    let has_qualifications = labels.iter().any(|label| {
                        label.contains("qualification") || label.contains("requirement")
                    });
                    if has_apply
                        && has_responsibilities
                        && has_qualifications
                        && !closed_block(main)
                    {
                        if let Some(title) = main.select(&title_selector).next() {
                            let title =
                                title.text().collect::<Vec<_>>().join(" ").trim().to_owned();
                            let posting_root = posting_block(&document, &title).unwrap_or(main);
                            if !title.is_empty()
                                && title.len() <= 200
                                && !closed_block(posting_root)
                            {
                                let posting = selector::SelectorPosting {
                                    title,
                                    url: actual.to_string(),
                                    external_id: actual.to_string(),
                                    location: String::new(),
                                    salary_text: None,
                                    description_source: Some(posting_root.html()),
                                    description_format: Some(
                                        scrape_core::enrich::DescriptionFormat::Html,
                                    ),
                                    date_posted: visible_date_in(posting_root),
                                    company_logo: None,
                                    detail_url: None,
                                };
                                let mut event = posting.event(&self.source);
                                selector::hydrate_public_html_job(
                                    &mut event,
                                    html,
                                    actual,
                                    &self.source,
                                )?;
                                if let WorkerEvent::Job {
                                    company,
                                    date_posted,
                                    ..
                                } = &mut event
                                {
                                    *company = explicit_company(&document)
                                        .unwrap_or_else(|| "Unknown company".into());
                                    *date_posted = visible_date_in(posting_root);
                                }
                                self.accept(
                                    event,
                                    entry,
                                    actual,
                                    "visible_publication",
                                    "explicit publication label",
                                    None,
                                );
                            }
                        }
                    }
                }
            }
        }
        // Wikipedia's category pages are company directories, not job boards.
        // Limit their navigation to listed articles and the infobox website field.
        if actual.host_str() == Some("en.wikipedia.org") {
            let selector = if actual.path().starts_with("/wiki/Category:") {
                "#mw-pages .mw-category-group a[href]"
            } else {
                "table.infobox .url a[href], table.infobox a.official-website[href]"
            };
            let selector = Selector::parse(selector).expect("valid directory selector");
            for anchor in document.select(&selector) {
                let Some(raw) = anchor.value().attr("href") else {
                    continue;
                };
                let Ok(url) = actual.join(raw) else { continue };
                self.enqueue(Entry {
                    url,
                    seed: entry.seed.clone(),
                    referrer: Some(actual.to_string()),
                    depth: entry.depth.saturating_add(1),
                    sitemap: false,
                    title_lead: false,
                    listing: None,
                });
            }
            return Ok(());
        }
        if actual.host_str() == Some("ncnonprofits.org") && actual.path().starts_with("/members/") {
            let directory = actual.path() == "/members/member-directory";
            let selector = Selector::parse(if directory {
                ".member__title--list a[href], .pager__item a[href]"
            } else {
                ".member__member-website a[href]"
            })
            .expect("valid directory selector");
            for anchor in document.select(&selector) {
                let Some(raw) = anchor.value().attr("href") else {
                    continue;
                };
                let Ok(url) = actual.join(raw) else { continue };
                let path = url.path();
                let profile = origin(&url) == origin(actual)
                    && path.starts_with("/members/")
                    && !path.trim_start_matches("/members/").contains('/')
                    && path != "/members/member-directory"
                    && url.query().is_none();
                let page = origin(&url) == origin(actual)
                    && path == "/members/member-directory"
                    && url.query_pairs().count() == 1
                    && url
                        .query_pairs()
                        .any(|(key, value)| key == "page" && value.parse::<u32>().is_ok());
                if (directory && (profile || page))
                    || (!directory && origin(&url) != origin(actual))
                {
                    self.enqueue(Entry {
                        url,
                        seed: entry.seed.clone(),
                        referrer: Some(actual.to_string()),
                        depth: entry.depth.saturating_add(1),
                        sitemap: false,
                        title_lead: false,
                        listing: None,
                    });
                }
            }
            return Ok(());
        }
        let anchors = Selector::parse("a[href]").expect("valid selector");
        for anchor in document.select(&anchors) {
            let Some(raw) = anchor.value().attr("href") else {
                continue;
            };
            let Ok(url) = actual.join(raw) else { continue };
            let text = anchor
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase();
            let path = url.path().to_ascii_lowercase();
            let host = url.host_str().unwrap_or("").to_ascii_lowercase();
            let social = [
                "linkedin.com",
                "facebook.com",
                "twitter.com",
                "instagram.com",
                "youtube.com",
            ]
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
            let excluded = excluded_path(&path);
            let apply = text.contains("apply") || path.contains("/apply");
            let href = url.as_str().to_ascii_lowercase();
            let blog = path
                .split('/')
                .any(|part| part == "blog" || part == "news" || part.ends_with("-blog"));
            let detail = path
                .split('/')
                .collect::<Vec<_>>()
                .windows(2)
                .any(|pair| matches!(pair[0], "jobs" | "job") && !pair[1].is_empty());
            let profile = origin(&url) == origin(actual)
                && matches!(
                    actual.path().trim_end_matches('/'),
                    "/companies" | "/company" | "/directory"
                )
                && (href.contains("/companies/") || href.contains("/company/"));
            let official = origin(&url) != origin(actual)
                && (text.contains("website")
                    || text.contains("homepage")
                    || text.contains("official site"));
            let title_lead = relevant(actual.path())
                && text.trim().len() <= 120
                && scrape_core::matching::PostingRules::matches_role_lead(
                    text.trim(),
                    &self.target_roles,
                );
            if !social
                && !excluded
                && !apply
                && !non_job_navigation(&path)
                && (!blog || detail)
                && (relevant(&path) || relevant(&text) || profile || official || title_lead)
            {
                self.enqueue(Entry {
                    url,
                    seed: entry.seed.clone(),
                    referrer: Some(actual.to_string()),
                    depth: entry.depth.saturating_add(1),
                    sitemap: false,
                    title_lead,
                    listing: None,
                });
            }
        }
        Ok(())
    }
    fn accept(
        &mut self,
        mut event: WorkerEvent,
        entry: &Entry,
        actual: &Url,
        kind: &str,
        locator: &str,
        context: Option<PublicationEvidence>,
    ) {
        if let WorkerEvent::Job {
            url,
            external_id,
            title,
            date_posted,
            discovery_provenance,
            ..
        } = &mut event
        {
            let context = context.unwrap_or_default();
            let observed = Utc::now();
            let mut evidence = context.date.or_else(|| {
                date_posted
                    .as_ref()
                    .map(|raw| date_evidence(raw, actual, kind, locator, observed))
            });
            if let Some(raw) = date_posted.as_deref() {
                *date_posted = normalize_visible_date(raw);
            }
            // Listing publication belongs to the listing observation, not the
            // subsequently fetched detail or its geography/company evidence.
            if let Ok(target) = actual.join(url.as_str()) {
                let target = canonical(target);
                if target != *actual {
                    self.enqueue(Entry {
                        url: target.clone(),
                        seed: entry.seed.clone(),
                        referrer: Some(actual.to_string()),
                        depth: entry.depth.saturating_add(1),
                        sitemap: false,
                        title_lead: false,
                        listing: Some(ListingEvidence {
                            detail_url: target,
                            identifier: context.identifier,
                            title: title.clone(),
                            date_posted: date_posted.clone(),
                            evidence,
                            indeterminate: context.indeterminate,
                        }),
                    });
                    return;
                }
            } else {
                return;
            }
            if context.indeterminate {
                return;
            }
            if let Some(listing) = entry.listing.as_ref().filter(|listing| {
                !listing.indeterminate
                    && listing.detail_url == *actual
                    && entry.url == listing.detail_url
                    && match (&listing.identifier, &context.identifier) {
                        (Some(a), Some(b)) => a == b,
                        (None, None) => listing.title == *title,
                        _ => false,
                    }
            }) {
                let listing_observed = listing
                    .evidence
                    .as_ref()
                    .and_then(|evidence| DateTime::parse_from_rfc3339(&evidence.observed_at).ok())
                    .map(|observed| observed.with_timezone(&Utc))
                    .unwrap_or(observed);
                match narrower_date(
                    date_posted.as_deref(),
                    observed,
                    listing.date_posted.as_deref(),
                    listing_observed,
                    self.timezone.as_deref(),
                ) {
                    Ok(true) => {
                        *date_posted = listing.date_posted.clone();
                        evidence = listing.evidence.clone();
                    }
                    Ok(false) => {}
                    Err(reason) => {
                        self.date_warning(actual, reason);
                        return;
                    }
                }
            }
            let key = format!("{}\n{}", actual, external_id);
            if !self.jobs_seen.insert(key) {
                return;
            }
            if evidence
                .as_ref()
                .is_some_and(|e| e.precision.starts_with("relative:"))
            {
                *date_posted = None;
            }
            let primary = Url::parse(&self.source.identifier)
                .ok()
                .map(canonical)
                .map(|url| url.to_string());
            let seed_manifest = (primary.as_deref() != Some(entry.seed.as_str()))
                .then(|| self.config.seed_manifest.clone())
                .flatten();
            *discovery_provenance = Some(DiscoveryProvenance {
                seed_url: entry.seed.clone(),
                referrer_url: entry.referrer.clone(),
                final_url: actual.to_string(),
                seed_manifest,
                date_evidence: evidence,
            });
            let evidence = MatchingEvidence::from_event(&event, false);
            self.pending.push_back((event, Some(evidence)));
        }
    }

    fn date_warning(&mut self, actual: &Url, reason: &str) {
        self.pending.push_back((
            WorkerEvent::Warning {
                source_id: self.source.id.clone(),
                message: format!("{reason} at {actual}"),
                eligibility_incomplete: true,
                termination: None,
            },
            None,
        ));
    }
}
fn structured_postings(document: &Html, actual: &Url) -> Vec<StructuredPosting> {
    fn collect(value: &serde_json::Value, actual: &Url, postings: &mut Vec<StructuredPosting>) {
        match value {
            serde_json::Value::Array(values) => {
                for value in values {
                    collect(value, actual, postings);
                }
            }
            serde_json::Value::Object(map) => {
                let is_job = match map.get("@type") {
                    Some(serde_json::Value::String(kind)) => {
                        kind.eq_ignore_ascii_case("JobPosting")
                    }
                    Some(serde_json::Value::Array(kinds)) => kinds.iter().any(|kind| {
                        kind.as_str()
                            .is_some_and(|kind| kind.eq_ignore_ascii_case("JobPosting"))
                    }),
                    _ => false,
                };
                if is_job {
                    if let Some(title) = map.get("title").and_then(|value| value.as_str()) {
                        let target = map
                            .get("url")
                            .and_then(|value| value.as_str())
                            .unwrap_or(actual.as_str());
                        if let Ok(url) = actual.join(target) {
                            postings.push(StructuredPosting {
                                url: canonical(url),
                                title: title.to_owned(),
                                identifier: map
                                    .get("identifier")
                                    .and_then(|value| {
                                        value.as_str().or_else(|| {
                                            value.get("value").and_then(|value| value.as_str())
                                        })
                                    })
                                    .filter(|value| !value.trim().is_empty())
                                    .map(str::to_owned),
                                valid_through: map
                                    .get("validThrough")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_owned),
                                date_posted: map
                                    .get("datePosted")
                                    .and_then(|value| value.as_str())
                                    .map(str::to_owned),
                            });
                        }
                    }
                }
                if let Some(graph) = map.get("@graph") {
                    collect(graph, actual, postings);
                }
            }
            _ => {}
        }
    }
    let selector = Selector::parse("script[type='application/ld+json']").expect("valid selector");
    let mut postings = Vec::new();
    for node in document.select(&selector) {
        if let Ok(value) = serde_json::from_str(&node.text().collect::<String>()) {
            collect(&value, actual, &mut postings);
        }
    }
    postings
}

fn narrower_date(
    current: Option<&str>,
    current_observed: DateTime<Utc>,
    candidate: Option<&str>,
    candidate_observed: DateTime<Utc>,
    timezone: Option<&str>,
) -> Result<bool, &'static str> {
    let Some(candidate) = candidate else {
        return Ok(false);
    };
    let Some(current) = current else {
        return Ok(true);
    };
    if current == candidate && scrape_core::posting_date::relative_age_seconds(current).is_none() {
        return Ok(false);
    }
    let a = scrape_core::posting_date::publication_interval(
        current,
        timezone.unwrap_or(""),
        current_observed,
    );
    let b = scrape_core::posting_date::publication_interval(
        candidate,
        timezone.unwrap_or(""),
        candidate_observed,
    );
    let Some((a, b)) = a.zip(b) else {
        return Err("Publication comparison requires a configured posting-date timezone or unambiguous ISO evidence");
    };
    if b.contains(a) {
        Ok(false)
    } else if a.contains(b) {
        Ok(true)
    } else {
        Err("Conflicting publication dates; date is indeterminate")
    }
}

fn merge_listing(
    a: Option<ListingEvidence>,
    b: Option<ListingEvidence>,
    timezone: Option<&str>,
) -> Option<ListingEvidence> {
    let Some(mut a) = a else { return b };
    let Some(b) = b else { return Some(a) };
    if a.indeterminate
        || b.indeterminate
        || a.detail_url != b.detail_url
        || match (&a.identifier, &b.identifier) {
            (Some(a), Some(b)) => a != b,
            (None, None) => a.title != b.title,
            _ => true,
        }
    {
        a.indeterminate = true;
        return Some(a);
    }
    let observed = |listing: &ListingEvidence| {
        listing
            .evidence
            .as_ref()
            .and_then(|evidence| DateTime::parse_from_rfc3339(&evidence.observed_at).ok())
            .map(|at| at.with_timezone(&Utc))
    };
    match observed(&a).zip(observed(&b)) {
        Some((a_observed, b_observed)) => match narrower_date(
            a.date_posted.as_deref(),
            a_observed,
            b.date_posted.as_deref(),
            b_observed,
            timezone,
        ) {
            Ok(true) => return Some(b),
            Ok(false) => {}
            Err(_) => a.indeterminate = true,
        },
        None if a.date_posted.is_none() => return Some(b),
        None => {}
    }
    Some(a)
}

fn date_evidence(
    raw: &str,
    page: &Url,
    kind: &str,
    locator: &str,
    observed: DateTime<Utc>,
) -> DiscoveryDateEvidence {
    let normalized = normalize_visible_date(raw);
    let precision = scrape_core::posting_date::relative_age_seconds(raw)
        .map(|(age, unit)| format!("relative:{age}:{unit}"))
        .unwrap_or_else(|| {
            if normalized.as_ref().is_some_and(|value| value.len() == 10) {
                "date".into()
            } else {
                "timestamp".into()
            }
        });
    DiscoveryDateEvidence {
        kind: kind.to_owned(),
        raw_value: raw.to_owned(),
        page_url: page.to_string(),
        locator: locator.to_owned(),
        observed_at: observed.to_rfc3339(),
        precision,
    }
}

fn expiry(raw: &str, timezone: Option<&str>, started: DateTime<Utc>) -> Option<bool> {
    use scrape_core::posting_date::PublicationInterval;
    match scrape_core::posting_date::publication_interval(raw, timezone.unwrap_or(""), started)? {
        PublicationInterval::Instant(end) => Some(started > end),
        PublicationInterval::Date { end, .. } => Some(started >= end),
        PublicationInterval::Relative { .. } => None,
    }
}

fn posting_block<'a>(document: &'a Html, title: &str) -> Option<scraper::ElementRef<'a>> {
    let headings = Selector::parse("h1, h2, h3, h4").expect("valid selector");
    document
        .select(&headings)
        .find(|node| node.text().collect::<Vec<_>>().join(" ").trim() == title.trim())
        .and_then(|heading| {
            heading
                .ancestors()
                .filter_map(scraper::ElementRef::wrap)
                .find(|node| {
                    matches!(node.value().name(), "article" | "section" | "main")
                        || node.value().attr("data-job-id").is_some()
                        || node.value().attr("data-requisition-id").is_some()
                })
        })
}

fn excluded_context(node: scraper::ElementRef<'_>, root: scraper::ElementRef<'_>) -> bool {
    std::iter::once(node)
        .chain(node.ancestors().filter_map(scraper::ElementRef::wrap))
        .take_while(|ancestor| ancestor.id() != root.id())
        .any(|ancestor| {
            matches!(
                ancestor.value().name(),
                "footer" | "nav" | "aside" | "article" | "script" | "style"
            )
        })
}

fn block_text(root: scraper::ElementRef<'_>) -> String {
    root.descendants()
        .filter_map(|node| {
            let text = node.value().as_text()?;
            let parent = node.parent().and_then(scraper::ElementRef::wrap)?;
            (!excluded_context(parent, root)).then_some(&**text)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn closed_posting(text: &str) -> bool {
    let fragment;
    let text = if text.contains('<') {
        fragment = Html::parse_fragment(text);
        block_text(fragment.root_element())
    } else {
        text.to_owned()
    };
    let text = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    [
        "position is closed",
        "position has been filled",
        "role is closed",
        "role has been filled",
        "job is closed",
        "job has expired",
        "no longer accepting applications",
        "applications are closed",
        "this position is no longer available",
        "this role is no longer available",
    ]
    .iter()
    .any(|phrase| text.contains(phrase))
}

fn closed_block(root: scraper::ElementRef<'_>) -> bool {
    closed_posting(&block_text(root))
}

fn visible_locator(root: scraper::ElementRef<'_>, raw: &str) -> &'static str {
    for (selector, locator) in [
        ("dt + dd", "dt + dd publication label"),
        ("time", "time publication label"),
    ] {
        let selector = Selector::parse(selector).expect("valid selector");
        if root.select(&selector).any(|node| {
            node.value().attr("datetime") == Some(raw)
                || node.text().collect::<Vec<_>>().join(" ").contains(raw)
        }) {
            return locator;
        }
    }
    "explicit publication label"
}
fn excluded_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.split('/').any(|part| {
        matches!(
            part,
            "privacy" | "terms" | "donate" | "donation" | "login" | "logout"
        )
    }) || path
        .rsplit('/')
        .next()
        .and_then(|part| part.rsplit_once('.'))
        .is_some_and(|(_, ext)| {
            matches!(
                ext,
                "png"
                    | "jpg"
                    | "jpeg"
                    | "svg"
                    | "gif"
                    | "webp"
                    | "pdf"
                    | "zip"
                    | "css"
                    | "js"
                    | "woff"
                    | "woff2"
            )
        })
}
fn non_job_navigation(path: &str) -> bool {
    path.split('/').any(|segment| {
        matches!(
            segment.to_ascii_lowercase().as_str(),
            "event-calendar" | "news-events" | "post" | "posts" | "documents"
        )
    })
}

fn relevant(value: &str) -> bool {
    value
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| {
            matches!(
                token,
                "career"
                    | "careers"
                    | "hiring"
                    | "job"
                    | "jobs"
                    | "vacancy"
                    | "vacancies"
                    | "opening"
                    | "openings"
                    | "position"
                    | "positions"
                    | "role"
                    | "roles"
            )
        })
}
fn explicit_company(document: &Html) -> Option<String> {
    let selector = Selector::parse(
        "[itemprop='hiringOrganization'] [itemprop='name'], meta[property='og:site_name']",
    )
    .ok()?;
    document.select(&selector).find_map(|node| {
        let value = node
            .value()
            .attr("content")
            .map(str::to_owned)
            .unwrap_or_else(|| node.text().collect::<Vec<_>>().join(" "));
        let value = value.trim();
        (!value.is_empty() && value.len() <= 200).then(|| value.to_owned())
    })
}
#[cfg(test)]
fn visible_date(document: &Html) -> Option<String> {
    visible_date_in(document.root_element())
}
fn visible_date_in(root: scraper::ElementRef<'_>) -> Option<String> {
    let found = visible_dates_in(root)?;
    if found.len() == 1 {
        found.into_iter().next()
    } else {
        None
    }
}
fn normalize_visible_date(raw: &str) -> Option<String> {
    if scrape_core::posting_date::posting_instant(raw, "UTC").is_some()
        || scrape_core::posting_date::relative_age_seconds(raw).is_some()
    {
        return Some(raw.to_owned());
    }
    let cleaned = raw.replace('.', "");
    ["%B %d, %Y", "%b %d, %Y"]
        .iter()
        .find_map(|format| chrono::NaiveDate::parse_from_str(&cleaned, format).ok())
        .map(|date| date.format("%Y-%m-%d").to_string())
}
fn visible_dates_in(root: scraper::ElementRef<'_>) -> Option<HashSet<String>> {
    use std::sync::LazyLock;
    static LABEL: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^(?:posted|published)\s*:?\s*(.+?)\s*$")
            .expect("valid publication-label regex")
    });
    static RELATIVE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^(\d+\s+(?:minutes?|hours?|days?)\s+ago)[.!]?$")
            .expect("valid relative-publication regex")
    });
    let selector = Selector::parse("time, p, dd, li, span").ok()?;
    let timed = Selector::parse("time").ok()?;
    let mut found = HashSet::new();
    for node in root.select(&selector) {
        if excluded_context(node, root)
            || (node.value().name() != "time" && node.select(&timed).next().is_some())
        {
            continue;
        }
        let text = node.text().collect::<Vec<_>>().join(" ");
        let context = match node.value().name() {
            "dd" => {
                let previous = node.prev_siblings().find_map(scraper::ElementRef::wrap);
                match previous.filter(|node| node.value().name() == "dt") {
                    Some(label) => format!("{} {text}", label.text().collect::<Vec<_>>().join(" ")),
                    None => text.clone(),
                }
            }
            "time" => node
                .value()
                .attr("aria-label")
                .map(str::to_owned)
                .filter(|label| LABEL.is_match(label))
                .or_else(|| LABEL.is_match(text.trim()).then(|| text.clone()))
                .or_else(|| {
                    node.parent()
                        .and_then(scraper::ElementRef::wrap)
                        .filter(|parent| {
                            parent.id() == root.id() || !excluded_context(*parent, root)
                        })
                        .map(|parent| {
                            let text = parent.text().collect::<Vec<_>>().join(" ");
                            if parent.value().name() == "dd" {
                                if let Some(label) = parent
                                    .prev_siblings()
                                    .find_map(scraper::ElementRef::wrap)
                                    .filter(|node| node.value().name() == "dt")
                                {
                                    return format!(
                                        "{} {text}",
                                        label.text().collect::<Vec<_>>().join(" ")
                                    );
                                }
                            }
                            text
                        })
                })
                .unwrap_or_default(),
            _ => text,
        };
        let label = context.trim();
        if label.len() > 200 {
            continue;
        }
        let lower = label.to_ascii_lowercase();
        if lower.contains("updated") || lower.contains("modified") {
            continue;
        }
        let Some(captures) = LABEL.captures(label) else {
            continue;
        };
        let value = node
            .value()
            .attr("datetime")
            .unwrap_or_else(|| captures.get(1).expect("publication value").as_str())
            .trim();
        let raw = RELATIVE
            .captures(value)
            .and_then(|captures| captures.get(1).map(|value| value.as_str()))
            .unwrap_or(value);
        if normalize_visible_date(raw).is_some() {
            found.insert(raw.to_owned());
        }
    }
    Some(found)
}

#[cfg(test)]
mod relative_date_tests {
    use super::*;

    #[test]
    fn relative_publication_requires_an_unambiguous_visible_posted_label() {
        assert_eq!(
            visible_date(&Html::parse_document("<p>Posted: 2 hours ago</p>")).as_deref(),
            Some("2 hours ago")
        );
        assert_eq!(
            visible_date(&Html::parse_document("<p>Updated: 2 hours ago</p>")),
            None
        );
        assert_eq!(
            visible_date(&Html::parse_document("<p>Posted: about 2 hours ago</p>")),
            None
        );
        assert_eq!(
            visible_date(&Html::parse_document("<p>Posted: 30+ days ago</p>")),
            None
        );
        assert_eq!(
            visible_date(&Html::parse_document(
                "<p>Posted: 1 day ago</p><p>Published: 3 days ago</p>"
            )),
            None
        );
    }

    fn observed() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-31T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn visible_publication_supports_scoped_definition_lists_and_labeled_times() {
        for (html, expected) in [
            ("<main><dl><dt>Posted:</dt><dd>Oct. 30, 2026</dd></dl></main>", "Oct. 30, 2026"),
            ("<main><p>Published: <time datetime='2026-10-30T09:00:00Z'>October 30</time></p></main>", "2026-10-30T09:00:00Z"),
            ("<main><time aria-label='Posted: October 30, 2026'>October 30, 2026</time></main>", "October 30, 2026"),
        ] {
            assert_eq!(visible_date(&Html::parse_document(html)).as_deref(), Some(expected), "{html}");
        }
        for html in [
            "<dl><dt>Updated:</dt><dd>October 30, 2026</dd></dl>",
            "<p>Posted: 10/30/2026</p>",
            "<footer><p>Posted: October 30, 2026</p></footer>",
            "<article><h2>Company news</h2><p>Published: October 30, 2026</p></article>",
            "<meta property='article:published_time' content='2026-10-30T09:00:00Z'>",
        ] {
            assert!(
                visible_date(&Html::parse_document(html)).is_none(),
                "{html}"
            );
        }
    }

    #[test]
    fn mixed_precision_selects_the_narrower_real_value() {
        let at = observed();
        assert_eq!(
            narrower_date(
                Some("2026-10-30"),
                at,
                Some("2026-10-30T09:00:00Z"),
                at,
                Some("UTC")
            ),
            Ok(true)
        );
        assert_eq!(
            narrower_date(
                Some("2026-10-30T09:00:00Z"),
                at,
                Some("2026-10-30"),
                at,
                Some("UTC")
            ),
            Ok(false)
        );
        assert_eq!(
            narrower_date(
                Some("2026-10-30T09:00:00Z"),
                at,
                Some("2026-10-30T05:00:00-04:00"),
                at,
                None
            ),
            Ok(false)
        );
        assert_eq!(
            narrower_date(Some("2026-10-30"), at, Some("2026-10-30"), at, None),
            Ok(false)
        );
        assert!(narrower_date(
            Some("2026-10-30"),
            at,
            Some("2026-10-30T09:00:00Z"),
            at,
            None
        )
        .unwrap_err()
        .contains("timezone"));
        assert!(narrower_date(
            Some("2026-10-30"),
            at,
            Some("2026-10-31T00:00:00Z"),
            at,
            Some("UTC")
        )
        .is_err());
        assert!(narrower_date(Some("2026-10-30"), at, Some("1 day ago"), at, Some("UTC")).is_err());
    }

    #[test]
    fn date_only_expiry_uses_the_next_local_midnight() {
        let before = DateTime::parse_from_rfc3339("2026-11-02T04:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        let boundary = DateTime::parse_from_rfc3339("2026-11-02T05:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            expiry("2026-11-01", Some("America/New_York"), before),
            Some(false)
        );
        assert_eq!(
            expiry("2026-11-01", Some("America/New_York"), boundary),
            Some(true)
        );
        assert_eq!(expiry("2026-11-01", None, boundary), None);
        assert_eq!(expiry("2026-11-02T05:00:00Z", None, boundary), Some(false));
    }

    #[test]
    fn closure_is_scoped_to_the_individual_posting_not_the_footer() {
        let document = Html::parse_document("<main><h1>Software Engineer</h1><p>Applications welcome.</p><footer>The previous position is closed.</footer></main>");
        assert!(!closed_block(
            posting_block(&document, "Software Engineer").unwrap()
        ));
        let document = Html::parse_document("<article><h2>Software Engineer</h2><p>No longer accepting applications.</p></article><article><h2>Other role</h2><p>Applications welcome.</p></article>");
        assert!(closed_block(
            posting_block(&document, "Software Engineer").unwrap()
        ));
        assert!(!closed_block(
            posting_block(&document, "Other role").unwrap()
        ));
    }

    #[test]
    fn listing_merges_preserve_original_relative_observation_and_reject_conflicts() {
        let url = Url::parse("https://employer.example/jobs/one").unwrap();
        let page = Url::parse("https://employer.example/careers").unwrap();
        let make = |raw: &str| ListingEvidence {
            detail_url: url.clone(),
            identifier: Some("req-1".into()),
            title: "Software Engineer".into(),
            date_posted: Some(raw.into()),
            evidence: Some(date_evidence(
                raw,
                &page,
                "json_ld",
                "JobPosting.datePosted",
                observed(),
            )),
            indeterminate: false,
        };
        let first = make("1 day ago");
        let merged = merge_listing(Some(first.clone()), None, Some("UTC")).unwrap();
        assert_eq!(
            merged.evidence.unwrap().observed_at,
            observed().to_rfc3339()
        );
        let conflicting = merge_listing(
            Some(make("2026-10-30")),
            Some(make("2026-10-28")),
            Some("UTC"),
        )
        .unwrap();
        assert!(conflicting.indeterminate);
        let mut other = first.clone();
        other.identifier = Some("req-2".into());
        assert!(
            merge_listing(Some(first), Some(other), Some("UTC"))
                .unwrap()
                .indeterminate
        );
    }
}
