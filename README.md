# SCRAPE — Local-First Job Scraper & Application Tracker

Scrape is a high-performance, local-first desktop application that runs in the background/system tray, crawls job boards and careers pages on customizable schedules, extracts structured compensation and location data, and tracks your job applications in a brutalist, NieR:Automata-inspired interface.

**Stack:** Tauri v2 · Rust (Tokio, Reqwest, Scraper, SQLx) · SQLite (WAL mode) · React 19 · TypeScript · Tailwind CSS v4 · Visx

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
│   crates/scrape-worker    │  │ (jobs, postings, runs,  │
│ (HTTP Client, ATS Parsers)│  │  sources, branding, ..) │
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
  - [`crates/scrape-core/`](crates/scrape-core/README.md) — Shared domain models, matching/filtering predicates, identity/geography/branding rules, IPC wire protocols (`PROTOCOL_VERSION` = 6), and TypeScript type binding generators.
    - [`crates/scrape-core/data/`](crates/scrape-core/data/README.md) — Pinned ISO country and subdivision catalog (`regions.json`).
    - [`crates/scrape-core/src/`](crates/scrape-core/src/README.md) — Core enums, categorized matching and rejection diagnostics, SQL filter compiler, canonical posting identity, geography, posting dates, and employer branding evidence.
    - [`crates/scrape-core/src/enrich/`](crates/scrape-core/src/enrich/README.md) — Information extraction heuristics (salary, location, workplace arrangement, eligibility, experience, HTML/description normalization).
  - [`crates/scrape-worker/`](crates/scrape-worker/README.md) — Scraper sidecar binary and crawl execution engine.
    - [`crates/scrape-worker/src/`](crates/scrape-worker/src/README.md) — Worker CLI entry point, rate-limited HTTP client, robots.txt validator, and ATS detector.
    - [`crates/scrape-worker/src/adapters/`](crates/scrape-worker/src/adapters/README.md) — Target adapters (Greenhouse, Lever, Ashby, JSON-LD, CSS Selectors, shared employer branding).
    - [`crates/scrape-worker/tests/`](crates/scrape-worker/tests/README.md) — Offline adapter and loopback-HTTP worker process integration test suites.
    - [`crates/scrape-worker/tests/fixtures/`](crates/scrape-worker/tests/fixtures/README.md) — Mock ATS payloads, HTML fixtures, and recorded branding assets.

### 2. Tauri Application Host (`src-tauri/`)
- [`src-tauri/`](src-tauri/README.md) — Tauri v2 desktop application host and native backend.
  - [`src-tauri/binaries/`](src-tauri/binaries/README.md) — Compiled architecture-specific sidecar binaries (`scrape-worker`).
  - [`src-tauri/capabilities/`](src-tauri/capabilities/README.md) — Tauri v2 security ACL and window capability permissions.
  - [`src-tauri/gen/`](src-tauri/gen/README.md) — Auto-generated configuration schemas.
    - [`src-tauri/gen/schemas/`](src-tauri/gen/schemas/README.md) — JSON schemas for capabilities and desktop configuration.
  - [`src-tauri/icons/`](src-tauri/icons/README.md) — Multi-platform application and system tray icons.
  - [`src-tauri/migrations/`](src-tauri/migrations/README.md) — Forward-only versioned SQLite migrations (0001–0024) executed via SQLx, paired with `src-tauri/historical-migrations/` for upgrade verification.
  - [`src-tauri/src/`](src-tauri/src/README.md) — Application initialization, state management, and error handling.
    - [`src-tauri/src/commands/`](src-tauri/src/commands/README.md) — Tauri IPC command endpoints exposed to the frontend.
    - [`src-tauri/src/db/`](src-tauri/src/db/README.md) — SQLite database repositories and query implementations (`jobs`, `postings`, `sources`, `runs`, `branding`, `cleanup`, `history`, `settings`, `metrics`, `notifications`).
    - [`src-tauri/src/services/`](src-tauri/src/services/README.md) — Background services: `workflow` (WorkflowSpec v5 validator/resolver), `runner`, `ingestion`, `scheduler`, `calendar`, `retention`, `process`, `startup`, `tray`, and `system`.

### 3. Specifications, Plans, Fixtures & Automation
- [`plans/`](plans/README.md) — System specifications, architectural designs, phased blueprints, and implementation evidence ledgers.
  - [`plans/assets/`](plans/assets/README.md) — Brutalist UI palette references, dithering samples, NieR design guides, and section review screenshots.
  - [`plans/errors/`](plans/errors/README.md) — Technical troubleshooting playbooks and cross-platform problem resolutions.
- [`fixtures/providers/`](fixtures/providers/README.md) — Recorded public provider captures with `manifest.json` provenance and deterministic regression fixtures.
- [`scripts/`](scripts/README.md) — Node.js automation scripts (sidecar compiler, style guard, Vite startup smoke, isolated native WKWebView/IPC harnesses, provider verification).
- [`workflows/`](workflows/README.md) — Sample declarative YAML workflow specifications.

### 4. React Frontend (`src/`)
- [`src/`](src/README.md) — React 19 frontend root, application bootstrap, repository pagination orchestration, and global hotkey routing.
  - [`src/styles/`](src/styles/README.md) — Tailwind CSS v4 styling rules, global typography tokens (`--font-ui`, `--font-code`), and NieR brutalist semantic theme variables (`paper` and `obsidian`).
  - [`src/hooks/`](src/hooks/README.md) — Custom hooks for workflow draft state (`useWorkflowDraft`), serialized job status transitions (`useJobStatusChange`), keyboard navigation (`useHotkeys`), and debouncing (`useDebounce`).
  - [`src/lib/`](src/lib/README.md) — IPC client, Zustand store, repository pagination contracts, local company logo cache, YAML/workflow import helpers, and formatters.
  - [`src/test/`](src/test/README.md) — Vitest + Testing Library test suites covering UI controls, state, and contracts.
  - [`src/bindings/`](src/bindings/README.md) — Auto-generated TypeScript types matching Rust domain models (`ts-rs`).
    - [`src/bindings/serde_json/`](src/bindings/serde_json/README.md) — TypeScript type definitions for arbitrary JSON values.
  - [`src/components/`](src/components/README.md) — Component architecture and design system guidelines.
    - [`src/components/ui/`](src/components/ui/README.md) — Reusable brutalist primitives (Button, IconButton, Icon, Select, ActionMenu, KeywordInput, CompanyLogo, Modal, RemoveJobButton, Tag, Toast, Tooltip, Dither, …).
    - [`src/components/layout/`](src/components/layout/README.md) — Application shell, top navigation, schedule control, resizable split panes, and footer bar.
    - [`src/components/filters/`](src/components/filters/README.md) — Consolidated **Filters & sort** panel and removable summary chips.
    - [`src/components/jobs/`](src/components/views/discovered/README.md) — Shared `JobInspector`, structured `JobDescriptionView`, and bottom `JobPaginator`.
    - [`src/components/notifications/`](src/components/notifications/README.md) — Notification bell and floating drawer popover.
    - [`src/components/views/`](src/components/views/README.md) — Primary application screens.
      - [`src/components/views/discovered/`](src/components/views/discovered/README.md) — Discovered job feed, card/table views, and shared detail inspector.
      - [`src/components/views/tracked/`](src/components/views/tracked/README.md) — Job pipeline Kanban board, 6-column default table list, and shared detail inspector.
      - [`src/components/views/scraper/`](src/components/views/scraper/README.md) — Workflow registry, manual/Run all execution controls, and run/source diagnostic logs.
      - [`src/components/views/metrics/`](src/components/views/metrics/README.md) — Analytics dashboard with Visx velocity and salary charts.
      - [`src/components/views/settings/`](src/components/views/settings/README.md) — Appearance, System, Removed & blocked jobs, Data management, and workflow editor components.

---

## Root File Manifest

| File | Type / Format | Purpose & Responsibilities |
|---|---|---|
| [`Cargo.toml`](Cargo.toml) | Cargo Manifest | Defines the root Cargo workspace containing `crates/scrape-core`, `crates/scrape-worker`, and `src-tauri`. Pins shared dependencies (`serde`, `tokio`, `chrono`, `regex`, `thiserror`). |
| [`Cargo.lock`](Cargo.lock) | Lockfile | Deterministic dependency version lockfile for all Rust workspace crates. |
| [`package.json`](package.json) | npm Manifest | Frontend and Tauri script definitions (`dev`, `build`, `test`, `tauri`). Declares frontend dependencies (`react`, `tailwindcss`, `@headlessui/react`, `@tanstack/react-query`, `zustand`, `@visx/*`, `yaml`, `@tauri-apps/api`). |
| [`package-lock.json`](package-lock.json) | Lockfile | Deterministic dependency version lockfile for Node.js modules. |
| [`tsconfig.json`](tsconfig.json) | TypeScript Config | TypeScript compiler configuration targeting ES2022 with DOM types, React JSX transform, strict type-checking, and path aliasing. |
| [`tsconfig.node.json`](tsconfig.node.json) | TypeScript Config | TypeScript compiler configuration for Vite configuration and Node scripts. |
| [`vite.config.ts`](vite.config.ts) | Vite Config | Vite build configuration: sets local development port (`1420`), configures Tailwind CSS integration, sets up Vitest environment, and configures Tauri dev server hooks. |
| [`index.html`](index.html) | HTML Entry | Single-page application HTML shell mounting `#root` with bundled offline font families (`@fontsource*` and `public/fonts/`). |
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
npm ci

# Run application in development mode (spawns Tauri window + Vite dev server)
npm run tauri dev

# Build the packaged worker before host/integration checks
npm run build:sidecar

# Run Rust and frontend suites
cargo test --workspace
npm test

# Run code linters and type checkers
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
npx tsc --noEmit

# Compile production frontend bundle
npm run build
```

### Browser demonstration versus desktop execution

Opening Vite in a browser shows a desktop-required screen. Choose the demonstration
link (`?demo=1`) explicitly to explore fictional postings and simulated runs. A
persistent banner identifies that mode. Demo storage uses its own versioned keys;
previous browser fallback records are not loaded into it. Fixture IDs and URLs
are not live-provider verification. Use the desktop application for real extraction
and database-backed workflows.

### Workflows and saved postings

Create a workflow with a source URL, preview actual matches, then choose Manual or
Daily (09:00 in the displayed device timezone) and save. Saving does not scrape;
Run now is explicit. Standard Matching provides keyword chips with Any/All,
exclusions and search fields. Enter, Add, commas or newlines commit terms; spaces
keep phrases together. Imported mixed Any/All groups remain visible. Preview and
run details explain rejection categories, unresolved evidence, examined counts and
separate provider totals. Optional enrichment failures are warnings; incomplete
required evidence produces a partial outcome. Cancellation retains confirmed
saved records and has its own outcome. Employer logos retain source evidence and
a local image cache; failed refreshes retain verified artwork. Workflow display
names are separate from employer identity.
Advanced and YAML editing share one validated v4 draft, with native compensation,
posting dates, selected weekdays and timezone. Matching, schedule, limit and
retention belong to each workflow. System settings controls the application-wide
automatic-scraping pause; explicit manual runs remain available. See
[the active revision](plans/11-reliable-ingestion-and-ui-repair.md) for multi-source
and browser/script requirements that are still pending.

Hourly filters compare explicitly hourly ranges in their original currency,
including cents and inclusive overlap. Annual analytics are separately derived.
Unknown units remain unknown. Saved-job search, sorting, counts and pagination
are performed in SQLite, including databases larger than 2,000 postings.

Remove job preserves saved state. The optional **Keep this job out of future
scrapes** checkbox starts unchecked. A block applies to the exact posting across
workflows. Settings → Removed & blocked jobs supports search, restore and unblock;
Undo is available after removal. Notes, favorites and tracked history survive
restoration and retention. Read [database recovery](plans/database-recovery.md)
before restoring an older database or application version.

### Browser and native regression checks

```bash
npx playwright install chromium
npm run test:browser
# Debug native app; the harness uses only a temporary database and local HTTP.
cargo build -p scrape
npm run test:native
npm run test:native-startup
node scripts/check-style.mjs
npm run gen:types
git diff --exit-code src/bindings
```

Browser checks exercise recorded public-provider examples and an explicit development-only synthetic fixture harness, real DOM
controls, both themes, contrast, keyboard focus narrow reflow and actual 200% browser zoom.
Screenshot baselines target macOS Chromium; Linux CI runs interaction tests.
The separate native harness exercises real Tauri IPC and the packaged sidecar,
including bundled font loading, global font selection, cancellation, malformed output, timeout, partial-result recovery and
external opening. It skips normal tray/autostart setup and cannot use the live
application database. Release builds ignore its debug-only opt-in. Required checks
use local fixtures; they do not establish current public-provider availability.


### Interface and source verification

Interface font selection applies globally, including dialogs; licensed font assets are bundled for offline use. Workflow creation uses a continuous Source, Matching, and Schedule form. Preview validates the current draft before save; advanced controls and YAML preserve the same configuration. Discovered and tracked postings share one details inspector. The application List defaults to six columns and recent stage activity, with optional persisted columns. Kanban labels its page-local counts explicitly.

Browser `?demo=1` shows recorded SEP public postings, with capture provenance in `fixtures/providers/manifest.json`. It cannot run a scraper. Synthetic fixtures require an explicit development-only `?demo=1&fixtures=1` harness; production ignores that fixture flag. Desktop previews use the actual adapter/worker and do not persist jobs.

```bash
npm run build:sidecar
npm run test:providers        # Recorded HTTP → actual worker; no public network needed.
npm run test:providers:live   # Bounded, timestamped public-provider verification.
```

The live check currently covers Lever/SEP, Ashby/Linear, and Greenhouse/Bitwarden. Generic JSON-LD and selector behavior is covered by local fixtures and native smoke, rather than a claim of live-provider verification. Required CI uses deterministic recordings. Regenerate the browser artifact with `node scripts/provider-verification.mjs --refresh-example` only after reviewing the raw captures and their checksums; preserve original provider IDs, titles and URLs.
