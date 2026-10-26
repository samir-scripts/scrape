//! Resumable static pages: one candidate per call; page boundaries yield to peers.
use super::selector::{extract_page, page_url, SelectorCandidate, SelectorPage, SelectorPosting};
use crate::browser::{resolve_source_session_cookies, BrowserPool};
use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, Utc};
use scrape_core::{
    browser::BrowserCookie,
    extraction::ExtractionMode,
    matching::{MatchingEvaluator, MatchingEvidence},
    protocol::{CoverageTermination, EnrichmentLimits},
    static_source::StaticSourceConfig,
    WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource,
};
use scraper::Html;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::time::{timeout_at, Instant};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct CrawlSearchTerms {
    pub target_roles: Vec<String>,
    pub any_keywords: Vec<String>,
    pub all_keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
}
impl CrawlSearchTerms {
    pub fn from_filters(filters: &WorkerFilters) -> Self {
        let mut target_roles = Vec::new();
        let mut any_keywords = filters.include_keywords.clone();
        let mut all_keywords = Vec::new();
        let mut exclude_keywords = filters.exclude_keywords.clone();
        if let Some(rules) = &filters.matching {
            target_roles.extend(rules.target_roles.iter().cloned());
            if let Some(kw) = &rules.keywords {
                any_keywords.extend(kw.any.iter().cloned());
                all_keywords.extend(kw.all.iter().cloned());
                exclude_keywords.extend(kw.exclude.iter().cloned());
            }
        }
        Self {
            target_roles,
            any_keywords,
            all_keywords,
            exclude_keywords,
        }
    }
}

pub fn score_crawl_link(url: &Url, anchor_text: &str, terms: &CrawlSearchTerms) -> i32 {
    let path_lower = url.path().to_ascii_lowercase();
    let text_lower = anchor_text.to_ascii_lowercase();
    let mut score = 0i32;

    for exclude in &terms.exclude_keywords {
        let ex = exclude.trim().to_ascii_lowercase();
        if !ex.is_empty() && (text_lower.contains(&ex) || path_lower.contains(&ex)) {
            return -30;
        }
    }

    if path_lower.contains("/privacy")
        || path_lower.contains("/terms")
        || path_lower.contains("/cookie")
        || path_lower.contains("/press")
        || path_lower.contains("/blog")
        || path_lower.contains("/news")
        || path_lower.contains("/legal")
        || path_lower.contains("/security")
        || path_lower.contains("/faq")
        || path_lower.contains("/help")
    {
        score -= 5;
    }

    if path_lower.contains("/job/")
        || path_lower.contains("/jobs/")
        || path_lower.contains("/career/")
        || path_lower.contains("/careers/")
        || path_lower.contains("/position/")
        || path_lower.contains("/positions/")
        || path_lower.contains("/opening/")
        || path_lower.contains("/openings/")
        || path_lower.contains("/vacancy/")
        || path_lower.contains("/vacancies/")
        || path_lower.contains("/roles/")
        || url.query().is_some_and(|q| {
            q.contains("gh_jid=") || q.contains("lever-") || q.contains("ashby_jid=")
        })
    {
        score += 5;
    }

    for role in &terms.target_roles {
        let r = role.trim().to_ascii_lowercase();
        if !r.is_empty() && (text_lower.contains(&r) || path_lower.contains(&r)) {
            score += 15;
            break;
        }
    }

    for kw in &terms.any_keywords {
        let k = kw.trim().to_ascii_lowercase();
        if !k.is_empty() && (text_lower.contains(&k) || path_lower.contains(&k)) {
            score += 8;
            break;
        }
    }

    for kw in &terms.all_keywords {
        let k = kw.trim().to_ascii_lowercase();
        if !k.is_empty() && (text_lower.contains(&k) || path_lower.contains(&k)) {
            score += 8;
            break;
        }
    }

    score
}

pub(crate) enum SelectorStep {
    Event(Box<(WorkerEvent, Option<MatchingEvidence>)>),
    Yield,
    Exhausted,
}
pub(crate) struct SelectorSession {
    source: WorkerSource,
    mode: ExtractionMode,
    browser: Option<Arc<BrowserPool>>,
    session_cookies: Vec<BrowserCookie>,
    config: StaticSourceConfig,
    page: SelectorPage,
    visited: HashSet<String>,
    crawl_origin: Option<Url>,
    frontier: VecDeque<(Url, u32)>,
    queued: HashSet<String>,
    crawl_bound_reached: bool,
    crawl_depth: u32,
    // Retained only when explicit detail enrichment needs already-fetched pages.
    crawl_cache: HashMap<String, (String, String)>,
    seen: HashSet<String>,
    pages_visited: u32,
    page_limit_reached: bool,
    source_deadline: Instant,
    detail_budget: Duration,
    navigation_timeout: Duration,
    detail_limit: u32,
    detail_requests: u32,
    detail_bound_reported: bool,
    exhausted: bool,
    pending: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
    crawl_terms: CrawlSearchTerms,
}
impl SelectorSession {
    pub async fn new(
        client: &HttpClient,
        browser: Option<Arc<BrowserPool>>,
        source: &WorkerSource,
        limits: &WorkerLimits,
        enrichment: &EnrichmentLimits,
        workflow_deadline: Instant,
        filters: &WorkerFilters,
    ) -> Result<Self, ClientError> {
        let mode =
            scrape_core::extraction::mode(&source.config).map_err(ClientError::Configuration)?;
        let config = StaticSourceConfig::parse(&source.config, limits)
            .map_err(ClientError::Configuration)?;
        // Compile all posting locators/mappings before any source request.
        if !config.crawl.as_ref().is_some_and(|crawl| {
            matches!(
                crawl.extraction,
                scrape_core::static_source::CrawlExtraction::Jsonld
                    | scrape_core::static_source::CrawlExtraction::PublicHtml
            )
        }) && scrape_core::hydration::HydrationConfig::parse(&source.config)
            .map_err(ClientError::Configuration)?
            .is_none()
        {
            scrape_core::locator::QueryEngine::parse_elements(
                source
                    .config
                    .get("item")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".job-item"),
            )
            .map_err(ClientError::Configuration)?;
            for (field, default) in [
                ("title", Some(".job-title")),
                ("link", Some("a")),
                ("id", None),
                ("description", None),
                ("salary", None),
                ("date", None),
                ("location", None),
            ] {
                scrape_core::locator::FieldLocator::from_config(&source.config, field, default)
                    .map_err(ClientError::Configuration)?;
            }
        }
        let source_deadline =
            workflow_deadline.min(Instant::now() + Duration::from_secs(config.duration_s));
        let navigation_timeout = Duration::from_secs(config.navigation_timeout_s);
        let url = page_url(&Url::parse(&source.identifier)?, &source.identifier)?;

        let (html, actual, pages_visited, page_limit_reached, session_cookies) = if mode
            == ExtractionMode::Browser
        {
            let cookies = resolve_source_session_cookies(&source.config, &url)?;
            let pool = browser.as_ref().ok_or_else(|| {
                ClientError::Configuration("Browser pool was not initialized".into())
            })?;
            let rendered = pool
                .fetch_rendered_page(
                    client,
                    &url,
                    source,
                    &config,
                    &cookies,
                    config.max_pages,
                    navigation_timeout,
                    source_deadline,
                )
                .await?;
            (
                rendered.html,
                rendered.actual_url,
                rendered.pages_consumed,
                rendered.page_limit_reached,
                cookies,
            )
        } else {
            let fetch = async {
                let scoped_client = client.for_static_source(&config);
                if config.crawl.is_some() {
                    scoped_client
                        .fetch_text_same_origin_at(url.as_str(), &url, true)
                        .await
                } else {
                    scoped_client.fetch_text_at(url.as_str(), true).await
                }
            };
            let (html, actual) = timeout_at(
                    source_deadline.min(Instant::now() + navigation_timeout),
                    fetch,
                )
                .await
                .map_err(|_| {
                    ClientError::TimeLimit(
                        "Static source initial navigation reached its time limit".into(),
                    )
                })?
                .map_err(|error| match error {
                    ClientError::HttpStatus(status, _) if config.crawl.is_some() && matches!(status.as_u16(), 401 | 403) =>
                        ClientError::NeedsLogin(format!("Website crawl requires login or permission (HTTP {status}); only public HTML is supported")),
                    error => error,
                })?;
            (html, actual, 1, false, Vec::new())
        };

        let actual = page_url(&url, &actual)?;
        let page = extract_page(&html, &actual, source, &config)?;
        if config.crawl.is_some()
            && scrape_core::browser::is_login_wall(
                &url,
                actual.as_str(),
                "",
                false,
                !page.candidates.is_empty(),
            )
        {
            return Err(ClientError::NeedsLogin(
                "Website crawl redirected to a login page; only public HTML is supported".into(),
            ));
        }
        let mut visited = HashSet::new();
        visited.insert(url.to_string());
        visited.insert(actual.to_string());
        // Redirect aliases must also prevent loops, without counting as another page.
        let crawl_origin = config.crawl.as_ref().map(|_| url.clone());
        let mut session = Self {
            source: source.clone(),
            mode,
            browser,
            session_cookies,
            config,
            page,
            visited,
            crawl_origin,
            frontier: VecDeque::new(),
            queued: HashSet::new(),
            crawl_bound_reached: false,
            crawl_depth: 0,
            crawl_cache: HashMap::new(),
            seen: HashSet::new(),
            pages_visited,
            page_limit_reached,
            source_deadline,
            detail_budget: Duration::from_millis(enrichment.timeout_ms),
            navigation_timeout,
            detail_limit: enrichment.max_requests,
            detail_requests: 0,
            detail_bound_reported: false,
            exhausted: false,
            pending: VecDeque::new(),
            crawl_terms: CrawlSearchTerms::from_filters(filters),
        };
        session.enqueue_links(&html, &actual, 0);
        if session.config.crawl.is_some() && session.config.has_details() {
            session
                .crawl_cache
                .insert(url.to_string(), (html, actual.to_string()));
        }
        Ok(session)
    }
    pub fn detail_requests(&self) -> u32 {
        self.detail_requests
    }
    pub async fn next(
        &mut self,
        client: &HttpClient,
        filters: &WorkerFilters,
        started: DateTime<Utc>,
    ) -> Result<SelectorStep, ClientError> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(SelectorStep::Event(Box::new(event)));
        }
        if self.is_exhausted() {
            self.exhausted = true;
            return Ok(SelectorStep::Exhausted);
        }
        if Instant::now() >= self.source_deadline {
            return Ok(self.stop("Static source duration limit reached; remaining candidates/pages were not examined", CoverageTermination::TimeLimit));
        }
        let candidate = if let Some(candidate) = self.page.candidates.pop_front() {
            candidate
        } else {
            if self.config.crawl.is_some() {
                return self.advance_crawl(client).await;
            }
            if self.page_limit_reached {
                self.page_limit_reached = false;
                return Ok(self.stop(
                    "Static page limit reached; remaining pages were not visited",
                    CoverageTermination::PageLimit,
                ));
            }
            if let Some(error) = self.page.next_page_error.take() {
                return Ok(self.stop(
                    &format!("Invalid next-page destination: {error}"),
                    CoverageTermination::Failed,
                ));
            }
            let Some(url) = self.page.next_page.take() else {
                self.exhausted = true;
                return Ok(SelectorStep::Exhausted);
            };
            if self.visited.contains(url.as_str()) {
                return Ok(self.stop(
                    "Pagination loop detected; this page has already been visited",
                    CoverageTermination::Failed,
                ));
            }
            if self.pages_visited >= self.config.max_pages {
                return Ok(self.stop(
                    "Static page limit reached; remaining pages were not visited",
                    CoverageTermination::PageLimit,
                ));
            }
            let (html, actual, pages_consumed, limit_hit) = if self.mode == ExtractionMode::Browser
            {
                let Some(pool) = self.browser.as_ref() else {
                    return Ok(self.stop(
                        "Browser pool unavailable for pagination",
                        CoverageTermination::Failed,
                    ));
                };
                let remaining = self.config.max_pages.saturating_sub(self.pages_visited);
                match pool
                    .fetch_rendered_page(
                        client,
                        &url,
                        &self.source,
                        &self.config,
                        &self.session_cookies,
                        remaining,
                        self.navigation_timeout,
                        self.source_deadline,
                    )
                    .await
                {
                    Ok(rendered) => (
                        rendered.html,
                        rendered.actual_url,
                        rendered.pages_consumed,
                        rendered.page_limit_reached,
                    ),
                    Err(ClientError::TimeLimit(_)) => {
                        return Ok(self.stop(
                            "Browser pagination navigation reached its time limit",
                            CoverageTermination::TimeLimit,
                        ))
                    }
                    Err(ClientError::NeedsLogin(msg)) => {
                        return Ok(self.stop(&msg, CoverageTermination::NeedsLogin))
                    }
                    Err(error) => {
                        return Ok(self.stop(
                            &format!("Next page {url} unavailable: {error}"),
                            CoverageTermination::Failed,
                        ))
                    }
                }
            } else {
                let result = timeout_at(
                    self.source_deadline
                        .min(Instant::now() + self.navigation_timeout),
                    client.fetch_text_at(url.as_str(), true),
                )
                .await;
                match result {
                    Ok(Ok((h, a))) => (h, a, 1, false),
                    Ok(Err(error)) => {
                        return Ok(self.stop(
                            &format!("Next page {url} unavailable: {error}"),
                            CoverageTermination::Failed,
                        ))
                    }
                    Err(_) => {
                        return Ok(self.stop(
                            "Static pagination navigation reached its time limit",
                            CoverageTermination::TimeLimit,
                        ))
                    }
                }
            };
            let actual = page_url(&url, &actual)?;
            if self.visited.contains(actual.as_str()) {
                return Ok(self.stop(
                    "Pagination redirect returned to a visited page",
                    CoverageTermination::Failed,
                ));
            }
            self.visited.insert(url.into());
            self.visited.insert(actual.to_string());
            self.pages_visited += pages_consumed;
            self.page_limit_reached = limit_hit;
            self.page = match extract_page(&html, &actual, &self.source, &self.config) {
                Ok(page) => page,
                Err(e) => {
                    return Ok(self.stop(
                        &format!("Next page extraction failed: {e}"),
                        CoverageTermination::Failed,
                    ))
                }
            };
            match self.page.candidates.pop_front() {
                Some(candidate) => candidate,
                None => return Ok(SelectorStep::Yield),
            }
        };
        let mut posting = match candidate {
            SelectorCandidate::Posting(posting) => *posting,
            SelectorCandidate::Extracted(event) => {
                return Ok(SelectorStep::Event(Box::new((*event, None))))
            }
            SelectorCandidate::Invalid(error) => {
                return Ok(SelectorStep::Event(Box::new((*error, None))))
            }
        };
        let candidate = posting.clone().event(&self.source);
        if !self.config.has_details() && !self.config.is_public_html_crawl() {
            return Ok(SelectorStep::Event(Box::new((candidate, None))));
        }
        let matcher = MatchingEvaluator::new(filters).map_err(ClientError::Configuration)?;
        let mut evidence = evidence(&candidate);
        let identity = scrape_core::identity::normalized_detail_url(&posting.url)
            .map_err(ClientError::Configuration)?;
        if !self.seen.insert(identity)
            || !matcher
                .assess(&candidate, started, &evidence)
                .rejected
                .is_empty()
        {
            return Ok(SelectorStep::Event(Box::new((candidate, Some(evidence)))));
        }
        let mut warning = None;
        let mut detail_received = false;
        let bound = if self.detail_budget.is_zero() || Instant::now() >= self.source_deadline {
            Some(CoverageTermination::TimeLimit)
        } else if self.config.crawl.as_ref().is_some_and(|crawl| {
            self.pages_visited >= self.config.max_pages || self.crawl_depth >= crawl.max_depth
        }) {
            Some(CoverageTermination::PageLimit)
        } else if self.detail_requests >= self.detail_limit {
            Some(CoverageTermination::RequestLimit)
        } else {
            None
        };
        let cached_detail = posting.detail_url.as_ref().and_then(|url| {
            self.crawl_cache
                .get(url)
                .or_else(|| self.crawl_cache.values().find(|(_, actual)| actual == url))
        });
        if let Some((html, actual)) = cached_detail {
            match merge_detail(&mut posting, html, actual, &self.config, &self.source) {
                Ok(()) => detail_received = true,
                Err(error) => {
                    warning = Some((
                        format!("Cached website detail could not be extracted: {error}"),
                        Some(CoverageTermination::Failed),
                    ))
                }
            }
        } else if let Some(bound) = bound {
            if !self.detail_bound_reported {
                warning = Some(("Static detail enrichment reached its page/depth/request/time bound; missing evidence remains unknown".to_string(),Some(bound)));
                self.detail_bound_reported = true;
            }
        } else if let Some(url) = posting.detail_url.clone() {
            self.detail_requests += 1;
            if self.config.crawl.is_some() {
                self.pages_visited += 1;
            }
            let scoped_client = client.for_static_source(&self.config);
            let detail_fetch = async {
                if self.mode == ExtractionMode::Browser {
                    let pool = self.browser.as_ref().ok_or_else(|| {
                        ClientError::Configuration("Browser pool unavailable".into())
                    })?;
                    let parsed_detail = Url::parse(&url)?;
                    let mut detail_cfg = self.config.clone();
                    detail_cfg.load_more = None;
                    detail_cfg.load_more_raw = None;
                    detail_cfg.wait_for = self.config.detail_container.clone();
                    let rendered = pool
                        .fetch_rendered_page(
                            client,
                            &parsed_detail,
                            &self.source,
                            &detail_cfg,
                            &self.session_cookies,
                            1,
                            self.navigation_timeout,
                            self.source_deadline
                                .min(Instant::now() + self.detail_budget),
                        )
                        .await?;
                    Ok::<(String, String), ClientError>((rendered.html, rendered.actual_url))
                } else {
                    if let Some(origin) = &self.crawl_origin {
                        scoped_client
                            .fetch_text_same_origin_at(&url, origin, true)
                            .await
                    } else {
                        scoped_client.fetch_text_at(&url, true).await
                    }
                }
            };
            let call_deadline = self
                .source_deadline
                .min(Instant::now() + self.detail_budget)
                .min(Instant::now() + self.navigation_timeout);
            let call_started = Instant::now();
            let call_res = timeout_at(call_deadline, detail_fetch).await;
            self.detail_budget = self.detail_budget.saturating_sub(call_started.elapsed());
            match call_res {
                Ok(Ok((html, actual))) => {
                    match merge_detail(&mut posting, &html, &actual, &self.config, &self.source) {
                        Ok(()) => detail_received = true,
                        Err(e) => {
                            warning = Some((
                                format!("Static detail {url} could not be extracted: {e}"),
                                None,
                            ))
                        }
                    }
                    if self.config.crawl.is_some() {
                        self.visited.insert(url.clone());
                        self.crawl_cache.insert(url, (html, actual));
                    }
                }
                Ok(Err(e)) => {
                    let termination = if self.config.crawl.is_some() {
                        self.visited.insert(url.clone());
                        Some(match &e {
                            ClientError::NeedsLogin(_) => CoverageTermination::NeedsLogin,
                            ClientError::HttpStatus(status, _)
                                if matches!(status.as_u16(), 401 | 403) =>
                            {
                                CoverageTermination::NeedsLogin
                            }
                            ClientError::TimeLimit(_) => CoverageTermination::TimeLimit,
                            _ => CoverageTermination::Failed,
                        })
                    } else {
                        None
                    };
                    warning = Some((format!("Static detail {url} unavailable: {e}"), termination))
                }
                Err(_) => {
                    self.detail_budget = Duration::ZERO;
                    self.detail_bound_reported = true;
                    if self.config.crawl.is_some() {
                        self.visited.insert(url.clone());
                    }
                    warning=Some((format!("Static detail {url} reached its time limit; missing evidence remains unknown"),Some(CoverageTermination::TimeLimit)));
                }
            }
        } else {
            warning = Some((
                "Configured detail link did not produce a URL; inspect the locator and retry"
                    .into(),
                None,
            ));
        }
        let event = posting.event(&self.source);
        evidence = evidence_for_final(&event, self.config.is_public_html_crawl());
        let assessment = matcher.assess(&event, started, &evidence);
        let incomplete = assessment.rejected.is_empty() && !assessment.unresolved.is_empty();
        if detail_received && incomplete {
            warning=Some(("Static detail did not supply required eligibility evidence; missing fields remain unknown".into(),None));
        }
        if let Some((message, termination)) = warning {
            self.pending.push_back((event, Some(evidence)));
            return Ok(SelectorStep::Event(Box::new((
                WorkerEvent::Warning {
                    source_id: self.source.id.clone(),
                    message,
                    eligibility_incomplete: incomplete
                        || (self.config.crawl.is_some() && termination.is_some()),
                    termination,
                },
                None,
            ))));
        }
        Ok(SelectorStep::Event(Box::new((event, Some(evidence)))))
    }
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
    pub fn pages_visited(&self) -> u32 {
        self.pages_visited
    }
    pub fn is_exhausted(&self) -> bool {
        self.exhausted
            || (self.pending.is_empty()
                && self.page.candidates.is_empty()
                && self.page.next_page.is_none()
                && self.page.next_page_error.is_none()
                && self.frontier.is_empty()
                && !self.crawl_bound_reached
                && !self.page_limit_reached)
    }
    fn enqueue_links(&mut self, html: &str, actual: &Url, depth: u32) {
        let Some(crawl) = self.config.crawl.as_ref() else {
            return;
        };
        let origin = self.crawl_origin.as_ref().unwrap();
        let document = Html::parse_document(html);
        let base = super::selector::resolve_document_base(&document, actual);
        let anchors = scraper::Selector::parse("a[href]").expect("constant anchor selector");
        let mut candidates = Vec::new();
        for anchor in document.select(&anchors) {
            let Some(raw) = anchor.value().attr("href") else {
                continue;
            };
            let Ok(url) = page_url(&base, raw) else {
                continue;
            };
            if url.origin() != origin.origin()
                || self.visited.contains(url.as_str())
                || self.queued.contains(url.as_str())
            {
                continue;
            }
            if depth >= crawl.max_depth {
                self.crawl_bound_reached = true;
                continue;
            }
            let text = anchor.text().collect::<Vec<_>>().join(" ");
            let score = score_crawl_link(&url, &text, &self.crawl_terms);
            if score <= -30 {
                continue;
            }
            candidates.push((url, score));
        }

        candidates.sort_by_key(|(_, score)| std::cmp::Reverse(*score));

        for (url, _) in candidates {
            if self.frontier.len() as u32 > self.config.max_pages.saturating_sub(self.pages_visited)
            {
                self.crawl_bound_reached = true;
                continue;
            }
            self.queued.insert(url.to_string());
            self.frontier.push_back((url, depth + 1));
        }
    }

    async fn advance_crawl(&mut self, client: &HttpClient) -> Result<SelectorStep, ClientError> {
        let Some((url, depth)) = self.frontier.pop_front() else {
            if self.crawl_bound_reached {
                return Ok(self.stop("Website crawl page/depth limit reached; remaining same-origin pages were not visited", CoverageTermination::PageLimit));
            }
            self.exhausted = true;
            return Ok(SelectorStep::Exhausted);
        };
        if self.visited.contains(url.as_str()) {
            return Ok(SelectorStep::Yield);
        }
        let cached = self.crawl_cache.remove(url.as_str());
        if cached.is_none() && self.pages_visited >= self.config.max_pages {
            return Ok(self.stop(
                "Website crawl page limit reached; remaining same-origin pages were not visited",
                CoverageTermination::PageLimit,
            ));
        }
        // Count attempted page navigations, not cached extraction or robots requests.
        if cached.is_none() {
            self.pages_visited += 1;
        }
        let navigation_deadline = Instant::now() + self.navigation_timeout;
        let source_deadline_owns_timeout = self.source_deadline <= navigation_deadline;
        let result = if let Some(page) = cached {
            Ok(Ok(page))
        } else {
            let scoped_client = client.for_static_source(&self.config);
            timeout_at(
                self.source_deadline.min(navigation_deadline),
                scoped_client.fetch_text_same_origin_at(
                    url.as_str(),
                    self.crawl_origin.as_ref().unwrap(),
                    true,
                ),
            )
            .await
        };
        let (html, actual) = match result {
            Ok(Ok(page)) => page,
            Ok(Err(ClientError::TimeLimit(message))) => {
                return Ok(self.stop(&message, CoverageTermination::TimeLimit))
            }
            Ok(Err(ClientError::NeedsLogin(message))) => {
                return Ok(self.stop(&message, CoverageTermination::NeedsLogin))
            }
            Ok(Err(ClientError::HttpStatus(status, message)))
                if matches!(status.as_u16(), 401 | 403) =>
            {
                return Ok(self.stop(&format!("Website crawl access requires login or permission (HTTP {status}): {message}"), CoverageTermination::NeedsLogin));
            }
            Ok(Err(error)) => {
                return Ok(self.stop(
                    &format!("Website crawl page {url} unavailable: {error}"),
                    CoverageTermination::Failed,
                ))
            }
            Err(_) => {
                return Ok(self.stop(
                    if source_deadline_owns_timeout {
                        "Static source duration limit reached; remaining candidates/pages were not examined"
                    } else {
                        "Website crawl navigation reached its time limit"
                    },
                    CoverageTermination::TimeLimit,
                ))
            }
        };
        let actual = page_url(&url, &actual)?;
        if actual != url && self.visited.contains(actual.as_str()) {
            return Ok(SelectorStep::Yield);
        }
        self.visited.insert(actual.to_string());
        self.page = match extract_page(&html, &actual, &self.source, &self.config) {
            Ok(page) => page,
            Err(ClientError::NeedsLogin(message)) => {
                return Ok(self.stop(&message, CoverageTermination::NeedsLogin))
            }
            Err(error) => {
                return Ok(self.stop(
                    &format!("Website crawl page extraction failed: {error}"),
                    CoverageTermination::Failed,
                ))
            }
        };
        if scrape_core::browser::is_login_wall(
            &url,
            actual.as_str(),
            "",
            false,
            !self.page.candidates.is_empty(),
        ) {
            return Ok(self.stop(
                "Website crawl redirected to a login page; only public HTML is supported",
                CoverageTermination::NeedsLogin,
            ));
        }
        self.enqueue_links(&html, &actual, depth);
        self.crawl_depth = depth;
        if self.config.has_details() {
            self.crawl_cache
                .insert(url.to_string(), (html, actual.to_string()));
        }
        // Yield at every page boundary to preserve coordinator fairness.
        Ok(SelectorStep::Yield)
    }
    fn stop(&mut self, message: &str, termination: CoverageTermination) -> SelectorStep {
        self.exhausted = true;
        SelectorStep::Event(Box::new((
            WorkerEvent::Warning {
                source_id: self.source.id.clone(),
                message: message.into(),
                eligibility_incomplete: true,
                termination: Some(termination),
            },
            None,
        )))
    }
}
fn evidence(event: &WorkerEvent) -> MatchingEvidence {
    MatchingEvidence::from_event(event, false)
}
fn evidence_for_final(event: &WorkerEvent, is_crawl: bool) -> MatchingEvidence {
    let mut result = evidence(event);
    result.opportunity_exhausted = true;
    result.heuristic_crawl = is_crawl;
    result
}
fn merge_detail(
    posting: &mut SelectorPosting,
    html: &str,
    actual: &str,
    config: &StaticSourceConfig,
    source: &WorkerSource,
) -> Result<(), ClientError> {
    if config.detail_fields.is_empty() && config.is_public_html_crawl() {
        return merge_heuristic_detail(posting, html, actual, source);
    }
    let document = Html::parse_document(html);
    let container = config
        .detail_container
        .as_ref()
        .map(|query| {
            query
                .elements(&document, None)
                .into_iter()
                .next()
                .ok_or_else(|| {
                    ClientError::Extraction("Detail container locator did not match".into())
                })
        })
        .transpose()?;
    let get = |field: &str, default| {
        config
            .detail_fields
            .get(field)
            .and_then(|locator| locator.extract(&document, container, default))
    };
    if posting.location.is_empty() {
        posting.location = get("location", "text").unwrap_or_default();
    }
    if matches!(
        posting.clone().event(source),
        WorkerEvent::Job { salary: None, .. }
    ) {
        if let Some(raw) = get("salary", "text") {
            if posting.salary_text.is_none()
                || scrape_core::enrich::Salary::from_text(&raw).is_some()
            {
                posting.salary_text = Some(raw);
            }
        }
    }
    if posting.date_posted.is_none() {
        posting.date_posted = get("date", "date")
            .filter(|raw| scrape_core::posting_date::posting_instant(raw, "UTC").is_some());
    }
    if let Some(detail_description) = get("description", "html") {
        posting.description_source = Some(detail_description);
        posting.description_format = config.detail_fields.get("description").map(|field| {
            if field.extracts_html() {
                scrape_core::enrich::DescriptionFormat::Html
            } else {
                scrape_core::enrich::DescriptionFormat::PlainText
            }
        });
    }
    let actual_url = Url::parse(actual).map_err(|e| ClientError::Extraction(e.to_string()))?;
    let detail_base = super::selector::resolve_document_base(&document, &actual_url);
    let detail_logo = super::branding::resolve_document_with_base(
        &document,
        actual,
        detail_base.as_str(),
        &super::branding::employer_name(source),
        source.config.get("employer_logo").and_then(|v| v.as_str()),
    );
    if detail_logo.as_ref().is_some_and(|logo| {
        posting
            .company_logo
            .as_ref()
            .is_none_or(|current| logo.method < current.method)
    }) {
        posting.company_logo = detail_logo;
    }

    Ok(())
}

fn merge_heuristic_detail(
    posting: &mut SelectorPosting,
    html: &str,
    actual: &str,
    source: &WorkerSource,
) -> Result<(), ClientError> {
    let document = Html::parse_document(html);

    if posting.description_source.is_none() {
        for selector_str in [
            "main",
            "article",
            "[role='main']",
            "[class*='description']",
            "[class*='job-content']",
            "[class*='job-detail']",
            "[class*='job-body']",
            "#job-description",
            ".job-description",
            "body",
        ] {
            if let Ok(sel) = scraper::Selector::parse(selector_str) {
                if let Some(elem) = document.select(&sel).next() {
                    let text = elem.text().collect::<Vec<_>>().join(" ");
                    if text.trim().len() >= 80 {
                        posting.description_source = Some(elem.html());
                        posting.description_format =
                            Some(scrape_core::enrich::DescriptionFormat::Html);
                        break;
                    }
                }
            }
        }
    }

    if posting.location.is_empty() {
        for selector_str in [
            "[class*='location']",
            "[data-testid*='location']",
            ".location",
        ] {
            if let Ok(sel) = scraper::Selector::parse(selector_str) {
                if let Some(elem) = document.select(&sel).next() {
                    let text = elem.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    if !text.is_empty() && text.len() <= 80 {
                        posting.location = text;
                        break;
                    }
                }
            }
        }
    }

    if posting.salary_text.is_none() {
        for selector_str in [
            "[class*='salary']",
            "[class*='compensation']",
            "[class*='pay']",
            ".salary",
            ".compensation",
        ] {
            if let Ok(sel) = scraper::Selector::parse(selector_str) {
                if let Some(elem) = document.select(&sel).next() {
                    let text = elem.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    if !text.is_empty() && text.len() <= 60 {
                        posting.salary_text = Some(text);
                        break;
                    }
                }
            }
        }
    }

    if posting.date_posted.is_none() {
        if let Ok(sel) = scraper::Selector::parse("time[datetime]") {
            if let Some(elem) = document.select(&sel).next() {
                if let Some(raw) = elem.value().attr("datetime") {
                    if scrape_core::posting_date::posting_instant(raw, "UTC").is_some() {
                        posting.date_posted = Some(raw.to_string());
                    }
                }
            }
        }
    }

    let actual_url = Url::parse(actual).map_err(|e| ClientError::Extraction(e.to_string()))?;
    let detail_base = super::selector::resolve_document_base(&document, &actual_url);
    let detail_logo = super::branding::resolve_document_with_base(
        &document,
        actual,
        detail_base.as_str(),
        &super::branding::employer_name(source),
        source.config.get("employer_logo").and_then(|v| v.as_str()),
    );
    if detail_logo.as_ref().is_some_and(|logo| {
        posting
            .company_logo
            .as_ref()
            .is_none_or(|current| logo.method < current.method)
    }) {
        posting.company_logo = detail_logo;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detail_branding_uses_evidence_priority_and_preserves_listing_identity() {
        let source = WorkerSource {
            id: "branding".into(),
            kind: scrape_core::SourceKind::Selector,
            name: "Engineering workflow".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({"__company":"Fixture","item":"article","title":"h2","link":"a","follow_detail":true,"detail_fields":{"description":".description"}}),
        };
        let config = StaticSourceConfig::parse(&source.config, &WorkerLimits::default()).unwrap();
        let mut page=extract_page("<img alt='Fixture Logo' src='/board.svg'><article><h2>Original Engineer</h2><a href='/jobs/42'>Apply</a></article>",&Url::parse(&source.identifier).unwrap(),&source,&config).unwrap();
        let SelectorCandidate::Posting(mut posting) = page.candidates.pop_front().unwrap() else {
            panic!("Expected original posting")
        };
        assert_eq!(
            posting.company_logo.as_ref().unwrap().method,
            scrape_core::branding::LogoMethod::EmployerLabel
        );
        merge_detail(&mut posting,r#"<h1>Changed title</h1><p class='description'>Role evidence</p><script type='application/ld+json'>{"@type":"JobPosting","hiringOrganization":{"name":"Fixture","logo":"/verified.svg"}}</script>"#,"https://fixture.test/details/42",&config,&source).unwrap();
        assert_eq!(posting.title, "Original Engineer");
        assert_eq!(posting.url, "https://fixture.test/jobs/42");
        assert_eq!(posting.external_id, "https://fixture.test/jobs/42");
        let logo = posting.company_logo.unwrap();
        assert_eq!(logo.method, scrape_core::branding::LogoMethod::JobPosting);
        assert_eq!(logo.url, "https://fixture.test/verified.svg");
        assert_eq!(logo.page_url, "https://fixture.test/details/42");
    }

    #[tokio::test]
    async fn expired_source_deadline_stops_before_consuming_a_discovered_candidate() {
        let source = WorkerSource {
            id: "deadline".into(),
            kind: scrape_core::SourceKind::Selector,
            name: "Fixture".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({}),
        };
        let config = StaticSourceConfig::parse(&source.config, &WorkerLimits::default()).unwrap();
        let page = extract_page(
            "<div class=job-item><a class=job-title href=/jobs/1>Engineer</a></div>",
            &Url::parse(&source.identifier).unwrap(),
            &source,
            &config,
        )
        .unwrap();
        let mut session = SelectorSession {
            source,
            mode: ExtractionMode::Static,
            browser: None,
            session_cookies: Vec::new(),
            config,
            page,
            visited: HashSet::new(),
            crawl_origin: None,
            frontier: VecDeque::new(),
            queued: HashSet::new(),
            crawl_bound_reached: false,
            crawl_depth: 0,
            crawl_cache: HashMap::new(),
            seen: HashSet::new(),
            pages_visited: 1,
            page_limit_reached: false,
            source_deadline: Instant::now() - Duration::from_secs(1),
            detail_budget: Duration::from_secs(45),
            navigation_timeout: Duration::from_secs(30),
            detail_limit: 100,
            detail_requests: 0,
            detail_bound_reported: false,
            exhausted: false,
            pending: VecDeque::new(),
            crawl_terms: CrawlSearchTerms::default(),
        };
        let client = HttpClient::new(1, 0);
        let SelectorStep::Event(event) = session
            .next(&client, &WorkerFilters::default(), Utc::now())
            .await
            .unwrap()
        else {
            panic!("Expected time coverage diagnostic")
        };
        assert!(matches!(
            event.0,
            WorkerEvent::Warning {
                eligibility_incomplete: true,
                termination: Some(CoverageTermination::TimeLimit),
                ..
            }
        ));
        assert_eq!(session.page.candidates.len(), 1);
        assert_eq!(session.detail_requests, 0);
        assert!(matches!(
            session
                .next(&client, &WorkerFilters::default(), Utc::now())
                .await
                .unwrap(),
            SelectorStep::Exhausted
        ));
    }
}
