//! Shared domain vocabulary for Scrape.
//!
//! Used by both the Tauri host (`src-tauri`) and the `scrape-worker` sidecar so
//! that enum values written to SQLite, sent over the NDJSON protocol, and
//! exported to TypeScript always agree.

pub mod enrich;
pub mod filter;
pub mod identity;
pub use identity::PostingIdentity;
pub mod protocol;
mod role;
mod types;

pub use filter::{FilterExpr, FilterField, FilterOp, JobFilterContext};
pub use protocol::{
    WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, WorkerStats,
    PROTOCOL_VERSION,
};
pub use types::{
    ClearanceLevel, JobStatus, RoleType, SalaryInterval, SalarySource, SourceCadence, SourceKind,
    TriState, WorkAuthRequirement, WorkplaceType,
};

use chrono::{SecondsFormat, Utc};

/// Current UTC time as an ISO-8601 string with millisecond precision
/// (e.g. `2026-10-09T05:12:00.123Z`). Fixed width, so it sorts lexically.
pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_iso_is_fixed_width_utc() {
        let s = now_iso();
        assert_eq!(s.len(), 24, "{s}");
        assert!(s.ends_with('Z'));
    }
}
