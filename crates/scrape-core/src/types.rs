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
    Seasonal,
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

/// Execution cadence for a scrape job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, Default)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SourceCadence {
    #[default]
    Global,
    Hourly,
    #[serde(rename = "every_6h")]
    #[cfg_attr(feature = "sqlx", sqlx(rename = "every_6h"))]
    Every6h,
    #[serde(rename = "every_12h")]
    #[cfg_attr(feature = "sqlx", sqlx(rename = "every_12h"))]
    Every12h,
    Daily,
    Weekly,
    Manual,
    Cron,
}

impl SourceCadence {
    /// Returns default 6-field cron expression if applicable.
    pub fn default_cron(self) -> Option<&'static str> {
        match self {
            SourceCadence::Global | SourceCadence::Manual => None,
            SourceCadence::Hourly => Some("0 0 * * * *"),
            SourceCadence::Every6h => Some("0 0 */6 * * *"),
            SourceCadence::Every12h => Some("0 0 */12 * * *"),
            SourceCadence::Daily => Some("0 0 9 * * *"),
            SourceCadence::Weekly => Some("0 0 9 * * MON"),
            SourceCadence::Cron => None,
        }
    }
}

/// Tri-state boolean used for optional qualification attributes (Sponsorship, Clearance, New Grad).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, Default)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TriState {
    Yes,
    No,
    #[default]
    Unknown,
}

/// Physical workplace arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, Default)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WorkplaceType {
    Remote,
    Hybrid,
    Onsite,
    #[default]
    Unknown,
}

/// Time unit interval for compensation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, Default)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SalaryInterval {
    #[default]
    Year,
    Month,
    Week,
    Day,
    Hour,
}

/// Origin of extracted compensation data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS, Default)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SalarySource {
    #[default]
    Structured,
    Text,
}

/// Security clearance level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ClearanceLevel {
    PublicTrust,
    Confidential,
    Secret,
    TopSecret,
    TsSci,
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
        assert_eq!(
            serde_json::to_string(&SourceCadence::Every12h).unwrap(),
            "\"every_12h\""
        );
        assert_eq!(
            SourceCadence::Every12h.default_cron(),
            Some("0 0 */12 * * *")
        );
        assert_eq!(SourceCadence::Global.default_cron(), None);
    }
}
