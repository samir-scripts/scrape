# `src-tauri/src/db` — SQLite Database Repositories

## Overview

The `src-tauri/src/db` directory encapsulates all data access logic, repository methods, and raw SQL queries for the local SQLite database. It leverages `sqlx` with compile-time checked queries and asynchronous connection pooling.

---

## File Manifest

| File | Target Table(s) | Key Functions & Operations |
|---|---|---|
| [`mod.rs`](mod.rs) | Database Connection Pool & Startup | Configures SQLite pragmas (`WAL`, `synchronous = NORMAL`, `foreign_keys = ON`), validates historical migration prefixes before writable changes, takes `VACUUM INTO` backups, executes forward-only migrations, and seeds the role-title catalog after migration when the table is empty. |
| [`jobs.rs`](jobs.rs) | `jobs`, `job_locations`, `job_status_events` | `JobsRepo`: SQLite-side filtering, sorting, counting, and pagination; upserting scraped postings while preserving user notes, favorites, and tracked status; soft removal (`remove_job`), restoration (`restore_posting`), and cross-workflow blocking via `postings`. |
| [`postings.rs`](postings.rs) | `postings`, `deleted_jobs` | Canonical posting identity registry (`PostingIdentity`), cross-workflow discovery suppression (`blocked`), and legacy tombstone reconciliation. |
| [`branding.rs`](branding.rs) | `employer_brandings`, `employer_branding_assets` | Stores verified employer logo evidence and one deduplicated asset per source/employer relationship, retaining verified artwork across temporary refresh failures and source deletion. The read model hides only the exactly identified legacy Ashby platform favicon without mutating storage. |
| [`cleanup.rs`](cleanup.rs) | All application tables | `preview` and `execute` for confirmed Data Management operations (`DeleteAll`, `KeepFavorites`, `KeepTracked`, `ResetAll`) with `VACUUM INTO` recovery snapshots and posting-identity protection. |
| [`compensation.rs`](compensation.rs) | `job_compensations` | Persists and queries native compensation ranges (hundredths of currency units, interval, currency, source evidence) separately from annual compatibility analytics. |
| [`history.rs`](history.rs) | `_sqlx_migrations` | Read-only historical migration checksum and schema verification against recognized migration histories before applying new upgrades. |
| [`sources.rs`](sources.rs) | `sources` | `SourcesRepo`: Manages workflow/source records, enabled flags, schedule cadences, review status (`needs_review`), script trust/fingerprints, and last-scraped timestamps. |
| [`runs.rs`](runs.rs) | `scrape_runs`, `scrape_logs` | `RunsRepo`: Records execution telemetry, `ScanOutcome`, `RunCounts` (including per-source `SourceScanStats` summaries), duration, log entries, and trigger types. |
| [`metrics.rs`](metrics.rs) | `jobs`, `job_status_events` | `MetricsRepo`: Aggregation queries calculating stage totals, conversion rates, role distributions, salary distribution buckets, streaks, and daily velocity. |
| [`settings.rs`](settings.rs) | `settings` | `SettingsRepo`: Serializes and retrieves user preferences, appearance settings, and automatic-scraping pause state. |
| [`role_titles.rs`](role_titles.rs) | `role_titles`, `role_title_terms` | Embeds and validates the normalized TSV; performs idempotent transactional batch seeding and indexed token-prefix search for workflow title suggestions. |
| [`companies.rs`](companies.rs) | `jobs`, generated company catalog | Bounded, case-insensitive employer-prefix suggestions merged with verified catalog presets; saved-posting counts and custom literals remain separate from board attachment. |
| [`company_search_tests.rs`](company_search_tests.rs) | Temporary SQLite databases | Verifies local/template suggestion merging, ranking, wildcard escaping, result bounds, indexed lookup, and populated v31→v32 preservation. |
| [`notifications.rs`](notifications.rs) | `notifications` | `NotificationsRepo`: Manages the in-app notification ledger for crawl completions, warnings/errors, and alerts. |

Role search returns at most 50 suggestions with title, SOC code and tier. Exact
titles rank ahead of full-title prefixes, then other token-prefix matches, with
tier/title tie-breaks. Queries are limited to 256 UTF-8 bytes and 16 unique words.
Empty/punctuation-only input returns a bounded tier-ordered set. Token lookups use
the term-first SQLite index; arbitrary within-word substring search is not offered
and no leading-wildcard scan is claimed. The async picker preserves server ranking.

Company suggestions combine verified career-board templates with prefix matches
from locally saved job-employer names. Results are bounded to 250, returned in
50-item UI pages, ranked exact/prefix then by local usage. The SQLite lookup uses
a case-insensitive index. Unlisted company terms remain selectable as literal
filters and never imply a career-board URL. This is not a complete employer
directory.

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

Run diagnostics store credential-redacted effective configurations and sanitize
historical configurations on reads. Per-workflow preparation and preflight errors
are retained in `scrape_logs`, including contexts from Run all; persistence failure
logs and counters retain earlier committed batches. No schema migration is needed
for Plan 12 section 1. These snapshots describe attempted execution and are not
executable workflow exports.

Migration 27 scopes source endpoint uniqueness to the owning workflow and preserves
job/run links through table replacement. Workflow listings and due-source queries
select parent rows; direct `get` still retrieves children for exact local trust.
Filtering jobs by a parent source ID also includes its workflow-owned children.
The typed workflow service owns atomic parent/child writes; the low-level source
repository remains available for legacy single-source commands and isolated
historical fixtures. See database recovery for the explicit backed-up ownership
repair helper.
Cleanup previews count parent workflows; owned child rows are not additional
workflows. Child completion resolves retention policy through parent ownership.

`geography_parity_tests.rs` uses independent expected IDs against the worker rule
and real temporary SQLite queries. Country/region/city/raw predicates stay on one
job_locations tuple; no aggregate jobs.location city fallback remains. Missing-city
policy, raw-only evidence, literal wildcard characters, exact parsed-city comparison
and Unicode whitespace/spelling are explicit. No schema/data migration is needed.
