# SCRAPE — Local-First Job Scraper & Application Tracker

Scrape is a high-performance, local-first desktop application that runs in the background/system tray, crawls job boards and careers pages on customizable schedules, extracts structured compensation and location data, and tracks your job applications in a brutalist, NieR:Automata-inspired interface.

**Stack:** Tauri v2 · Rust (Tokio, Reqwest, Scraper, SQLx) · SQLite (WAL mode) · React 18 · TypeScript · Tailwind CSS v4 · Visx

---

## Architecture Overview

Scrape uses a multi-tiered architecture that separates the native desktop shell and database from an isolated web scraper worker sidecar and a high-performance webview frontend:

```
┌────────────────────────────────────────────────────────┐
│               React Frontend (WebView)                 │
│         src/ (Components, Hooks, Lib, Styles)          │
└──────────────────────────┬─────────────────────────────┘
                           │ Tauri IPC (invoke / listen)
                           ▼
┌────────────────────────────────────────────────────────┐
│                Tauri Host Process                      │
│        src-tauri/ (Commands, Services, SQLite DB)      │
└─────────────┬────────────────────────────┬─────────────┘
              │ Spawns & configures via    │ Local SQLite
              │ stdin / stdout (NDJSON)    │ (Embedded SQLx)
              ▼                            ▼
┌───────────────────────────┐  ┌─────────────────────────┐
│   Worker Sidecar Binary   │  │    Local SQLite DB      │
│   crates/scrape-worker    │  │ (jobs, sources, runs,   │
│ (HTTP Client, ATS Parsers)│  │  notifications, tombst) │
└─────────────┬─────────────┘  └─────────────────────────┘
              │ Depends on
              ▼
┌───────────────────────────┐
│     Shared Core Domain    │
│     crates/scrape-core    │
│(Types, Protocol, Enrich)  │
└───────────────────────────┘
```

---

## Codebase Directory Map

Each directory in this repository contains its own dedicated `README.md` documenting its specific purpose, internal file manifest, and relationships to the broader project:

### 1. Rust Workspace Crates (`crates/`)
- [`crates/`](crates/README.md) — Rust workspace overview and sidecar architecture.
  - [`crates/scrape-core/`](crates/scrape-core/README.md) — Shared domain models, IPC wire protocols, and TypeScript type binding generators.
    - [`crates/scrape-core/src/`](crates/scrape-core/src/README.md) — Core enums, role classification heuristics, and SQL filter engine.
    - [`crates/scrape-core/src/enrich/`](crates/scrape-core/src/enrich/README.md) — Information extraction heuristics (salary, location, workplace arrangement, eligibility, experience).
  - [`crates/scrape-worker/`](crates/scrape-worker/README.md) — Scraper sidecar binary and crawl execution engine.
    - [`crates/scrape-worker/src/`](crates/scrape-worker/src/README.md) — Worker CLI entry point, rate-limited HTTP client, robots.txt validator, and ATS detector.
    - [`crates/scrape-worker/src/adapters/`](crates/scrape-worker/src/adapters/README.md) — Target adapters (Greenhouse, Lever, Ashby, JSON-LD, CSS Selectors).
    - [`crates/scrape-worker/tests/`](crates/scrape-worker/tests/README.md) — Offline integration test harness.
    - [`crates/scrape-worker/tests/fixtures/`](crates/scrape-worker/tests/fixtures/README.md) — Mock ATS payloads and HTML fixtures.

### 2. Tauri Application Host (`src-tauri/`)
- [`src-tauri/`](src-tauri/README.md) — Tauri v2 desktop application host and native backend.
  - [`src-tauri/binaries/`](src-tauri/binaries/README.md) — Compiled architecture-specific sidecar binaries (`scrape-worker`).
  - [`src-tauri/capabilities/`](src-tauri/capabilities/README.md) — Tauri v2 security ACL and window capability permissions.
  - [`src-tauri/gen/`](src-tauri/gen/README.md) — Auto-generated configuration schemas.
    - [`src-tauri/gen/schemas/`](src-tauri/gen/schemas/README.md) — JSON schemas for capabilities and desktop configuration.
  - [`src-tauri/icons/`](src-tauri/icons/README.md) — Multi-platform application and system tray icons.
  - [`src-tauri/migrations/`](src-tauri/migrations/README.md) — Versioned SQLite migrations executed via SQLx.
  - [`src-tauri/src/`](src-tauri/src/README.md) — Application initialization, state management, and error handling.
    - [`src-tauri/src/commands/`](src-tauri/src/commands/README.md) — Tauri IPC command endpoints exposed to the frontend.
    - [`src-tauri/src/db/`](src-tauri/src/db/README.md) — SQLite database repositories and query implementations.
    - [`src-tauri/src/services/`](src-tauri/src/services/README.md) — Background services: ScrapeRunner, Scheduler, Tray, and OS notifications.

### 3. Specifications, Plans & Automation
- [`plans/`](plans/README.md) — System specifications, architectural designs, and phased blueprints.
  - [`plans/assets/`](plans/assets/README.md) — Brutalist UI palette references, dithering samples, and NieR design guides.
  - [`plans/errors/`](plans/errors/README.md) — Technical troubleshooting playbooks and cross-platform problem resolutions.
- [`scripts/`](scripts/README.md) — Node.js automation scripts (sidecar compiler, app startup smoke test).
- [`workflows/`](workflows/README.md) — Declarative YAML workflow specifications for automated crawling.

### 4. React Frontend (`src/`)
- [`src/`](src/README.md) — React 18 frontend root, application bootstrap, and global hotkey routing.
  - [`src/styles/`](src/styles/README.md) — Tailwind CSS v4 styling rules, custom typography, and NieR brutalist color variables.
  - [`src/hooks/`](src/hooks/README.md) — Custom hooks for keyboard navigation (`useHotkeys`) and debouncing (`useDebounce`).
  - [`src/lib/`](src/lib/README.md) — IPC client, Zustand store, client-side filtering algorithms, and formatters.
  - [`src/test/`](src/test/README.md) — Vitest test suites covering state, UI components, and filtering math.
  - [`src/bindings/`](src/bindings/README.md) — Auto-generated TypeScript types matching Rust domain models (`ts-rs`).
    - [`src/bindings/serde_json/`](src/bindings/serde_json/README.md) — TypeScript type definitions for arbitrary JSON values.
  - [`src/components/`](src/components/README.md) — Component architecture and design system guidelines.
    - [`src/components/ui/`](src/components/ui/README.md) — Reusable brutalist primitives (buttons, inputs, modals, tags, clock picker).
    - [`src/components/layout/`](src/components/layout/README.md) — Application shell, top navigation, split panes, and footer bar.
    - [`src/components/filters/`](src/components/filters/README.md) — Multi-faceted filter controls (role, salary, location, eligibility).
    - [`src/components/notifications/`](src/components/notifications/README.md) — Notification bell and floating drawer popover.
    - [`src/components/views/`](src/components/views/README.md) — Primary application screens.
      - [`src/components/views/discovered/`](src/components/views/discovered/README.md) — Discovered job feed, card view, and detail panel.
      - [`src/components/views/tracked/`](src/components/views/tracked/README.md) — Job pipeline Kanban board and spreadsheet list view.
      - [`src/components/views/scraper/`](src/components/views/scraper/README.md) — Crawler control room, source editor, and live log stream.
      - [`src/components/views/metrics/`](src/components/views/metrics/README.md) — Analytics dashboard with Visx velocity and salary charts.
      - [`src/components/views/settings/`](src/components/views/settings/README.md) — User settings, schedule configuration, and backups.

---

## Root File Manifest

| File | Type / Format | Purpose & Responsibilities |
|---|---|---|
| [`Cargo.toml`](Cargo.toml) | Cargo Manifest | Defines the root Cargo workspace containing `crates/scrape-core`, `crates/scrape-worker`, and `src-tauri`. Pins shared dependencies (`serde`, `tokio`, `chrono`, `regex`, `thiserror`). |
| [`Cargo.lock`](Cargo.lock) | Lockfile | Deterministic dependency version lockfile for all Rust workspace crates. |
| [`package.json`](package.json) | npm Manifest | Frontend and Tauri script definitions (`dev`, `build`, `test`, `tauri`). Declares frontend dependencies (`react`, `tailwindcss`, `@tanstack/react-query`, `zustand`, `@visx/*`, `yaml`, `@tauri-apps/api`). |
| [`package-lock.json`](package-lock.json) | Lockfile | Deterministic dependency version lockfile for Node.js modules. |
| [`tsconfig.json`](tsconfig.json) | TypeScript Config | TypeScript compiler configuration targeting ES2022 with DOM types, React JSX transform, strict type-checking, and path aliasing. |
| [`tsconfig.node.json`](tsconfig.node.json) | TypeScript Config | TypeScript compiler configuration for Vite configuration and Node scripts. |
| [`vite.config.ts`](vite.config.ts) | Vite Config | Vite build configuration: sets local development port (`1420`), configures Tailwind CSS integration, sets up Vitest environment, and configures Tauri dev server hooks. |
| [`index.html`](index.html) | HTML Entry | Single-page application HTML shell mounting `#root` and loading Google Fonts (JetBrains Mono, IBM Plex Sans). |
| [`rust-toolchain.toml`](rust-toolchain.toml) | Toolchain Config | Pins the Rust stable toolchain version across development and CI environments. |
| [`.gitignore`](.gitignore) | Git Ignore | Specifies ignored patterns (build outputs `target/`, `dist/`, dependencies `node_modules/`, local SQLite databases `*.db`, OS metadata). |

---

## Development & Verification

### Prerequisites
- **Node.js**: v18+ (LTS recommended)
- **Rust**: 1.78+ (stable)
- **System Dependencies**: [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

### Common Commands

```bash
# Install frontend dependencies
npm install

# Run application in development mode (spawns Tauri window + Vite dev server)
npm run tauri dev

# Run all automated tests (Rust workspace + Vitest frontend)
cargo test --workspace
npm test

# Run code linters and type checkers
cargo clippy --workspace --all-targets
npx tsc --noEmit

# Compile production frontend bundle
npm run build
```
