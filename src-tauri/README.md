# `src-tauri` — Tauri Application Host & Native Backend

## Overview

`src-tauri` contains the desktop application host built on Tauri v2 and Rust. It acts as the orchestrator for Scrape, managing native OS interactions, the local SQLite database, system tray integration, scheduled background crawls, and the sidecar worker lifecycle.

---

## Architecture & Responsibilities

1. **Native Window & Tray Shell**: Initializes the application window with custom brutalist styling, preserves window state, and hosts the persistent system tray menu (`src-tauri/src/services/tray.rs`).
2. **Local-First SQLite Database**: Runs embedded migrations and exposes a high-performance SQLite connection pool via `sqlx` (`src-tauri/src/db/`).
3. **IPC Bridge**: Exposes strongly-typed Tauri commands (`src-tauri/src/commands/`) invoked from the React frontend via Tauri IPC; persisted-job logo lookups derive employer, source and posting URL from the database job ID and save against that exact relationship.
4. **Worker Orchestration**: Spawns and manages the `scrape-worker` sidecar process, validating NDJSON protocol v14 output and candidate decisions, acknowledging committed job outcomes, persisting jobs/branding/outcomes into SQLite, and broadcasting progress to the webview (`src-tauri/src/services/runner.rs`).
5. **Background Scheduler & Retention**: Evaluates per-workflow IANA calendar schedules (`src-tauri/src/services/scheduler.rs`, `calendar.rs`) under the global automatic-scraping pause, and runs automatic 24-hour safe data retention (`src-tauri/src/services/retention.rs`).

```
┌────────────────────────────────────────────────────────┐
│               React Frontend (WebView)                 │
└──────────────────────────┬─────────────────────────────┘
                           │ Tauri IPC (invoke / listen)
                           ▼
┌────────────────────────────────────────────────────────┐
│                      src-tauri                         │
│  ├── commands/   (IPC Handler Endpoints)               │
│  ├── services/   (Workflow, Runner, Scheduler, ..)     │
│  └── db/         (SQLite Pool, Migrations, Repositories)│
└─────────────┬────────────────────────────┬─────────────┘
              │ Spawns Sidecar             │ Queries / Updates
              ▼                            ▼
┌───────────────────────────┐  ┌─────────────────────────┐
│     binaries/             │  │   SQLite Database       │
│  scrape-worker (Sidecar)  │  │   (Local Application)   │
└───────────────────────────┘  └─────────────────────────┘
```

---

## Directory Structure

```
src-tauri/
├── Cargo.toml              # Rust package manifest with Tauri v2 and sqlx dependencies
├── build.rs                # Build script compiling Tauri attributes and resources
├── tauri.conf.json         # Tauri v2 bundle, window, security, and plugin configuration
├── binaries/               # Bundled architecture-specific sidecar binaries
├── capabilities/           # Tauri v2 security ACL and window permission rules
├── examples/               # Standalone database repair utility (repair_database.rs)
├── gen/                    # Auto-generated Tauri schema configurations
├── historical-migrations/  # Immutable historical SQL variants for upgrade verification
├── icons/                  # Application and tray icons (PNG, ICNS, ICO)
├── migrations/             # Forward-only embedded SQL migrations (0001–0032)
├── src/                    # Rust backend source code
│   ├── commands/           # Tauri IPC commands callable from TypeScript
│   ├── db/                 # SQLite database queries and repositories
│   └── services/           # Workflow validation, worker runner, scheduler, retention, startup
└── tests/                  # Real packaged worker → temporary SQLite integration tests
```

---

## File Manifest

| File | Purpose |
|---|---|
| [`Cargo.toml`](Cargo.toml) | Declares backend dependencies: `tauri`, `tauri-plugin-opener`, `tauri-plugin-notification`, `sqlx` (SQLite), `tokio`, `scrape-core`, `serde`, and `chrono`. |
| [`build.rs`](build.rs) | Build automation script invoking `tauri_build::build()`. |
| [`tauri.conf.json`](tauri.conf.json) | Central Tauri v2 configuration file defining bundle identity, window dimensions, system tray behavior, plugin permissions, and sidecar definitions. |
| [`examples/repair_database.rs`](examples/repair_database.rs) | CLI utility for inspecting and safely repairing recognized historical SQLite databases. |
| [`tests/worker_repository.rs`](tests/worker_repository.rs) | End-to-end integration tests driving the actual packaged `scrape-worker` binary against local HTTP fixtures into temporary SQLite databases. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`binaries/`](binaries/README.md) | Compiled target binaries for the `scrape-worker` sidecar. |
| [`capabilities/`](capabilities/README.md) | Tauri v2 permission declarations and security capabilities. |
| [`gen/`](gen/README.md) | Auto-generated configuration schemas. |
| [`historical-migrations/`](migrations/README.md) | Preserved original migration SQL variants used to verify populated historical upgrades. |
| [`icons/`](icons/README.md) | Multi-resolution platform application and tray icon assets. |
| [`migrations/`](migrations/README.md) | Versioned forward-only SQL migrations establishing the local SQLite database schema. |
| [`src/`](src/README.md) | Rust application entry points, state containers, error types, commands, and services. |

Phase 6B–6C uses WorkflowSpec v6 and migration 26. The host exposes `import_workflow`,
`trust_workflow_script`, `detect_browser_runtime`, `list_browser_sessions`,
`open_browser_session`, `save_browser_session`, `clear_browser_session`, and
`test_extractor`. It stores executable trust separately from exported configuration,
persists domain-scoped browser session cookies in `<app_data_dir>/browser_sessions/<session_id>.json`
with `0700` directory and `0600` file permissions on Unix, passes `SCRAPE_SESSION_DIR`
to the worker sidecar, reconciles `CoverageTermination::NeedsLogin` scan outcomes back
to `browser_sessions`, removes session files on full data reset, and rejects unavailable
custom script execution explicitly until Phase 6D.

Workflow Preview modifies only a transient worker configuration: public-HTML
sources receive a six-navigation cap, while the existing 20-second source and
45-second workflow deadlines remain hard bounds. Saved page limits and normal
execution are unchanged. Preview diagnostics report page/request/time coverage
as sampled and distinguish source-duration from navigation timeouts; zero
unresolved candidates do not produce an eligibility-incomplete error.
