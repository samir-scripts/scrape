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

