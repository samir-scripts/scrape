pub mod ashby;
pub mod greenhouse;
pub mod jsonld;
pub mod lever;
pub mod selector;

use crate::client::{ClientError, HttpClient};
use scrape_core::{SourceKind, WorkerEvent, WorkerSource};

pub async fn scrape_source(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let events = match source.kind {
        SourceKind::Greenhouse => greenhouse::scrape_greenhouse(client, source).await,
        SourceKind::Lever => lever::scrape_lever(client, source).await,
        SourceKind::Ashby => ashby::scrape_ashby(client, source).await,
        SourceKind::Jsonld => jsonld::scrape_jsonld(client, source).await,
        SourceKind::Selector => selector::scrape_selector(client, source).await,
    }?;
    Ok(events
        .into_iter()
        .map(|event| {
            if let WorkerEvent::Job {
                ref url, ref title, ..
            } = event
            {
                let valid =
                    url::Url::parse(url).is_ok_and(|u| matches!(u.scheme(), "http" | "https"));
                if !valid {
                    return WorkerEvent::Error {
                        source_id: source.id.clone(),
                        message: format!("Posting '{title}' has an invalid HTTP(S) destination"),
                        retryable: false,
                    };
                }
            }
            event
        })
        .collect())
}
