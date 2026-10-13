# `crates/` — Workspace Rust Libraries & Sidecars

## Overview

The `crates/` directory contains the foundational Rust workspace packages that power the scraping, data enrichment, and worker execution engine of the Scrape application. These crates are separated from the Tauri application host (`src-tauri/`) to maintain clean architectural boundaries, enable independent unit and integration testing, and allow the scraper worker to be compiled into a standalone, isolated sidecar binary.

```
crates/
├── scrape-core/      # Shared domain types, protocol definitions, and enrichment heuristics
└── scrape-worker/    # Standalone scraper worker binary and HTTP/ATS extraction engine
```

---

## Architecture & Project Role

In Scrape's local-first architecture:
1. **`scrape-core`** serves as the single source of truth for domain models across both Rust backends and the TypeScript frontend. It defines the job schema, status enums, filter algebra, and enrichment algorithms. TypeScript bindings in `src/bindings/` are generated directly from `scrape-core` types via `ts-rs`.
2. **`scrape-worker`** is compiled into a lightweight sidecar binary (`scrape-worker`). When a scrape run is scheduled or manually triggered, the Tauri host spawns `scrape-worker` as a child process. The worker communicates with the host over `stdin`/`stdout` using line-delimited JSON (NDJSON) defined in `scrape-core::protocol`.

```
┌──────────────────────────────────────────────┐
│                  src-tauri                   │
│   (Tauri Host, SQLite Database, Scheduler)   │
└──────────────┬───────────────────────────────┘
               │
               │ Spawns & configures via stdin (WorkerConfig)
               │ Receives streaming events via stdout (WorkerEvent NDJSON)
               ▼
┌──────────────────────────────────────────────┐
│             crates/scrape-worker             │
│   (Sidecar: HTTP Client, ATS Adapters, HTML) │
└──────────────┬───────────────────────────────┘
               │
               │ Depends on
               ▼
┌──────────────────────────────────────────────┐
│              crates/scrape-core              │
│ (Types, Protocol, Location/Salary/Enrichment)│
└──────────────────────────────────────────────┘
```

---

## Directory Manifest

| Directory | Crate Name | Type | Description |
|---|---|---|---|
| [`scrape-core/`](scrape-core/README.md) | `scrape-core` | Library | Core domain models, IPC protocol definitions, role inference, SQL filter parser, and extraction heuristics (location, salary, clearance, sponsorship, work authorization, experience, workplace type). |
| [`scrape-worker/`](scrape-worker/README.md) | `scrape-worker` | Binary & Library | Standalone executable sidecar that fetches, parses, and normalizes job listings from Greenhouse, Lever, Ashby, JSON-LD schemas, and custom CSS selectors. |

---

## Development & Testing

All crates participate in the root Cargo workspace:

```bash
# Check all workspace targets
cargo check --workspace --all-targets

# Run unit and integration tests across crates
cargo test --workspace

# Run linter
cargo clippy --workspace --all-targets
```
