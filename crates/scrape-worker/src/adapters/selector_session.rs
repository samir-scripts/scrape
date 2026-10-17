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
    FilterField, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource,
};
use scraper::Html;
use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::time::{timeout_at, Instant};
use url::Url;

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
    seen: HashSet<String>,
    pages_visited: u32,
    page_limit_reached: bool,
    source_deadline: Instant,
    detail_deadline: Instant,
    navigation_timeout: Duration,
    detail_limit: u32,
    detail_requests: u32,
    detail_bound_reported: bool,
    exhausted: bool,
    pending: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
}
impl SelectorSession {
    pub async fn new(
        client: &HttpClient,
        browser: Option<Arc<BrowserPool>>,
        source: &WorkerSource,
        limits: &WorkerLimits,
        enrichment: &EnrichmentLimits,
        workflow_deadline: Instant,
    ) -> Result<Self, ClientError> {
        let mode =
            scrape_core::extraction::mode(&source.config).map_err(ClientError::Configuration)?;
        let config = StaticSourceConfig::parse(&source.config, limits)
            .map_err(ClientError::Configuration)?;
        // Compile all posting locators/mappings before any source request.
        if scrape_core::hydration::HydrationConfig::parse(&source.config)
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

        let (html, actual, pages_visited, page_limit_reached, session_cookies) =
            if mode == ExtractionMode::Browser {
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
                let (html, actual) = timeout_at(
                    source_deadline.min(Instant::now() + navigation_timeout),
                    client.fetch_text_at(url.as_str(), true),
                )
                .await
                .map_err(|_| {
                    ClientError::TimeLimit(
                        "Static source initial navigation reached its time limit".into(),
                    )
                })??;
                (html, actual, 1, false, Vec::new())
            };

        let actual = page_url(&url, &actual)?;
        let page = extract_page(&html, &actual, source, &config)?;
        let mut visited = HashSet::new();
        visited.insert(url.to_string());
        visited.insert(actual.to_string());
        // Redirect aliases must also prevent loops, without counting as another page.
        let session = Self {
            source: source.clone(),
            mode,
            browser,
            session_cookies,
            config,
            page,
            visited,
            seen: HashSet::new(),
            pages_visited,
            page_limit_reached,
            source_deadline,
            detail_deadline: Instant::now() + Duration::from_millis(enrichment.timeout_ms),
            navigation_timeout,
            detail_limit: enrichment.max_requests,
            detail_requests: 0,
            detail_bound_reported: false,
            exhausted: false,
            pending: VecDeque::new(),
        };
        Ok(session)
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
            SelectorCandidate::Invalid(error) => {
                return Ok(SelectorStep::Event(Box::new((*error, None))))
            }
        };
        let candidate = posting.clone().event(&self.source);
        if !self.config.has_details() {
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
        let bound = if Instant::now() >= self.detail_deadline {
            Some(CoverageTermination::TimeLimit)
        } else if self.detail_requests >= self.detail_limit {
            Some(CoverageTermination::RequestLimit)
        } else {
            None
        };
        if let Some(bound) = bound {
            if !self.detail_bound_reported {
                warning = Some(("Static detail enrichment reached its request/time bound; missing evidence remains unknown".to_string(),Some(bound)));
                self.detail_bound_reported = true;
            }
        } else if let Some(url) = posting.detail_url.clone() {
            self.detail_requests += 1;
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
                            self.source_deadline.min(self.detail_deadline),
                        )
                        .await?;
                    Ok::<(String, String), ClientError>((rendered.html, rendered.actual_url))
                } else {
                    client.fetch_text_at(&url, true).await
                }
            };
            match timeout_at(
                self.source_deadline
                    .min(self.detail_deadline)
                    .min(Instant::now() + self.navigation_timeout),
                detail_fetch,
            )
            .await
            {
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
                }
                Ok(Err(e)) => {
                    warning = Some((format!("Static detail {url} unavailable: {e}"), None))
                }
                Err(_) => {
                    self.detail_bound_reported = true;
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
        evidence = evidence_for_final(&event);
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
                    eligibility_incomplete: incomplete,
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
                && !self.page_limit_reached)
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
    let mut fields = vec![
        FilterField::Title,
        FilterField::Company,
        FilterField::RoleType,
    ];
    let WorkerEvent::Job {
        location,
        description_text,
        date_posted,
        salary,
        ..
    } = event
    else {
        return MatchingEvidence::default();
    };
    if !location.is_empty() {
        fields.push(FilterField::Location);
    }
    if description_text.is_some() {
        fields.push(FilterField::Description);
    }
    MatchingEvidence {
        fields,
        posting_date: date_posted.is_some(),
        compensation: salary.is_some(),
        opportunity_exhausted: false,
    }
}
fn evidence_for_final(event: &WorkerEvent) -> MatchingEvidence {
    let mut result = evidence(event);
    result.opportunity_exhausted = true;
    result
}
fn merge_detail(
    posting: &mut SelectorPosting,
    html: &str,
    actual: &str,
    config: &StaticSourceConfig,
    source: &WorkerSource,
) -> Result<(), ClientError> {
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
    if posting.description_source.is_none() {
        posting.description_source = get("description", "html");
        posting.description_format = config.detail_fields.get("description").map(|field| {
            if field.extracts_html() {
                scrape_core::enrich::DescriptionFormat::Html
            } else {
                scrape_core::enrich::DescriptionFormat::PlainText
            }
        });
    }
    let detail_logo = super::branding::resolve_document(
        &document,
        actual,
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
            seen: HashSet::new(),
            pages_visited: 1,
            page_limit_reached: false,
            source_deadline: Instant::now() - Duration::from_secs(1),
            detail_deadline: Instant::now() + Duration::from_secs(45),
            navigation_timeout: Duration::from_secs(30),
            detail_limit: 100,
            detail_requests: 0,
            detail_bound_reported: false,
            exhausted: false,
            pending: VecDeque::new(),
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
