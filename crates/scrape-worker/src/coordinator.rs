use crate::adapters::branding::CompanyBranding;
use crate::adapters::greenhouse::GreenhouseJob;
use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, Utc};
use scrape_core::branding::{CompanyLogo, LogoStatus};
use scrape_core::matching::{MatchingEvaluator, MatchingEvidence, RejectionReason};
use scrape_core::protocol::{
    validate_job_limit, CoverageTermination, EnrichmentLimits, RejectionCount, RejectionExample,
    ScanOutcome, SourceScanStats, WorkerStats, DEFAULT_MAX_JOBS_PER_RUN, PREVIEW_MAX_JOBS,
};
use scrape_core::{
    PostingIdentity, SourceKind, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits,
    WorkerSource,
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
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
    greenhouse_deadline: Option<Instant>,
    greenhouse_limits: EnrichmentLimits,
    greenhouse_bound_reported: bool,
    greenhouse_discovered: HashSet<String>,
    pending_events: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
    jsonld_links: Option<Vec<Url>>,
    jsonld_link_index: usize,
}

impl CoordinatorSession {
    pub async fn init(
        client: &HttpClient,
        source: WorkerSource,
        filters: WorkerFilters,
        _started: DateTime<Utc>,
        enrichment_limits: &EnrichmentLimits,
        dry_run: bool,
        workflow_cap: u32,
    ) -> Result<Self, ClientError> {
        let limit = if dry_run {
            workflow_cap.min(PREVIEW_MAX_JOBS)
        } else if let Ok(source_limit) = source.max_jobs_per_run() {
            source_limit
        } else {
            workflow_cap
        };

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
            greenhouse_deadline: None,
            greenhouse_limits: enrichment_limits.clone(),
            greenhouse_bound_reported: false,
            greenhouse_discovered: HashSet::new(),
            pending_events: VecDeque::new(),
            jsonld_links: None,
            jsonld_link_index: 0,
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
                session.greenhouse_deadline = Some(
                    Instant::now() + std::time::Duration::from_millis(enrichment_limits.timeout_ms),
                );
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
                let mut events = Vec::new();
                crate::adapters::selector::scrape_selector_with(
                    client,
                    &session.source,
                    &mut |event| {
                        events.push(event);
                        true
                    },
                )
                .await?;
                session.candidates = events;
            }
        }

        Ok(session)
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
                if !assessment.rejected.is_empty() {
                    return Ok(Some((candidate, Some(evidence))));
                }

                let deadline = self.greenhouse_deadline.unwrap();
                let bound = if tokio::time::Instant::now() >= deadline {
                    Some(CoverageTermination::TimeLimit)
                } else if self.greenhouse_requests >= self.greenhouse_limits.max_requests {
                    Some(CoverageTermination::RequestLimit)
                } else {
                    None
                };

                let mut unavailable = None;
                let mut detail_received = false;
                if let Some(bound) = bound {
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
                    let board_url = format!(
                        "{}/v1/boards/{}/jobs",
                        self.greenhouse_base.as_ref().unwrap(),
                        self.source.identifier.trim()
                    );
                    let detail_url = format!("{board_url}/{}?pay_transparency=true", job.id);
                    match tokio::time::timeout_at(
                        deadline,
                        client.fetch_json_cached::<GreenhouseJob>(&detail_url),
                    )
                    .await
                    {
                        Ok(Ok(detail)) if detail.id == job.id => {
                            job.merge_detail(detail);
                            detail_received = true;
                            evidence = job.evidence();
                            evidence.opportunity_exhausted = true;
                        }
                        result => {
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
                if detail_received {
                    if let WorkerEvent::Job {
                        salary: Some(_), ..
                    } = &candidate
                    {
                        evidence.compensation = true;
                    }
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
                    && tokio::time::Instant::now() < deadline
                {
                    match tokio::time::timeout_at(
                        deadline,
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
                    .await
                    {
                        Ok(asset) => {
                            if let Some(warning) = asset.warning(&self.source.id) {
                                self.pending_events.push_back((warning, None));
                            }
                            self.branding = Some(asset);
                        }
                        Err(_) => {
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

pub struct SourceCoordinator {
    pub client: HttpClient,
    pub sources: Vec<WorkerSource>,
    pub filters: WorkerFilters,
    pub filters_by_source: HashMap<String, WorkerFilters>,
    pub blocked: HashSet<String>,
    pub started: DateTime<Utc>,
    pub limits: WorkerLimits,
    pub dry_run: bool,
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

        Ok(Self {
            client,
            sources: config.sources.clone(),
            filters: config.filters.clone(),
            filters_by_source: config.filters_by_source.clone(),
            blocked: config.blocked_postings.iter().cloned().collect(),
            started,
            limits: config.limits.clone(),
            dry_run,
            max_jobs_per_run: cap,
            starting_source_index: config.starting_source_index.unwrap_or(0),
        })
    }

    pub async fn run<F>(&mut self, mut emit: F) -> Result<WorkerStats, Box<dyn std::error::Error>>
    where
        F: FnMut(&WorkerEvent) -> Result<(), serde_json::Error>,
    {
        let start_time = Instant::now();
        let enrichment = self.limits.enrichment.clone().unwrap_or_default();
        enrichment.validate()?;

        let mut sessions: Vec<CoordinatorSession> = Vec::new();
        let mut failed_sources: Vec<SourceScanStats> = Vec::new();

        for source in &self.sources {
            let filters = self
                .filters_by_source
                .get(&source.id)
                .unwrap_or(&self.filters)
                .clone();
            match CoordinatorSession::init(
                &self.client,
                source.clone(),
                filters,
                self.started,
                &enrichment,
                self.dry_run,
                self.max_jobs_per_run,
            )
            .await
            {
                Ok(session) => sessions.push(session),
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
                        limit: self.max_jobs_per_run,
                        termination: CoverageTermination::Failed,
                        outcome: Some(ScanOutcome::Failed),
                        ..Default::default()
                    };
                    failed_sources.push(stat);
                }
            }
        }

        let mut total_accepted = 0u32;
        let mut workflow_seen_identities: HashSet<String> = HashSet::new();
        let mut workflow_seen_urls: HashSet<String> = HashSet::new();
        let n = sessions.len();

        if n > 0 {
            let start_idx = self.starting_source_index % n;

            while total_accepted < self.max_jobs_per_run {
                let mut round_progress = false;

                for step in 0..n {
                    let i = (start_idx + step) % n;
                    let session = &mut sessions[i];
                    if session.is_finished() {
                        continue;
                    }
                    if session.accepted >= session.stats.limit {
                        session.stats.limit_reached = true;
                        continue;
                    }

                    // Visit this source for at most ONE eligible unique posting in this round:
                    let mut source_yielded_in_round = false;

                    while !source_yielded_in_round && !session.is_finished() {
                        match session.next_candidate(&self.client, self.started).await {
                            Ok(Some((event, evidence))) => {
                                match &event {
                                    WorkerEvent::SourceInfo { provider_total, .. } => {
                                        session.stats.provider_total = *provider_total;
                                        emit(&event)?;
                                        continue;
                                    }
                                    WorkerEvent::Error {
                                        invalid_posting: Some(true),
                                        ..
                                    } => {
                                        session.stats.examined += 1;
                                        session.stats.invalid += 1;
                                        session.had_errors = true;
                                        emit(&event)?;
                                        continue;
                                    }
                                    WorkerEvent::Warning { termination, .. } => {
                                        session.stats.enrichment_warnings += 1;
                                        if let Some(t) = termination {
                                            session.stats.termination = *t;
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
                                            || workflow_seen_urls.contains(&normalized_url)
                                        {
                                            session.stats.duplicates += 1;
                                            // Duplicate: continue candidate traversal to find a unique job!
                                            continue;
                                        }
                                        workflow_seen_identities.insert(identity.clone());
                                        workflow_seen_urls.insert(normalized_url);

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
                                            session.record_rejection(primary, reasons, title);
                                            continue;
                                        }

                                        if assessment
                                            .as_ref()
                                            .is_some_and(|a| !a.unresolved.is_empty())
                                        {
                                            session.stats.unresolved += 1;
                                            continue;
                                        }

                                        session.stats.matched += 1;

                                        if self.blocked.contains(&identity)
                                            || self.blocked.contains(&format!(
                                                "{}:{}",
                                                session.source.id, external_id
                                            ))
                                        {
                                            session.stats.blocked += 1;
                                            continue;
                                        }

                                        // ACCEPTED ELIGIBLE UNIQUE POSTING!
                                        session.accepted += 1;
                                        session.stats.accepted = u64::from(session.accepted);
                                        total_accepted += 1;
                                        source_yielded_in_round = true;
                                        round_progress = true;

                                        session.record_logo(company, company_logo);
                                        emit(&event)?;

                                        if total_accepted >= self.max_jobs_per_run
                                            || session.accepted >= session.stats.limit
                                        {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            Ok(None) => {
                                session.exhausted = true;
                                break;
                            }
                            Err(error) => {
                                session.failed = true;
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

                    if total_accepted >= self.max_jobs_per_run {
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
        let logo_deadline = Instant::now() + std::time::Duration::from_secs(4);

        for mut session in sessions {
            session.stats.limit_reached = session.stats.accepted == u64::from(session.stats.limit);
            if (total_accepted >= self.max_jobs_per_run || session.stats.limit_reached)
                && !session.exhausted
                && !session.failed
            {
                if session.stats.unresolved == 0
                    || session.stats.termination == CoverageTermination::Exhausted
                {
                    session.stats.termination = CoverageTermination::JobLimit;
                }
            } else if session.exhausted {
                if session.stats.termination != CoverageTermination::TimeLimit
                    && session.stats.termination != CoverageTermination::RequestLimit
                {
                    session.stats.termination = CoverageTermination::Exhausted;
                }
            } else if session.failed {
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
                    if Instant::now() >= logo_deadline {
                        logo.status = LogoStatus::DownloadFailed;
                        logo.diagnostic = Some(
                            "Source branding deadline reached; no new download scheduled".into(),
                        );
                    } else {
                        match tokio::time::timeout_at(
                            logo_deadline,
                            self.client.fetch_logo(&evidence.url),
                        )
                        .await
                        {
                            Ok(Ok(bytes)) => match scrape_core::branding::cache_asset(&bytes) {
                                Ok(asset) => {
                                    logo.asset = Some(asset);
                                    logo.status = LogoStatus::Verified;
                                }
                                Err(error) => {
                                    logo.status = LogoStatus::DecodeFailed;
                                    logo.diagnostic = Some(error);
                                }
                            },
                            result => {
                                logo.status = LogoStatus::DownloadFailed;
                                logo.diagnostic = Some(format!(
                                    "Logo download unavailable or deadline reached: {result:?}"
                                ));
                            }
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
                session.stats.termination = CoverageTermination::Failed;
                if session.stats.examined > session.stats.invalid {
                    ScanOutcome::Partial
                } else {
                    ScanOutcome::Failed
                }
            } else if session.stats.unresolved > 0 {
                if session.stats.termination == CoverageTermination::Exhausted {
                    session.stats.termination = CoverageTermination::Failed;
                }
                ScanOutcome::Partial
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
