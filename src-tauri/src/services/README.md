# `src-tauri/src/services` — Background Services & System Integrations

## Overview

The `src-tauri/src/services` directory contains long-running background tasks, process managers, scheduling daemons, and operating system integration services.

---

## File Manifest

| File | Service Domain | Core Responsibilities & Implementation |
|---|---|---|
| [`mod.rs`](mod.rs) | Service Root | Consolidates service modules and exports public service constructs. |
| [`workflow.rs`](workflow.rs) | Authoritative Workflow Specification | Defines and validates `WorkflowSpec` (v5) with path-specific diagnostics, converts legacy v1–v4 configurations through a single compatibility resolver, and prepares worker execution inputs and previews. |
| [`runner.rs`](runner.rs) | Worker Process Orchestration | `ScrapeRunner`: Spawns the `scrape-worker` sidecar with `PROTOCOL_VERSION = 6`, streams and revalidates NDJSON events (`Job`, `Branding`, `Warning`, `SourceCompleted`, `Completed`), enforces write coordination, and broadcasts `scrape://event` updates. |
| [`ingestion.rs`](ingestion.rs) | Ingestion & Revalidation | Validates worker-emitted job records, source scan summaries, and employer branding reports before persisting them to SQLite. |
| [`scheduler.rs`](scheduler.rs) | Automated Crawl Dispatcher | `SchedulerService`: Unified background dispatcher that evaluates per-workflow schedules and startup triggers, coalesces due runs, respects the global automatic-scraping pause, and coordinates with manual runs. |
| [`calendar.rs`](calendar.rs) | IANA Calendar Scheduling | Computes next-run UTC instants for `CalendarSchedule` (local `HH:MM`, ISO weekdays, and IANA timezone), handling DST gaps and overlaps deterministically. |
| [`retention.rs`](retention.rs) | Automatic Data Retention | `RetentionService`: Runs after startup and every 24 elapsed hours, independent of scraping pause and workflow enablement. Shares the fixed-clock, protected-record predicate with post-run and explicit retention routes; defers without cancelling work when the shared writer is busy and records the last result. |
| [`process.rs`](process.rs) | Subprocess Lifecycle & Reaping | Structured argument handling, bounded/redacted `stderr` capture, cancellation, and process-tree termination/reaping for sidecar workers. |
| [`startup.rs`](startup.rs) | Safe Database Startup | Coordinates pre-migration read-only history checks, `VACUUM INTO` backups, and migration execution during host initialization. |
| [`tray.rs`](tray.rs) | System Tray Integration | Manages the native OS menu bar / taskbar tray icon, context menus, and window visibility toggling. |
| [`system.rs`](system.rs) | OS Interop & Notifications | Manages desktop system notifications via `tauri-plugin-notification` and coordinates external URL navigation via `tauri-plugin-opener`. |
| [`native_smoke.rs`](native_smoke.rs) | Debug-Only Native Test Harness | Provides isolated temporary-profile IPC, sidecar, window-resize, and startup smoke verification hooks (compiled/enabled only for explicit debug smoke runs, never affecting release builds or user data). |

---

## Service Lifecycle & Architecture

```
Application Boot (lib.rs)
        │
        ├──► startup.rs (Read-only history check, VACUUM INTO backup, migrations)
        ├──► SchedulerService::start()
        │         │ (Evaluates per-workflow CalendarSchedule & pause state)
        │         ▼
        │    ScrapeRunner
        │         │
        │         ├──► Spawns scrape-worker Sidecar (Protocol v6)
        │         ├──► Revalidates NDJSON Events (ingestion.rs)
        │         ├──► Upserts Jobs, Branding & Outcomes into SQLite
        │         └──► Emits Tauri Window Events
        ├──► RetentionService (Startup + 24h safe cleanup sweep)
        └──► TrayService::init() (Maintains background presence)
```

Each workflow owns an explicit `WorkflowSpec` v5 configuration (sources collection, matching rules, `CalendarSchedule`, `max_jobs_per_run`, and retention policy) without global inheritance. Legacy v1–v4 workflows and imports are converted once through `workflow.rs`; invalid stored specs remain preserved and marked `needs_review` so they cannot broaden matches or run automatically. Explicit empty collections remove restrictions.

Calendar scheduling (`calendar.rs`) computes next runs from local time, ISO weekdays, and IANA timezone across creation, edits, and post-run updates. DST gaps run at the first valid instant; overlaps use their first occurrence once. One dispatcher queue in `scheduler.rs` coalesces due workflows and waits through manual-run overlap; missed occurrences become one catch-up run before the next future date. All automatic paths share the persisted global pause flag, while explicit manual runs remain available.

`PostingRules` is shared by worker evaluation and repository query compilation. It supports Any/All/Exclude keywords by explicit field, partial-evidence evaluation for bounded detail enrichment, native compensation, experience, role/workplace, eligibility, and geography. Posting-date rules use one captured run-start instant and explicit IANA timezone; missing publication dates are never replaced with discovery or provider `updated_at` edit time. Worker protocol v6 enforces exact contract compatibility between the host and sidecar.
