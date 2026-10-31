pub mod ashby;
pub mod branding;
pub mod greenhouse;
pub mod jsonld;
pub mod lever;
pub mod search;
pub(crate) mod search_cache;
pub mod selector;
pub(crate) mod selector_session;
pub(crate) mod web_discovery;
pub mod workday;

use crate::client::{ClientError, HttpClient};
use scrape_core::{SourceKind, WorkerEvent, WorkerSource};

/// Compatibility collector used by parser/adapter tests. Production uses the stopping sink.
pub async fn scrape_source(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let mut events = Vec::new();
    scrape_source_with(client, source, &mut |event| {
        events.push(event);
        true
    })
    .await?;
    Ok(events)
}

/// Returning false stops candidate traversal and further detail requests.
pub async fn scrape_source_with(
    client: &HttpClient,
    source: &WorkerSource,
    sink: &mut dyn FnMut(WorkerEvent) -> bool,
) -> Result<(), ClientError> {
    scrape_source_with_evidence(
        client,
        source,
        &scrape_core::WorkerFilters::default(),
        chrono::Utc::now(),
        &scrape_core::protocol::EnrichmentLimits::default(),
        &mut |event, _| sink(event),
    )
    .await
}

pub async fn scrape_source_with_evidence(
    client: &HttpClient,
    source: &WorkerSource,
    filters: &scrape_core::WorkerFilters,
    started: chrono::DateTime<chrono::Utc>,
    limits: &scrape_core::protocol::EnrichmentLimits,
    sink: &mut dyn FnMut(WorkerEvent, Option<scrape_core::matching::MatchingEvidence>) -> bool,
) -> Result<(), ClientError> {
    if let Some(value) = source.config.get("employer_logo") {
        let selector = value.as_str().ok_or_else(|| {
            ClientError::Configuration("employer_logo must be a CSS selector string".into())
        })?;
        scraper::Selector::parse(selector).map_err(|error| {
            ClientError::Configuration(format!("Invalid employer_logo selector: {error}"))
        })?;
    }
    let mut validated = |event, evidence| {
        let event = match &event {
            WorkerEvent::Job { url, title, .. }
                if !url::Url::parse(url).is_ok_and(|u| matches!(u.scheme(), "http" | "https")) =>
            {
                WorkerEvent::Error {
                    invalid_posting: Some(true),
                    source_id: source.id.clone(),
                    message: format!("Posting '{title}' has an invalid HTTP(S) destination"),
                    retryable: false,
                }
            }
            _ => event,
        };
        sink(event, evidence)
    };
    match source.kind {
        SourceKind::Greenhouse => {
            greenhouse::scrape_greenhouse_filtered_with(
                client,
                source,
                filters,
                started,
                limits,
                &mut validated,
            )
            .await
        }
        SourceKind::Lever => {
            lever::scrape_lever_with(client, source, &mut |event| validated(event, None)).await
        }
        SourceKind::Ashby => {
            ashby::scrape_ashby_with(client, source, &mut |event| validated(event, None)).await
        }
        SourceKind::Jsonld => {
            jsonld::scrape_jsonld_with(client, source, &mut |event| validated(event, None)).await
        }
        SourceKind::Selector => {
            selector::scrape_selector_with(client, source, filters, started, limits, &mut validated)
                .await
        }
        SourceKind::Workday => {
            workday::scrape_workday_with(client, source, &mut |event| validated(event, None)).await
        }
        SourceKind::WebDiscovery => {
            let limits = scrape_core::WorkerLimits::default();
            let mut transport = crate::public_web::PublicWeb::new(
                (limits.max_pages_per_source as usize).saturating_mul(8),
                std::time::Duration::from_secs(limits.request_timeout_s),
                std::time::Duration::from_millis(limits.per_host_delay_ms),
            );
            if let Some(origin) = client.replay_origin() {
                transport = transport
                    .with_test_replay_origin(origin)
                    .map_err(|error| ClientError::Configuration(error.to_string()))?;
            }
            let mut session = web_discovery::WebDiscoverySession::new(
                source,
                &limits,
                tokio::time::Instant::now()
                    + std::time::Duration::from_secs(limits.max_source_duration_s),
                &transport,
                started,
            )?;
            loop {
                match session.next().await? {
                    selector_session::SelectorStep::Event(event) => {
                        let (event, evidence) = *event;
                        if !validated(event, evidence) {
                            break;
                        }
                    }
                    selector_session::SelectorStep::Yield => {}
                    selector_session::SelectorStep::Exhausted => break,
                }
            }
            Ok(())
        }
        SourceKind::Search => {
            search::scrape_search_with(client, source, filters, started, limits, &mut |event| {
                let evidence = scrape_core::matching::MatchingEvidence::from_event(&event, true);
                validated(event, Some(evidence))
            })
            .await
        }
    }
}
