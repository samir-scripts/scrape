use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, Utc};
use scrape_core::enrich::{
    classify_clearance, classify_sponsorship, classify_work_auth, classify_workplace,
    extract_experience, location as loc_enrich, normalize_description_with_format,
    DescriptionFormat, Salary,
};
use scrape_core::locator::{FieldLocator, QueryEngine};
use scrape_core::{
    matching::MatchingEvidence, protocol::EnrichmentLimits, RoleType, TriState, WorkerEvent,
    WorkerFilters, WorkerSource,
};
use scraper::Html;
use url::Url;

pub async fn scrape_selector(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    scrape_selector_with(
        client,
        source,
        &WorkerFilters::default(),
        Utc::now(),
        &EnrichmentLimits::default(),
        &mut |event, _| {
            events.push(event);
            true
        },
    )
    .await?;
    Ok(events)
}
pub async fn scrape_selector_with(
    client: &HttpClient,
    source: &WorkerSource,
    filters: &WorkerFilters,
    started: DateTime<Utc>,
    enrichment: &EnrichmentLimits,
    sink: &mut dyn FnMut(WorkerEvent, Option<MatchingEvidence>) -> bool,
) -> Result<(), ClientError> {
    let browser = Some(std::sync::Arc::new(crate::browser::BrowserPool::new(2)));
    let mut session = super::selector_session::SelectorSession::new(
        client,
        browser,
        source,
        &scrape_core::WorkerLimits::default(),
        enrichment,
        tokio::time::Instant::now()
            + std::time::Duration::from_secs(scrape_core::protocol::DEFAULT_WORKFLOW_DURATION_S),
        filters,
    )
    .await?;
    loop {
        match session.next(client, filters, started).await? {
            super::selector_session::SelectorStep::Event(event) => {
                if !sink(event.0, event.1) {
                    break;
                }
            }
            super::selector_session::SelectorStep::Yield => continue,
            super::selector_session::SelectorStep::Exhausted => break,
        }
    }
    Ok(())
}

pub fn glob_to_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let mut regex_str = String::from("^");
    let trimmed = pattern.trim();
    let has_prefix = trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with('/')
        || trimmed.starts_with('*');
    if !has_prefix {
        regex_str.push_str(".*");
    }

    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                }
                regex_str.push_str(".*");
            }
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(c);
            }
            _ => regex_str.push(c),
        }
    }
    regex_str.push('$');
    regex::Regex::new(&regex_str)
}

pub fn extract_jobs_with_selectors(
    html_text: &str,
    base_url: &Url,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    extract_jobs_with_selectors_with(html_text, base_url, source, &mut |event| {
        events.push(event);
        true
    })?;
    Ok(events)
}

pub fn extract_jobs_with_selectors_with(
    html_text: &str,
    base_url: &Url,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    let document = Html::parse_document(html_text);
    let config = scrape_core::static_source::StaticSourceConfig::parse(
        &source.config,
        &scrape_core::WorkerLimits::default(),
    )
    .map_err(ClientError::Configuration)?;
    let doc_base = resolve_document_base(&document, base_url);
    let mut any = false;
    extract_candidates(
        &document,
        &doc_base,
        base_url.as_str(),
        source,
        &config,
        &mut |candidate| {
            any = true;
            sink(candidate.event(source))
        },
    )?;
    if !any && source.config.get("item").is_none() {
        extract_heuristic_candidates(
            &document,
            &doc_base,
            base_url.as_str(),
            source,
            &config,
            &mut |candidate| sink(candidate.event(source)),
        )?;
    }
    Ok(())
}

pub struct SelectorPage {
    pub candidates: std::collections::VecDeque<SelectorCandidate>,
    pub next_page: Option<Url>,
    pub next_page_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SelectorCandidate {
    Posting(Box<SelectorPosting>),
    Extracted(Box<WorkerEvent>),
    Invalid(Box<WorkerEvent>),
}

impl SelectorCandidate {
    pub fn event(self, source: &WorkerSource) -> WorkerEvent {
        match self {
            Self::Posting(posting) => posting.event(source),
            Self::Extracted(event) => *event,
            Self::Invalid(error) => *error,
        }
    }
}

pub fn resolve_document_base(document: &Html, response_url: &Url) -> Url {
    let selector = match scraper::Selector::parse("base[href]") {
        Ok(s) => s,
        Err(_) => return response_url.clone(),
    };
    if let Some(element) = document.select(&selector).next() {
        if let Some(href) = element.value().attr("href") {
            let trimmed = href.trim();
            if !trimmed.is_empty() {
                if let Ok(mut resolved) = response_url.join(trimmed) {
                    if matches!(resolved.scheme(), "http" | "https")
                        && resolved.host_str().is_some()
                        && resolved.username().is_empty()
                        && resolved.password().is_none()
                        && HttpClient::crawl_scope(
                            response_url,
                            &resolved,
                            HttpClient::crawl_path_prefix(response_url),
                        )
                        && !HttpClient::encoded_path_escape(trimmed)
                    {
                        resolved.set_fragment(None);
                        return resolved;
                    }
                }
            }
        }
    }
    response_url.clone()
}

pub fn extract_page(
    html: &str,
    base: &Url,
    source: &WorkerSource,
    config: &scrape_core::static_source::StaticSourceConfig,
) -> Result<SelectorPage, ClientError> {
    let document = Html::parse_document(html);
    if config
        .wait_for
        .as_ref()
        .is_some_and(|query| query.elements(&document, None).is_empty())
    {
        return Err(ClientError::Extraction("Static readiness locator did not match; inspect the selector or choose browser extraction".into()));
    }
    let doc_base = resolve_document_base(&document, base);
    let mut candidates = std::collections::VecDeque::new();
    if config.crawl.as_ref().is_some_and(|crawl| {
        crawl.extraction == scrape_core::static_source::CrawlExtraction::Jsonld
    }) {
        super::jsonld::extract_job_postings_with(html, base.as_str(), source, &mut |event| {
            candidates.push_back(SelectorCandidate::Extracted(Box::new(event)));
            true
        });
    } else if config.crawl.as_ref().is_some_and(|crawl| {
        crawl.extraction == scrape_core::static_source::CrawlExtraction::PublicHtml
    }) {
        extract_heuristic_candidates(
            &document,
            &doc_base,
            base.as_str(),
            source,
            config,
            &mut |candidate| {
                candidates.push_back(candidate);
                true
            },
        )?;
    } else {
        extract_candidates(
            &document,
            &doc_base,
            base.as_str(),
            source,
            config,
            &mut |candidate| {
                candidates.push_back(candidate);
                true
            },
        )?;
        if candidates.is_empty() && source.config.get("item").is_none() {
            extract_heuristic_candidates(
                &document,
                &doc_base,
                base.as_str(),
                source,
                config,
                &mut |candidate| {
                    candidates.push_back(candidate);
                    true
                },
            )?;
        }
    }
    if config.crawl.is_some() && candidates.is_empty() {
        let title_query = scraper::Selector::parse("title").expect("constant title selector");
        let title = document
            .select(&title_query)
            .next()
            .map(|node| node.text().collect::<String>())
            .unwrap_or_default();
        let password_query =
            scraper::Selector::parse("input[type=password]").expect("constant password selector");
        if scrape_core::browser::is_login_wall(
            base,
            base.as_str(),
            &title,
            document.select(&password_query).next().is_some(),
            false,
        ) {
            return Err(ClientError::NeedsLogin(
                "Website crawl encountered a login wall; only public HTML pages are supported"
                    .into(),
            ));
        }
        if matches!(
            title.trim().to_ascii_lowercase().as_str(),
            "just a moment..."
                | "attention required! | cloudflare"
                | "verify you are human"
                | "access denied"
        ) {
            return Err(ClientError::Extraction("Website crawl encountered an access challenge; no proxy/browser bypass is attempted".into()));
        }
    }
    let navigation = config
        .next_page
        .as_ref()
        .and_then(|field| field.extract(&document, None, "href"))
        .map(|raw| page_url(&doc_base, &raw))
        .transpose();
    let (next_page, next_page_error) = match navigation {
        Ok(url) => (url, None),
        Err(error) => (None, Some(error.to_string())),
    };
    Ok(SelectorPage {
        candidates,
        next_page,
        next_page_error,
    })
}

pub fn page_url(base: &Url, raw: &str) -> Result<Url, ClientError> {
    if HttpClient::encoded_path_escape(raw) {
        return Err(ClientError::Extraction(
            "Encoded path separators or dot segments are not crawl destinations".into(),
        ));
    }
    let mut url = base.join(raw)?;
    if !scrape_core::branding::http_url(url.as_str()) {
        return Err(ClientError::Extraction(
            "Navigation requires an HTTP(S) URL without credentials".into(),
        ));
    }
    url.set_fragment(None);
    Ok(url)
}

fn extract_candidates(
    document: &Html,
    base_url: &Url,
    evidence_page_url: &str,
    source: &WorkerSource,
    config: &scrape_core::static_source::StaticSourceConfig,
    sink: &mut dyn FnMut(SelectorCandidate) -> bool,
) -> Result<(), ClientError> {
    let patterns = UrlPatterns::parse(&source.config)?;
    if let Some(hydration) = scrape_core::hydration::HydrationConfig::parse(&source.config)
        .map_err(ClientError::Configuration)?
    {
        let script = hydration.script.elements(document, None).into_iter().next().filter(|e| e.value().name() == "script")
            .ok_or_else(|| ClientError::Extraction("Hydration script was not found; inspect hydration_script or choose browser extraction".into()))?;
        let payload: serde_json::Value = serde_json::from_str(&script.text().collect::<String>())
            .map_err(|e| {
            ClientError::Extraction(format!("Hydration script is not JSON: {e}"))
        })?;
        let jobs = payload.pointer(&hydration.pointer).and_then(serde_json::Value::as_array).ok_or_else(|| ClientError::Extraction("hydration_pointer did not select a candidate array; flattened/reference-encoded state must be decoded by a custom script".into()))?;
        let logo = super::branding::resolve_document_with_base(
            document,
            evidence_page_url,
            base_url.as_str(),
            &super::branding::employer_name(source),
            source.config.get("employer_logo").and_then(|v| v.as_str()),
        );
        for job in jobs {
            let get = |key| hydration.extract(job, key);
            let event = match (get("title"), get("link")) {
                (Some(title), Some(link)) => {
                    let url = base_url
                        .join(&link)
                        .ok()
                        .filter(|u| scrape_core::branding::http_url(u.as_str()));
                    if url
                        .as_ref()
                        .is_some_and(|url| !patterns.matches(url.as_str()))
                    {
                        continue;
                    }
                    match url {
                        Some(mut url) if !hydration.fields.contains_key("id") || get("id").is_some() => {
                            url.set_fragment(None);
                            SelectorCandidate::Posting(Box::new(SelectorPosting {
                                detail_url: config.has_details().then(|| url.to_string()),
                                title,
                                external_id: get("id").unwrap_or_else(|| url.to_string()),
                                url: url.into(),
                                location: get("location").unwrap_or_default(),
                                salary_text: get("salary"),
                                description_source: get("description"),
                                description_format: Some(if hydration.description_is_text { DescriptionFormat::PlainText } else { DescriptionFormat::Html }),
                                date_posted: get("date").filter(|raw| scrape_core::posting_date::posting_instant(raw,"UTC").is_some()),
                                company_logo: logo.clone(),
                            }))
                        }
                        _ => invalid_candidate(source, "Hydration candidate has an invalid HTTP(S) destination or missing configured ID"),
                    }
                }
                _ => invalid_candidate(
                    source,
                    "Hydration candidate title/link locator or regex did not produce a value",
                ),
            };
            if !sink(event) {
                break;
            }
        }
        return Ok(());
    }
    let config_string = |key: &str| -> Result<Option<&str>, ClientError> {
        source
            .config
            .get(key)
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| ClientError::Configuration(format!("{key} must be a string")))
            })
            .transpose()
    };
    let item_sel = QueryEngine::parse_elements(config_string("item")?.unwrap_or(".job-item"))
        .map_err(ClientError::Configuration)?;
    let field = |key: &str, default: Option<&str>| {
        FieldLocator::from_config(&source.config, key, default).map_err(ClientError::Configuration)
    };
    let title_sel = field("title", Some(".job-title"))?.unwrap();
    let link_sel = field("link", Some("a"))?.unwrap();
    let loc_sel = field("location", None)?;
    let sal_sel = field("salary", None)?;
    let desc_sel = field("description", None)?;
    let date_sel = field("date", None)?;
    let id_sel = field("id", None)?;

    let company_logo = super::branding::resolve_document_with_base(
        document,
        evidence_page_url,
        base_url.as_str(),
        &super::branding::employer_name(source),
        source.config.get("employer_logo").and_then(|v| v.as_str()),
    );

    for item in item_sel.elements(document, None) {
        let Some(title) = title_sel.extract(document, Some(item), "text") else {
            if !sink(invalid_candidate(
                source,
                "Required title locator or regex did not produce a value",
            )) {
                break;
            }
            continue;
        };
        let Some(clean_href) = link_sel.extract(document, Some(item), "href").or_else(|| {
            // Legacy CSS cards can themselves be the destination anchor.
            if source
                .config
                .get("link")
                .and_then(|v| v.as_str())
                .unwrap_or("a")
                == "a"
                && item.value().name() == "a"
                && link_sel.regex.is_none()
                && link_sel.attribute.is_none()
            {
                item.value().attr("href").map(str::to_string)
            } else {
                None
            }
        }) else {
            if !sink(invalid_candidate(
                source,
                &format!("Posting '{title}' has no destination from its link locator or regex"),
            )) {
                break;
            }
            continue;
        };
        let job_url = match page_url(base_url, &clean_href) {
            Ok(mut u) if matches!(u.scheme(), "http" | "https") => {
                u.set_fragment(None);
                u.to_string()
            }
            _ => {
                if !sink(SelectorCandidate::Invalid(Box::new(WorkerEvent::Error {
                    invalid_posting: Some(true),
                    source_id: source.id.clone(),
                    message: format!("Posting '{title}' has an invalid HTTP(S) destination"),
                    retryable: false,
                }))) {
                    break;
                }
                continue;
            }
        };

        if !patterns.matches(&job_url) {
            continue;
        }

        let location = loc_sel
            .as_ref()
            .and_then(|field| field.extract(document, Some(item), "text"))
            .unwrap_or_default();
        let salary_text = sal_sel
            .as_ref()
            .and_then(|field| field.extract(document, Some(item), "text"));
        let description_source = desc_sel
            .as_ref()
            .and_then(|field| field.extract(document, Some(item), "html"));
        let description_format = desc_sel.as_ref().map(|field| {
            if field.extracts_html() {
                DescriptionFormat::Html
            } else {
                DescriptionFormat::PlainText
            }
        });
        let date_posted = date_sel
            .as_ref()
            .and_then(|field| field.extract(document, Some(item), "date"))
            .filter(|raw| scrape_core::posting_date::posting_instant(raw, "UTC").is_some());
        let external_id = match &id_sel {
            Some(field) => match field.extract(document, Some(item), "text") {
                Some(id) => id,
                None => {
                    if !sink(invalid_candidate(
                        source,
                        &format!("Posting '{title}' has no configured original ID"),
                    )) {
                        break;
                    }
                    continue;
                }
            },
            None => job_url.clone(),
        };

        if !sink(SelectorCandidate::Posting(Box::new(SelectorPosting {
            title,
            url: job_url.clone(),
            external_id,
            location,
            salary_text,
            description_source,
            description_format,
            date_posted,
            company_logo: company_logo.clone(),
            detail_url: match &config.detail_link {
                Some(field) => field
                    .extract(document, Some(item), "href")
                    .and_then(|raw| page_url(base_url, &raw).ok().map(|u| u.to_string())),
                None => config.follow_detail.then(|| job_url.clone()),
            },
        }))) {
            break;
        }
    }

    Ok(())
}

pub(crate) fn extract_heuristic_candidates(
    document: &Html,
    base_url: &Url,
    evidence_page_url: &str,
    source: &WorkerSource,
    config: &scrape_core::static_source::StaticSourceConfig,
    sink: &mut dyn FnMut(SelectorCandidate) -> bool,
) -> Result<(), ClientError> {
    let mut jsonld_events = Vec::new();
    super::jsonld::extract_job_postings_with(
        &document.html(),
        base_url.as_str(),
        source,
        &mut |event| {
            jsonld_events.push(event);
            true
        },
    );
    if jsonld_events
        .iter()
        .any(|event| matches!(event, WorkerEvent::Job { .. }))
    {
        for event in jsonld_events {
            if !sink(SelectorCandidate::Extracted(Box::new(event))) {
                return Ok(());
            }
        }
        return Ok(());
    }

    let patterns = UrlPatterns::parse(&source.config)?;
    let company_logo = super::branding::resolve_document_with_base(
        document,
        evidence_page_url,
        base_url.as_str(),
        &super::branding::employer_name(source),
        source.config.get("employer_logo").and_then(|v| v.as_str()),
    );

    let anchor_sel = match scraper::Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let mut heuristic_found = false;
    let configured_date = FieldLocator::from_config(&source.config, "date", None)
        .map_err(ClientError::Configuration)?;

    for anchor in document.select(&anchor_sel) {
        if anchor
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .any(|node| {
                node.value().name() == "nav" || node.value().attr("role") == Some("navigation")
            })
        {
            continue;
        }
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let href_lower = href.to_ascii_lowercase();
        let is_job_link = href_lower.contains("/job/")
            || href_lower.contains("/jobs/")
            || href_lower.contains("/career/")
            || href_lower.contains("/careers/")
            || href_lower.contains("/position/")
            || href_lower.contains("/positions/")
            || href_lower.contains("/opening/")
            || href_lower.contains("/openings/")
            || href_lower.contains("/vacancy/")
            || href_lower.contains("/vacancies/")
            || href_lower.contains("/roles/")
            || href_lower.contains("gh_jid=")
            || href_lower.contains("lever-");

        let credible_card = anchor
            .ancestors()
            .filter_map(scraper::ElementRef::wrap)
            .take(4)
            .any(|node| {
                let class = node.value().attr("class").unwrap_or("");
                matches!(node.value().name(), "article" | "li" | "tr")
                    || class
                        .split_whitespace()
                        .any(|c| c == "posting" || c.contains("card") || c.contains("job-item"))
            });
        if !is_job_link && !credible_card {
            continue;
        }

        let Ok(mut job_url) = page_url(base_url, href) else {
            continue;
        };
        if !matches!(job_url.scheme(), "http" | "https") {
            continue;
        }
        job_url.set_fragment(None);
        let job_url_str = job_url.to_string();

        if !patterns.matches(&job_url_str) {
            continue;
        }

        let heading = scraper::Selector::parse("h2, h3, h4, h5, [class*='job-title']").unwrap();
        let raw_title = anchor
            .select(&heading)
            .next()
            .map(|node| node.text().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| anchor.text().collect::<Vec<_>>().join(" "))
            .trim()
            .to_string();
        if raw_title.is_empty() || raw_title.len() < 3 || raw_title.len() > 120 {
            continue;
        }

        let title_lower = raw_title.to_ascii_lowercase();
        if matches!(
            title_lower.as_str(),
            "apply"
                | "apply now"
                | "read more"
                | "view job"
                | "view details"
                | "details"
                | "learn more"
                | "more"
                | "careers"
                | "jobs"
                | "search jobs"
                | "sign in"
                | "log in"
                | "all jobs"
                | "back"
                | "home"
                | "privacy policy"
                | "terms of service"
        ) {
            continue;
        }
        let mut location = String::new();
        let mut salary_text = None;
        let mut date_posted = None;
        if let Some(parent) = anchor.parent().and_then(scraper::ElementRef::wrap) {
            let card = if scraper::Selector::parse("[class*='location'], [class*='salary']")
                .ok()
                .and_then(|sel| parent.select(&sel).next())
                .is_some()
            {
                parent
            } else if let Some(grandparent) = parent.parent().and_then(scraper::ElementRef::wrap) {
                let tag = grandparent.value().name();
                let class = grandparent.value().attr("class").unwrap_or("");
                if matches!(tag, "article" | "li" | "tr")
                    || class.contains("card")
                    || class.contains("job-item")
                    || class.contains("posting")
                {
                    grandparent
                } else {
                    parent
                }
            } else {
                parent
            };
            if let Ok(loc_sel) = scraper::Selector::parse(
                "[class*='location'], [data-testid*='location'], .location",
            ) {
                if let Some(loc_el) = card.select(&loc_sel).next() {
                    let text = loc_el
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string();
                    if !text.is_empty() && text.len() <= 80 {
                        location = text;
                    }
                }
            }
            if let Ok(sal_sel) = scraper::Selector::parse(
                "[class*='salary'], [class*='compensation'], [class*='pay'], .salary",
            ) {
                if let Some(sal_el) = card.select(&sal_sel).next() {
                    let text = sal_el
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string();
                    if !text.is_empty() && text.len() <= 60 {
                        salary_text = Some(text);
                    }
                }
            }
            date_posted = if let Some(locator) = &configured_date {
                locator
                    .extract(document, Some(card), "date")
                    .filter(|raw| scrape_core::posting_date::posting_instant(raw, "UTC").is_some())
            } else {
                publication_date(card)
            };
        }

        heuristic_found = true;
        let posting = SelectorCandidate::Posting(Box::new(SelectorPosting {
            title: raw_title,
            url: job_url_str.clone(),
            external_id: job_url_str.clone(),
            location,
            salary_text,
            description_source: None,
            description_format: Some(DescriptionFormat::PlainText),
            date_posted,
            company_logo: company_logo.clone(),
            detail_url: (config.follow_detail
                || config.has_details()
                || config.is_public_html_crawl())
            .then_some(job_url_str),
        }));
        if !sink(posting) {
            break;
        }
    }
    if !heuristic_found {
        if let Ok(h1_sel) = scraper::Selector::parse(
            "h1, .posting-headline h2, [class*='job-title'], main h2, article h2",
        ) {
            if let Some(h1) = document.select(&h1_sel).next() {
                let h1_title = h1.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if h1_title.len() >= 3 && h1_title.len() <= 120 {
                    let body_text = document.root_element().text().collect::<Vec<_>>().join(" ");
                    let lower = body_text.to_ascii_lowercase();
                    let has_job_cues = lower.contains("responsibilities")
                        || lower.contains("qualifications")
                        || lower.contains("requirements")
                        || lower.contains("experience")
                        || lower.contains("about the role")
                        || scraper::Selector::parse(
                            ".job-description, [itemprop='description'], [class*='job-content']",
                        )
                        .ok()
                        .is_some_and(|selector| {
                            document
                                .select(&selector)
                                .any(|node| node.text().collect::<String>().trim().len() >= 80)
                        });
                    if has_job_cues && body_text.len() >= 150 {
                        let self_url = evidence_page_url.to_string();
                        let posting = SelectorCandidate::Posting(Box::new(SelectorPosting {
                            title: h1_title,
                            url: self_url.clone(),
                            external_id: self_url.clone(),
                            location: String::new(),
                            salary_text: None,
                            description_source: Some(document.html()),
                            description_format: Some(DescriptionFormat::Html),
                            date_posted: None,
                            company_logo: company_logo.clone(),
                            detail_url: None,
                        }));
                        heuristic_found = true;
                        let _ = sink(posting);
                    }
                }
            }
        }
    }

    if !heuristic_found {
        for event in jsonld_events {
            if !sink(SelectorCandidate::Extracted(Box::new(event))) {
                break;
            }
        }
    }
    Ok(())
}

pub(crate) struct UrlPatterns {
    include: Vec<regex::Regex>,
    exclude: Vec<regex::Regex>,
}
fn publication_date(root: scraper::ElementRef<'_>) -> Option<String> {
    let selector = scraper::Selector::parse("time[datetime]").unwrap();
    root.select(&selector).find_map(|time| {
        let attributes = ["itemprop", "class", "id", "aria-label"]
            .iter()
            .filter_map(|key| time.value().attr(key))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        let text = time.text().collect::<String>().to_ascii_lowercase();
        let local_label = time
            .parent()
            .and_then(scraper::ElementRef::wrap)
            .map(|node| node.text().collect::<String>().to_ascii_lowercase())
            .unwrap_or_default();
        if attributes.contains("modified")
            || attributes.contains("updated")
            || text.contains("updated")
        {
            return None;
        }
        let identified = attributes.contains("dateposted")
            || attributes.contains("posted")
            || attributes.contains("published")
            || text.contains("posted")
            || text.contains("published")
            || (local_label.len() < 160
                && (local_label.contains("posted") || local_label.contains("published")));
        let raw = time.value().attr("datetime")?;
        (identified && scrape_core::posting_date::posting_instant(raw, "UTC").is_some())
            .then(|| raw.to_string())
    })
}

/// Merge only evidence belonging to this destination, retaining listing provenance.
pub(crate) fn hydrate_public_html_job(
    job: &mut WorkerEvent,
    html: &str,
    actual_url: &Url,
    source: &WorkerSource,
) -> Result<(), ClientError> {
    let WorkerEvent::Job { url, title, .. } = job else {
        return Ok(());
    };
    let identity =
        scrape_core::identity::normalized_detail_url(url).map_err(ClientError::Extraction)?;
    let actual_identity = scrape_core::identity::normalized_detail_url(actual_url.as_str())
        .map_err(ClientError::Extraction)?;
    let mut structured = Vec::new();
    super::jsonld::extract_detail_job_postings_with(
        html,
        actual_url.as_str(),
        source,
        &mut |event| {
            if let WorkerEvent::Job { url, .. } = &event {
                if scrape_core::identity::normalized_detail_url(url)
                    .ok()
                    .is_some_and(|key| key == identity || key == actual_identity)
                {
                    structured.push(event);
                }
            }
            true
        },
    );
    if structured.len() > 1 {
        macro_rules! field {
            ($event:expr, $field:ident) => {
                match $event {
                    WorkerEvent::Job { $field, .. } => $field,
                    _ => unreachable!(),
                }
            };
        }
        macro_rules! option_conflict {
            ($left:expr, $right:expr, $field:ident) => {
                match (field!($left, $field), field!($right, $field)) {
                    (Some(left), Some(right)) => left != right,
                    _ => false,
                }
            };
        }
        macro_rules! classification_conflict {
            ($left:expr, $right:expr, $field:ident, $unknown:expr) => {
                match (field!($left, $field), field!($right, $field)) {
                    (Some(left), Some(right)) => {
                        left.value != $unknown
                            && right.value != $unknown
                            && left.value != right.value
                    }
                    _ => false,
                }
            };
        }
        let conflict = structured.iter().enumerate().any(|(index, left)| {
            structured.iter().skip(index + 1).any(|right| {
                let left_date = field!(left, date_posted)
                    .as_deref()
                    .and_then(|raw| scrape_core::posting_date::posting_instant(raw, "UTC"));
                let right_date = field!(right, date_posted)
                    .as_deref()
                    .and_then(|raw| scrape_core::posting_date::posting_instant(raw, "UTC"));
                (left_date.is_some() && right_date.is_some() && left_date != right_date)
                    || option_conflict!(left, right, salary)
                    || (*field!(left, location_data_complete)
                        && *field!(right, location_data_complete)
                        && field!(left, locations) != field!(right, locations))
                    || (*field!(left, workplace_type) != scrape_core::WorkplaceType::Unknown
                        && *field!(right, workplace_type) != scrape_core::WorkplaceType::Unknown
                        && field!(left, workplace_type) != field!(right, workplace_type))
                    || (*field!(left, role_type) != RoleType::Unknown
                        && *field!(right, role_type) != RoleType::Unknown
                        && field!(left, role_type) != field!(right, role_type))
                    || classification_conflict!(left, right, sponsorship, TriState::Unknown)
                    || classification_conflict!(left, right, clearance, TriState::Unknown)
                    || classification_conflict!(
                        left,
                        right,
                        work_auth,
                        scrape_core::WorkAuthRequirement::Unknown
                    )
                    || (*field!(left, is_new_grad) != TriState::Unknown
                        && *field!(right, is_new_grad) != TriState::Unknown
                        && field!(left, is_new_grad) != field!(right, is_new_grad))
                    || match (field!(left, experience), field!(right, experience)) {
                        (Some(left), Some(right)) => {
                            (left.min_years.is_some()
                                && right.min_years.is_some()
                                && left.min_years != right.min_years)
                                || (left.max_years.is_some()
                                    && right.max_years.is_some()
                                    && left.max_years != right.max_years)
                        }
                        _ => false,
                    }
            })
        });
        if conflict {
            let message =
                format!("Conflicting JobPosting eligibility evidence for {url} on {actual_url}");
            if let WorkerEvent::Job {
                date_posted,
                salary,
                locations,
                location,
                role_type,
                location_data_complete,
                workplace_type,
                sponsorship,
                work_auth,
                clearance,
                experience,
                is_new_grad,
                ..
            } = job
            {
                *date_posted = None;
                *salary = None;
                locations.clear();
                *location_data_complete = false;
                location.clear();
                *role_type = RoleType::Unknown;
                *workplace_type = scrape_core::WorkplaceType::Unknown;
                *sponsorship = None;
                *work_auth = None;
                *clearance = None;
                *experience = None;
                *is_new_grad = TriState::Unknown;
            }
            return Err(ClientError::Extraction(message));
        }
    }
    let document = Html::parse_document(html);
    let text_at = |selector: &str| {
        scraper::Selector::parse(selector)
            .ok()
            .and_then(|selector| {
                document
                    .select(&selector)
                    .next()
                    .map(|node| node.text().collect::<Vec<_>>().join(" ").trim().to_string())
            })
    };
    let description = [
        "[class*='description']",
        "[class*='job-content']",
        "main",
        "article",
        "[role='main']",
        "body",
    ]
    .iter()
    .find_map(|selector| {
        scraper::Selector::parse(selector)
            .ok()
            .and_then(|selector| {
                document
                    .select(&selector)
                    .find(|node| node.text().collect::<String>().trim().len() >= 80)
                    .map(|node| node.html())
            })
    });
    let (config, listing_date) = if source.kind == scrape_core::SourceKind::Selector {
        let cfg = scrape_core::static_source::StaticSourceConfig::parse(
            &source.config,
            &scrape_core::WorkerLimits::default(),
        )
        .map_err(ClientError::Configuration)?;
        let date = FieldLocator::from_config(&source.config, "date", None)
            .map_err(ClientError::Configuration)?;
        (Some(cfg), date)
    } else {
        (None, None)
    };
    let configured_date = config
        .as_ref()
        .and_then(|c| c.detail_fields.get("date"))
        .or(listing_date.as_ref());
    let date_posted = if let Some(locator) = configured_date {
        locator
            .extract(&document, None, "date")
            .filter(|raw| scrape_core::posting_date::posting_instant(raw, "UTC").is_some())
    } else {
        publication_date(document.root_element())
    };
    let dom = SelectorPosting {
        title: title.clone(),
        url: url.clone(),
        external_id: url.clone(),
        location: text_at("[class*='location'], [data-testid*='location']").unwrap_or_default(),
        salary_text: text_at("[class*='salary'], [class*='compensation']"),
        description_source: description,
        description_format: Some(DescriptionFormat::Html),
        date_posted,
        company_logo: None,
        detail_url: None,
    }
    .event(source);
    for record in structured.into_iter().chain(std::iter::once(dom)) {
        merge_normalized_job(job, record);
    }
    Ok(())
}

pub(crate) fn merge_normalized_job(target: &mut WorkerEvent, incoming: WorkerEvent) {
    if let (
        WorkerEvent::Job {
            date_posted,
            salary,
            locations,
            location_data_complete,
            location,
            workplace_type,
            role_type,
            sponsorship,
            work_auth,
            clearance,
            experience,
            is_new_grad,
            description_text,
            description,
            description_source,
            description_format,
            ..
        },
        WorkerEvent::Job {
            date_posted: next_date,
            salary: next_salary,
            locations: next_locations,
            location_data_complete: next_complete,
            location: next_location,
            workplace_type: next_workplace,
            role_type: next_role,
            sponsorship: next_sponsorship,
            work_auth: next_auth,
            clearance: next_clearance,
            experience: next_experience,
            is_new_grad: next_grad,
            description_text: next_text,
            description: next_description,
            description_source: next_source,
            description_format: next_format,
            ..
        },
    ) = (target, incoming)
    {
        if date_posted.is_none() {
            *date_posted = next_date;
        }
        if salary.is_none() {
            *salary = next_salary;
        }
        let remote_requirements = next_locations
            .iter()
            .any(|item| item.raw.starts_with("applicantLocationRequirements:"));
        let stronger_remote = remote_requirements
            && !locations
                .iter()
                .any(|item| item.raw.starts_with("applicantLocationRequirements:"));
        if !*location_data_complete || stronger_remote {
            *locations = next_locations;
            *location_data_complete = next_complete;
        }
        if location.is_empty() {
            *location = next_location;
        }
        if *workplace_type == scrape_core::WorkplaceType::Unknown || stronger_remote {
            *workplace_type = next_workplace;
        }
        if *role_type == RoleType::Unknown {
            *role_type = next_role;
        }
        if sponsorship
            .as_ref()
            .is_none_or(|item| item.value == TriState::Unknown)
        {
            *sponsorship = next_sponsorship;
        }
        if work_auth
            .as_ref()
            .is_none_or(|item| item.value == scrape_core::WorkAuthRequirement::Unknown)
        {
            *work_auth = next_auth;
        }
        if clearance
            .as_ref()
            .is_none_or(|item| item.value == TriState::Unknown)
        {
            *clearance = next_clearance;
        }
        if experience
            .as_ref()
            .is_none_or(|item| item.min_years.is_none() && item.max_years.is_none())
        {
            *experience = next_experience;
        }
        if *is_new_grad == TriState::Unknown {
            *is_new_grad = next_grad;
        }
        if description_text.is_none() {
            *description_text = next_text;
        }
        if description.is_none() {
            *description = next_description;
        }
        if description_source.is_none() {
            *description_source = next_source;
            *description_format = next_format;
        }
    }
}

impl UrlPatterns {
    pub(crate) fn parse(config: &serde_json::Value) -> Result<Self, ClientError> {
        let mut include = Vec::new();
        let compile = |value: &serde_json::Value| {
            let text = value
                .as_str()
                .ok_or_else(|| ClientError::Configuration("URL patterns must be strings".into()))?;
            glob_to_regex(text)
                .map_err(|e| ClientError::Configuration(format!("Invalid URL pattern: {e}")))
        };
        if let Some(value) = config.get("url_pattern") {
            if value.as_str().is_none_or(|s| !s.trim().is_empty()) {
                include.push(compile(value)?);
            }
        }
        let array = |key: &str| -> Result<Vec<regex::Regex>, ClientError> {
            let Some(value) = config.get(key) else {
                return Ok(Vec::new());
            };
            value
                .as_array()
                .ok_or_else(|| {
                    ClientError::Configuration(format!("{key} must be an array of URL patterns"))
                })?
                .iter()
                .filter(|v| v.as_str().is_none_or(|s| !s.trim().is_empty()))
                .map(compile)
                .collect()
        };
        include.extend(array("url_patterns")?);
        Ok(Self {
            include,
            exclude: array("exclude_url_patterns")?,
        })
    }
    pub(crate) fn matches(&self, url: &str) -> bool {
        (self.include.is_empty() || self.include.iter().any(|p| p.is_match(url)))
            && !self.exclude.iter().any(|p| p.is_match(url))
    }
}

#[derive(Debug, Clone)]
pub struct SelectorPosting {
    pub title: String,
    pub url: String,
    pub external_id: String,
    pub location: String,
    pub salary_text: Option<String>,
    pub description_source: Option<String>,
    pub description_format: Option<DescriptionFormat>,
    pub date_posted: Option<String>,
    pub company_logo: Option<scrape_core::branding::LogoEvidence>,
    pub detail_url: Option<String>,
}
impl SelectorPosting {
    pub fn event(self, source: &WorkerSource) -> WorkerEvent {
        let Self {
            title,
            url,
            external_id,
            location,
            salary_text,
            description_source,
            description_format,
            date_posted,
            company_logo,
            ..
        } = self;
        let role_type = RoleType::infer(&title, None);
        let description = description_source.as_deref().map(|html| {
            normalize_description_with_format(
                html,
                description_format.unwrap_or(DescriptionFormat::Html),
                32768,
            )
        });
        let desc_text = description.as_ref().map(|d| d.plain_text.clone());

        let mut salary = salary_text.as_deref().and_then(|t| {
            Salary::from_text(t).or_else(|| Salary::from_text(&format!("salary: {t}")))
        });
        if salary.is_none() {
            if let Some(text) = &desc_text {
                salary = Salary::from_text(text);
            }
        }

        let mut locations = loc_enrich::parse(&location);
        let section_locations = description
            .as_ref()
            .map(loc_enrich::from_description_sections);
        let location_data_complete = loc_enrich::has_geographic_value(&locations)
            || section_locations
                .as_ref()
                .is_some_and(|evidence| evidence.section_found);
        if let Some(evidence) = section_locations {
            locations.extend(evidence.locations);
        }
        let workplace_type = classify_workplace(None, &location, desc_text.as_deref());

        let text_for_enrich = desc_text.as_deref().unwrap_or("");
        let sponsorship = if !text_for_enrich.is_empty() {
            Some(classify_sponsorship(text_for_enrich))
        } else {
            None
        };
        let work_auth = if !text_for_enrich.is_empty() {
            Some(classify_work_auth(text_for_enrich))
        } else {
            None
        };
        let clearance = if !text_for_enrich.is_empty() {
            Some(classify_clearance(text_for_enrich))
        } else {
            None
        };
        let experience = Some(extract_experience(&title, text_for_enrich));
        let is_new_grad = experience
            .as_ref()
            .map(|e| e.is_new_grad)
            .unwrap_or(TriState::Unknown);

        WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id,
            title,
            company: super::branding::employer_name(source),
            role_type,
            location,
            url,
            provider_updated_at: None,
            date_posted,
            salary,
            locations,
            location_data_complete,
            workplace_type,
            sponsorship,
            work_auth,
            clearance,
            experience,
            is_new_grad,
            description_text: desc_text,
            description,
            description_format: description_source.as_ref().and(description_format),
            description_source,
            company_icon_url: company_logo.as_ref().map(|logo| logo.url.clone()),
            company_logo,
        }
    }
}

fn invalid_candidate(source: &WorkerSource, message: &str) -> SelectorCandidate {
    SelectorCandidate::Invalid(Box::new(WorkerEvent::Error {
        invalid_posting: Some(true),
        source_id: source.id.clone(),
        message: message.into(),
        retryable: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::{SourceKind, WorkplaceType};

    #[test]
    fn hydration_json_pointers_use_shared_transforms_and_original_destinations() {
        let source = WorkerSource {
            id: "hydration".into(),
            kind: SourceKind::Selector,
            name: "Fixture".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({
                "hydration_script":"#__NEXT_DATA__", "hydration_pointer":"/props/pageProps/jobs",
                "hydration_fields":{"title":"/name", "link":"/apply", "id":"/requisition", "salary":"/pay", "description":"/body", "date":"/published"},
                "title_regex":"Role: (?P<value>.+)", "description_attribute":"text"
            }),
        };
        let html = r#"<script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"jobs":[{"name":"Role: Rust Engineer","apply":"/jobs/42?ref=original","requisition":42,"pay":"$45.25 - $55.75 per hour","body":"Literal <b> text","published":"2026-10-15"},{"name":"Invalid missing link"}]}}}</script>"#;
        let events =
            extract_jobs_with_selectors(html, &Url::parse(&source.identifier).unwrap(), &source)
                .unwrap();
        assert_eq!(events.len(), 2);
        let WorkerEvent::Job {
            title,
            url,
            external_id,
            description_text,
            date_posted,
            salary,
            ..
        } = &events[0]
        else {
            panic!("expected job")
        };
        assert_eq!(title, "Rust Engineer");
        assert_eq!(url, "https://fixture.test/jobs/42?ref=original");
        assert_eq!(external_id, "42");
        assert_eq!(description_text.as_deref(), Some("Literal <b> text"));
        assert_eq!(date_posted.as_deref(), Some("2026-10-15"));
        assert_eq!(salary.as_ref().unwrap().ranges[0].min_minor, Some(4525));
        assert!(matches!(
            &events[1],
            WorkerEvent::Error {
                invalid_posting: Some(true),
                ..
            }
        ));
        let mut restricted = source.clone();
        restricted.config["exclude_url_patterns"] = serde_json::json!(["*/jobs/*"]);
        let excluded = extract_jobs_with_selectors(
            html,
            &Url::parse(&source.identifier).unwrap(),
            &restricted,
        )
        .unwrap();
        assert!(!excluded
            .iter()
            .any(|e| matches!(e, WorkerEvent::Job { .. })));
        for invalid in [
            "<p>No script</p>",
            "<script id='__NEXT_DATA__'>{bad}</script>",
            "<script id='__NEXT_DATA__'>{}</script>",
        ] {
            assert!(extract_jobs_with_selectors(
                invalid,
                &Url::parse(&source.identifier).unwrap(),
                &source
            )
            .is_err());
        }
    }

    #[test]
    fn mixed_locators_extract_original_id_attributes_and_regex() {
        let source = WorkerSource {
            id: "mixed".into(),
            kind: SourceKind::Selector,
            name: "Fixture".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({
                "item": ".job-card", "title": "xpath:.//h2/text()",
                "title_regex": "Role: (?P<value>.+)", "link": "xpath:.//a[1]/@href",
                "id": ".", "id_attribute": "data-job-id", "id_regex": "req-(?P<value>[0-9]+)",
                "location": "xpath:.//h2/following-sibling::span[1]",
                "date": "time", "date_attribute": "datetime",
            }),
        };
        let html = "<article class='job-card' data-job-id='req-42'><h2>Role: Rust Engineer</h2><span>Remote - US</span><a href='/jobs/42'>Apply</a><time datetime='2026-10-15'></time></article>";
        let events =
            extract_jobs_with_selectors(html, &Url::parse(&source.identifier).unwrap(), &source)
                .unwrap();
        match &events[0] {
            WorkerEvent::Job {
                title,
                external_id,
                location,
                url,
                date_posted,
                ..
            } => {
                assert_eq!(title, "Rust Engineer");
                assert_eq!(external_id, "42");
                assert_eq!(location, "Remote - US");
                assert_eq!(url, "https://fixture.test/jobs/42");
                assert_eq!(date_posted.as_deref(), Some("2026-10-15"));
            }
            other => panic!("Expected original extracted candidate, got {other:?}"),
        }
    }

    #[test]
    fn xpath_containers_mix_css_fields_and_keep_optional_unknowns() {
        let source = WorkerSource {
            id: "mixed".into(),
            kind: SourceKind::Selector,
            name: "Fixture".into(),
            identifier: "https://fixture.test/careers".into(),
            config: serde_json::json!({
                "item": "//article[contains(@class,'job-card')]", "title": "h2@data-title",
                "link": "a@data-url", "salary": "span", "salary_regex": "Pay: (?P<value>.+)",
                "description": "p", "description_attribute": "text", "description_regex": "Description: (?P<value>.+)",
                "date": "time", "date_attribute": "data-published", "date_regex": "Published: (?P<value>.+)",
            }),
        };
        let html = "<article class='job-card'><h2 data-title='Rust Engineer'></h2><a data-url='/jobs/42'></a><span>Unknown pay</span><p>Description: Literal &lt;b&gt; text</p><time data-published='Published: 2026-10-15'></time></article>";
        let events =
            extract_jobs_with_selectors(html, &Url::parse(&source.identifier).unwrap(), &source)
                .unwrap();
        match &events[0] {
            WorkerEvent::Job {
                salary,
                description_text,
                description_format,
                date_posted,
                ..
            } => {
                assert!(salary.is_none());
                assert_eq!(description_text.as_deref(), Some("Literal <b> text"));
                assert_eq!(*description_format, Some(DescriptionFormat::PlainText));
                assert_eq!(date_posted.as_deref(), Some("2026-10-15"));
            }
            other => panic!("Expected Job, got {other:?}"),
        }
    }

    #[test]
    fn failed_required_field_transforms_are_invalid_candidates() {
        for (field, regex) in [
            ("title", "Missing (.+)"),
            ("link", "missing/(.+)"),
            ("id", "missing-(.+)"),
        ] {
            let mut source = WorkerSource {
                id: "required".into(),
                kind: SourceKind::Selector,
                name: "Fixture".into(),
                identifier: "https://fixture.test/careers".into(),
                config: serde_json::json!({"item":"article", "title":"h2", "link":"a", "id":"article@data-id"}),
            };
            source.config["id"] = serde_json::json!(".@data-id");
            source.config[format!("{field}_regex")] = regex.into();
            let events = extract_jobs_with_selectors(
                "<article data-id='req-42'><h2>Engineer</h2><a href='/jobs/42'>Apply</a></article>",
                &Url::parse(&source.identifier).unwrap(),
                &source,
            )
            .unwrap();
            assert_eq!(events.len(), 1);
            assert!(
                matches!(
                    &events[0],
                    WorkerEvent::Error {
                        invalid_posting: Some(true),
                        ..
                    }
                ),
                "{field}: {:?}",
                events[0]
            );
        }
    }

    #[test]
    fn test_extract_jobs_with_selectors() {
        let html = r#"
        <div class="careers-list">
            <div class="job-card">
                <h3 class="job-header"><a class="link" href="/jobs/eng-1">Staff Platform Engineer</a></h3>
                <span class="city">New York, NY</span>
            </div>
            <div class="job-card">
                <h3 class="job-header"><a class="link" href="/jobs/eng-2">Software Engineering Intern</a></h3>
                <span class="city">San Francisco, CA</span>
            </div>
        </div>
        "#;

        let config = serde_json::json!({
            "item": ".job-card",
            "title": ".job-header",
            "link": ".link",
            "location": ".city"
        });

        let source = WorkerSource {
            id: "src-selector".to_string(),
            kind: SourceKind::Selector,
            name: "Custom Corp".to_string(),
            identifier: "https://customcorp.com/careers".to_string(),
            config,
        };

        let base_url = Url::parse("https://customcorp.com/careers").unwrap();
        let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
        assert_eq!(events.len(), 2);

        if let WorkerEvent::Job {
            title,
            role_type,
            location,
            url,
            ..
        } = &events[1]
        {
            assert_eq!(title, "Software Engineering Intern");
            assert_eq!(*role_type, RoleType::Internship);
            assert_eq!(location, "San Francisco, CA");
            assert_eq!(url, "https://customcorp.com/jobs/eng-2");
        } else {
            panic!("Expected Job event");
        }
    }

    #[test]
    fn test_extract_jobs_with_custom_regex() {
        let html = r#"
        <div class="career-feed">
            <div class="listing">
                <h3 class="raw-header">Role: Senior Rust Architect [Ref #42]</h3>
                <a class="apply-btn" href="/postings/req-999?source=web&tracker=1">Apply</a>
                <div class="meta-row">Location: Remote - US | Salary: $180,000 - $220,000 / yr</div>
            </div>
        </div>
        "#;

        let config = serde_json::json!({
            "item": ".listing",
            "title": ".raw-header",
            "title_regex": r"Role:\s*(.*?)\s*\[Ref",
            "link": ".apply-btn",
            "link_regex": r"(/postings/req-\d+)",
            "location": ".meta-row",
            "location_regex": r"Location:\s*([^|]+)",
            "salary": ".meta-row",
            "salary_regex": r"Salary:\s*(\$\d+[\d,]*\s*-\s*\$\d+[\d,]*\s*/\s*yr)"
        });

        let source = WorkerSource {
            id: "src-regex".to_string(),
            kind: SourceKind::Selector,
            name: "Regex Corp".to_string(),
            identifier: "https://regexcorp.com/careers".to_string(),
            config,
        };

        let base_url = Url::parse("https://regexcorp.com/careers").unwrap();
        let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
        assert_eq!(events.len(), 1);

        if let WorkerEvent::Job {
            title,
            location,
            url,
            salary,
            workplace_type,
            ..
        } = &events[0]
        {
            assert_eq!(title, "Senior Rust Architect");
            assert_eq!(location, "Remote - US");
            assert_eq!(url, "https://regexcorp.com/postings/req-999");
            assert_eq!(*workplace_type, WorkplaceType::Remote);

            let sal = salary.as_ref().expect("Salary should be parsed");
            assert_eq!(sal.min, Some(180000));
            assert_eq!(sal.max, Some(220000));
        } else {
            panic!("Expected Job event");
        }
    }

    #[test]
    fn test_glob_to_regex() {
        let r1 = glob_to_regex("https://company.com/careers/*").unwrap();
        assert!(r1.is_match("https://company.com/careers/123"));
        assert!(!r1.is_match("https://other.com/careers/123"));

        let r2 = glob_to_regex("example/link/*").unwrap();
        assert!(r2.is_match("https://myco.com/example/link/engineer-1"));
        assert!(!r2.is_match("https://myco.com/about/team"));

        let r3 = glob_to_regex("*/privacy").unwrap();
        assert!(r3.is_match("https://myco.com/privacy"));
        assert!(!r3.is_match("https://myco.com/privacy/more"));
    }

    #[test]
    fn test_extract_jobs_with_url_pattern_glob() {
        let html = r#"
        <div class="jobs-list">
            <div class="item">
                <span class="title">Staff Engineer</span>
                <a href="/careers/jobs/101">Apply</a>
            </div>
            <div class="item">
                <span class="title">Privacy Policy</span>
                <a href="/legal/privacy">Read Policy</a>
            </div>
            <div class="item">
                <span class="title">Product Manager</span>
                <a href="/careers/jobs/102">Apply</a>
            </div>
            <div class="item">
                <span class="title">Company Blog</span>
                <a href="/blog/hello-world">Read Blog</a>
            </div>
        </div>
        "#;

        let config = serde_json::json!({
            "item": ".item",
            "title": ".title",
            "link": "a",
            "url_pattern": "*/careers/jobs/*",
            "exclude_url_patterns": ["*/privacy", "*/legal/*"]
        });

        let source = WorkerSource {
            id: "src-glob".to_string(),
            kind: SourceKind::Selector,
            name: "Glob Corp".to_string(),
            identifier: "https://globcorp.com".to_string(),
            config,
        };

        let base_url = Url::parse("https://globcorp.com").unwrap();
        let events = extract_jobs_with_selectors(html, &base_url, &source).unwrap();
        assert_eq!(events.len(), 2);

        if let WorkerEvent::Job { title, url, .. } = &events[0] {
            assert_eq!(title, "Staff Engineer");
            assert_eq!(url, "https://globcorp.com/careers/jobs/101");
        } else {
            panic!("Expected Job event");
        }

        if let WorkerEvent::Job { title, url, .. } = &events[1] {
            assert_eq!(title, "Product Manager");
            assert_eq!(url, "https://globcorp.com/careers/jobs/102");
        } else {
            panic!("Expected Job event");
        }
    }
}
