# `src-tauri/src/commands` — Tauri IPC Command Handlers

## Overview

The `src-tauri/src/commands` directory defines the backend functions exposed to the webview frontend via Tauri's IPC system (`invoke("command_name", { ... })`). Each module handles payload deserialization, validates input parameters, queries the SQLite repository or orchestrates services, and returns strongly-typed results.

---

## File Manifest

| File | Command Domain | Key Exported Commands | Description |
|---|---|---|---|
| [`mod.rs`](mod.rs) | Module root | Re-exports | Consolidates command handlers for registration in `src-tauri/src/lib.rs`. |
| [`jobs.rs`](jobs.rs) | Job Management | `get_jobs`, `get_job`, `update_job_status`, `toggle_favorite`, `delete_job`, `get_filter_options` | Provides querying, filtering, full-text searching, pagination, stage updates (Discovered, Applied, Interviewing, Offer, Rejected), favoriting, and tombstone deletion. |
| [`sources.rs`](sources.rs) | Source Configuration | `get_sources`, `create_source`, `update_source`, `delete_source`, `test_source`, `detect_source_ats` | Manages target job boards, custom cadence configurations, live dry-run scrape tests, and ATS auto-detection probes. |
| [`scrape.rs`](scrape.rs) | Scrape Execution | `start_scrape`, `stop_scrape`, `get_active_runs`, `get_scrape_runs`, `get_scrape_run_details` | Triggers manual scrapes across all sources or individual targets, monitors running sidecar jobs, aborts runs, and retrieves run log histories. |
| [`metrics.rs`](metrics.rs) | Application Analytics | `get_metrics`, `get_application_funnel`, `get_salary_distribution`, `get_velocity_series` | Aggregates application metrics, conversion funnels, salary percentiles, and application velocity for the Metrics view. |
| [`settings.rs`](settings.rs) | Application Settings | `get_settings`, `update_settings`, `export_database`, `import_database` | Reads and updates user preferences (appearance, cron schedules, filters, retention policies, font selections) and manages JSON database backups. |
| [`notifications.rs`](notifications.rs) | In-App Alerts | `get_notifications`, `mark_notification_read`, `mark_all_notifications_read`, `clear_notifications` | Fetches, updates, and dismisses desktop/in-app notifications for scrape runs, crawl errors, and newly discovered opportunities. |
