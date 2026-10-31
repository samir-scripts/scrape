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
}
type FetchResult = (Entry, Result<(Url, Vec<u8>, Vec<String>), PublicWebError>);

pub(crate) struct WebDiscoverySession {
    source: WorkerSource,
    config: WebDiscoveryConfig,
    extraction: StaticSourceConfig,
    transport: PublicWeb,
    deadline: Instant,
    started: DateTime<Utc>,
    page_limit: u32,
    pages: u32,
    origins: HashSet<String>,
    sitemap_counts: HashMap<String, u32>,
    sitemap_entries: HashMap<String, u32>,
    queues: HashMap<String, VecDeque<Entry>>,
    round_robin: VecDeque<String>,
    in_flight: HashSet<String>,
    seen: HashSet<String>,
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

impl WebDiscoverySession {
    pub fn new(
        source: &WorkerSource,
        limits: &WorkerLimits,
        workflow_deadline: Instant,
        root_transport: &PublicWeb,
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
            config,
            extraction,
            transport,
            started,
            deadline: workflow_deadline
                .min(Instant::now() + Duration::from_secs(limits.max_source_duration_s)),
            page_limit,
            pages: 0,
            origins: HashSet::new(),
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
        if !self.origins.contains(&host) && self.origins.len() >= self.config.max_domains as usize {
            self.bounded = true;
            return;
        }
        if entry.sitemap {
            let count = self.sitemap_counts.entry(host.clone()).or_default();
            if *count >= 5 {
                self.bounded = true;
                return;
            }
            *count += 1;
        }
        self.origins.insert(host.clone());
        self.seen.insert(key);
        let queue = self.queues.entry(host.clone()).or_default();
        if queue.is_empty() {
            self.round_robin.push_back(host);
        }
        queue.push_back(entry);
    }
    fn schedule(&mut self) {
        let mut attempts = self.round_robin.len();
        while self.tasks.len() < 4
            && attempts > 0
            && self.pages + (self.tasks.len() as u32) < self.page_limit
        {
            attempts -= 1;
            let Some(host) = self.round_robin.pop_front() else {
                break;
            };
            if self.in_flight.contains(&host) {
                self.round_robin.push_back(host);
                continue;
            }
            let Some(queue) = self.queues.get_mut(&host) else {
                continue;
            };
            let Some(entry) = queue.pop_front() else {
                continue;
            };
            if !queue.is_empty() {
                self.round_robin.push_back(host.clone());
            }
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
                self.seen.insert(actual.to_string());
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
                    });
                }
            } else if relevant(url.as_str()) {
                self.enqueue(Entry {
                    url,
                    seed: entry.seed.clone(),
                    referrer: Some(actual.to_string()),
                    depth: entry.depth.saturating_add(1),
                    sitemap: false,
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
            let visible_values = if structured.len() == 1 {
                visible_dates_in(document.root_element()).unwrap_or_default()
            } else {
                HashSet::new()
            };
            let visible = (visible_values.len() == 1)
                .then(|| visible_values.iter().next().cloned())
                .flatten();
            for mut event in structured {
                if let WorkerEvent::Job { date_posted, .. } = &mut event {
                    if visible_values.len() > 1
                        || date_posted
                            .as_ref()
                            .zip(visible.as_ref())
                            .is_some_and(|(posted, date)| posted != date)
                    {
                        self.pending.push_back((WorkerEvent::Warning {
                            source_id: self.source.id.clone(),
                            message: format!("Conflicting publication dates at {actual}; date is indeterminate"),
                            eligibility_incomplete: true,
                            termination: None,
                        }, None));
                        *date_posted = None;
                    } else if date_posted.is_none()
                        && visible.as_deref().is_some_and(|v| {
                            scrape_core::posting_date::relative_age_seconds(v).is_some()
                        })
                    {
                        *date_posted = visible.clone();
                    }
                }
                if !expired_structured(&document, &event, self.started) {
                    self.accept(event, entry, actual, "json_ld", "JobPosting.datePosted");
                }
            }
        } else {
            let block_selector =
                Selector::parse("article, section, [data-requisition-id], [data-job-id]")
                    .expect("valid selector");
            let heading_selector = Selector::parse("h2, h3, h4").expect("valid selector");
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
                let block_text = block.text().collect::<Vec<_>>().join(" ");
                let lower = block_text.to_ascii_lowercase();
                if title.len() < 3
                    || title.len() > 120
                    || lower.contains("position is closed")
                    || lower.contains("no longer accepting")
                {
                    continue;
                }
                let identifier = block
                    .value()
                    .attr("data-requisition-id")
                    .or_else(|| block.value().attr("data-job-id"))
                    .map(str::to_owned)
                    .or_else(|| {
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
                    let visible = document
                        .root_element()
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_ascii_lowercase();
                    if visible.contains("position is closed")
                        || visible.contains("no longer accepting applications")
                    {
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
                        *date_posted = visible_date(&document);
                    }
                    emitted_detail = true;
                    self.accept(
                        event,
                        entry,
                        actual,
                        "visible_publication",
                        "explicit publication label",
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
                    let text = main
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_ascii_lowercase();
                    if has_apply
                        && has_responsibilities
                        && has_qualifications
                        && !text.contains("position is closed")
                        && !text.contains("no longer accepting applications")
                    {
                        if let Some(title) = main.select(&title_selector).next() {
                            let title =
                                title.text().collect::<Vec<_>>().join(" ").trim().to_owned();
                            if !title.is_empty() && title.len() <= 200 {
                                let posting = selector::SelectorPosting {
                                    title,
                                    url: actual.to_string(),
                                    external_id: actual.to_string(),
                                    location: String::new(),
                                    salary_text: None,
                                    description_source: Some(main.html()),
                                    description_format: Some(
                                        scrape_core::enrich::DescriptionFormat::Html,
                                    ),
                                    date_posted: visible_date_in(main),
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
                                    *date_posted = visible_date_in(main);
                                }
                                self.accept(
                                    event,
                                    entry,
                                    actual,
                                    "visible_publication",
                                    "explicit publication label",
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
                });
            }
        }
        let anchors = Selector::parse("a[href]").expect("valid selector");
        for anchor in document.select(&anchors) {
            let Some(raw) = anchor.value().attr("href") else {
                continue;
            };
            let Ok(url) = actual.join(raw) else { continue };
            // Wikipedia references are not official employer websites.
            // Its infobox website was enqueued separately above.
            if actual.host_str() == Some("en.wikipedia.org") && origin(&url) != origin(actual) {
                continue;
            }
            let text = anchor
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase();
            let href = url.as_str().to_ascii_lowercase();
            let social = [
                "linkedin.com",
                "facebook.com",
                "twitter.com",
                "instagram.com",
                "youtube.com",
            ]
            .iter()
            .any(|v| href.contains(v));
            let apply = text.contains("apply") || href.contains("/apply");
            let profile = origin(&url) == origin(actual)
                && (href.contains("/companies/") || href.contains("/company/"));
            let official = origin(&url) != origin(actual)
                && (text.contains("website")
                    || text.contains("homepage")
                    || text.contains("official site"));
            if !social && !apply && (relevant(&href) || relevant(&text) || profile || official) {
                self.enqueue(Entry {
                    url,
                    seed: entry.seed.clone(),
                    referrer: Some(actual.to_string()),
                    depth: entry.depth.saturating_add(1),
                    sitemap: false,
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
    ) {
        if let WorkerEvent::Job {
            url,
            external_id,
            date_posted,
            discovery_provenance,
            ..
        } = &mut event
        {
            // A listing's structured data is a lead, not proof that the employer page was fetched.
            if url.as_str() != actual.as_str() {
                if let Ok(target) = actual.join(url.as_str()) {
                    self.enqueue(Entry {
                        url: target,
                        seed: entry.seed.clone(),
                        referrer: Some(actual.to_string()),
                        depth: entry.depth.saturating_add(1),
                        sitemap: false,
                    });
                }
                return;
            }
            let key = format!("{}\n{}", actual, external_id);
            if !self.jobs_seen.insert(key) {
                return;
            }
            let observed = Utc::now();
            let evidence = date_posted.as_ref().map(|raw| {
                let precision = scrape_core::posting_date::relative_age_seconds(raw)
                    .map(|(age, unit)| format!("relative:{age}:{unit}"))
                    .unwrap_or_else(|| {
                        if raw.len() == 10 {
                            "date".into()
                        } else {
                            "timestamp".into()
                        }
                    });
                DiscoveryDateEvidence {
                    kind: if precision.starts_with("relative:") {
                        "visible_publication".into()
                    } else {
                        kind.into()
                    },
                    raw_value: raw.clone(),
                    page_url: actual.to_string(),
                    locator: if precision.starts_with("relative:") {
                        "explicit publication label".into()
                    } else {
                        locator.into()
                    },
                    observed_at: observed.to_rfc3339(),
                    precision,
                }
            });
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
}
fn expired_structured(document: &Html, event: &WorkerEvent, started: DateTime<Utc>) -> bool {
    let WorkerEvent::Job {
        title, external_id, ..
    } = event
    else {
        return false;
    };
    fn collect(
        value: &serde_json::Value,
        title: &str,
        id: &str,
        matches: &mut Vec<bool>,
        started: DateTime<Utc>,
    ) {
        match value {
            serde_json::Value::Array(items) => {
                for item in items {
                    collect(item, title, id, matches, started);
                }
            }
            serde_json::Value::Object(map) => {
                let is_job = match map.get("@type") {
                    Some(serde_json::Value::String(kind)) => kind == "JobPosting",
                    Some(serde_json::Value::Array(kinds)) => {
                        kinds.iter().any(|kind| kind.as_str() == Some("JobPosting"))
                    }
                    _ => false,
                };
                let identifier = map.get("identifier").and_then(|value| {
                    value
                        .as_str()
                        .or_else(|| value.get("value").and_then(|v| v.as_str()))
                });
                if is_job
                    && map.get("title").and_then(|v| v.as_str()) == Some(title)
                    && identifier.is_none_or(|value| value == id)
                {
                    let expired = map
                        .get("validThrough")
                        .and_then(|value| value.as_str())
                        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                        .is_some_and(|end| end.with_timezone(&Utc) < started);
                    matches.push(expired);
                }
                if let Some(graph) = map.get("@graph") {
                    collect(graph, title, id, matches, started);
                }
            }
            _ => {}
        }
    }
    let selector = Selector::parse("script[type='application/ld+json']").expect("valid selector");
    let mut matches = Vec::new();
    for node in document.select(&selector) {
        if let Ok(value) =
            serde_json::from_str::<serde_json::Value>(&node.text().collect::<String>())
        {
            collect(&value, title, external_id, &mut matches, started);
        }
    }
    matches.len() == 1 && matches[0]
}
fn relevant(value: &str) -> bool {
    [
        "career", "hiring", "jobs", "job/", "vacanc", "opening", "position", "role/",
    ]
    .iter()
    .any(|needle| value.contains(needle))
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
fn visible_dates_in(root: scraper::ElementRef<'_>) -> Option<HashSet<String>> {
    let selector =
        Selector::parse("time[datetime], p, dd, li, span, meta[property='article:published_time']")
            .ok()?;
    let iso = regex::Regex::new(
        r"\b\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2}))?\b",
    )
    .ok()?;
    let relative = regex::Regex::new(
        r"(?i)^(?:posted|published)\s*:?\s*(\d+\s+(?:minutes?|hours?|days?)\s+ago)\s*[.!]?$",
    )
    .ok()?;
    let mut found = HashSet::new();
    for node in root.select(&selector) {
        let (label, raw) = if node.value().name() == "meta" {
            (
                "published".to_owned(),
                node.value().attr("content").unwrap_or("").to_owned(),
            )
        } else {
            let text = node.text().collect::<Vec<_>>().join(" ");
            let context = if node.value().name() == "time" {
                node.parent()
                    .and_then(scraper::ElementRef::wrap)
                    .map(|p| p.text().collect::<Vec<_>>().join(" "))
                    .unwrap_or_default()
            } else {
                text.clone()
            };
            let raw = node
                .value()
                .attr("datetime")
                .map(str::to_owned)
                .or_else(|| iso.find(&text).map(|m| m.as_str().to_owned()))
                .or_else(|| {
                    relative
                        .captures(context.trim())
                        .and_then(|caps| caps.get(1))
                        .map(|m| m.as_str().to_owned())
                })
                .unwrap_or_default();
            (context.to_ascii_lowercase(), raw)
        };
        if label.len() > 200
            || label.contains("updated")
            || label.contains("modified")
            || !(label.contains("posted") || label.contains("published"))
        {
            continue;
        }
        if scrape_core::posting_date::posting_instant(&raw, "UTC").is_some()
            || scrape_core::posting_date::relative_age_seconds(&raw).is_some()
        {
            found.insert(raw);
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
}
