pub mod ashby;
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
    let mut validated = |event| {
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
        sink(event)
    };
    match source.kind {
        SourceKind::Greenhouse => {
            greenhouse::scrape_greenhouse_with(client, source, &mut validated).await
        }
        SourceKind::Lever => lever::scrape_lever_with(client, source, &mut validated).await,
        SourceKind::Ashby => ashby::scrape_ashby_with(client, source, &mut validated).await,
        SourceKind::Jsonld => jsonld::scrape_jsonld_with(client, source, &mut validated).await,
        SourceKind::Selector => {
            selector::scrape_selector_with(client, source, &mut validated).await
        }
    }
}
