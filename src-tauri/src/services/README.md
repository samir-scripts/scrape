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
