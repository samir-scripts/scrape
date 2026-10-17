# `src-tauri/src/db` — SQLite Database Repositories

## Overview

The `src-tauri/src/db` directory encapsulates all data access logic, repository methods, and raw SQL queries for the local SQLite database. It leverages `sqlx` with compile-time checked queries and asynchronous connection pooling.

---

## File Manifest

| File | Target Table(s) | Key Functions & Operations |
|---|---|---|
| [`mod.rs`](mod.rs) | Database Connection Pool & Startup | Configures SQLite pragmas (`WAL`, `synchronous = NORMAL`, `foreign_keys = ON`), validates historical migration prefixes before writable changes, takes `VACUUM INTO` backups, and executes forward-only migrations from `src-tauri/migrations/`. |
| [`jobs.rs`](jobs.rs) | `jobs`, `job_locations`, `job_status_events` | `JobsRepo`: SQLite-side filtering, sorting, counting, and pagination; upserting scraped postings while preserving user notes, favorites, and tracked status; soft removal (`remove_job`), restoration (`restore_posting`), and cross-workflow blocking via `postings`. |
| [`postings.rs`](postings.rs) | `postings`, `deleted_jobs` | Canonical posting identity registry (`PostingIdentity`), cross-workflow discovery suppression (`blocked`), and legacy tombstone reconciliation. |
| [`branding.rs`](branding.rs) | `employer_brandings`, `employer_branding_assets` | Stores verified employer logo evidence (URL, page, method, timestamp, SHA-256) and one deduplicated PNG asset per source/employer relationship, retaining verified artwork across temporary refresh failures and source deletion. |
| [`cleanup.rs`](cleanup.rs) | All application tables | `preview` and `execute` for confirmed Data Management operations (`DeleteAll`, `KeepFavorites`, `KeepTracked`, `ResetAll`) with `VACUUM INTO` recovery snapshots and posting-identity protection. |
| [`compensation.rs`](compensation.rs) | `job_compensations` | Persists and queries native compensation ranges (hundredths of currency units, interval, currency, source evidence) separately from annual compatibility analytics. |
| [`history.rs`](history.rs) | `_sqlx_migrations` | Read-only historical migration checksum and schema verification against recognized migration histories before applying new upgrades. |
| [`sources.rs`](sources.rs) | `sources` | `SourcesRepo`: Manages workflow/source records, enabled flags, schedule cadences, review status (`needs_review`), script trust/fingerprints, and last-scraped timestamps. |
| [`runs.rs`](runs.rs) | `scrape_runs`, `scrape_logs` | `RunsRepo`: Records execution telemetry, `ScanOutcome`, `RunCounts` (including per-source `SourceScanStats` summaries), duration, log entries, and trigger types. |
| [`metrics.rs`](metrics.rs) | `jobs`, `job_status_events` | `MetricsRepo`: Aggregation queries calculating stage totals, conversion rates, role distributions, salary distribution buckets, streaks, and daily velocity. |
| [`settings.rs`](settings.rs) | `settings` | `SettingsRepo`: Serializes and retrieves user preferences, appearance settings, and automatic-scraping pause state. |
| [`notifications.rs`](notifications.rs) | `notifications` | `NotificationsRepo`: Manages the in-app notification ledger for crawl completions, warnings/errors, and alerts. |

Migration startup now refuses checksum mismatches instead of rewriting history;
pending disk upgrades receive a consistent SQLite snapshot. See
[database recovery](../../../plans/database-recovery.md). Both retention policies
share one parameterized cutoff and eligibility predicate, based on discovery age;
tracked jobs, favorites, nonblank notes and removed records are protected. Currency
filters use bound parameters and require a known matching currency even if unknown
pay is allowed.

Saved-job lists now filter, sort, count and paginate in the repository. All values
in text, eligibility, experience and geographic predicates are bound parameters;
search treats percent/underscore literally and includes saved notes. Native
compensation predicates use the shared Rust range model. Missing bounds are open,
new-graduate aliases agree, explicit workplace data controls remote matching, and
country/region restrictions must match the same parsed location. Page ties sort
by ID. A 2,105-record regression verifies search/counts beyond the old client cap.

Migration 26 adds isolated `browser_sessions` metadata and per-source script
trust/hash columns. Full reset clears browser-session metadata and parent
workflows; jobs-only cleanup preserves them. Session credentials and browser
process management are deferred to Phase 6C. `browser_upgrade_tests.rs` tests the
populated v25 upgrade, all historical rows/checksums, backup, rollback and restart.
