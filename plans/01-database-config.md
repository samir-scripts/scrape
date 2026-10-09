# Phase 1 — Database & Configuration Architecture

**Goal:** Scaffold the repo and establish SQLite as the single source of truth, with a typed Rust data layer exposed to the frontend via Tauri commands.

← [Master Plan](./00-master-plan.md) · Next → [Phase 2](./02-background-services.md)

---

## 1.0 Bootstrap the repository

- [x] `git init -b main`; initial commit (plans, README, `.gitignore`, PR template) on `main`; create `develop` from `main`; work on `feature/phase-1-database-config` (see [branching model](./00-master-plan.md#7-git-branching--github-actions)).
- [ ] Create GitHub repo, push `main` + `develop`, set `develop` as default branch, apply rulesets (PR + `ci` check required on `main`/`develop`) via `gh` (awaiting valid token/remote).
- [x] Add `.github/workflows/ci.yml` (fmt, clippy, test, tsc, vitest, style guard) and `.github/dependabot.yml` (cargo, npm, github-actions → `develop`).
- [x] `.gitignore` covers node_modules, target, dist, `src-tauri/binaries/*`, `*.db`.
- [x] Scaffold with React + TypeScript + Vite, npm. Identifier: **`dev.scrape.app`**.
- [x] Convert to a Cargo **workspace** at root.
- [x] Create `crates/scrape-core` (lib) with shared enums so host and worker agree on vocabulary.
- [x] Add `rust-toolchain.toml` pinning `stable` + components `rustfmt`, `clippy`.

## 1.1 Schema (`src-tauri/migrations/0001_init.sql`)

- [x] Write migration; embed with `sqlx::migrate!("./migrations")` and run on startup.
- [x] Connection options: `create_if_missing(true)`, `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5s`, pool size 4.
- [x] DB path: `app.path().app_data_dir()/scrape_data.db` (macOS: `~/Library/Application Support/dev.scrape.app/`).

## 1.2 Shared domain types (`crates/scrape-core`)

- [x] `RoleType::infer(title, hint)` heuristic (`intern|co-op` → Internship, `contract|contractor|freelance|temporary` → Contract, `part-time|part time` → PartTime; explicit hint from ATS wins). Unit-tested.
- [x] `sqlx` derive gated behind a `sqlx` feature so the worker doesn't pull sqlx.

## 1.3 Rust data layer (`src-tauri/src/db/`)

- [x] `jobs.rs`: `list`, `get`, `update_status` (tx: update row + insert event), `bulk_update_status`, `update_notes`, `upsert_scraped` (preserves status).
- [x] `settings.rs`: `get_all() -> Settings`, `get_value<T>`, `set_value`, `ensure_defaults` (`INSERT OR IGNORE`).
- [x] `sources.rs`: `list`, `get`, `create`, `update`, `delete`, `set_last_result`.
- [x] `runs.rs`: `start`, `finish`, `recent`.
- [x] `JobFilter` with dynamic SQL search, pagination, and sorting.
- [x] `AppError` (`thiserror`) implementing `Serialize`.
- [x] Integration tests against `sqlite in-memory` with migrations applied.

## 1.4 Tauri commands (`src-tauri/src/commands/`)

- [x] `list_jobs`, `get_job`, `update_job_status`, `bulk_update_job_status`, `update_job_notes`, `get_job_status_events`.
- [x] `get_settings`, `update_setting`.
- [x] `list_sources`, `get_source`, `create_source`, `update_source`, `delete_source`.
- [x] `AppState { db: SqlitePool }` registered via `app.manage()`.
- [x] Emit `jobs://changed`, `settings://changed`, `sources://changed`.
- [x] `ts-rs` exports to `src/bindings/`; `npm run gen:types`.
- [x] Thin typed wrapper `src/lib/api.ts`.

## 1.5 System integration & defaults (`services/system.rs`)

- [x] On first launch, `ensure_defaults` inserts OS-detected locale, timezone, and week_start (never overwrites).
- [x] `Settings` is a typed struct assembled from rows.

## Acceptance Criteria

- [x] Fresh launch creates `scrape_data.db` with all tables and default settings populated from the OS.
- [x] Relaunch does not duplicate or overwrite settings.
- [x] `update_job_status` updates the row **and** writes a `job_status_events` row.
- [x] `cargo test --workspace` passes (32/32 tests pass).
- [x] `src/bindings/*.ts` generated and `tsc --noEmit` passes.
