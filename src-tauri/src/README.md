# `src-tauri/src` — Tauri Application Backend Core

## Overview

The `src-tauri/src` directory contains the core Rust code for the desktop application, including application lifecycle setup, state management, error handling, IPC commands, and background services.

---

## File Manifest

| File | Responsibilities | Key Functions & Structures |
|---|---|---|
| [`main.rs`](main.rs) | Executable entry point | Launches the desktop process by invoking `scrape_lib::run()`. |
| [`lib.rs`](lib.rs) | Application initialization & builder | Configures the Tauri v2 builder runtime: registers plugins (`tauri-plugin-opener`, `tauri-plugin-notification`), resolves the local SQLite application path, executes database migrations, mounts the system tray, starts the background scheduler service, and registers all Tauri IPC command handlers. |
| [`state.rs`](state.rs) | Global application state container | Declares `AppState` struct managed by Tauri. Holds the SQLite connection pool (`SqlitePool`), active background runner handles, and cancellation channels. |
| [`error.rs`](error.rs) | Centralized error taxonomy | Defines `AppError` using `thiserror`. Implements `serde::Serialize` so database errors, IO failures, scheduler exceptions, and worker errors serialize into structured JSON payloads over the Tauri IPC bridge to the frontend. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`commands/`](commands/README.md) | Tauri IPC command handlers exposed directly to the TypeScript frontend. |
| [`db/`](db/README.md) | SQLite database connection pooling, query logic, and repository access layers. |
| [`services/`](services/README.md) | Background workers: sidecar process execution, automated cron scheduling, system tray management, and OS integrations. |
