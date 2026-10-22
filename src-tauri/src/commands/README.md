# `src-tauri/src/commands` — Tauri IPC Command Handlers

## Overview

The `src-tauri/src/commands` directory defines the backend functions exposed to the webview frontend via Tauri's IPC system (`invoke("command_name", { ... })`). Each module handles payload deserialization, validates input parameters, queries the SQLite repository or orchestrates services, and returns strongly-typed results.

---

## File Manifest

| File | Command Domain | Key Exported Commands | Description |
|---|---|---|---|
| [`mod.rs`](mod.rs) | Module root | Re-exports | Consolidates command handlers for registration in `src-tauri/src/lib.rs`. |
| [`jobs.rs`](jobs.rs) | Job & Posting Management | `list_jobs`, `get_job`, `get_job_locations`, `update_job_status`, `bulk_update_job_status`, `update_job_notes`, `get_job_status_events`, `toggle_job_favorite`, `set_job_favorite`, `remove_job`, `restore_posting`, `set_posting_blocked`, `list_removed_postings`, `posting_saved_copies`, `get_company_logo_asset`, `preview_filter_impact`, `list_location_options`, `search_role_titles`, `cleanup_expired_jobs`, `reclassify_jobs`, `delete_job` | Provides SQLite-backed job querying and persistence plus indexed role-title suggestions from the embedded catalog. |
| [`cleanup.rs`](cleanup.rs) | Bulk Data Cleanup | `preview_data_cleanup`, `execute_data_cleanup` | Previews affected posting/note/history counts and executes confirmed bulk deletion or full application reset after taking a consistent SQLite recovery snapshot. |
| [`retention.rs`](retention.rs) | Retention Status | `get_retention_status` | Returns the last automatic cleanup result and consecutive failure status for Data management. |
| [`sources.rs`](sources.rs) | Source Configuration | `list_sources`, `get_source`, `create_source`, `update_source`, `delete_source`, `seed_sample_workflow`, `seed_all_sample_workflows` | Manages persisted workflow/source configurations and sample workflow seeding through the shared atomic `services/sample_workflows.rs` service; suite seeding returns generated `SeedWorkflowReport` counts and records. |
| [`scrape.rs`](scrape.rs) | Workflow & Scrape Execution | `run_scrape_now`, `run_source_now`, `cancel_scrape`, `list_scrape_runs`, `list_source_runs`, `get_scrape_run`, `get_scrape_logs`, `validate_cron`, `get_scheduler_status`, `set_scheduler_paused`, `detect_source`, `test_source`, `test_source_transient`, `validate_workflow`, `describe_workflow_filters`, `import_workflow`, `trust_workflow_script`, `preview_workflow`, `save_workflow`, `cancel_workflow_preview`, `open_terminal_console` | Validates and previews `WorkflowSpec` drafts, saves workflows, triggers manual/Run all crawls, cancels active runs/previews, manages scheduler pause state, and retrieves run/source execution histories and logs. |
| [`metrics.rs`](metrics.rs) | Application Analytics | `get_metrics` | Aggregates application metrics, stage totals, conversion funnels, role breakdowns, salary distribution buckets, streaks, and daily velocity series for the Metrics view. |
| [`settings.rs`](settings.rs) | Application Settings | `get_settings`, `update_setting` | Reads and updates application-wide user preferences (theme, global font, motion, dithering, date/time format, scrollbar mode, automatic-scraping pause). |
| [`notifications.rs`](notifications.rs) | In-App Alerts & Updater | `create_notification`, `list_notifications`, `mark_notification_read`, `mark_all_notifications_read`, `clear_all_notifications`, `count_unread_notifications`, `check_app_update`, `install_app_update` | Fetches, updates, and dismisses in-app notifications for scrape runs, warnings/errors, and newly discovered opportunities, plus update checks. |
