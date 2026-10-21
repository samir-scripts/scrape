//! Posting identity is independent of the workflow that discovered it.
use crate::SourceKind;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PostingIdentity {
    pub key: String,
}

pub fn normalized_detail_url(destination: &str) -> Result<String, String> {
    let mut url = Url::parse(destination).map_err(|e| format!("Invalid posting URL: {e}"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Posting destinations must be HTTP(S) URLs".into());
    }
    url.set_fragment(None);
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            !key.starts_with("utm_") && !matches!(key.as_str(), "gclid" | "fbclid" | "msclkid")
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    Ok(url.into())
}

impl PostingIdentity {
    pub fn for_posting(
        kind: Option<SourceKind>,
        board: Option<&str>,
        external_id: Option<&str>,
        destination: &str,
    ) -> Result<Self, String> {
        let normalized = normalized_detail_url(destination)?;
        let key = match (kind, board, external_id.filter(|id| !id.trim().is_empty())) {
            (
                Some(
                    kind @ (SourceKind::Greenhouse
                    | SourceKind::Lever
                    | SourceKind::Ashby
                    | SourceKind::Workday),
                ),
                Some(board),
                Some(id),
            ) => serde_json::to_string(&(kind, board.trim().to_ascii_lowercase(), id.trim()))
                .unwrap(),
            (Some(SourceKind::Jsonld | SourceKind::Selector), _, Some(id))
                if Url::parse(id).is_err() =>
            {
                let origin = Url::parse(&normalized)
                    .unwrap()
                    .origin()
                    .ascii_serialization();
                serde_json::to_string(&("publisher", origin, id.trim())).unwrap()
            }
            _ => serde_json::to_string(&("url", normalized)).unwrap(),
        };
        Ok(Self { key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ats_identity_survives_workflow_and_destination_changes() {
        let identity = |url| {
            PostingIdentity::for_posting(
                Some(SourceKind::Greenhouse),
                Some("Acme"),
                Some("42"),
                url,
            )
            .unwrap()
        };
        assert_eq!(
            identity("https://boards.greenhouse.io/acme/jobs/42"),
            identity("https://careers.acme.test/?gh_jid=42")
        );
        assert_eq!(
            PostingIdentity::for_posting(
                Some(SourceKind::Workday),
                Some("nvidia/NVIDIAExternalCareerSite"),
                Some("JR2020731"),
                "https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite/job/123",
            )
            .unwrap()
            .key,
            serde_json::to_string(&(
                SourceKind::Workday,
                "nvidia/nvidiaexternalcareersite",
                "JR2020731"
            ))
            .unwrap()
        );
    }
    #[test]
    fn strips_only_known_tracking_and_keeps_provider_query_identity() {
        assert_eq!(
            normalized_detail_url("https://EXAMPLE.com/careers?gh_jid=42&utm_source=test#apply")
                .unwrap(),
            "https://example.com/careers?gh_jid=42"
        );
        assert_ne!(
            normalized_detail_url("https://example.com/careers?gh_jid=42").unwrap(),
            normalized_detail_url("https://example.com/careers?gh_jid=43").unwrap()
        );
        assert!(normalized_detail_url("javascript:alert(1)").is_err());
    }
}
