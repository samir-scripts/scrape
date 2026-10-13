# `src-tauri/src/db` — SQLite Database Repositories

## Overview

The `src-tauri/src/db` directory encapsulates all data access logic, repository methods, and raw SQL queries for the local SQLite database. It leverages `sqlx` with compile-time checked queries and asynchronous connection pooling.

---

## File Manifest

| File | Target Table(s) | Key Functions & Operations |
|---|---|---|
| [`mod.rs`](mod.rs) | Database Connection Pool | `init_pool(db_path) -> Result<SqlitePool, ...>`: Configures SQLite pragmas (`PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`, `PRAGMA foreign_keys = ON`), establishes the connection pool, and executes schema migrations from `src-tauri/migrations/`. |
| [`jobs.rs`](jobs.rs) | `jobs`, `deleted_jobs` | `list_jobs`, `get_job`, `upsert_scraped_job`, `update_status`, `toggle_favorite`, `delete_job`. Handles dynamic filtering, text search, and pagination. When jobs are scraped, checks `deleted_jobs` tombstones to prevent resurrecting user-deleted jobs. When a job is deleted, atomically records a tombstone and removes the job record. |
| [`sources.rs`](sources.rs) | `sources` | `list_sources`, `get_source`, `create_source`, `update_source`, `delete_source`, `record_source_run`. Manages custom scraping targets, enabled flags, schedule cadences, and last scraped timestamps. |
| [`runs.rs`](runs.rs) | `scrape_runs` | `create_run`, `complete_run`, `list_recent_runs`, `get_run_details`. Records execution telemetry, duration, items discovered, error logs, and trigger types (manual vs. scheduled). |
| [`metrics.rs`](metrics.rs) | `jobs` | `compute_overview_metrics`, `get_daily_application_velocity`, `get_salary_distribution_buckets`. Executes aggregation queries calculating application funnel conversion rates, status distributions, and salary quartiles. |
| [`settings.rs`](settings.rs) | `settings` | `get_settings`, `save_settings`. Serializes and retrieves global user preferences, crawl cadences, retention parameters, and filter presets. |
| [`notifications.rs`](notifications.rs) | `notifications` | `insert_notification`, `list_notifications`, `mark_read`, `mark_all_read`, `clear_all`. Manages the in-app notification ledger, tracking crawl completions, scrape failures, and alerts. |

Migration startup now refuses checksum mismatches instead of rewriting history;
pending disk upgrades receive a consistent SQLite snapshot. See
[database recovery](../../../plans/database-recovery.md). Both retention policies
protect tracked jobs, favorites, and nonblank notes. Currency filters use bound
parameters and require a known matching currency even if unknown pay is allowed.

Saved-job lists now filter, sort, count and paginate in the repository. All values
in text, eligibility, experience and geographic predicates are bound parameters;
search treats percent/underscore literally and includes saved notes. Native
compensation predicates use the shared Rust range model. Missing bounds are open,
new-graduate aliases agree, explicit workplace data controls remote matching, and
country/region restrictions must match the same parsed location. Page ties sort
by ID. A 2,105-record regression verifies search/counts beyond the old client cap.
