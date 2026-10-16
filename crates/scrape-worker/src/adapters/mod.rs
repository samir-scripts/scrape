pub mod ashby;
pub mod branding;
pub mod greenhouse;
pub mod jsonld;
pub mod lever;
pub mod selector;

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
            selector::scrape_selector_with(client, source, &mut |event| validated(event, None))
                .await
        }
    }
}
