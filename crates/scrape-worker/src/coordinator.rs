use crate::adapters::branding::CompanyBranding;
use crate::adapters::greenhouse::GreenhouseJob;
use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, Utc};
use futures::{stream, StreamExt};
use scrape_core::branding::{CompanyLogo, LogoStatus};
use scrape_core::matching::{MatchingEvaluator, MatchingEvidence, RejectionReason};
use scrape_core::protocol::{
    validate_job_limit, CandidateDisposition, CandidateEvaluation, CandidateFacts,
    CoverageTermination, EnrichmentLimits, RejectionCount, RejectionExample, ScanOutcome,
    SourceScanStats, WorkerCommitOutcome, WorkerStats, DEFAULT_MAX_JOBS_PER_RUN, PREVIEW_MAX_JOBS,
};
use scrape_core::{
    PostingIdentity, SourceKind, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits,
    WorkerSource,
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::future::Future;
use tokio::time::Instant;
use url::Url;

pub struct CoordinatorSession {
    pub source: WorkerSource,
    pub filters: WorkerFilters,
    pub stats: SourceScanStats,
    pub accepted: u32,
    pub exhausted: bool,
    pub failed: bool,
    pub had_errors: bool,
    pub logos: BTreeMap<String, (String, Option<scrape_core::branding::LogoEvidence>)>,
    pub branding: Option<CompanyBranding>,
    candidates: Vec<WorkerEvent>,
    candidate_cursor: usize,
    greenhouse_jobs: Option<Vec<serde_json::Value>>,
    greenhouse_base: Option<String>,
    greenhouse_requests: u32,
    greenhouse_budget: std::time::Duration,
    source_deadline: Instant,
    greenhouse_limits: EnrichmentLimits,
    greenhouse_bound_reported: bool,
    greenhouse_discovered: HashSet<String>,
    pending_events: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
    jsonld_links: Option<Vec<Url>>,
    jsonld_link_index: usize,
    selector_session: Option<crate::adapters::selector_session::SelectorSession>,
    script_session: Option<crate::script_runner::ScriptSession>,
    workday_session: Option<crate::adapters::workday::WorkdaySession>,
    search_session: Option<crate::adapters::search::SearchSession>,
    web_discovery_session: Option<crate::adapters::web_discovery::WebDiscoverySession>,
    coverage_incomplete: bool,
    yield_after_candidate: bool,
}

impl CoordinatorSession {
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        client: &HttpClient,
        browser: Option<std::sync::Arc<crate::browser::BrowserPool>>,
        web_transport: &crate::public_web::PublicWeb,
        source: WorkerSource,
        filters: WorkerFilters,
        extraction_limits: &WorkerLimits,
        dry_run: bool,
        workflow_cap: u32,
        workflow_deadline: Instant,
        started: DateTime<Utc>,
    ) -> Result<Self, ClientError> {
        if source.config.get("crawl").is_some() {
            if source.kind != SourceKind::Selector {
                return Err(ClientError::Configuration(
                    "crawl: requires a selector source".into(),
                ));
            }
            scrape_core::static_source::StaticSourceConfig::parse(
                &source.config,
                extraction_limits,
            )
            .map_err(ClientError::Configuration)?;
            if !scrape_core::branding::http_url(&source.identifier) {
                return Err(ClientError::Configuration(
                    "crawl: seed must be an HTTP(S) URL without credentials".into(),
                ));
            }
        }
        match scrape_core::extraction::mode(&source.config).map_err(ClientError::Configuration)? {
            scrape_core::extraction::ExtractionMode::Static => {}
            scrape_core::extraction::ExtractionMode::Browser => {
                if source.kind != SourceKind::Selector {
                    return Err(ClientError::Configuration(
                        "Browser extraction is only supported for selector sources".into(),
                    ));
                }
            }
            scrape_core::extraction::ExtractionMode::Script => {
                if source.kind != SourceKind::Selector {
                    return Err(ClientError::Configuration(
                        "Script extraction is only supported for selector sources".into(),
                    ));
                }
                scrape_core::extraction::script(&source.config)
                    .map_err(ClientError::Configuration)?;
            }
        }
        let enrichment_limits = extraction_limits.enrichment.clone().unwrap_or_default();
        let limit = if dry_run {
            workflow_cap.min(PREVIEW_MAX_JOBS)
        } else if let Ok(source_limit) = source.max_jobs_per_run() {
            source_limit
        } else {
            workflow_cap
        };
        let source_limits =
            scrape_core::extraction::limits(&source.config).map_err(ClientError::Configuration)?;
        let source_duration_s = source_limits
            .max_source_duration_s
            .min(extraction_limits.max_source_duration_s);
        let source_deadline = workflow_deadline
            .min(Instant::now() + std::time::Duration::from_secs(source_duration_s));

        let mut session = Self {
            source,
            filters,
            stats: SourceScanStats {
                source_id: String::new(),
                limit,
                ..Default::default()
            },
            accepted: 0,
            exhausted: false,
            failed: false,
            had_errors: false,
            logos: BTreeMap::new(),
            branding: None,
            candidates: Vec::new(),
            candidate_cursor: 0,
            greenhouse_jobs: None,
            greenhouse_base: None,
            greenhouse_requests: 0,
            greenhouse_budget: std::time::Duration::from_millis(enrichment_limits.timeout_ms),
            source_deadline,
            greenhouse_limits: enrichment_limits.clone(),
            greenhouse_bound_reported: false,
            greenhouse_discovered: HashSet::new(),
            pending_events: VecDeque::new(),
            jsonld_links: None,
            jsonld_link_index: 0,
            selector_session: None,
            script_session: None,
            workday_session: None,
            search_session: None,
            web_discovery_session: None,
            coverage_incomplete: false,
            yield_after_candidate: false,
        };
        session.stats.source_id = session.source.id.clone();

        if let Some(value) = session.source.config.get("employer_logo") {
            let selector = value.as_str().ok_or_else(|| {
                ClientError::Configuration("employer_logo must be a CSS selector string".into())
            })?;
            scraper::Selector::parse(selector).map_err(|error| {
                ClientError::Configuration(format!("Invalid employer_logo selector: {error}"))
            })?;
        }

        match session.source.kind {
            SourceKind::Greenhouse => {
                let base = "https://boards-api.greenhouse.io".to_string();
                let board_url =
                    format!("{base}/v1/boards/{}/jobs", session.source.identifier.trim());
                let response: crate::adapters::greenhouse::GreenhouseResponse = client
                    .fetch_json(&format!("{board_url}?content=true"))
                    .await?;
                if let Some(total) = response.meta.and_then(|m| m.total) {
                    session.stats.provider_total = Some(total);
                    session.pending_events.push_back((
                        WorkerEvent::SourceInfo {
                            source_id: session.source.id.clone(),
                            provider_total: Some(total),
                        },
                        None,
                    ));
                }
                session.greenhouse_jobs = Some(response.jobs);
                session.greenhouse_base = Some(base);
            }
            SourceKind::Lever => {
                let mut events = Vec::new();
                crate::adapters::lever::scrape_lever_with(client, &session.source, &mut |event| {
                    events.push(event);
                    true
                })
                .await?;
                session.candidates = events;
            }
            SourceKind::Ashby => {
                let mut events = Vec::new();
                crate::adapters::ashby::scrape_ashby_with(client, &session.source, &mut |event| {
                    events.push(event);
                    true
                })
                .await?;
                session.candidates = events;
            }
            SourceKind::Jsonld => {
                let base_url = Url::parse(&session.source.identifier)?;
                let (html_text, actual_page) =
                    client.fetch_text_at(base_url.as_str(), true).await?;
                let base_url = Url::parse(&actual_page)?;
                let mut events = Vec::new();
                crate::adapters::jsonld::extract_job_postings_with(
                    &html_text,
                    base_url.as_str(),
                    &session.source,
                    &mut |event| {
                        events.push(event);
                        true
                    },
                );
                session.candidates = events;
                if let Some(selector) = session
                    .source
                    .config
                    .get("follow_links")
                    .and_then(|v| v.as_str())
                {
                    let max_pages = session
                        .source
                        .config
                        .get("max_pages")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(25) as usize;
                    let links = crate::adapters::jsonld::find_links(
                        &html_text,
                        &base_url,
                        selector,
                        max_pages.saturating_add(1),
                    );
                    session.jsonld_links = Some(links.into_iter().take(max_pages).collect());
                }
            }
            SourceKind::Selector => {
                let mode = scrape_core::extraction::mode(&session.source.config)
                    .map_err(ClientError::Configuration)?;
                if mode == scrape_core::extraction::ExtractionMode::Script {
                    session.script_session = Some(crate::script_runner::ScriptSession::new(
                        &session.source,
                        extraction_limits,
                        workflow_deadline,
                    )?);
                    session.exhausted = session.script_session.as_ref().unwrap().is_exhausted();
                } else {
                    session.selector_session = Some(
                        crate::adapters::selector_session::SelectorSession::new(
                            client,
                            browser,
                            &session.source,
                            extraction_limits,
                            &enrichment_limits,
                            workflow_deadline,
                            &session.filters,
                        )
                        .await?,
                    );
                    session.exhausted = session.selector_session.as_ref().unwrap().is_exhausted();
                }
            }
            SourceKind::Workday => {
                let ws = crate::adapters::workday::WorkdaySession::new(
                    client,
                    &session.source,
                    extraction_limits,
                    &enrichment_limits,
                    workflow_deadline,
                )
                .await?;
                if let Some(total) = ws.total() {
                    session.stats.provider_total = Some(total);
                    session.pending_events.push_back((
                        WorkerEvent::SourceInfo {
                            source_id: session.source.id.clone(),
                            provider_total: Some(total),
                        },
                        None,
                    ));
                }
                session.exhausted = ws.is_exhausted();
                session.workday_session = Some(ws);
            }
            SourceKind::Search => {
                session.search_session = Some(crate::adapters::search::SearchSession::new(
                    &session.source,
                    extraction_limits,
                    &enrichment_limits,
                    workflow_deadline,
                    &session.filters,
                    started,
                )?);
            }
            SourceKind::WebDiscovery => {
                session.web_discovery_session = Some(
                    crate::adapters::web_discovery::WebDiscoverySession::new(
                        &session.source,
                        extraction_limits,
                        workflow_deadline,
                        web_transport,
                        started,
                    )?,
                );
            }
        }

        Ok(session)
    }

    fn has_pending(&self) -> bool {
        !self.pending_events.is_empty()
            || self
                .selector_session
                .as_ref()
                .is_some_and(|s| s.has_pending())
            || self
                .script_session
                .as_ref()
                .is_some_and(|s| s.has_pending())
            || self
                .workday_session
                .as_ref()
                .is_some_and(|s| s.has_pending())
            || self
                .web_discovery_session
                .as_ref()
                .is_some_and(|s| s.has_pending())
    }
    fn workflow_timeout(&mut self) -> WorkerEvent {
        self.exhausted = true;
        self.coverage_incomplete = true;
        self.stats.termination = CoverageTermination::TimeLimit;
        WorkerEvent::Warning {
            source_id: self.source.id.clone(),
            message: "Workflow duration limit reached; remaining source coverage was not completed"
                .into(),
            eligibility_incomplete: true,
            termination: Some(CoverageTermination::TimeLimit),
        }
    }
    pub fn is_finished(&self) -> bool {
        self.exhausted || self.failed
    }

    pub async fn next_candidate(
        &mut self,
        client: &HttpClient,
        started: DateTime<Utc>,
    ) -> Result<Option<(WorkerEvent, Option<MatchingEvidence>)>, ClientError> {
        if self.is_finished() {
            return Ok(None);
        }

        if let Some(pending) = self.pending_events.pop_front() {
            return Ok(Some(pending));
        }

        if let Some(selector) = &mut self.selector_session {
            let before_pages = selector.pages_visited();
            let before_details = selector.detail_requests();
            let step = selector.next(client, &self.filters, started).await?;
            self.yield_after_candidate |= selector.pages_visited() > before_pages
                || selector.detail_requests() > before_details;
            return match step {
                crate::adapters::selector_session::SelectorStep::Event(event) => {
                    self.exhausted = selector.is_exhausted();
                    Ok(Some(*event))
                }
                crate::adapters::selector_session::SelectorStep::Yield => Ok(None),
                crate::adapters::selector_session::SelectorStep::Exhausted => {
                    self.exhausted = true;
                    Ok(None)
                }
            };
        }
        if let Some(script) = &mut self.script_session {
            let before = script.turns_run();
            let step = script
                .next(&self.filters, started, self.stats.limit)
                .await?;
            self.yield_after_candidate |= script.turns_run() > before;
            return match step {
                crate::script_runner::ScriptStep::Event(event) => {
                    self.exhausted = script.is_exhausted();
                    Ok(Some(*event))
                }
                crate::script_runner::ScriptStep::Yield => Ok(None),
                crate::script_runner::ScriptStep::Exhausted => {
                    self.exhausted = true;
                    Ok(None)
                }
            };
        }
        if let Some(workday) = &mut self.workday_session {
            let before_details = workday.detail_requests();
            let step = workday.next(client, &self.filters, started).await?;
            self.yield_after_candidate |= workday.detail_requests() > before_details;
            return match step {
                crate::adapters::workday::WorkdayStep::Event(event) => {
                    self.exhausted = workday.is_exhausted();
                    Ok(Some(*event))
                }
                crate::adapters::workday::WorkdayStep::Yield => Ok(None),
                crate::adapters::workday::WorkdayStep::Exhausted => {
                    self.exhausted = true;
                    Ok(None)
                }
            };
        }
        if let Some(search) = &mut self.search_session {
            let step = search.next(client).await?;
            return match step {
                crate::adapters::search::SearchStep::Event(event) => {
                    self.exhausted = search.is_exhausted();
                    let evidence = MatchingEvidence::from_event(&event, true);
                    Ok(Some((*event, Some(evidence))))
                }
                crate::adapters::search::SearchStep::Yield => Ok(None),
                crate::adapters::search::SearchStep::Exhausted => {
                    self.exhausted = true;
                    Ok(None)
                }
            };
        }
        if let Some(discovery) = &mut self.web_discovery_session {
            let before = discovery.pages_visited();
            let step = discovery.next().await?;
            self.stats.discovery_pages = discovery.pages_visited();
            self.stats.discovery_origins = discovery.origins_visited();
            self.yield_after_candidate |= discovery.pages_visited() > before;
            return match step {
                crate::adapters::selector_session::SelectorStep::Event(event) => {
                    self.exhausted = discovery.is_exhausted();
                    Ok(Some(*event))
                }
                crate::adapters::selector_session::SelectorStep::Yield => Ok(None),
                crate::adapters::selector_session::SelectorStep::Exhausted => {
                    self.exhausted = true;
                    Ok(None)
                }
            };
        }
        if let Some(ref jobs) = self.greenhouse_jobs {
            let matcher =
                MatchingEvaluator::new(&self.filters).map_err(ClientError::Configuration)?;

            if self.candidate_cursor < jobs.len() {
                let index = self.candidate_cursor;
                self.candidate_cursor += 1;

                let value = &jobs[index];
                let mut job: GreenhouseJob = match serde_json::from_value(value.clone()) {
                    Ok(j) => j,
                    Err(error) => {
                        self.had_errors = true;
                        return Ok(Some((
                            WorkerEvent::Error {
                                source_id: self.source.id.clone(),
                                message: format!("Greenhouse candidate {index} is invalid: {error}. A posting must provide its original ID, title and destination."),
                                retryable: false,
                                invalid_posting: Some(true),
                            },
                            None,
                        )));
                    }
                };

                let candidate = match job.event(&self.source) {
                    Ok(c) => c,
                    Err(e) => {
                        self.had_errors = true;
                        return Err(e);
                    }
                };

                if job.title.trim().is_empty()
                    || scrape_core::identity::normalized_detail_url(&job.absolute_url).is_err()
                {
                    self.had_errors = true;
                    return Ok(Some((
                        WorkerEvent::Error {
                            source_id: self.source.id.clone(),
                            message: format!(
                                "Posting {} has a missing title or invalid HTTP(S) destination",
                                job.id
                            ),
                            retryable: false,
                            invalid_posting: Some(true),
                        },
                        None,
                    )));
                }

                let identity = scrape_core::PostingIdentity::for_posting(
                    Some(self.source.kind),
                    Some(&self.source.identifier),
                    Some(&job.id.to_string()),
                    &job.absolute_url,
                )
                .map_err(ClientError::Configuration)?
                .key;

                if !self.greenhouse_discovered.insert(identity) {
                    return Ok(Some((candidate, Some(job.evidence()))));
                }

                let mut evidence = job.evidence();
                let assessment = matcher.assess(&candidate, started, &evidence);
                if !assessment.rejected.is_empty()
                    || !job.can_resolve_unresolved(&assessment.unresolved)
                {
                    evidence.opportunity_exhausted = true;
                    return Ok(Some((candidate, Some(evidence))));
                }

                let bound =
                    if self.greenhouse_budget.is_zero() || Instant::now() >= self.source_deadline {
                        Some(CoverageTermination::TimeLimit)
                    } else if self.greenhouse_requests >= self.greenhouse_limits.max_requests {
                        Some(CoverageTermination::RequestLimit)
                    } else {
                        None
                    };

                let mut unavailable = None;
                let mut detail_received = false;
                if !job.needs_detail(&candidate) {
                    evidence.opportunity_exhausted = true;
                } else if let Some(bound) = bound {
                    evidence.opportunity_exhausted = true;
                    if !self.greenhouse_bound_reported {
                        unavailable = Some((
                            format!(
                                "Greenhouse detail enrichment reached its {} bound; missing publication dates and compensation remain unknown",
                                if bound == CoverageTermination::TimeLimit { "time" } else { "request" }
                            ),
                            Some(bound),
                        ));
                        self.greenhouse_bound_reported = true;
                    }
                } else {
                    self.greenhouse_requests += 1;
                    self.yield_after_candidate = true;
                    let board_url = format!(
                        "{}/v1/boards/{}/jobs",
                        self.greenhouse_base.as_ref().unwrap(),
                        self.source.identifier.trim()
                    );
                    let detail_url = format!("{board_url}/{}?pay_transparency=true", job.id);
                    let call_deadline = self
                        .source_deadline
                        .min(Instant::now() + self.greenhouse_budget);
                    let call_started = Instant::now();
                    let detail_result = tokio::time::timeout_at(
                        call_deadline,
                        client.fetch_json_cached::<GreenhouseJob>(&detail_url),
                    )
                    .await;
                    self.greenhouse_budget = self
                        .greenhouse_budget
                        .saturating_sub(call_started.elapsed());
                    match detail_result {
                        Ok(Ok(detail)) if detail.id == job.id => {
                            job.merge_detail(detail);
                            detail_received = true;
                            evidence = job.evidence();
                            evidence.opportunity_exhausted = true;
                        }
                        result => {
                            self.greenhouse_budget = std::time::Duration::ZERO;
                            evidence.opportunity_exhausted = true;
                            let termination = result
                                .as_ref()
                                .err()
                                .map(|_| CoverageTermination::TimeLimit);
                            unavailable = Some((
                                format!(
                                    "Greenhouse detail {} unavailable or has a mismatched ID: {result:?}. Retry the source to obtain missing evidence.",
                                    job.id
                                ),
                                termination,
                            ));
                        }
                    }
                }

                let candidate = job.event(&self.source)?;
                if let WorkerEvent::Job {
                    salary: Some(_), ..
                } = &candidate
                {
                    evidence.compensation = true;
                }

                let final_assessment = matcher.assess(&candidate, started, &evidence);
                if detail_received
                    && final_assessment.rejected.is_empty()
                    && !final_assessment.unresolved.is_empty()
                {
                    unavailable = Some((
                        format!(
                            "Greenhouse detail {} did not provide required eligibility evidence. Missing fields remain unknown; inspect filters and retry.",
                            job.id
                        ),
                        None,
                    ));
                }

                if let Some((message, termination)) = unavailable {
                    self.greenhouse_bound_reported |= termination.is_some();
                    self.pending_events.push_back((
                        WorkerEvent::Warning {
                            source_id: self.source.id.clone(),
                            message,
                            eligibility_incomplete: !final_assessment.unresolved.is_empty(),
                            termination,
                        },
                        None,
                    ));
                }

                if !final_assessment.rejected.is_empty() || !final_assessment.unresolved.is_empty()
                {
                    if let Some(pending) = self.pending_events.pop_front() {
                        self.pending_events.push_back((candidate, Some(evidence)));
                        return Ok(Some(pending));
                    }
                    return Ok(Some((candidate, Some(evidence))));
                }

                if self.branding.is_none()
                    && self.greenhouse_requests < self.greenhouse_limits.max_requests
                    && !self.greenhouse_budget.is_zero()
                    && Instant::now() < self.source_deadline
                {
                    let branding_deadline = self
                        .source_deadline
                        .min(Instant::now() + self.greenhouse_budget);
                    let branding_started = Instant::now();
                    let branding_result = tokio::time::timeout_at(
                        branding_deadline,
                        CompanyBranding::fetch_source(
                            client,
                            &job.absolute_url,
                            job.company_name
                                .as_deref()
                                .filter(|name| !name.trim().is_empty())
                                .or_else(|| crate::adapters::branding::employer_hint(&self.source))
                                .unwrap_or(""),
                            &self.source,
                        ),
                    )
                    .await;
                    self.greenhouse_budget = self
                        .greenhouse_budget
                        .saturating_sub(branding_started.elapsed());
                    match branding_result {
                        Ok(asset) => {
                            if let Some(warning) = asset.warning(&self.source.id) {
                                self.pending_events.push_back((warning, None));
                            }
                            self.branding = Some(asset);
                        }
                        Err(_) => {
                            self.greenhouse_budget = std::time::Duration::ZERO;
                            self.greenhouse_bound_reported = true;
                            self.pending_events.push_back((
                                WorkerEvent::Warning {
                                    source_id: self.source.id.clone(),
                                    message: "Optional company branding lookup reached the remaining enrichment time limit; logo remains unknown.".into(),
                                    eligibility_incomplete: false,
                                    termination: Some(CoverageTermination::TimeLimit),
                                },
                                None,
                            ));
                        }
                    }
                }
                let candidate = match &self.branding {
                    Some(branding) => branding.apply(candidate),
                    None => candidate,
                };

                if let Some(pending) = self.pending_events.pop_front() {
                    self.pending_events.push_back((candidate, Some(evidence)));
                    return Ok(Some(pending));
                }

                return Ok(Some((candidate, Some(evidence))));
            }

            self.exhausted = true;
            return Ok(None);
        }

        while self.candidate_cursor >= self.candidates.len() {
            if let Some(ref links) = self.jsonld_links {
                if self.jsonld_link_index < links.len() {
                    let link = links[self.jsonld_link_index].clone();
                    self.jsonld_link_index += 1;
                    match client.fetch_text_at(link.as_str(), true).await {
                        Ok((page, actual)) => {
                            let mut more = Vec::new();
                            crate::adapters::jsonld::extract_job_postings_with(
                                &page,
                                &actual,
                                &self.source,
                                &mut |event| {
                                    more.push(event);
                                    true
                                },
                            );
                            self.candidates.extend(more);
                            continue;
                        }
                        Err(error) => {
                            self.had_errors = true;
                            return Ok(Some((
                                WorkerEvent::Error {
                                    source_id: self.source.id.clone(),
                                    message: format!(
                                        "Could not extract detail page {link}: {error}"
                                    ),
                                    retryable: true,
                                    invalid_posting: Some(false),
                                },
                                None,
                            )));
                        }
                    }
                }
            }
            self.exhausted = true;
            return Ok(None);
        }

        let event = self.candidates[self.candidate_cursor].clone();
        self.candidate_cursor += 1;
        Ok(Some((event, None)))
    }

    pub fn record_logo(
        &mut self,
        company: &str,
        logo: &Option<scrape_core::branding::LogoEvidence>,
    ) {
        let key = company.trim().to_lowercase();
        let entry = self
            .logos
            .entry(key)
            .or_insert_with(|| (company.to_string(), logo.clone()));
        if logo.as_ref().is_some_and(|l| {
            entry
                .1
                .as_ref()
                .is_none_or(|current| l.method < current.method)
        }) {
            entry.1 = logo.clone();
        }
    }

    pub fn record_rejection(
        &mut self,
        primary: RejectionReason,
        reasons: Vec<RejectionReason>,
        title: &str,
    ) {
        if let Some(count) = self
            .stats
            .rejections
            .iter_mut()
            .find(|c| c.reason == primary)
        {
            count.count += 1;
        } else {
            self.stats.rejections.push(RejectionCount {
                reason: primary,
                count: 1,
            });
        }
        if self.stats.rejection_examples.len() < 5 {
            self.stats.rejection_examples.push(RejectionExample {
                title: title.to_string(),
                primary,
                overlapping: reasons,
            });
        }
    }
}

fn candidate_evaluation(
    event: &WorkerEvent,
    sequence: u64,
    disposition: CandidateDisposition,
    rejected: Vec<RejectionReason>,
    unresolved: Vec<RejectionReason>,
    diagnostic: Option<&str>,
) -> WorkerEvent {
    WorkerEvent::CandidateEvaluated {
        source_id: event.source_id().unwrap_or_default().to_string(),
        evaluation: CandidateEvaluation {
            sequence,
            disposition,
            candidate: CandidateFacts::from_event(event),
            rejected,
            unresolved,
            diagnostic: diagnostic.map(|text| text.chars().take(512).collect()),
        },
    }
}

pub struct SourceCoordinator {
    pub client: HttpClient,
    pub browser: Option<std::sync::Arc<crate::browser::BrowserPool>>,
    pub sources: Vec<WorkerSource>,
    pub filters: WorkerFilters,
    pub filters_by_source: HashMap<String, WorkerFilters>,
    pub blocked: HashSet<String>,
    pub started: DateTime<Utc>,
    pub limits: WorkerLimits,
    pub dry_run: bool,
    pub commit_feedback: bool,
    pub candidate_diagnostics: bool,
    pub max_jobs_per_run: u32,
    pub starting_source_index: usize,
}

impl SourceCoordinator {
    pub fn new(
        client: HttpClient,
        config: &WorkerConfig,
        dry_run: bool,
        started: DateTime<Utc>,
    ) -> Result<Self, String> {
        config.limits.validate()?;
        let mut limits = config.limits.clone();
        for source in &config.sources {
            let source_limits = scrape_core::extraction::limits(&source.config)?;
            if source.config.get("max_workflow_duration_s").is_some() {
                limits.max_workflow_duration_s = limits
                    .max_workflow_duration_s
                    .min(source_limits.max_workflow_duration_s);
            }
        }
        let configured_cap = match config.max_jobs_per_run {
            Some(cap) => validate_job_limit(cap)?,
            None => {
                let sum: u32 = config
                    .sources
                    .iter()
                    .filter_map(|s| s.max_jobs_per_run().ok())
                    .sum();
                if sum == 0 {
                    DEFAULT_MAX_JOBS_PER_RUN
                } else {
                    sum
                }
            }
        };

        let cap = if dry_run {
            configured_cap.min(PREVIEW_MAX_JOBS)
        } else {
            configured_cap
        };

        let browser = Some(std::sync::Arc::new(
            crate::browser::BrowserPool::new(limits.max_browser_contexts as usize)
                .with_proxy(config.http_proxy.as_ref())
                .map_err(|error| error.to_string())?,
        ));

        Ok(Self {
            client,
            browser,
            sources: config.sources.clone(),
            filters: config.filters.clone(),
            filters_by_source: config.filters_by_source.clone(),
            blocked: config.blocked_postings.iter().cloned().collect(),
            started,
            limits,
            dry_run,
            commit_feedback: config.commit_feedback && !dry_run,
            candidate_diagnostics: config.candidate_diagnostics && !dry_run,
            max_jobs_per_run: cap,
            starting_source_index: config.starting_source_index.unwrap_or(0),
        })
    }

    pub async fn run<F>(&mut self, emit: F) -> Result<WorkerStats, Box<dyn std::error::Error>>
    where
        F: FnMut(&WorkerEvent) -> Result<(), serde_json::Error>,
    {
        self.run_at(emit, Instant::now()).await
    }

    pub async fn run_with_feedback<F, G, Fut>(
        &mut self,
        emit: F,
        feedback: G,
    ) -> Result<WorkerStats, Box<dyn std::error::Error>>
    where
        F: FnMut(&WorkerEvent) -> Result<(), serde_json::Error>,
        G: FnMut(String, String) -> Fut,
        Fut: Future<Output = Result<WorkerCommitOutcome, String>>,
    {
        self.run_at_feedback(emit, feedback, Instant::now()).await
    }

    async fn run_at<F>(
        &mut self,
        emit: F,
        start_time: Instant,
    ) -> Result<WorkerStats, Box<dyn std::error::Error>>
    where
        F: FnMut(&WorkerEvent) -> Result<(), serde_json::Error>,
    {
        self.run_at_feedback(
            emit,
            |_, _| async { Err("Commit feedback unavailable".into()) },
            start_time,
        )
        .await
    }

    async fn run_at_feedback<F, G, Fut>(
        &mut self,
        mut emit: F,
        mut feedback: G,
        start_time: Instant,
    ) -> Result<WorkerStats, Box<dyn std::error::Error>>
    where
        F: FnMut(&WorkerEvent) -> Result<(), serde_json::Error>,
        G: FnMut(String, String) -> Fut,
        Fut: Future<Output = Result<WorkerCommitOutcome, String>>,
    {
        let workflow_deadline =
            start_time + std::time::Duration::from_secs(self.limits.max_workflow_duration_s);
        let enrichment = self.limits.enrichment.clone().unwrap_or_default();
        enrichment.validate()?;
        // One transport owns the global four-request semaphore and origin pacing;
        // each discovery session derives its own bounded request budget.
        let request_budget = (self.limits.max_pages_per_source as usize)
            .saturating_mul(8)
            .saturating_mul(self.sources.len().max(1));
        let mut web_transport = crate::public_web::PublicWeb::new(
            request_budget,
            std::time::Duration::from_secs(self.limits.request_timeout_s),
            std::time::Duration::from_millis(self.limits.per_host_delay_ms),
        );
        if let Some(origin) = self.client.replay_origin() {
            web_transport = web_transport.with_test_replay_origin(origin)?;
        }


        let mut sessions: Vec<CoordinatorSession> = Vec::new();
        let mut failed_sources: Vec<SourceScanStats> = Vec::new();

        for source in &self.sources {
            let filters = self
                .filters_by_source
                .get(&source.id)
                .unwrap_or(&self.filters)
                .clone();
            let source_limits = scrape_core::extraction::limits(&source.config).unwrap_or_default();
            let source_duration_s = source_limits
                .max_source_duration_s
                .min(self.limits.max_source_duration_s);
            let init_deadline = workflow_deadline
                .min(Instant::now() + std::time::Duration::from_secs(source_duration_s));
            let initialized = if Instant::now() >= workflow_deadline {
                Err(ClientError::TimeLimit(
                    "Workflow duration reached before this source could initialize".into(),
                ))
            } else {
                tokio::time::timeout_at(
                    init_deadline,
                    CoordinatorSession::init(
                        &self.client,
                        self.browser.clone(),
                        &web_transport,
                        source.clone(),
                        filters,
                        &self.limits,
                        self.dry_run,
                        self.max_jobs_per_run,
                        workflow_deadline,
                        self.started,
                    ),
                )
                .await
                .unwrap_or_else(|_| {
                    Err(ClientError::TimeLimit(
                        "Workflow initialization reached its time limit".into(),
                    ))
                })
            };
            match initialized {
                Ok(mut session) => {
                    if self.commit_feedback {
                        session.stats.limit = self.max_jobs_per_run;
                    }
                    sessions.push(session);
                }
                Err(error) => {
                    emit(&WorkerEvent::Error {
                        invalid_posting: Some(false),
                        source_id: source.id.clone(),
                        message: error.to_string(),
                        retryable: matches!(
                            error,
                            ClientError::Reqwest(_) | ClientError::HttpStatus(..)
                        ),
                    })?;
                    let stat = SourceScanStats {
                        source_id: source.id.clone(),
                        commit_feedback: self.commit_feedback,
                        limit: self.max_jobs_per_run,
                        termination: match error {
                            ClientError::TimeLimit(_) => CoverageTermination::TimeLimit,
                            ClientError::NeedsLogin(_) => CoverageTermination::NeedsLogin,
                            _ => CoverageTermination::Failed,
                        },
                        outcome: Some(ScanOutcome::Failed),
                        ..Default::default()
                    };
                    failed_sources.push(stat);
                }
            }
        }

        let mut total_accepted = 0u32;
        let mut total_committed = 0u32;
        let mut workflow_seen_identities: HashSet<String> = HashSet::new();
        let mut workflow_seen_urls: HashSet<String> = HashSet::new();
        let n = sessions.len();

        if n > 0 {
            let start_idx = self.starting_source_index % n;

            while (if self.commit_feedback {
                total_committed
            } else {
                total_accepted
            }) < self.max_jobs_per_run
            {
                let mut round_progress = false;

                for step in 0..n {
                    let i = (start_idx + step) % n;
                    let session = &mut sessions[i];
                    if session.is_finished() {
                        continue;
                    }
                    if !self.commit_feedback && session.accepted >= session.stats.limit {
                        session.stats.limit_reached = true;
                        continue;
                    }

                    // Visit this source for at most ONE eligible unique posting in this round:
                    let mut source_yielded_in_round = false;
                    let mut events_this_turn = 0;
                    session.yield_after_candidate = false;

                    while !source_yielded_in_round && !session.is_finished() {
                        if events_this_turn >= 32
                            || (session.yield_after_candidate && !session.has_pending())
                        {
                            round_progress = true;
                            break;
                        }
                        events_this_turn += 1;
                        if Instant::now() >= workflow_deadline && !session.has_pending() {
                            emit(&session.workflow_timeout())?;
                            break;
                        }
                        let next = if session.selector_session.is_some()
                            || session.script_session.is_some()
                            || session.workday_session.is_some()
                            || session.search_session.is_some()
                            || session.web_discovery_session.is_some()
                            || session.has_pending()
                        {
                            session.next_candidate(&self.client, self.started).await
                        } else {
                            match tokio::time::timeout_at(
                                workflow_deadline,
                                session.next_candidate(&self.client, self.started),
                            )
                            .await
                            {
                                Ok(result) => result,
                                Err(_) => {
                                    emit(&session.workflow_timeout())?;
                                    break;
                                }
                            }
                        };
                        match next {
                            Ok(Some((event, evidence))) => {
                                match &event {
                                    WorkerEvent::SourceInfo { provider_total, .. } => {
                                        session.stats.provider_total = *provider_total;
                                        emit(&event)?;
                                        continue;
                                    }
                                    WorkerEvent::Error {
                                        invalid_posting,
                                        message,
                                        ..
                                    } => {
                                        if invalid_posting.as_ref() == Some(&true) {
                                            session.stats.examined += 1;
                                            session.stats.invalid += 1;
                                            if self.candidate_diagnostics {
                                                emit(&candidate_evaluation(
                                                    &event,
                                                    session.stats.examined,
                                                    CandidateDisposition::Invalid,
                                                    vec![],
                                                    vec![],
                                                    Some(message),
                                                ))?;
                                            }
                                        } else {
                                            session.failed = true;
                                        }
                                        session.had_errors = true;
                                        emit(&event)?;
                                        continue;
                                    }
                                    WorkerEvent::Warning {
                                        termination,
                                        eligibility_incomplete,
                                        ..
                                    } => {
                                        // Coverage diagnostics are not detail-enrichment failures.
                                        if !session.exhausted && !eligibility_incomplete {
                                            session.stats.enrichment_warnings += 1;
                                        }
                                        session.coverage_incomplete |= *eligibility_incomplete;
                                        if let Some(t) = termination {
                                            if !matches!(
                                                session.stats.termination,
                                                CoverageTermination::Failed
                                                    | CoverageTermination::NeedsLogin
                                                    | CoverageTermination::TimeLimit
                                            ) || !matches!(
                                                t,
                                                CoverageTermination::PageLimit
                                                    | CoverageTermination::RequestLimit
                                            ) {
                                                session.stats.termination = *t;
                                            }
                                        }
                                        emit(&event)?;
                                        continue;
                                    }
                                    WorkerEvent::Job {
                                        external_id,
                                        title,
                                        url,
                                        company,
                                        company_logo,
                                        ..
                                    } => {
                                        session.stats.examined += 1;
                                        let facts = self
                                            .candidate_diagnostics
                                            .then(|| CandidateFacts::from_event(&event))
                                            .flatten();
                                        let decision_source = session.source.id.clone();
                                        let sequence = session.stats.examined;
                                        let decision = |disposition, rejected, unresolved, diagnostic: Option<&str>| WorkerEvent::CandidateEvaluated {
                                            source_id: decision_source,
                                            evaluation: CandidateEvaluation {
                                                sequence,
                                                disposition, candidate: facts, rejected, unresolved,
                                                diagnostic: diagnostic.map(|text| text.chars().take(512).collect()),
                                            },
                                        };
                                        if !company.is_empty() && company != "Unknown company" {
                                            session.record_logo(company, company_logo);
                                        }

                                        let identity = match PostingIdentity::for_posting(
                                            Some(session.source.kind),
                                            Some(&session.source.identifier),
                                            Some(external_id),
                                            url,
                                        ) {
                                            Ok(id) => id.key,
                                            Err(msg) => {
                                                session.stats.invalid += 1;
                                                session.had_errors = true;
                                                if self.candidate_diagnostics {
                                                    emit(&decision(
                                                        CandidateDisposition::Invalid,
                                                        vec![],
                                                        vec![],
                                                        Some(&msg),
                                                    ))?;
                                                }
                                                emit(&WorkerEvent::Error {
                                                    source_id: session.source.id.clone(),
                                                    message: msg,
                                                    retryable: false,
                                                    invalid_posting: Some(true),
                                                })?;
                                                continue;
                                            }
                                        };

                                        let normalized_url =
                                            scrape_core::identity::normalized_detail_url(url)
                                                .unwrap_or_else(|_| url.clone());

                                        // Workflow-wide canonical deduplication check:
                                        if workflow_seen_identities.contains(&identity)
                                            || (session.source.kind != SourceKind::WebDiscovery
                                                && workflow_seen_urls.contains(&normalized_url))
                                        {
                                            session.stats.duplicates += 1;
                                            if self.candidate_diagnostics {
                                                emit(&decision(
                                                    CandidateDisposition::Duplicate,
                                                    vec![],
                                                    vec![],
                                                    None,
                                                ))?;
                                            }
                                            // Duplicate: continue candidate traversal to find a unique job!
                                            continue;
                                        }

                                        // Matching assessment
                                        let matcher = MatchingEvaluator::new(&session.filters)
                                            .map_err(ClientError::Configuration)?;
                                        let assessment = evidence
                                            .as_ref()
                                            .map(|ev| matcher.assess(&event, self.started, ev));
                                        let reasons = assessment
                                            .as_ref()
                                            .map(|a| a.rejected.clone())
                                            .unwrap_or_else(|| {
                                                matcher.rejection_reasons(&event, self.started)
                                            });

                                        if let Some(primary) = reasons.first().copied() {
                                            session.stats.rejected += 1;
                                            if self.candidate_diagnostics {
                                                emit(&decision(
                                                    CandidateDisposition::Rejected,
                                                    reasons.clone(),
                                                    assessment
                                                        .as_ref()
                                                        .map(|a| a.unresolved.clone())
                                                        .unwrap_or_default(),
                                                    None,
                                                ))?;
                                            }
                                            session.record_rejection(primary, reasons, title);
                                            continue;
                                        }

                                        if assessment
                                            .as_ref()
                                            .is_some_and(|a| !a.unresolved.is_empty())
                                        {
                                            session.stats.unresolved += 1;
                                            if self.candidate_diagnostics {
                                                emit(&decision(
                                                    CandidateDisposition::Unresolved,
                                                    vec![],
                                                    assessment.as_ref().unwrap().unresolved.clone(),
                                                    None,
                                                ))?;
                                            }
                                            if session.stats.unresolved <= 5 {
                                                if let Some(warning) =
                                                    crate::filter::unresolved_warning(
                                                        &event,
                                                        &session.filters,
                                                        &assessment.as_ref().unwrap().unresolved,
                                                    )
                                                {
                                                    emit(&warning)?;
                                                }
                                            }
                                            continue;
                                        }

                                        workflow_seen_identities.insert(identity.clone());
                                        workflow_seen_urls.insert(normalized_url);
                                        session.stats.matched += 1;

                                        if self.blocked.contains(&identity)
                                            || self.blocked.contains(&format!(
                                                "{}:{}",
                                                session.source.id, external_id
                                            ))
                                        {
                                            session.stats.blocked += 1;
                                            if self.candidate_diagnostics {
                                                emit(&decision(
                                                    CandidateDisposition::Blocked,
                                                    vec![],
                                                    vec![],
                                                    None,
                                                ))?;
                                            }
                                            continue;
                                        }

                                        // ACCEPTED ELIGIBLE UNIQUE POSTING!
                                        session.accepted += 1;
                                        session.stats.accepted = u64::from(session.accepted);
                                        total_accepted += 1;
                                        source_yielded_in_round = true;
                                        round_progress = true;

                                        session.record_logo(company, company_logo);
                                        if self.candidate_diagnostics {
                                            emit(&decision(
                                                CandidateDisposition::Eligible,
                                                vec![],
                                                vec![],
                                                None,
                                            ))?;
                                        }
                                        emit(&event)?;
                                        if self.commit_feedback {
                                            let outcome = tokio::time::timeout_at(
                                                workflow_deadline,
                                                feedback(
                                                    session.source.id.clone(),
                                                    external_id.clone(),
                                                ),
                                            )
                                            .await
                                            .map_err(|_| "Commit acknowledgement timed out")?
                                            .map_err(|error| {
                                                format!("Commit acknowledgement failed: {error}")
                                            })?;
                                            if outcome == WorkerCommitOutcome::Inserted {
                                                total_committed += 1;
                                                session.stats.newly_committed += 1;
                                            }
                                        }

                                        if (if self.commit_feedback {
                                            total_committed
                                        } else {
                                            total_accepted
                                        }) >= self.max_jobs_per_run
                                            || (!self.commit_feedback
                                                && session.accepted >= session.stats.limit)
                                        {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Ok(None) => {
                                round_progress |= !session.is_finished();
                                break;
                            }
                            Err(error) => {
                                session.failed = true;
                                if matches!(error, ClientError::NeedsLogin(_)) {
                                    session.stats.termination = CoverageTermination::NeedsLogin;
                                } else if matches!(error, ClientError::TimeLimit(_)) {
                                    session.stats.termination = CoverageTermination::TimeLimit;
                                }
                                emit(&WorkerEvent::Error {
                                    invalid_posting: Some(false),
                                    source_id: session.source.id.clone(),
                                    message: error.to_string(),
                                    retryable: false,
                                })?;
                                break;
                            }
                        }
                    }

                    if (if self.commit_feedback {
                        total_committed
                    } else {
                        total_accepted
                    }) >= self.max_jobs_per_run
                    {
                        break;
                    }
                }

                if !round_progress {
                    break;
                }
            }
        }

        // Finalize branding, stats, and terminations
        let mut final_stats = WorkerStats::default();
        let logo_deadline =
            workflow_deadline.min(Instant::now() + std::time::Duration::from_secs(4));
        // Round-robin sources before deduplicating assets: a large board must not
        // monopolize the bounded download slots ahead of another employer.
        let mut per_source = sessions
            .iter()
            .map(|session| session.logos.values())
            .collect::<Vec<_>>();
        let mut asset_urls = Vec::new();
        let mut seen_urls = HashSet::new();
        loop {
            let mut remaining = false;
            for logos in &mut per_source {
                if let Some((_, evidence)) = logos.next() {
                    remaining = true;
                    if let Some(evidence) = evidence {
                        if seen_urls.insert(evidence.url.clone()) {
                            asset_urls.push(evidence.url.clone());
                        }
                    }
                }
            }
            if !remaining {
                break;
            }
        }
        let client = &self.client;
        let assets: HashMap<_, _> = stream::iter(asset_urls.into_iter().map(|url| async move {
            let result = if Instant::now() >= logo_deadline {
                Err((
                    LogoStatus::DownloadFailed,
                    "Source branding deadline reached; no new download scheduled".into(),
                ))
            } else {
                match tokio::time::timeout_at(logo_deadline, client.fetch_logo(&url)).await {
                    Ok(Ok(bytes)) => scrape_core::branding::cache_asset(&bytes)
                        .map_err(|error| (LogoStatus::DecodeFailed, error)),
                    result => Err((
                        LogoStatus::DownloadFailed,
                        format!("Logo download unavailable or deadline reached: {result:?}"),
                    )),
                }
            };
            (url, result)
        }))
        .buffer_unordered(4)
        .collect()
        .await;

        for mut session in sessions {
            session.stats.commit_feedback = self.commit_feedback;
            session.stats.limit_reached = if self.commit_feedback {
                total_committed >= self.max_jobs_per_run && !session.exhausted && !session.failed
            } else {
                session.stats.accepted == u64::from(session.stats.limit)
            };
            if ((if self.commit_feedback {
                total_committed
            } else {
                total_accepted
            }) >= self.max_jobs_per_run
                || session.stats.limit_reached)
                && !session.exhausted
                && !session.failed
            {
                if session.stats.termination == CoverageTermination::Exhausted
                    || (!session.coverage_incomplete
                        && session.stats.unresolved == 0
                        && matches!(
                            session.stats.termination,
                            CoverageTermination::RequestLimit | CoverageTermination::PageLimit
                        ))
                {
                    session.stats.termination = CoverageTermination::JobLimit;
                }
            } else if session.failed
                && !matches!(
                    session.stats.termination,
                    CoverageTermination::NeedsLogin | CoverageTermination::TimeLimit
                )
            {
                session.stats.termination = CoverageTermination::Failed;
            }

            // Download employer branding logos for accepted companies
            for (_, (company, evidence)) in session.logos {
                let mut logo = CompanyLogo {
                    evidence,
                    checked_at: Utc::now().to_rfc3339(),
                    status: LogoStatus::MissingMetadata,
                    asset: None,
                    cached_asset_reference: None,
                    diagnostic: None,
                };

                if let Some(evidence) = &logo.evidence {
                    match &assets[&evidence.url] {
                        Ok(asset) => {
                            logo.asset = Some(asset.clone());
                            logo.status = LogoStatus::Verified;
                        }
                        Err((status, diagnostic)) => {
                            logo.status = *status;
                            logo.diagnostic = Some(diagnostic.clone());
                        }
                    }
                }

                if matches!(
                    logo.status,
                    LogoStatus::DownloadFailed | LogoStatus::DecodeFailed
                ) {
                    session.stats.enrichment_warnings += 1;
                    emit(&WorkerEvent::Warning {
                        source_id: session.source.id.clone(),
                        message: format!(
                            "Employer logo {:?} for {company}: {}. Retain any last verified asset.",
                            logo.status,
                            logo.diagnostic.as_deref().unwrap_or_default()
                        ),
                        eligibility_incomplete: false,
                        termination: None,
                    })?;
                }

                emit(&WorkerEvent::Branding {
                    source_id: session.source.id.clone(),
                    company,
                    logo,
                })?;
            }

            let outcome = if session.failed || session.had_errors {
                if !matches!(
                    session.stats.termination,
                    CoverageTermination::NeedsLogin | CoverageTermination::TimeLimit
                ) {
                    session.stats.termination = CoverageTermination::Failed;
                }
                if session.stats.examined > session.stats.invalid {
                    ScanOutcome::Partial
                } else {
                    ScanOutcome::Failed
                }
            } else if session.coverage_incomplete || session.stats.unresolved > 0 {
                if matches!(
                    session.stats.termination,
                    CoverageTermination::NeedsLogin | CoverageTermination::Failed
                ) && session.stats.examined == 0
                {
                    ScanOutcome::Failed
                } else {
                    ScanOutcome::Partial
                }
            } else if session.stats.enrichment_warnings > 0 {
                ScanOutcome::SuccessWithWarnings
            } else {
                ScanOutcome::Success
            };
            session.stats.outcome = Some(outcome);

            if matches!(outcome, ScanOutcome::Failed | ScanOutcome::Partial) {
                final_stats.sources_failed += 1;
            } else {
                final_stats.sources_ok += 1;
            }

            emit(&WorkerEvent::Progress {
                source_id: session.source.id.clone(),
                fetched: session.stats.examined,
                matched: session.stats.matched,
            })?;

            final_stats.sources.push(session.stats);
        }

        // Explicitly drop the shared BrowserPool so Browser.close is sent and processes are reaped.
        let _ = self.browser.take();

        for stat in failed_sources {
            final_stats.sources_failed += 1;
            final_stats.sources.push(stat);
        }

        final_stats.jobs_emitted = u64::from(total_accepted);
        final_stats.duration_ms = start_time.elapsed().as_millis() as u64;

        emit(&WorkerEvent::Done {
            stats: final_stats.clone(),
        })?;
        Ok(final_stats)
    }
}

#[cfg(test)]
mod deadline_tests {
    use super::*;
    #[tokio::test]
    async fn source_workflow_override_stops_before_network_and_cannot_raise_global_bound() {
        let mut config = WorkerConfig {
            sources: vec![WorkerSource {
                id: "bounded".into(),
                kind: SourceKind::Selector,
                name: "Fixture".into(),
                identifier: "not-a-url".into(),
                config: serde_json::json!({"max_workflow_duration_s":30}),
            }],
            limits: WorkerLimits {
                max_workflow_duration_s: 120,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut coordinator =
            SourceCoordinator::new(HttpClient::new(1, 0), &config, false, Utc::now()).unwrap();
        let stats = coordinator
            .run_at(
                |_| Ok(()),
                Instant::now() - std::time::Duration::from_secs(31),
            )
            .await
            .unwrap();
        assert_eq!(stats.sources[0].termination, CoverageTermination::TimeLimit);
        assert_eq!(stats.sources[0].examined, 0);
        config.sources[0].config["max_workflow_duration_s"] = 600.into();
        let coordinator =
            SourceCoordinator::new(HttpClient::new(1, 0), &config, false, Utc::now()).unwrap();
        assert_eq!(coordinator.limits.max_workflow_duration_s, 120);
    }
    #[tokio::test]
    async fn expired_workflow_deadline_stops_before_initializing_any_source() {
        let config = WorkerConfig {
            sources: vec![WorkerSource {
                id: "unvisited".into(),
                kind: SourceKind::Selector,
                name: "Fixture".into(),
                identifier: "not-a-url".into(),
                config: serde_json::json!({}),
            }],
            limits: WorkerLimits {
                max_workflow_duration_s: 30,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut coordinator =
            SourceCoordinator::new(HttpClient::new(1, 0), &config, false, Utc::now()).unwrap();
        let mut events = Vec::new();
        let stats = coordinator
            .run_at(
                |event| {
                    events.push(event.clone());
                    Ok(())
                },
                Instant::now() - std::time::Duration::from_secs(31),
            )
            .await
            .unwrap();
        assert_eq!(stats.sources.len(), 1);
        assert_eq!(stats.sources[0].termination, CoverageTermination::TimeLimit);
        assert_eq!(stats.sources[0].outcome, Some(ScanOutcome::Failed));
        assert_eq!(stats.sources[0].examined, 0);
        assert!(events.iter().any(|e|matches!(e,WorkerEvent::Error{message,..} if message.contains("before this source could initialize"))));
    }
}
