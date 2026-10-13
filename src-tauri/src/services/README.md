# `src-tauri/src/services` — Background Services & System Integrations

## Overview

The `src-tauri/src/services` directory contains long-running background tasks, process managers, scheduling daemons, and operating system integration services.

---

## File Manifest

| File | Service Domain | Core Responsibilities & Implementation |
|---|---|---|
| [`mod.rs`](mod.rs) | Service Root | Consolidates service modules and exports public service constructs. |
| [`runner.rs`](runner.rs) | Worker Process Orchestration | `ScrapeRunner`: Manages the lifecycle of the `scrape-worker` sidecar. Prepares `WorkerConfig`, spawns the child process, streams line-delimited JSON events from `stdout`, verifies against deletion tombstones, writes jobs into SQLite via `db::jobs::upsert_scraped_job`, and broadcasts progress events (`scrape-event`) to the webview. |
| [`scheduler.rs`](scheduler.rs) | Automated Crawl Scheduling | `SchedulerService`: Continuous background task that evaluates cron schedules and per-source cadences (`hourly`, `six_hours`, `twelve_hours`, `daily`). Automatically triggers `ScrapeRunner` passes when targets become due. |
| [`tray.rs`](tray.rs) | System Tray Integration | Manages the native OS menu bar / taskbar tray icon. Builds context menus (Open Scrape, Run Scrape Now, Preferences, Quit) and toggles window visibility when clicked. |
| [`system.rs`](system.rs) | OS Interop & Notifications | Manages desktop system notifications via `tauri-plugin-notification` and coordinates external URL navigation via `tauri-plugin-opener`. |

---

## Service Lifecycle & Architecture

```
Application Boot (lib.rs)
        │
        ├──► SchedulerService::start()
        │         │ (Periodically checks due cadences)
        │         ▼
        │    ScrapeRunner::run_sources()
        │         │
        │         ├──► Spawns scrape-worker Sidecar
        │         ├──► Reads NDJSON Events (WorkerEvent)
        │         ├──► Upserts into SQLite Database
        │         └──► Emits Tauri Window Events
        │
        └──► TrayService::init() (Maintains background presence)
```

Workflow filters now resolve absent fields from global defaults, while explicit
empty collections remove restrictions. Invalid/unsupported values are errors.
Run all sends effective filters per source; preview accepts the same draft filters.
All automatic scheduler paths share a pause flag, including delayed startup and
per-source dispatch. Due-date comparisons use parsed instants. Per-workflow IANA
calendar scheduling, DST policy, and the unified dispatcher remain pending.

Calendar scheduling and a single dispatcher are now implemented. Daily/weekly
sources can carry generated `CalendarSchedule` settings in their configuration:
local time, ISO weekdays and IANA timezone. Creation, edits and post-run updates
use the same calculation. Gaps run at the first valid instant; overlaps use their
first occurrence once. UTC instants are stored consistently. One queue coalesces
due workflows and waits through manual-run overlap; missed occurrences become one
catch-up run before the next future date. All automatic paths share persisted
pause, and actual schedule/startup triggers are recorded for per-workflow runs.

`PostingRules` is shared by worker evaluation and repository query compilation.
It supports any/all/exclude keywords by explicit field, native compensation,
experience, role/workplace, eligibility and geography. Posting-date rules use one
captured run-start instant and explicit IANA timezone; date-only upper bounds end
at the following midnight. Missing dates are never replaced with discovery/update
time. The repository caches normalized date evidence without changing originals.
Worker protocol 2 prevents an older sidecar from silently ignoring these rules.
