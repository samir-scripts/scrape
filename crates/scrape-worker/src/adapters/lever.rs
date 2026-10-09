use crate::client::{ClientError, HttpClient};
use chrono::{DateTime, SecondsFormat};
use scrape_core::{RoleType, WorkerEvent, WorkerSource};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LeverPosting {
    id: String,
    text: String,
    #[serde(rename = "hostedUrl")]
    hosted_url: String,
    #[serde(rename = "createdAt")]
    created_at: Option<i64>,
    categories: Option<LeverCategories>,
}

#[derive(Debug, Deserialize)]
struct LeverCategories {
    location: Option<String>,
    commitment: Option<String>,
}

pub async fn scrape_lever(
    client: &HttpClient,
    source: &WorkerSource,
) -> Result<Vec<WorkerEvent>, ClientError> {
    let url = format!(
        "https://api.lever.co/v0/postings/{}?mode=json",
        source.identifier.trim()
    );

    let postings: Vec<LeverPosting> = client.fetch_json(&url).await?;
    let mut events = Vec::with_capacity(postings.len());

    for posting in postings {
        let commitment = posting
            .categories
            .as_ref()
            .and_then(|c| c.commitment.as_deref());
        let role_type = RoleType::infer(&posting.text, commitment);

        let location = posting
            .categories
            .as_ref()
            .and_then(|c| c.location.clone())
            .unwrap_or_default();

        let date_posted = posting.created_at.and_then(|ms| {
            DateTime::from_timestamp_millis(ms)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
        });

        events.push(WorkerEvent::Job {
            source_id: source.id.clone(),
            external_id: posting.id,
            title: posting.text,
            company: source.name.clone(),
            role_type,
            location,
            url: posting.hosted_url,
            date_posted,
        });
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lever_json() {
        let json = r#"[
            {
                "id": "lever-123",
                "text": "Frontend Engineer",
                "hostedUrl": "https://jobs.lever.co/figma/lever-123",
                "createdAt": 1727740800000,
                "categories": {
                    "location": "San Francisco",
                    "commitment": "Full Time"
                }
            }
        ]"#;

        let postings: Vec<LeverPosting> = serde_json::from_str(json).unwrap();
        assert_eq!(postings.len(), 1);
        let p = &postings[0];
        assert_eq!(p.id, "lever-123");
        assert_eq!(p.text, "Frontend Engineer");
        let dt = p.created_at.and_then(|ms| {
            DateTime::from_timestamp_millis(ms)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
        });
        assert!(dt.is_some());
    }
}
