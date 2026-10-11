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
16. [SQLite CHECK Constraint Alteration & ISO 3166-2 Suffix Matching](#16-sqlite-check-constraint-alteration--iso-3166-2-suffix-matching)
17. [Tracked Kanban Board: Header Seam Bleed, Asymmetrical Column Capping, and Scrollbar Gutter Misalignment](#17-tracked-kanban-board-header-seam-bleed-asymmetrical-column-capping-and-scrollbar-gutter-misalignment)
18. [Cross-View Selection Orphan & Asymmetric Hotkey Listener Churn in Kanban Navigation](#18-cross-view-selection-orphan--asymmetric-hotkey-listener-churn-in-kanban-navigation)
19. [Global Hotkeys Lifecycle Severance & View Unmount Void in Root Navigation](#19-global-hotkeys-lifecycle-severance--view-unmount-void-in-root-navigation)
20. [Modal Window Escape Trapping & Dialog Event Dissociation](#20-modal-window-escape-trapping--dialog-event-dissociation)
21. [SQLx SQLite Text Column NULL Coercion in Option Decoding](#21-sqlx-sqlite-text-column-null-coercion-in-option-decoding)
23. [Vite / esbuild JSX Transformation Error in `.test.ts` Test Files](#23-vite--esbuild-jsx-transformation-error-in-testts-test-files)
24. [SQLx Migration Checksum VersionMismatch & Non-Unwinding macOS Startup Panic](#24-sqlx-migration-checksum-versionmismatch--non-unwinding-macos-startup-panic)

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

---

### 17. Tracked Kanban Board: Header Seam Bleed, Asymmetrical Column Capping, and Scrollbar Gutter Misalignment

- **Phase:** Phase 3 / Phase 4 (Tracked Kanban Board UI)
- **Error Signature:**
  Kanban column headers appear misaligned with column bodies, melt into a single unbordered black slab across columns while cards below have 1px vertical borders, terminate abruptly leaving an empty void on displays > 1536px, and shift out of alignment when vertical scrollbars appear.
- **Root Cause:**
  1. **Color Bleed & Missing Separators:** `KanbanColumn` header applied `bg-[var(--color-ink)]` directly abutting `border-r border-[var(--color-ink)]`. Because the background and border shared the identical CSS color, the 1px vertical column dividers vanished in the header row, creating a monolithic solid black bar. Furthermore, no bottom border (`border-b border-[var(--color-ink)]`) separated the header from the cards list.
  2. **Asymmetrical Width Clamping:** Columns used `flex-1 min-w-[240px] max-w-sm`. At $4 \times 384\text{px} = 1536\text{px}$, the columns clamped, leaving up to 40% dead space on 1080p/1440p monitors, while `last:border-r-0` caused Column 4's body to dissolve into the void without a right border.
  3. **Scrollbar Gutter Asymmetry:** Dynamic vertical scrollbars (8px width) appeared only on overflowing columns, shifting the right edge of cards 8px inward while column headers had no scrollbar and remained unshifted.
  4. **Status Theme Bypassing:** `KanbanBoard` defined an ad-hoc `STAGE_TITLES` map and rendered solid black headers, completely bypassing `STATUS_CONFIG` semantic colors (`lime`, `signal`, `ink`, `cherry`).
- **Solution & Rule:**
  1. **Strict 1px Brutalist Grid:** Enclose Kanban in a CSS Grid track container `grid grid-cols-4 min-w-[1120px] w-full h-full divide-x divide-[var(--color-ink)]` without `max-w-sm`, guaranteeing 100% viewport utilization and equal column widths.
  2. **Standardized Header Baseline:** Set column headers to `h-9 px-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40` matching `SubNav` rhythm, with visible 1px dividing lines, telemetry indicators (`// 0X`), and `STATUS_CONFIG` status accent pips.
  3. **Stable Scrollbar Tracks:** Apply `overflow-y-scroll` (or `scrollbar-gutter: stable`) to all column card containers to guarantee identical gutter geometry regardless of card counts.

---

### 18. Cross-View Selection Orphan & Asymmetric Hotkey Listener Churn in Kanban Navigation

- **Phase:** Phase 3 / Phase 4 (Tracked Kanban Board & Hotkeys Engine)
- **Error Signature:**
  When navigating the Kanban board via keyboard shortcuts (`ArrowRight`/`l` and `ArrowLeft`/`h`), moving right appears to work initially, but navigating back left fails completely or becomes stuck. Additionally, pressing `Shift+H` or `Shift+L` to stage-shift jobs fails silently, and rapid keyboard navigation causes listener re-registration thrashing.
- **Root Cause:**
  1. **Cross-View Selection Orphan (Primary Failure Mechanism):**
     `useUIStore` maintains a single global `selectedJobId`. In `DiscoveredView`, an automatic synchronization effect sets `selectedJobId` to the first discovered job (status `scraped` or `viewed`). However, `TrackedView` lacked a corresponding validation or fallback effect. When the user navigated from Discovered to Tracked, `selectedJobId` pointed to a Discovered job that did not exist in the Tracked `columnMatrix` (statuses `applied`, `interviewing`, `offer`, `rejected`).
     - When the user pressed `Right` (`ArrowRight` or `l`), `findJobCoordinates(columnMatrix, selectedJobId)` returned `null`. The horizontal navigation fallback triggered: `firstNonEmpty?.jobs[0]?.id` (Column 0, Row 0). The selection visually snapped to Column 0, Row 0, creating the illusion that pressing Right "worked".
     - When the user immediately pressed `Left` (`ArrowLeft` or `h`), `selectedJobId` was now at Column 0 (`colIdx = 0`). The leftward iteration loop `while (nextCol >= 0)` evaluated `nextCol = -1`, immediately exiting and returning `currentJobId` unchanged. The user observed: *"going right worked, but going back left failed"*.
  2. **Destructive Row Clamping & Vertical Memory Loss:**
     `navigateHorizontal` destructively clamped the row index via `Math.min(coords.rowIdx, targetJobs.length - 1)` without retaining the original target row index across sparse columns. Navigating Col 0 (Row 3) $\to$ Col 1 (Row 0) permanently clamped the selection to Row 0, preventing symmetric return to Row 3 when moving back left.
  3. **Vim Key Asymmetry with Shift (`Shift+H` / `Shift+L`):**
     `useHotkeys.ts` strictly checked `e.shiftKey && e.key === "ArrowLeft"` and `e.shiftKey && e.key === "ArrowRight"`, omitting `H` and `L`. Because pressing `Shift+h` produces `e.key = "H"` and `e.shiftKey = true`, it bypassed both the stage shift handler and the navigation handler (`!e.shiftKey`), resulting in a silent dead key.
  4. **Event Listener Churn on Render Cycles:**
     `useHotkeys` accepted an inline options object literal from `TrackedView` as a `useEffect` dependency (`[view, setView, options]`). Every render cycle, hotkey press, or state update caused the global `keydown` event listener to be detached and re-attached to `window`, introducing micro-latencies and potential dropped keystrokes.
- **Solution & Rule:**
  1. **View Selection Self-Healing:** In `TrackedView`, implement an auto-validation effect ensuring that if `selectedJobId` is null or does not exist within `trackedJobs`, it resets to `trackedJobs[0]?.id`, preventing orphaned cross-view selection IDs from corrupting 2D navigation coordinates.
  2. **Ref-Stabilized Hotkey Dispatcher:** In `useHotkeys.ts`, store options and callbacks in a mutable ref (`optionsRef.current = options`) with an empty dependency array `[]` on listener attachment, completely eliminating listener churn on render cycles.
  3. **Symmetric Modifier & Key Handling:** Support both arrow keys and Vim keys for all actions: `(e.key === "ArrowLeft" || e.key === "H")` for left stage shift, `(e.key === "ArrowRight" || e.key === "L")` for right stage shift. Ensure modifier guards check `!(e.metaKey || e.ctrlKey || e.altKey)`.
  4. **Symmetric 2D Column Navigation with Target Memory:** In `kanbanNavigation.ts`, provide robust matrix navigation with virtual row memory or intelligent clamping that guarantees symmetric bidirectional transitions across columns.

---

### 19. Global Hotkeys Lifecycle Severance & View Unmount Void in Root Navigation

- **Phase:** Phase 3 / Phase 4 (App Shell Architecture & Global Keyboard Navigation)
- **Error Signature:**
  Pressing `Q` or `E` (and `[` or `]`) to navigate through application tabs works partially or unpredictably between `Discovered` and `Tracked`, but completely ceases to function once the user enters `Metrics`, `Scraper`, or `Settings`. The user becomes trapped on those tabs with no response to keyboard navigation.
- **Root Cause:**
  1. **Sub-View Lifecycle Coupling:**
     Global application hotkeys (such as tab cycling via `Q`/`E`/`[`/`]`, global search focus via `/`, filter drawer toggling via `F`, and `Escape`) were implemented inside the `useHotkeys` hook, which was only mounted in leaf view components: `DiscoveredView.tsx` and `TrackedView.tsx`.
     When the user navigated from `Tracked` to `Metrics`, `TrackedView` unmounted, tearing down its `keydown` event listener. Because neither `MetricsView`, `ScraperView`, nor `SettingsView` mounted `useHotkeys`, the desktop application had **zero** event listeners registered on `window`. The keyboard shortcuts ceased functioning entirely.
  2. **Stale Closure Race on Rapid Keypresses:**
     Tab calculations referenced `view` captured in the component closure rather than reading the live Zustand store state directly (`useUIStore.getState().view`).
  3. **Absence of Root Lifecycle Listener:**
     Global application navigation was not bound to the root application shell (`App.tsx`), which remains mounted throughout the entire desktop runtime.
- **Solution & Rule:**
  1. **Two-Tier Hotkey Architecture:**
     - **Tier 1 (Global Shell Shortcuts):** Decouple global application shortcuts into `useGlobalHotkeys()`, mounted permanently at the root in `App.tsx`. Handles tab cycling (`Q`/`E`/`[`/`]`), search focus (`/`), filter drawer toggle (`F`), and `Escape`. Attaches once on mount with an empty dependency array `[]` and reads canonical instantaneous state via `useUIStore.getState().view`.
     - **Tier 2 (View-Specific Shortcuts):** Keep `useHotkeys(options)` scoped strictly to in-view item/column navigation and actions (`J`/`K`/`H`/`L`, `Shift+H`/`Shift+L`, `A`, `X`, `V`, `U`, `Enter`), eliminating duplicate tab-switch handlers or unmount dead zones.
  2. **Footer Telemetry Synchronization:**
     Ensure `Footer.tsx` displays consistent `Q/E SWITCH TAB` and `ESC CLOSE` hints across all views (`metrics`, `scraper`, `settings`).

---

### 20. Modal Window Escape Trapping & Dialog Event Dissociation

- **Phase:** Phase 3 / Phase 4 (Modal Dialogs & Keyboard Ergonomics)
- **Error Signature:**
  Pressing `Escape` while a modal window is open (such as the "ADD SCRAPE JOB" dialog) fails to close the window. If an input field is focused, the first `Escape` merely blurs the input, and subsequent `Escape` keypresses are completely ignored, leaving the modal stuck open until closed via mouse click.
- **Root Cause:**
  1. **Absence of Window Keydown Listener in Modal:**
     [`Modal.tsx`](file:///Users/samirkatakamsetty/Desktop/Home/Data%20Engineering%20Projects/scrape/src/components/ui/Modal.tsx) relied exclusively on the native HTML `<dialog>` element's `cancel` event (`dialog.addEventListener("cancel", handleCancel)`).
  2. **Dialog Event Dissociation on Input Focus:**
     Native `<dialog>` elements only emit a `cancel` event when focus is currently inside the dialog and not captured or intercepted by ancestor key handlers. When the user was typing inside an `<input>` in `AddSourceModal`, `useGlobalHotkeys` caught the `Escape` key on `window`, saw `isInput === true`, executed `(activeEl as HTMLElement).blur()`, and returned without closing the modal.
  3. **Loss of Focus to Document Body:**
     Once `activeEl.blur()` executed, focus shifted to `document.body` (outside the `<dialog>`). Subsequent `Escape` keypresses neither triggered `cancel` on the `<dialog>` nor matched any open drawers in `useGlobalHotkeys`, resulting in complete dead-key failure.
- **Solution & Rule:**
  1. **Capture-Phase Window Escape Listener in Modal:**
     In `Modal.tsx`, attach a capture-phase keydown listener to `window` (`window.addEventListener("keydown", handleKeyDown, true)`) while `isOpen === true`. When `e.key === "Escape"`, call `e.preventDefault()`, `e.stopPropagation()`, and `onClose()`.
  2. **Overlay Priority Guarantee:**
     Capture-phase execution ensures modal dismissals execute before any child input blurs or background global hotkey handlers can intercept the event, guaranteeing 100% reliable single-press Escape dismissals across all dialogs and drawers.

---

### 21. SQLx SQLite Text Column NULL Coercion in Option Decoding

- **Phase:** Phase 3.7 (Workflow Data Retention & Job Favoriting)
- **Error Signature:**
  Postgres/SQLite row decoding via `r.try_get("favorited_at").ok()` without explicit type parameters caused nullable ISO timestamps (`TEXT NULL`) to be inferred as `String` and decoded as `Some("")` rather than `None`.
- **Root Cause:**
  When SQLx reads a NULL text column into a `String` without explicit `Option<String>` typing, some drivers/versions return an empty string or type mismatch error rather than propagating `None`.
- **Solution & Rule:**
  Explicitly qualify the nullable type and filter out empty strings:
  ```rust
  let favorited_at = r
      .try_get::<Option<String>, _>("favorited_at")
      .unwrap_or(None)
      .filter(|s| !s.trim().is_empty());
  ```

---

---

### 23. Vite / esbuild JSX Transformation Error in `.test.ts` Test Files

- **Phase:** Phase 3.7 (NieR Brutalist Tooltip & Dual-Mode Scraper Testing)
- **Error Signature:**
  ```text
  Error: Transform failed with 1 error:
  ERROR: Expected ">" but found "content"
    Plugin: vite:esbuild
    File: .../src/test/tooltip.test.ts:16:15
  ```
- **Root Cause:**
  Vite's `esbuild` preprocessor only parses JSX/TSX syntax when the file extension is `.tsx` or `.jsx`. When testing UI components with JSX syntax inside a `.test.ts` file, `esbuild` interprets angle brackets `<Component>` as TypeScript generics rather than JSX elements, throwing a syntax parse failure.
- **Solution & Rule:**
  Either name test files containing JSX `.test.tsx`, or test component coordinate geometry, mathematical projections, and state machines in pure TypeScript `.test.ts` files without DOM/JSX dependencies.

---

### 24. SQLx Migration Checksum VersionMismatch & Non-Unwinding macOS Startup Panic

- **Phase:** Phase 3.10 (Workflow Hardening & Notification Error Storage)
- **Error Signature:**
  ```text
  thread 'main' panicked at src-tauri/src/lib.rs:35:22:
  Failed to initialize database: Migration(VersionMismatch(6))
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
  thread caused non-unwinding panic. aborting.
  ```
- **Root Cause:**
  1. **Migration Immutability Violation:** When adding the new `'error'` variant to the `app_notifications` `kind` column, `0006_notifications.sql` was modified in place (`CHECK (kind IN ('update', 'news', 'system', 'scraper', 'error'))`).
  2. **Checksum Verification Mismatch in `_sqlx_migrations`:** SQLx computes a SHA-384 checksum for every migration file and records it in `_sqlx_migrations` upon execution. On existing installations where migration 6 had already run, SQLx compared the hash of the modified `0006_notifications.sql` against the stored checksum in `_sqlx_migrations`, detected the divergence, and threw `sqlx::migrate::MigrateError::VersionMismatch(6)`.
  3. **Non-Unwinding FFI Panic:** Because `db::init_db` was invoked inside Tauri's `did_finish_launching` callback (an `extern "C"` Objective-C runtime delegate on macOS), the panic crossed an FFI boundary that cannot unwind, causing macOS to immediately abort the process with a non-unwinding panic.
  4. **Test Gap:** In-memory SQLite tests (`init_memory_db()`) created a fresh database on every run, meaning `_sqlx_migrations` was always populated from scratch and never encountered an existing DB with prior migration checksums.
- **Solution & Rule:**
  1. **Revert Mutated Migrations:** Revert `0006_notifications.sql` back to its pristine bytes so its SHA-384 checksum matches prior database records.
  2. **Sequential Schema Evolution via New Migration:** Create `0007_notification_error_kind.sql` using SQLite's table recreation pattern (`CREATE TABLE app_notifications_new`, `INSERT INTO app_notifications_new SELECT ...`, `DROP TABLE app_notifications`, `ALTER TABLE app_notifications_new RENAME TO app_notifications`). Never alter an existing committed migration file.
  3. **Resilient Startup & Self-Healing Migration Runner:** In `src-tauri/src/db/mod.rs`, wrap `sqlx::migrate!()` in `run_migrations(&pool)`. If `MigrateError::VersionMismatch(v)` is encountered during local development, dynamically locate `v` in `migrator.iter()`, synchronize `_sqlx_migrations.checksum = target_mig.checksum`, and retry execution, preventing unhandled aborts.
  4. **Regression Tests:** Added `test_all_migrations_apply_cleanly_up_to_version_7` and `test_migration_recovery_on_version_mismatch` in `src-tauri/src/db/mod.rs` to guarantee both clean execution of all 7 migrations and recovery when `_sqlx_migrations` contains a mismatched checksum.





