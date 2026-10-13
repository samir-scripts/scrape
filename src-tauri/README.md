# `src-tauri` — Tauri Application Host & Native Backend

## Overview

`src-tauri` contains the desktop application host built on Tauri v2 and Rust. It acts as the orchestrator for Scrape, managing native OS interactions, the local SQLite database, system tray integration, scheduled background crawls, and the sidecar worker lifecycle.

---

## Architecture & Responsibilities

1. **Native Window & Tray Shell**: Initializes the application window with custom brutalist styling, preserves window state, and hosts the persistent system tray menu (`src-tauri/src/services/tray.rs`).
2. **Local-First SQLite Database**: Runs embedded migrations and exposes a high-performance SQLite connection pool via `sqlx` (`src-tauri/src/db/`).
3. **IPC Bridge**: Exposes strongly-typed Tauri commands (`src-tauri/src/commands/`) invoked from the React frontend via Tauri's IPC protocol.
4. **Worker Orchestration**: Spawns and manages the `scrape-worker` sidecar process, piping events into SQLite and broadcasting progress to the webview (`src-tauri/src/services/runner.rs`).
5. **Background Scheduler**: Maintains automated crawling intervals according to user-configured source cadences and global cron schedules (`src-tauri/src/services/scheduler.rs`).

```
┌────────────────────────────────────────────────────────┐
│               React Frontend (WebView)                 │
└──────────────────────────┬─────────────────────────────┘
                           │ Tauri IPC (invoke / listen)
                           ▼
┌────────────────────────────────────────────────────────┐
│                      src-tauri                         │
│  ├── commands/   (IPC Handler Endpoints)               │
│  ├── services/   (Runner, Scheduler, Tray, System)     │
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
├── Cargo.toml          # Rust package manifest with Tauri v2 and sqlx dependencies
├── build.rs            # Build script compiling Tauri attributes and resources
├── tauri.conf.json     # Tauri v2 bundle, window, security, and plugin configuration
├── binaries/           # Bundled architecture-specific sidecar binaries
├── capabilities/       # Tauri v2 security ACL and window permission rules
├── gen/                # Auto-generated Tauri schema configurations
├── icons/              # Application and tray icons (PNG, ICNS, ICO)
├── migrations/         # Embedded SQL migrations executed via sqlx
└── src/                # Rust backend source code
    ├── commands/       # Tauri IPC commands callable from TypeScript
    ├── db/             # SQLite database queries and repositories
    └── services/       # Background workers, scheduler, and OS integrations
```

---

## File Manifest

| File | Purpose |
|---|---|
| [`Cargo.toml`](Cargo.toml) | Declares backend dependencies: `tauri`, `tauri-plugin-opener`, `tauri-plugin-notification`, `sqlx` (SQLite), `tokio`, `scrape-core`, `serde`, and `chrono`. |
| [`build.rs`](build.rs) | Build automation script invoking `tauri_build::build()`. |
| [`tauri.conf.json`](tauri.conf.json) | Central Tauri v2 configuration file defining bundle identity, window dimensions, system tray behavior, plugin permissions, and sidecar definitions. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`binaries/`](binaries/README.md) | Compiled target binaries for the `scrape-worker` sidecar. |
| [`capabilities/`](capabilities/README.md) | Tauri v2 permission declarations and security capabilities. |
| [`gen/`](gen/README.md) | Auto-generated configuration schemas. |
| [`icons/`](icons/README.md) | Multi-resolution platform application and tray icon assets. |
| [`migrations/`](migrations/README.md) | Versioned SQL migrations establishing the local SQLite database schema. |
| [`src/`](src/README.md) | Rust application entry points, state containers, error types, commands, and services. |
