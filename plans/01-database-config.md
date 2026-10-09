# Phase 1 — Database & Configuration Architecture

**Goal:** Scaffold the repo and establish SQLite as the single source of truth, with a typed Rust data layer exposed to the frontend via Tauri commands.

← [Master Plan](./00-master-plan.md) · Next → [Phase 2](./02-background-services.md)

---

## 1.0 Bootstrap the repository

- [ ] `git init -b main`; initial commit (plans, README, `.gitignore`, PR template) on `main`; create `develop` from `main`; work on `feature/phase-1-database-config` (see [branching model](./00-master-plan.md#7-git-branching--github-actions)).
- [ ] Create GitHub repo, push `main` + `develop`, set `develop` as default branch, apply rulesets (PR + `ci` check required on `main`/`develop`) via `gh`.
- [ ] Add `.github/workflows/ci.yml` (fmt, clippy, test, tsc, vitest, style guard) and `.github/dependabot.yml` (cargo, npm, github-actions → `develop`).
- [ ] `.gitignore` covers node_modules, target, dist, `src-tauri/binaries/*`, `*.db`.
- [ ] Scaffold with `npm create tauri-app@latest` → React + TypeScript + Vite, npm. Identifier: **`dev.scrape.app`** *(confirm)*.
- [ ] Convert to a Cargo **workspace** at root:
  ```toml
  # Cargo.toml
  [workspace]
  resolver = "2"
  members = ["src-tauri", "crates/scrape-core", "crates/scrape-worker"]

  [workspace.dependencies]
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  chrono = { version = "0.4", features = ["serde"] }
  uuid = { version = "1", features = ["v7", "serde"] }
  tokio = { version = "1", features = ["full"] }
  thiserror = "2"
  tracing = "0.1"
  ts-rs = { version = "10", features = ["chrono-impl", "uuid-impl"] }
  ```
- [ ] Create `crates/scrape-core` (lib) with shared enums (below) so host and worker agree on vocabulary.
- [ ] Add `rust-toolchain.toml` pinning `stable` + components `rustfmt`, `clippy`.
- [ ] Note: `~/.cargo/bin` isn't on the current shell `PATH` — add `source "$HOME/.cargo/env"` to `~/.zshrc`.

## 1.1 Schema (`src-tauri/migrations/0001_init.sql`)

Conventions: UUID v7 as `TEXT` (time-sortable), timestamps as ISO-8601 UTC `TEXT`, enums as `TEXT` + `CHECK`.

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE app_settings (
  key         TEXT PRIMARY KEY,
  value       TEXT NOT NULL,              -- JSON-encoded value
  updated_at  TEXT NOT NULL
);

CREATE TABLE sources (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('greenhouse','lever','ashby','jsonld','selector')),
  name         TEXT NOT NULL,             -- display / company name
  identifier   TEXT NOT NULL,             -- board token, slug, or URL
  config       TEXT NOT NULL DEFAULT '{}',-- JSON: selectors, follow-links, etc.
  enabled      INTEGER NOT NULL DEFAULT 1,
  last_run_at  TEXT,
  last_status  TEXT CHECK (last_status IN ('ok','error','empty')),
  last_error   TEXT,
  created_at   TEXT NOT NULL,
  UNIQUE (kind, identifier)
);

CREATE TABLE jobs (
  id                 TEXT PRIMARY KEY,
  dedupe_key         TEXT NOT NULL UNIQUE,      -- "{kind}:{identifier}:{external_id}" or normalized URL
  source_id          TEXT REFERENCES sources(id) ON DELETE SET NULL,
  external_id        TEXT,
  title              TEXT NOT NULL,
  company            TEXT NOT NULL,
  role_type          TEXT NOT NULL DEFAULT 'unknown'
                     CHECK (role_type IN ('full_time','contract','internship','part_time','unknown')),
  location           TEXT,
  url                TEXT NOT NULL,
  date_posted        TEXT,
  date_added         TEXT NOT NULL,
  last_seen_at       TEXT NOT NULL,
  status             TEXT NOT NULL DEFAULT 'scraped'
                     CHECK (status IN ('scraped','viewed','applied','interviewing','offer','rejected','ignored')),
  status_changed_at  TEXT NOT NULL,
  notes              TEXT
);
CREATE INDEX idx_jobs_status      ON jobs(status, date_added DESC);
CREATE INDEX idx_jobs_role_type   ON jobs(role_type);

CREATE TABLE job_status_events (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id       TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  from_status  TEXT,
  to_status    TEXT NOT NULL,
  changed_at   TEXT NOT NULL
);
CREATE INDEX idx_events_to_status ON job_status_events(to_status, changed_at);

CREATE TABLE scrape_runs (
  id           TEXT PRIMARY KEY,
  trigger      TEXT NOT NULL CHECK (trigger IN ('schedule','manual','startup')),
  status       TEXT NOT NULL CHECK (status IN ('running','success','partial','failed')),
  started_at   TEXT NOT NULL,
  finished_at  TEXT,
  jobs_found   INTEGER NOT NULL DEFAULT 0,
  jobs_new     INTEGER NOT NULL DEFAULT 0,
  error        TEXT
);
```

**Tasks**
- [ ] Write migration; embed with `sqlx::migrate!("./migrations")` and run on startup.
- [ ] Connection options: `create_if_missing(true)`, `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5s`, pool size 4.
- [ ] DB path: `app.path().app_data_dir()/scrape_data.db` (macOS: `~/Library/Application Support/dev.scrape.app/`).

## 1.2 Shared domain types (`crates/scrape-core`)

```rust
#[derive(Serialize, Deserialize, TS, sqlx::Type, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")] #[sqlx(rename_all = "snake_case")]
pub enum RoleType { FullTime, Contract, Internship, PartTime, Unknown }

pub enum JobStatus { Scraped, Viewed, Applied, Interviewing, Offer, Rejected, Ignored }
pub enum SourceKind { Greenhouse, Lever, Ashby, Jsonld, Selector }
```
- [ ] `RoleType::infer(title, hint)` heuristic (`intern|co-op` → Internship, `contract|contractor|freelance|temporary` → Contract, `part-time|part time` → PartTime; explicit hint from ATS wins). Unit-tested.
- [ ] `sqlx` derive gated behind a `sqlx` feature so the worker doesn't pull sqlx.

## 1.3 Rust data layer (`src-tauri/src/db/`)

| Module | Functions |
|---|---|
| `jobs.rs` | `list(filter: JobFilter) -> Page<Job>`, `get(id)`, `update_status(id, status)` *(tx: update row + insert event)*, `bulk_update_status(ids, status)`, `update_notes(id, notes)`, `upsert_scraped(batch) -> UpsertStats` *(used in Phase 2; never overwrites `status`)* |
| `settings.rs` | `get_all() -> Settings`, `get<T>(key)`, `set<T>(key, value)`, `ensure_defaults(defaults)` *(`INSERT OR IGNORE`)* |
| `sources.rs` | `list`, `create`, `update`, `delete`, `set_last_result` |
| `runs.rs` | `start(trigger)`, `finish(id, stats)`, `recent(limit)` |

- [ ] `JobFilter { statuses: Vec<JobStatus>, role_types: Vec<RoleType>, search: Option<String>, sort: JobSort, limit, offset }` — search uses `LIKE` on title/company/location.
- [ ] `AppError` (`thiserror`) implementing `Serialize` so commands return `Result<T, AppError>` and the frontend receives `{ kind, message }`.
- [ ] Integration tests against `sqlite::memory:` with migrations applied (status transitions write events; upsert preserves status; filters work).

## 1.4 Tauri commands (`src-tauri/src/commands/`)

| Command | Signature (TS view) |
|---|---|
| `list_jobs` | `(filter: JobFilter) => Page<Job>` |
| `get_job` | `(id: string) => Job` |
| `update_job_status` | `(id: string, status: JobStatus) => Job` |
| `bulk_update_job_status` | `(ids: string[], status: JobStatus) => number` |
| `update_job_notes` | `(id: string, notes: string) => void` |
| `get_settings` | `() => Settings` |
| `update_setting` | `(key: SettingKey, value: JsonValue) => Settings` |
| `list_sources` / `create_source` / `update_source` / `delete_source` | CRUD |

- [ ] `AppState { db: SqlitePool }` registered via `app.manage()` in `setup`.
- [ ] Emit `jobs://changed` after any mutation so other open views refetch.
- [ ] `ts-rs` exports to `src/bindings/`; `npm run gen:types` runs `cargo test -p scrape-core -p scrape export_bindings`.
- [ ] Thin typed wrapper `src/lib/api.ts` (`api.updateJobStatus(id, 'applied')`).

## 1.5 System integration & defaults (`services/system.rs`)

On first launch, `ensure_defaults` inserts (never overwrites):

| Key | Default | Source |
|---|---|---|
| `system.locale` | e.g. `"en-US"` | `sys-locale` crate |
| `system.timezone` | e.g. `"America/New_York"` | `iana-time-zone` crate |
| `system.week_start` | `"sunday"` / `"monday"` | derived from locale region |
| `ui.theme` | `"paper"` (alt: `"obsidian"`) | — |
| `ui.reduce_motion` | `false` | — |
| `scraper.enabled` | `true` | — |
| `scraper.cron` | `"0 0 */6 * * *"` (every 6h, 6-field) | — |
| `scraper.run_on_startup` | `false` | — |
| `filters.include_keywords` | `[]` | — |
| `filters.exclude_keywords` | `[]` | — |
| `filters.locations` | `[]` | — |
| `filters.role_types` | `[]` (= all) | — |
| `notifications.enabled` | `true` | — |
| `app.start_hidden` | `true` | — |
| `app.launch_at_login` | `false` | — |

- [ ] `Settings` is a typed struct (serde) assembled from rows, so the frontend gets one strongly-typed object rather than raw KV.

## Acceptance Criteria

- [ ] Fresh launch creates `scrape_data.db` with all tables and default settings populated from the OS.
- [ ] Relaunch does not duplicate or overwrite settings.
- [ ] From devtools: `await __TAURI__.core.invoke('update_job_status', {id, status:'applied'})` updates the row **and** writes a `job_status_events` row.
- [ ] `cargo test --workspace` passes, including in-memory DB tests.
- [ ] `src/bindings/*.ts` generated and `tsc --noEmit` passes.
