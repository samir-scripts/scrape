use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Employment type of a posting. `Unknown` is used when neither the source nor
/// the title heuristic can determine it (keeps the Job Types chart honest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoleType {
    FullTime,
    Contract,
    Internship,
    PartTime,
    Unknown,
}

/// Lifecycle of a job inside Scrape. `scraped`/`viewed` live in the Discovered
/// inbox; `applied` → `offer`/`rejected` form the Tracked pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum JobStatus {
    Scraped,
    Viewed,
    Applied,
    Interviewing,
    Offer,
    Rejected,
    Ignored,
}

/// How a source is scraped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SourceKind {
    Greenhouse,
    Lever,
    Ashby,
    Jsonld,
    Selector,
}

impl SourceKind {
    /// ATS kinds take a board slug as identifier; page kinds take a URL.
    pub fn expects_url(self) -> bool {
        matches!(self, SourceKind::Jsonld | SourceKind::Selector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&RoleType::FullTime).unwrap(),
            "\"full_time\""
        );
        assert_eq!(
            serde_json::to_string(&JobStatus::Interviewing).unwrap(),
            "\"interviewing\""
        );
        assert_eq!(
            serde_json::to_string(&SourceKind::Jsonld).unwrap(),
            "\"jsonld\""
        );
    }
}
