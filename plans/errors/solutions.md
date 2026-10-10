# Error & Resolution Log

> Living knowledge base of encountered issues, root causes, and verified solutions during Project Scrape development. Consult this log whenever an error occurs or before implementing similar patterns.

---

## Index of Recorded Errors

1. [Sqlx: Query Macro DATABASE_URL Requirement](#1-sqlx-query-macro-database_url-requirement)
2. [Tauri: Icon Format RGBA Panic](#2-tauri-icon-format-rgba-panic)
3. [Rust: Result Type Alias Shadowing Serde Trait Signature](#3-rust-result-type-alias-shadowing-serde-trait-signature)
4. [Clippy: Needless Borrows on Copy Enums](#4-clippy-needless-borrows-on-copy-enums)
5. [Clippy: Field Reassignment After Default Initializer](#5-clippy-field-reassignment-after-default-initializer)
6. [Texting Robots: Non-Clone Type in Mutex Cache](#6-texting-robots-non-clone-type-in-mutex-cache)
7. [Sqlx Settings: Nested Option in Result Unwrapping](#7-sqlx-settings-nested-option-in-result-unwrapping)
8. [Tauri Plugin Autostart: Missing Platform Launcher Argument](#8-tauri-plugin-autostart-missing-platform-launcher-argument)
9. [Tauri CLI: Cargo Metadata OS Error 2 (Cargo Not on PATH)](#9-tauri-cli-cargo-metadata-os-error-2-cargo-not-on-path)
10. [Tauri v2: MenuItem Runtime Generic Parameter](#10-tauri-v2-menuitem-runtime-generic-parameter)
11. [Tauri v2: Retaining Menu Items for Dynamic Tray Updates](#11-tauri-v2-retaining-menu-items-for-dynamic-tray-updates)
12. [TypeScript: ts-rs BigInt IPC Serialization](#12-typescript-ts-rs-bigint-ipc-serialization)
13. [Zustand & React 19: Vitest Headless LocalStorage & noUnusedLocals](#13-zustand--react-19-vitest-headless-localstorage--nounusedlocals)
14. [Dual Range Input: Pointer Capture Inversion in Multi-Thumb HTML Sliders](#14-dual-range-input-pointer-capture-inversion-in-multi-thumb-html-sliders)
15. [SQLite In-Memory Multi-Connection Table Isolation in SqlitePool](#15-sqlite-in-memory-multi-connection-table-isolation-in-sqlitepool)

---

### 1. Sqlx: Query Macro DATABASE_URL Requirement

- **Phase:** Phase 1 (Database)
- **Error Signature:**
  ```text
  error: set `DATABASE_URL` to use query macros online, or run `cargo sqlx prepare` to update the query cache
  ```
- **Root Cause:**
  `sqlx::query!` and `sqlx::query_as!` perform compile-time SQL verification against a live database instance or pre-generated `.sqlx/` query cache. When building without `DATABASE_URL` set, the build fails.
- **Solution & Rule:**
  Use runtime `sqlx::query(...)` with dynamic parameter binding `.bind(...)` and fetch columns using `sqlx::Row::get(...)`. Reserve `sqlx::query!` only if an offline query cache is deliberately committed.

---

### 2. Tauri: Icon Format RGBA Panic

- **Phase:** Phase 1 (Tauri Host Scaffold)
- **Error Signature:**
  ```text
  error: proc macro panicked
    --> src-tauri/src/lib.rs:
    = help: message: icon .../src-tauri/icons/32x32.png is not RGBA
  ```
- **Root Cause:**
  Tauri's icon bundling logic mandates 32-bit RGBA images (PNG color type 6: 8-bit per channel + alpha channel). 24-bit RGB (color type 2) without alpha causes `tauri::generate_context!()` to panic at compile time.
- **Solution & Rule:**
  Ensure all desktop icons in `src-tauri/icons/` are 32-bit RGBA PNGs (IHDR color type 6).

---

### 3. Rust: Result Type Alias Shadowing Serde Trait Signature

- **Phase:** Phase 1 (Error Handling)
- **Error Signature:**
  ```text
  error[E0053]: method `serialize` has an incompatible type for trait
    = note: expected signature `fn(&AppError, _) -> std::result::Result<_, <S as Serializer>::Error>`
            found signature `fn(&AppError, _) -> std::result::Result<_, AppError>`
  ```
- **Root Cause:**
  Defining a crate-level alias `pub type Result<T> = std::result::Result<T, AppError>;` in `error.rs` caused the unqualified `Result<S::Ok, S::Error>` in `impl Serialize for AppError` to be interpreted as `Result<S::Ok, AppError>`, violating the `serde::Serializer` contract.
- **Solution & Rule:**
  When implementing external traits (like `Serialize`, `Deserialize`, `FromStr`), always write out the explicit standard library type: `std::result::Result<S::Ok, S::Error>`.

---

### 4. Clippy: Needless Borrows on Copy Enums

- **Phase:** Phase 1 (Data Repositories)
- **Error Signature:**
  ```text
  error: the borrowed expression implements the required traits
    --> src-tauri/src/db/jobs.rs
    = help: change this to: `new_status`
    = note: `-D clippy::needless-borrows-for-generic-args` implied by `-D warnings`
  ```
- **Root Cause:**
  Domain enums that derive `Copy` (e.g., `RoleType`, `JobStatus`, `TriggerType`) do not need to be passed by reference to functions taking generic `T: Serialize` (e.g. `serde_json::to_value(&val)`).
- **Solution & Rule:**
  Pass `Copy` enum values by value: `serde_json::to_value(val)`.

---

### 5. Clippy: Field Reassignment After Default Initializer

- **Phase:** Phase 1 (Settings Tests)
- **Error Signature:**
  ```text
  error: field assignment outside of initializer for an instance created with Default::default()
    = note: `-D clippy::field-reassign-with-default` implied by `-D warnings`
  ```
- **Root Cause:**
  Creating `let mut s = Settings::default();` and then reassigning fields `s.system_locale = ...;` triggers Clippy's `field_reassign_with_default` lint.
- **Solution & Rule:**
  Use struct update syntax directly in the initializer:
  ```rust
  let s = Settings {
      system_locale: "fr-FR".to_string(),
      scraper_cron: "0 0 12 * * *".to_string(),
      ..Default::default()
  };
  ```

---

### 6. Texting Robots: Non-Clone Type in Mutex Cache

- **Phase:** Phase 2 (Sidecar HTTP Client)
- **Error Signature:**
  ```text
  error[E0599]: the method `clone` exists for enum `std::option::Option<Robot>`, but its trait bounds were not satisfied
  = note: `Robot: Clone` is not satisfied
  ```
- **Root Cause:**
  `texting_robots::Robot` parses and holds `robots.txt` AST without implementing `Clone`. Attempting to cache `Option<Robot>` in a `Mutex<HashMap<String, Option<Robot>>>` and cloning on lookup fails.
- **Solution & Rule:**
  Wrap non-`Clone` cached objects in `Arc`: `Mutex<HashMap<String, Option<Arc<Robot>>>>`.

---

### 7. Sqlx Settings: Nested Option in Result Unwrapping

- **Phase:** Phase 2 (Scheduler & Runner Settings Queries)
- **Error Signature:**
  ```text
  error[E0308]: mismatched types: expected `Option<_>`, found `bool`
  ```
- **Root Cause:**
  `SettingsRepo::get_value::<T>` returns `Result<Option<T>>` because individual setting keys may be absent from the DB table. Calling `.await.unwrap_or(default)` tries to provide a default of type `Option<T>` rather than unwrapping both levels.
- **Solution & Rule:**
  Chain `.await.ok().flatten().unwrap_or(default)` to safely flatten the nested `Result` and `Option`.

---

### 8. Tauri Plugin Autostart: Missing Platform Launcher Argument

- **Phase:** Phase 2 (Tauri Host Setup)
- **Error Signature:**
  ```text
  error[E0061]: this function takes 2 arguments but 0 arguments were supplied
  --> tauri_plugin_autostart::init()
  ```
- **Root Cause:**
  `tauri-plugin-autostart` v2 requires platform-specific launcher configuration: `MacosLauncher` on macOS (`MacosLauncher::LaunchAgent`) and an optional arguments vector `Option<Vec<&'static str>>`.
- **Solution & Rule:**
  Initialize with `tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None)`.

---

### 9. Tauri CLI: Cargo Metadata OS Error 2 (Cargo Not on PATH)

- **Phase:** Phase 1 / Local Developer Environment
- **Error Signature:**
  ```text
  failed to run 'cargo metadata' command to get workspace directory: failed to run command cargo metadata --no-deps --format-version 1: No such file or directory (os error 2)
  ```
- **Root Cause:**
  The `@tauri-apps/cli` binary delegates workspace and build inspection to `cargo`. If `cargo` (installed in `$HOME/.cargo/bin`) is not exported in the shell's `PATH` variable, Node child processes cannot locate the `cargo` binary and fail with OS error 2 (`ENOENT` / file not found).
- **Solution & Rule:**
  Add Cargo to your user shell configuration (`~/.zshrc` on macOS):
  ```bash
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
  source ~/.zshrc
  ```
  Ensure all development terminal sessions inherit this PATH.

---

### 10. Tauri v2: MenuItem Runtime Generic Parameter

- **Phase:** Phase 2 (Tray Service Dynamic Update)
- **Error Signature:**
  ```text
  error[E0107]: missing generics for struct `tauri::menu::MenuItem`
    --> src-tauri/src/services/tray.rs
    = expected 1 generic argument: `MenuItem<R>`
  ```
- **Root Cause:**
  In Tauri v2, `tauri::menu::MenuItem` is generic over `R: Runtime`. When used in struct definitions (e.g., `pub struct TrayState { pub last_run_item: MenuItem }`), the compiler cannot infer the default `Wry` runtime unless explicitly qualified.
- **Solution & Rule:**
  Specify `tauri::Wry` when storing menu items in application state structs:
  ```rust
  pub struct TrayState {
      pub last_run_item: tauri::menu::MenuItem<tauri::Wry>,
  }
  ```

---

### 11. Tauri v2: Retaining Menu Items for Dynamic Tray Updates

- **Phase:** Phase 2 (Dynamic Tray Labeling)
- **Root Cause:**
  `app.menu()` only returns the window-level menu bar, and `TrayIcon` does not provide a `.menu()` getter. Therefore, dynamic tray updates cannot inspect the menu tree after setup.
- **Solution & Rule:**
  Create and retain the `MenuItem<tauri::Wry>` reference directly in managed state (`app.manage(TrayState { last_run_item })`). Call `state.last_run_item.set_text(...)` directly from runner or event handlers without rebuilding the tray icon or menu.

---

### 12. TypeScript: ts-rs BigInt IPC Serialization

- **Phase:** Phase 3 (Frontend Data Layer)
- **Error Signature:**
  ```text
  TypeError: Do not know how to serialize a BigInt
  ```
- **Root Cause:**
  `ts-rs` translates Rust's signed 64-bit integer (`i64`) into TypeScript `bigint`. However, Tauri's IPC invokes use `JSON.stringify` under the hood, which throws an exception when attempting to serialize native JavaScript `BigInt` values in command arguments.
- **Solution & Rule:**
  Define front-end API wrappers with flexible typing (`number | bigint | null`) and ensure all payload parameters passed to `invoke` are standard JavaScript `number`s rather than raw `BigInt` literals.

---

### 13. Zustand & React 19: Vitest Headless LocalStorage & noUnusedLocals

- **Phase:** Phase 3 (UI State & Testing)
- **Error Signature:**
  ```text
  TypeError: Cannot read properties of undefined (reading 'getItem')
  error TS6133: 'React' is declared but its value is never read.
  ```
- **Root Cause:**
  1. In React 19 with `"noUnusedLocals": true` and `"jsx": "react-jsx"`, default `import React from "react"` triggers TS6133 when only JSX or named types are referenced.
  2. Direct top-level access to `localStorage` in Zustand stores throws an unhandled error in headless Node/Vitest test environments where `window` is undefined.
- **Solution & Rule:**
  1. Use named imports (`import { useState } from "react"`, `import type { ReactNode } from "react"`).
  2. Wrap browser `localStorage` reads and writes in safe conditional guards (`typeof window !== "undefined" && window.localStorage`).

---

### 14. Dual Range Input: Pointer Capture Inversion in Multi-Thumb HTML Sliders

- **Phase:** Phase 3/4 (Salary Filter)
- **Error Signature:**
  One slider end is unresponsive or cannot be dragged; clicking anywhere on the track moves only the higher z-index thumb.
- **Root Cause:**
  In dual `<input type="range">` implementations sharing the same track, applying `pointer-events: auto` to both full-width input elements causes the higher z-index input's transparent track to sit over the entire container, blocking click and drag events from reaching the lower thumb.
- **Solution & Rule:**
  Set `pointer-events: none` on both `<input type="range">` elements, and enable `pointer-events: auto` exclusively on the thumbs via pseudo-elements (`[&::-webkit-slider-thumb]:pointer-events-auto` and `[&::-moz-range-thumb]:pointer-events-auto`). Dynamically swap `z-index` when the lower thumb approaches the upper value or based on pointer proximity so neither thumb gets occluded.

---

### 15. SQLite In-Memory Multi-Connection Table Isolation in SqlitePool

- **Phase:** Phase 4 (Metrics Testing)
- **Error Signature:**
  ```text
  Database(SqliteError { code: 1, message: "no such table: app_settings" })
  ```
- **Root Cause:**
  SQLite in-memory databases with URI `:memory:` are private to each distinct database connection. A `SqlitePool` with `max_connections > 1` creates separate, empty in-memory databases for subsequent connections, meaning migrations executed on connection 1 are invisible to connection 2.
- **Solution & Rule:**
  In unit tests using in-memory SQLite, always use `max_connections(1)` (via `crate::db::init_memory_db()`) or a shared in-memory URI (`file:memdb?mode=memory&cache=shared`) to guarantee all pooled queries access the same migrated schema.

---

### 16. SQLite CHECK Constraint Alteration & ISO 3166-2 Suffix Matching

- **Phase:** Phase 4 (Employment Type & Cascading Location Dropdowns)
- **Error Signature:**
  ```text
  Database(SqliteError { code: 19, message: "CHECK constraint failed: jobs" })
  ```
- **Root Cause:**
  1. SQLite does not support `ALTER TABLE ... MODIFY CONSTRAINT` or modifying an existing `CHECK` clause. Adding a new enum variant (e.g., `'seasonal'` to `role_type`) throws a constraint violation unless the table is recreated.
  2. Standard gazetteers store ISO 3166-2 codes (e.g. `"US-CA"`, `"CA-ON"`). Querying with 2-letter state codes (e.g., `"CA"`) via direct `IN ('CA')` produces zero results despite matches existing.
- **Solution & Rule:**
  1. Recreate the table via migration: `CREATE TABLE jobs_new (... CHECK (role_type IN (..., 'seasonal', ...))); INSERT INTO jobs_new SELECT ... FROM jobs; DROP TABLE jobs; ALTER TABLE jobs_new RENAME TO jobs;` and re-create all indexes.
  2. Implement dual-mode matching across SQL and client filters: check both exact match and suffix match (`jl.region_code = 'CA' OR jl.region_code LIKE '%-CA'`), allowing both full ISO codes and state abbreviations to filter seamlessly.



