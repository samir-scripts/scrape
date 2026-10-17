# Plan 11 — Section 6 Execution Blueprint: Low-Resource Browser & Script Extraction

**Target Executor:** Gemini 3.8 Flash / GPT-6.1 Sol  
**Parent Specification:** [Reliable ingestion, multi-source workflows, and UI repair](11-reliable-ingestion-and-ui-repair.md#6-browser-and-script-extraction)  
**Prerequisite State:** Sections 1–5 and Incident 27 (Migrations 0001–0025) implemented and verified on branch `implementation/trustworthy-scraping`.

**Implementation status:** Phase 6A static extraction is implemented: shared
CSS/XPath fields/regexes, explicit hydration JSON, resumable pagination, bounded
detail enrichment, readiness and preserved identity/branding. Static load-more
controls fail explicitly until browser mode exists. Protocol v7 termination/limits
were pulled forward from 6B to report/enforce 6A coverage correctly. Phase 6B
now adds migration 26, WorkflowSpec v6 with legacy conversion, browser/test
contracts and persisted local script trust/import quarantine. Phases 6C–6F remain
open; browser/session/script runners and their editor controls are unavailable. See
[implementation evidence](11-implementation-evidence.md#section-6b--contracts-migration-26-and-local-script-trust).

---

## 1. Executive Summary & Resource Budget

Traditional Python (`selenium-stealth` + `chromedriver` + `BeautifulSoup4`) or bundled Node/Playwright stacks consume **450–900 MB of RAM**, spawn multiple unmanaged language runtimes, and burn significant CPU rasterizing images, fonts, and stylesheets.

Section 6 replaces that heavy stack with a **three-tier, Rust-native extraction architecture** inside `scrape-worker` and `src-tauri`:

| Tier | Engine | Typical RAM | Typical CPU / Page | When Used |
|---|---|---|---|---|
| **Tier 1 (Default)** | **Pure-Rust Static HTTP + Single-Pass `html5ever` DOM (`scraper`) + XPath 1.0 + SPA Hydration JSON** | **8–15 MB** | **1–5 ms** | Server-rendered HTML, static tables/lists, Next.js/Nuxt hydration blobs (`__NEXT_DATA__`), paginated HTML, and detail-page enrichment. |
| **Tier 2 (Dynamic / Auth)** | **Direct Rust Chrome DevTools Protocol (CDP) over WebSocket (`tokio-tungstenite`)** with resource blocking & pre-document stealth | **110–180 MB** (shared single process) | **30–90 ms** (no image/font/CSS rasterization) | Client-rendered SPAs, "Load more" buttons, readiness waits (`wait_for`), and isolated authenticated sessions (`Open`/`Save`/`Clear`). |
| **Tier 3 (Custom Code)** | **Versioned Stdin/NDJSON Script Subprocess (`SCRIPT_PROTOCOL_VERSION = 1`)** | **15–45 MB** (bounded, reaped after turn) | Bounded by `timeout_s` | User-authored JS or Python scripts emitting raw candidate JSON frames to authoritative Rust validation and matching. |

Resource figures above are design targets, not measured implementation results.
Browser/script budgets remain acceptance requirements until their phases are verified.

### Hard Resource & Concurrency Guardrails (to enforce in Rust)

1. **Single Shared Browser Process (`max_processes = 1`)**: At most one headless Chromium-family browser process may exist per worker run, launched lazily only if a source uses `mode = "browser"`.
2. **Bounded Browser Contexts (`max_contexts = 2`) & Immediate Tab Teardown**: At most two isolated browser contexts (`Target.createBrowserContext`). Every page target (`Target.createTarget`) is closed via `Target.closeTarget` immediately when a round-robin turn completes so V8 heaps never accumulate across sources.
3. **Aggressive CDP Resource Blocking (Default On)**: Before `Page.navigate`, CDP intercepts and blocks `Image`, `Media`, `Font`, `Stylesheet`, and known analytics/tracker domains (`*.google-analytics.com`, `*.doubleclick.net`, `*.segment.io`, `*.hotjar.com`). Employer logos are **never** fetched by the browser renderer; they continue to go through the bounded Rust `CompanyBranding` HTTP verifier.
4. **Hard V8 & Chromium Flags**: Spawn with `--headless=new --disable-gpu --disable-dev-shm-usage --disable-extensions --disable-background-networking --disable-sync --no-first-run --no-default-browser-check --mute-audio --js-flags=--max-old-space-size=128`.
5. **Authoritative Time & Page Bounds**:
   - Navigation timeout: **30s** (`navigation_timeout_s = 30`, range `5..=120`)
   - Source pagination cap: **100 pages** (`max_pages = 100`, range `1..=100`)
   - Source duration cap: **180s** (`max_source_duration_s = 180`, range `10..=600`)
   - Workflow duration cap: **600s** (`max_workflow_duration_s = 600`, range `30..=1800`)
   - Reaching any bound halts discovery for that source/workflow and records a truthful `CoverageTermination` (`PageLimit`, `TimeLimit`, `RequestLimit`, `NeedsLogin`).

---

## 2. Domain Ownership & Invariants

Follow the project's strict DRY and data-safety rules from [`AGENTS.md`](../AGENTS.md):

| Responsibility | Single Authoritative Owner | Contract |
|---|---|---|
| Locator parsing (CSS & XPath 1.0) & regex compilation | `crates/scrape-core/` + `crates/scrape-worker/src/adapters/selector.rs` | Validated at workflow save/import (`src-tauri/src/services/workflow.rs`) and executed in `scrape-worker`. |
| Candidate identity, normalization, & matching | `crates/scrape-core/src/{identity,matching,enrich}.rs` | Browser and script extractors emit **raw candidates only**. Rust normalizes URLs, applies regex captures, derives canonical `PostingIdentity`, evaluates `MatchingEvaluator`, and records categorized rejections. |
| Fair multi-source round-robin & cap enforcement | `crates/scrape-worker/src/coordinator.rs` | Browser and script sources yield **1 candidate per round** via resumable continuations (`next_page` URL / cursor / DOM index) just like ATS sources. |
| Browser session storage | `src-tauri/src/services/browser_session.rs` | Cookies/storage state saved in `<app_data_dir>/browser_sessions/<session_id>.json` with `0700` dir / `0600` file permissions. YAML exports contain `session_id` references only—never raw cookies or tokens. |
| Script execution & trust gate | `src-tauri/src/services/workflow.rs` + `crates/scrape-worker/src/script_runner.rs` | Structured `argv` only (never `sh -c`). Imported scripts start `enabled = false`, `script_trusted = false`, `needs_review = true`. Any edit changes `script_sha256` and requires local user trust. |
| Database schema (`sources.kind` invariant) | `src-tauri/migrations/0026_browser_sessions_and_script_trust.sql` | **Do not rebuild `sources` or alter `sources.kind` CHECK constraint** (`'greenhouse','lever','ashby','jsonld','selector'`). Keep `SourceKind::Selector` as the database kind and store `mode: "static" | "browser" | "script"` in `config` (while accepting `"browser"` / `"script"` as YAML aliases normalized to `SourceKind::Selector` in `workflow::validate`). |

---

## 3. Step-by-Step Implementation Phases

Execute one phase at a time. Follow the mandatory loop for every phase:  
**Research → Failing Regression Test → Implementation → Targeted Verification → Commit**.

---

### Phase 6A — Tier 1: Single-Pass CSS + XPath 1.0 + SPA Hydration + Resumable Pagination & Detail Enrichment

Implementation note: the locator grammar and evaluator live in
`crates/scrape-core/src/{locator,xpath}.rs`, shared by host validation and worker
execution. This follows the domain-ownership rule above without making the host
depend on the worker executable crate. `selector.rs` consumes those modules.

#### 1. Goal
Upgrade `crates/scrape-worker/src/adapters/selector.rs` and `crates/scrape-worker/src/coordinator.rs` so `SourceKind::Selector` in `static` mode supports:
- Both **CSS selectors** and **XPath 1.0 expressions** over a single parsed `scraper::Html` (`html5ever`) DOM tree.
- Explicit **attribute or text extraction** (`text`, `html`, or any `@attribute` such as `href`, `src`, `data-job-id`, `datetime`) on every field.
- Authoritative **Rust `regex` transforms** (`title_regex`, `link_regex`, `location_regex`, `salary_regex`, `date_regex`, `description_regex`, `id_regex`).
- **SPA Hydration State Extraction** (`hydration_script` / `__NEXT_DATA__` / `__NUXT_DATA__` JSON pointer extraction) without launching a browser.
- **Resumable Pagination (`next_page`)**, **Readiness / Load-More metadata**, and **Bounded Detail Page Enrichment (`detail`)** integrated with `CoordinatorSession`.

#### 2. Files to Create or Modify
- **Create** `crates/scrape-worker/src/adapters/xpath.rs`:
  - Implement a lightweight, pure-Rust XPath 1.0 subset evaluator operating directly on `scraper::Html` and `scraper::ElementRef<'a>` (`ego_tree::NodeRef<'a, scraper::Node>`).
  - Supported XPath syntax (must validate at config time and evaluate deterministically):
    - Prefixes: `xpath:...` or expressions starting with `/`, `//`, `./`, `.//`, `(/`, `(//`.
    - Axes: child (`/`), descendant-or-self (`//`), self (`.`), parent (`..` or `parent::tag`), following-sibling (`following-sibling::tag`).
    - Node tests: tag name (`div`, `a`, `tr`), wildcard (`*`), `text()`.
    - Predicates: `[@attr]`, `[@attr='val']`, `[contains(@attr, 'sub')]`, `[starts-with(@attr, 'pre')]`, `[contains(text(), 'sub')]`, `[normalize-space()='val']`, and 1-based index `[1]`, `[last()]`.
    - Terminal attribute extraction: trailing `/@attr` or `/text()` returns the attribute/text value directly.
  - Why over `scraper::Html` instead of a second XML parser crate?
    - Parses HTML5 once via `html5ever` (~2 MB DOM), handles malformed real-world HTML tags without XML parser panics, and allows mixing CSS container selectors with relative XPath field locators (or XPath containers with CSS field locators).
- **Modify** `crates/scrape-worker/src/adapters/selector.rs`:
  - Introduce `LocatorSpec`:
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum QueryEngine {
        Css(scraper::Selector),
        XPath(crate::adapters::xpath::XPathExpr),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct FieldLocator {
        pub query: QueryEngine,
        pub attribute: Option<String>, // None = text (or href/src default for link/logo), Some("text") = text, Some(attr) = element.attr(attr)
        pub regex: Option<regex::Regex>,
    }
    ```
  - Support both shorthand strings (e.g., `"title": "xpath:.//h2[@data-testid='title']/text()"`, `"link": "a.apply@href"`) and explicit config fields (`title_attribute`, `link_attribute`, `location_attribute`, `salary_attribute`, `date_attribute`, `description_attribute`, `id`, `id_attribute`, `id_regex`).
  - Support `next_page` locator (extracting the next page `Url`, preventing cycles via a `visited_pages: HashSet<String>` capped at `max_pages`, default `100`).
  - Support `detail` locator (when present or when `follow_detail = true`, `CoordinatorSession` fetches the candidate's detail URL using the same bounded `EnrichmentLimits` pattern as Greenhouse in Section 3—only after initial title/url pre-filtering passes!).
- **Modify** `crates/scrape-worker/src/coordinator.rs`:
  - Replace the eager one-shot `SourceKind::Selector` vector load in `CoordinatorSession::init` with a resumable `SelectorSessionState` that:
    - Fetches page 1 in `init` (respecting `max_source_duration_s = 180`).
    - Yields one candidate per `next_candidate` call.
    - Performs bounded detail-page enrichment (`detail_selector` / `description` / `salary` / `date` on the detail HTML) only for candidates not rejected by title/keyword pre-filtering, tracking `enrichment_requests` and `deadline` identically to Greenhouse.
    - When `candidate_cursor >= candidates.len()` and `next_page_url` is `Some(url)`:
      - If `pages_visited >= max_pages`, set `self.stats.termination = Some(CoverageTermination::PageLimit)` and mark exhausted.
      - If `Instant::now() >= source_deadline`, set `self.stats.termination = Some(CoverageTermination::TimeLimit)` and mark exhausted.
      - Otherwise fetch `next_page_url`, extract the next batch of candidates and the subsequent `next_page_url`, and continue.

#### 3. Phase 6A Verification
- Write unit tests in `crates/scrape-worker/src/adapters/xpath.rs` and `selector.rs` covering:
  - CSS container + XPath fields, XPath container + CSS fields, `@data-job-id`, `[contains(@class, 'job-card')]`, `following-sibling::`, `[1]`, `/@href`, regex named capture `(?P<value>...)`, multi-page pagination with loop detection, and detail-page enrichment after pre-filter rejection.
- Run: `cargo test -p scrape-worker`

---

### Phase 6B — Protocol v7, Workflow Validation, & Forward Migration 0026

#### 1. Goal
Extend the shared Rust boundary contracts (`scrape-core`), workflow validator (`src-tauri/src/services/workflow.rs`), and SQLite schema (`0026_browser_sessions_and_script_trust.sql`) with full backward compatibility and zero destructive table rebuilds.

Implementation clarification: the extractor additions use **WorkflowSpec v6** to
follow project versioning rules. Versions 1–5 still normalize through the single
Rust resolver; historical storage backfills retain their original v4/v5 versions.
The migration is additive; it does not rewrite old workflow rows. A path script's
trust fingerprint also includes file contents, so an in-place file edit requires
new trust. `import_workflow` quarantines executable sources; `save_workflow` carries
an explicit import origin, and `trust_workflow_script` requires confirmation and
the reviewed hash. Trust does not enable a source or start a run. Until 6C/6D,
the host and worker reject those modes explicitly before execution.

#### 2. Files to Create or Modify
- **Modify** `crates/scrape-core/src/protocol.rs`:
  - Bump `pub const PROTOCOL_VERSION: u32 = 7;`.
  - Add variants to `CoverageTermination`:
    - `PageLimit` (`page_limit`)
    - `NeedsLogin` (`needs_login`)
  - Add `BrowserRuntimeStatus` and `ExtractorTestResult` / `ExtractorSampleCandidate` structs with `#[derive(Serialize, Deserialize, TS)]` and `#[ts(export)]`.
  - Add `WorkerLimits` fields with authoritative defaults:
    - `max_browser_processes: u32` (default `1`)
    - `max_browser_contexts: u32` (default `2`)
    - `navigation_timeout_s: u64` (default `30`)
    - `max_pages_per_source: u32` (default `100`)
    - `max_source_duration_s: u64` (default `180`)
    - `max_workflow_duration_s: u64` (default `600`)
- **Create** `src-tauri/migrations/0026_browser_sessions_and_script_trust.sql`:
  ```sql
  -- 0026: Isolated browser session metadata and custom script trust state.
  -- Forward-only additive migration; preserves sources.kind CHECK constraint.

  CREATE TABLE IF NOT EXISTS browser_sessions (
    id                 TEXT PRIMARY KEY,
    label              TEXT NOT NULL,
    domain             TEXT NOT NULL,
    storage_relpath    TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'ready'
                       CHECK (status IN ('ready', 'needs_login', 'expired')),
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    last_verified_at   TEXT
  );

  CREATE INDEX IF NOT EXISTS idx_browser_sessions_domain
    ON browser_sessions(domain);

  ALTER TABLE sources ADD COLUMN script_trusted INTEGER NOT NULL DEFAULT 0;
  ALTER TABLE sources ADD COLUMN script_sha256 TEXT;
  ```
- **Modify** `src-tauri/src/services/startup.rs` and `plans/database-recovery.md`:
  - Update the expected latest migration version constant in tests from `25` to `26`.
  - Add a populated upgrade test from version `25` → `26` verifying that existing workflows, sources, jobs, notes, favorites, and branding survive untouched.
  - Document migration `0026` in `plans/database-recovery.md` and `src-tauri/migrations/README.md`.
- **Modify** `src-tauri/src/services/workflow.rs`:
  - Extend `WorkflowExtractor` with optional fields for Section 6:
    - `mode: Option<ExtractionMode>` (`Static`, `Browser`, `Script` — defaults to `Static`)
    - `id_selector: Option<String>`, `id_regex: Option<String>`, `date_regex: Option<String>`, `description_regex: Option<String>`
    - `title_attribute: Option<String>`, `url_attribute: Option<String>`, `location_attribute: Option<String>`, `salary_attribute: Option<String>`, `description_attribute: Option<String>`, `date_attribute: Option<String>`, `id_attribute: Option<String>`
    - `detail_selector: Option<String>`, `next_page_selector: Option<String>`, `load_more_selector: Option<String>`, `wait_for_selector: Option<String>`
    - `navigation_timeout_s: Option<u64>`, `max_pages: Option<u32>`, `max_source_duration_s: Option<u64>`, `max_workflow_duration_s: Option<u64>`
    - `session_id: Option<String>` (reference only; reject raw `cookies`, `headers`, `authorization`, `storage_state`, or `token` fields inside `WorkflowExtractor` so credentials can never leak into YAML exports!)
    - `script: Option<WorkflowScriptConfig>` (`runtime`: `"js" | "python" | "custom"`, `interpreter: Option<String>`, `code: Option<String>`, `path: Option<String>`, `args: Option<Vec<String>>`, `timeout_s: Option<u64>`)
  - In `validate_config`:
    - Validate CSS and `xpath:` / `/` locators using the shared locator parser.
    - Validate `navigation_timeout_s` (`5..=120`), `max_pages` (`1..=100`), `max_source_duration_s` (`10..=600`), `max_workflow_duration_s` (`30..=1800`).
    - When `mode == Some(ExtractionMode::Script)` or `script.is_some()`:
      - Compute canonical `script_sha256` over `(runtime, interpreter, code, path, args)`.
      - On YAML import (`import_workflow`), force `enabled = false` and `needs_review = true` (`script_trusted = 0`) until the user explicitly calls `trust_workflow_script` or saves with explicit confirmation.
      - On workflow update, if `script_sha256` changed from the stored source's `script_sha256` without explicit local trust confirmation, reset `script_trusted = 0` and `needs_review = true`.

#### 3. Phase 6B Verification
- Run `npm run gen:types` and verify `src/bindings/` exports cleanly.
- Run `cargo test --workspace` (including startup migration tests and workflow validation tests).

---

### Phase 6C — Tier 2: Direct Rust CDP Browser Engine, Stealth Evasion, Resource Blocking, & Isolated Sessions

#### 1. Goal
Implement a low-memory, low-CPU headless browser engine in `crates/scrape-worker/src/browser/` using direct WebSocket Chrome DevTools Protocol (CDP), plus isolated session management in `src-tauri/src/services/browser_session.rs`.

#### 2. Dependencies (`crates/scrape-worker/Cargo.toml`)
- Add `tokio-tungstenite = { version = "0.24", default-features = false, features = ["connect", "handshake"] }` and `futures-util = "0.3"` (for async WebSocket streams). Do **not** add heavy browser bundling crates that download Chromium during build.

#### 3. Files to Create or Modify
- **Create** `crates/scrape-worker/src/browser/mod.rs`, `discovery.rs`, `cdp.rs`, `stealth.rs`:
  - **`discovery.rs`**:
    - Resolves browser executable or WebSocket endpoint in priority order:
      1. `SCRAPE_CDP_WS_URL` env var or configured `cdp_ws_url` (connects directly over WebSocket; used by deterministic mock CDP tests and Lightpanda/remote CDP).
      2. `SCRAPE_BROWSER_PATH` env var or user-configured browser binary path.
      3. Standard OS installation paths:
         - macOS: `/Applications/Google Chrome.app/Contents/MacOS/Google Chrome`, `/Applications/Brave Browser.app/Contents/MacOS/Brave Browser`, `/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge`, `/Applications/Chromium.app/Contents/MacOS/Chromium`.
         - Linux: `chromium`, `chromium-browser`, `google-chrome`, `google-chrome-stable`, `brave-browser`, `microsoft-edge`.
         - Windows: standard `Program Files` / `LocalAppData` paths for Chrome, Edge, Brave.
    - If none is found, return `ClientError::BrowserRuntimeMissing` with actionable setup guidance: `"No compatible Chromium browser found. Install Chrome, Brave, Edge, or Chromium, or configure the browser executable path in Scraper settings. Automatic background downloads are disabled."`
  - **`stealth.rs`**:
    - Constant `STEALTH_INIT_SCRIPT: &str` injected via `Page.addScriptToEvaluateOnNewDocument`:
      ```javascript
      (() => {
        Object.defineProperty(navigator, 'webdriver', { get: () => false });
        if (!window.chrome) { window.chrome = { runtime: {} }; }
        Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
        Object.defineProperty(navigator, 'plugins', { get: () => [1, 2, 3] });
      })();
      ```
    - Realistic User-Agent + `Emulation.setUserAgentOverride` Client Hints (`Sec-CH-UA`, platform, mobile=false).
  - **`cdp.rs` (`BrowserPool` & `CdpSession`)**:
    - `BrowserPool` is held once on `SourceCoordinator` (`Option<BrowserPool>`), enforcing `max_processes = 1` and a `tokio::sync::Semaphore` with `max_contexts = 2`.
    - Spawns the browser process with an ephemeral temp `--user-data-dir`, `--remote-debugging-port=0`, and the low-memory flags (`--headless=new --disable-gpu --disable-dev-shm-usage --disable-extensions --disable-background-networking --no-first-run --mute-audio --js-flags=--max-old-space-size=128`), reading `DevTools listening on ws://...` from stderr with a bounded 10s startup timeout.
    - Implements `Drop` on `BrowserPool` to send `Browser.close` and immediately reap the process tree (`kill_process_tree` / `SIGKILL` fallback) so orphaned renderer processes can never survive worker exit or cancellation.
    - Per source turn (`fetch_rendered_page`):
      1. Acquire context permit (`max_contexts = 2`).
      2. Create target (`Target.createTarget`) inside an isolated `browserContextId`.
      3. Enable `Page.enable` and `Network.enable`.
      4. Call `Network.setBlockedURLs` (and/or `Fetch.enable` aborting resource types `Image`, `Media`, `Font`, `Stylesheet`) to prevent downloading and decoding heavy visual assets.
      5. Inject `STEALTH_INIT_SCRIPT` via `Page.addScriptToEvaluateOnNewDocument`.
      6. If `session_cookies` are provided for this source's `session_id`, inject them via `Network.setCookies` **only for matching domain/host** after verifying URL safety (`client::validate_public_http_url`).
      7. Navigate (`Page.navigate`) with hard `navigation_timeout_s` (default 30s).
      8. If `wait_for` locator is configured, poll DOM readiness (`Runtime.evaluate` checking `document.querySelector` / `document.evaluate`) with bounded 250ms backoff up to `navigation_timeout_s`.
      9. Detect authentication walls / expired sessions: if final URL or page title/form matches login redirect patterns (`/login`, `/signin`, `/sso`, `input[type="password"]` when no job container matches), close the target and return `ClientError::NeedsLogin("Session expired or login required; re-authenticate in workflow settings")`.
      10. If `load_more` locator is configured, click up to `remaining_pages` times while new items appear and within `source_deadline`.
      11. Extract outer HTML via `Runtime.evaluate` (`document.documentElement.outerHTML`), **immediately close the target (`Target.closeTarget`)**, and hand the HTML to the Tier 1 `selector.rs` CSS/XPath extractor!
- **Create** `src-tauri/src/services/browser_session.rs` & **Modify** `src-tauri/src/commands/scraper.rs`:
  - Add IPC commands:
    - `detect_browser_runtime() -> Result<BrowserRuntimeStatus>`
    - `list_browser_sessions() -> Result<Vec<BrowserSessionSummary>>`
    - `open_browser_session(label: String, login_url: String) -> Result<BrowserSessionSummary>`: launches a visible (headed) isolated browser context pointed at `login_url` **only when explicitly invoked by the user via this interactive command**; never during scheduled or background runs.
    - `save_browser_session(session_id: String) -> Result<BrowserSessionSummary>`: extracts cookies via CDP `Storage.getCookies`, writes them to `<app_data_dir>/browser_sessions/<session_id>.json` with `0600` permissions (dir `0700`), closes the headed browser, and updates `browser_sessions.status = 'ready'`.
    - `clear_browser_session(session_id: String) -> Result<()>`: deletes the session file on disk and removes/marks the row in `browser_sessions`.
    - `test_extractor(extractor: WorkflowExtractor) -> Result<ExtractorTestResult>`: runs a bounded preview (`PREVIEW_MAX_JOBS = 10`, max 1 page, max 15s) and returns extracted sample candidates + field-level match counts + warnings.

#### 4. Phase 6C Verification
- Create `crates/scrape-worker/tests/browser_cdp_process.rs`:
  - Spin up a local loopback WebSocket mock CDP server (`tokio::net::TcpListener` + `tokio_tungstenite::accept_async`) and verify:
    1. `Network.setBlockedURLs` and stealth script injection happen before `Page.navigate`.
    2. `wait_for` readiness, `load_more` interaction, and `next_page` pagination work across round-robin turns.
    3. `Target.closeTarget` is called after every turn and `max_contexts = 2` is respected.
    4. Login redirect returns `CoverageTermination::NeedsLogin` and never opens a headed browser.
    5. Navigation timeout (e.g., 1s in test) and `max_pages` (e.g., 2 in test) terminate cleanly with `CoverageTermination::TimeLimit` / `CoverageTermination::PageLimit`.

---

### Phase 6D — Tier 3: Versioned Stdin/NDJSON Script Runner & Trust Gate

#### 1. Goal
Implement the versioned stdin/stdout NDJSON script extraction engine (`crates/scrape-worker/src/script_runner.rs`) for custom user scripts (`js`, `python`, or custom executable) with strict process-tree reaping, bounded redacted `stderr`, and the explicit trust gate.

#### 2. Stdin/Stdout NDJSON Protocol (`SCRIPT_PROTOCOL_VERSION = 1`)
- **Stdin (1 line JSON sent by `scrape-worker` before closing stdin)**:
  ```json
  {
    "version": 1,
    "source_id": "src-1",
    "url": "https://careers.example.test/jobs",
    "continuation": null,
    "max_candidates": 10,
    "timeout_ms": 30000,
    "config": {}
  }
  ```
- **Stdout (NDJSON lines emitted by script)**:
  - Candidate frame:
    ```json
    {"type":"candidate","version":1,"external_id":"job-42","title":"Staff Data Engineer","url":"https://careers.example.test/jobs/42","location":"Remote - US","salary":"$180,000 - $210,000","description":"<p>Build reliable pipelines.</p>","date_posted":"2026-10-15","company":"Example Corp","employer_logo_url":null}
    ```
  - Continuation frame (for resumable round-robin turns in `coordinator.rs`):
    ```json
    {"type":"continuation","version":1,"cursor":"page=2"}
    ```
  - Diagnostic frame:
    ```json
    {"type":"diagnostic","version":1,"level":"warning","message":"Rate limit header observed; slowing down"}
    ```
  - Completion frame (required before exit 0):
    ```json
    {"type":"complete","version":1,"exhausted":false}
    ```

#### 3. Safety, Backpressure, & Redaction Rules
- **Structured `argv` Only**: Spawn via `tokio::process::Command::new(&program).args(&args)` with `kill_on_drop(true)` and process-group isolation (`process_group(0)` on Unix). Never invoke `sh -c`, `bash -c`, or `cmd /C`.
- **Interpreter Resolution & Diagnostics (`check_script_environment` IPC)**:
  - For `runtime = "js"`: locate `node` (or user-selected JS binary) and report version.
  - For `runtime = "python"`: require an explicit user-selected interpreter path (e.g., `/usr/bin/python3` or `.venv/bin/python3`), verify `--version`, and optionally probe configured modules (e.g., `selenium`, `bs4`) via `python -c "import importlib.util, sys; ..."` using structured `argv` to report clear missing-dependency diagnostics without ever running `pip install` automatically.
- **Bounded Redacted `stderr` (Max 16 KiB)**:
  - Read `stderr` up to `16_384` bytes; redact any token matching `(?i)(authorization|cookie|bearer|token|api[_-]?key|secret|password)\s*[:=]\s*\S+` to `$1=[REDACTED]`.
- **Strict Failure Semantics**:
  - Any line on `stdout` with invalid JSON, `version != 1`, unknown `type`, non-zero process exit status, or exit without a `{"type":"complete", ...}` frame fails the turn explicitly with a non-silent `WorkerEvent::Error`.
- **Rust Owns Acceptance**:
  - Every `candidate` frame is converted into a raw `WorkerEvent::Job`, validated for non-empty `title` and valid HTTP(S) `url`, transformed by any configured Rust regexes, assigned a canonical `PostingIdentity`, and evaluated by `MatchingEvaluator` in `CoordinatorSession` identically to native adapters.

#### 4. Phase 6D Verification
- Create `crates/scrape-worker/tests/script_runner_process.rs`:
  - Test valid NDJSON script emitting candidates + continuations across multiple round-robin turns.
  - Test malformed JSON output, wrong `version: 99`, missing `complete` frame, and non-zero exit status.
  - Test `stderr` secret redaction (`Cookie: session=secret123` → `Cookie=[REDACTED]`) and 16 KiB truncation.
  - Test timeout/cancellation killing a sleeping child process tree (`kill_process_tree`).
  - Test in `src-tauri/tests/worker_repository.rs` that an untrusted imported script workflow is blocked from running until explicitly trusted, and that editing the script resets trust.

---

### Phase 6E — Frontend UI: Custom Extractor, Browser Sessions, & Script Trust in Workflow Editor

#### 1. Goal
Expose Section 6 capabilities cleanly inside the existing single-draft Workflow Editor (`src/components/views/settings/`) using shared NieR brutalist UI primitives (`Select`, `Button`, `IconButton`, `Tag`, `Modal`) and semantic CSS tokens (`src/styles/index.css`).

#### 2. Files to Modify
- `src/hooks/useWorkflowDraft.ts`:
  - Include Section 6 extractor fields (`mode`, CSS/XPath locators, attributes, `detail_selector`, `next_page_selector`, `load_more_selector`, `wait_for_selector`, `navigation_timeout_s`, `max_pages`, `max_source_duration_s`, `session_id`, `script`) in the single draft state.
  - Ensure any change to locators, mode, session, or script invalidates the current preview (`previewStale = true`) and summarizes active advanced settings when collapsed.
- `src/components/views/settings/WorkflowSections.tsx` (and extracted subcomponents if helpful):
  - When a source uses `Custom Extractor (Selector / Browser / Script)`:
    1. **Extraction Mode Select (`Select` primitive)**:
       - `Static HTML · CSS & XPath (Fastest, low memory)`
       - `Headless Browser · CDP (Dynamic SPAs & authenticated sessions)`
       - `Custom Script · Stdin/NDJSON (Local JS or Python runner)`
    2. **Locators & Regex Transforms Panel** (for Static & Browser modes):
       - Required locators: `Job container` (`item`), `Title` (`title`), `Destination link` (`link`).
       - Optional field locators: `Location`, `Compensation`, `Description`, `Posting date`, `Posting ID`, `Employer logo`.
       - Navigation & dynamic locators: `Detail page link` (`detail`), `Next page` (`next_page`), `Load more control` (`load_more`, Browser mode), `Readiness wait` (`wait_for`, Browser mode).
       - Support inline or expandable attribute (`text`, `href`, `src`, `@data-*`) and Rust regex capture (`(?P<value>...)`) inputs per field, with immediate backend validation error display (never hidden inside a collapsed disclosure).
       - **`Test extractor` button**: runs `test_extractor` and displays up to 10 sample candidates with extracted fields and any locator warnings.
    3. **Browser Runtime & Authenticated Session Panel** (when `mode === "browser"`):
       - Shows detected local Chromium runtime status (`Ready: Chrome/Brave/Edge` or actionable setup guidance if missing).
       - Bounds controls: `Navigation timeout (s)` (default 30), `Max pages` (default 100), `Max source duration (s)` (default 180).
       - Isolated Session selector & actions: `Open login session`, `Save session`, `Clear session`, and status badge (`Ready` / `Needs login`). Explain that YAML exports store only the session reference ID, never credentials.
    4. **Custom Script & Trust Panel** (when `mode === "script"`):
       - Runtime `Select` (`JavaScript (Node)` · `Python 3 (Explicit interpreter)` · `Custom executable`), interpreter path input, `Check environment` diagnostic button, structured arguments list, and script code/path editor.
       - **Truthful Privilege & Security Disclosure**: Clear notice stating: *"Custom scripts execute with your local user account privileges. Scrape enforces timeouts, structured arguments, and output validation, but does not provide OS sandboxing or guaranteed bot-challenge bypass."*
       - **Trust Banner for Imported / Modified Scripts**: When `needs_review` / `!script_trusted`, show a prominent warning banner with an explicit `Trust and enable script` action before the workflow can be run or scheduled.
- `src/lib/api.ts`:
  - Wire new Tauri IPC commands (`detectBrowserRuntime`, `listBrowserSessions`, `openBrowserSession`, `saveBrowserSession`, `clearBrowserSession`, `checkScriptEnvironment`, `testExtractor`, `trustWorkflowScript`) with explicit `?demo=1` fixtures labeled as demonstration data.

#### 3. Phase 6E Verification
- Add Vitest interaction tests (`src/test/workflow_browser_script.test.tsx`) using `@testing-library/user-event`:
  - Switching extraction modes (`Static`, `Browser`, `Script`), entering CSS and `xpath:` locators, running `Test extractor`, managing an isolated session (`Open` → `Save` → `Clear`, plus `Needs login` state), and reviewing/trusting an imported script workflow before saving.
- Run `node scripts/check-style.mjs` to ensure zero `rounded-*` or `shadow-*` violations.

---

### Phase 6F — End-to-End Persistence, Native Verification, & Evidence Gate

#### 1. Goal
Verify that authored dynamic, paginated, XPath, authenticated (`Needs login` + valid session), and NDJSON script fixtures flow through the real `scrape-worker` binary into a temporary SQLite database (`src-tauri/tests/worker_repository.rs`), pass all browser and native harnesses, and are recorded in `plans/11-implementation-evidence.md`.

#### 2. End-to-End Test Matrix (`src-tauri/tests/worker_repository.rs`)
1. **Static CSS + XPath + Pagination + Detail Enrichment**:
   - Local HTTP fixture with 3 pages of HTML job cards queried via XPath container (`//article[@data-job-id]`) + CSS fields + regex salary transform + detail-page description enrichment → persisted to SQLite → verified via repository filter queries.
2. **Dynamic CDP Browser Extraction & Resource Blocking**:
   - Local loopback HTTP + mock/headless CDP fixture verifying `wait_for`, `load_more`, `Network.setBlockedURLs` (asserting 0 image/font/stylesheet requests hit the HTTP fixture!), and immediate tab closure → persisted to SQLite.
3. **Authenticated Session & Expiry (`NeedsLogin`)**:
   - Local HTTP fixture requiring a session cookie:
     - Without valid cookie / expired session: worker emits `NeedsLogin` warning/outcome, persists `ScanOutcome::Partial` / `CoverageTermination::NeedsLogin`, and **never** opens a visible window.
     - With saved isolated session cookie: worker injects cookie via CDP, extracts protected postings, and persists them; exported YAML contains `session_id` only and zero secret bytes.
4. **Versioned Stdin/NDJSON Script Extraction & Trust Gate**:
   - Imported workflow with a custom script starts disabled (`needs_review = true`, `script_trusted = false`); attempting to run before trusting fails explicitly.
   - After trusting, worker runs the script via structured `argv`, streams `candidate` + `continuation` + `complete` frames across round-robin turns with a second source (proving fair budgeting!), filters candidates through `MatchingEvaluator`, and persists accepted postings to SQLite.
   - Editing the script invalidates trust until re-confirmed.

#### 3. Required Verification Commands (Run Sequentially)
```bash
npm run build:sidecar
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run gen:types && git diff --exit-code src/bindings
npx tsc --noEmit
npx vitest run --maxWorkers=1
npm run build
node scripts/check-style.mjs
git diff --check
PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflow-sources.pw.ts
cargo build -p scrape && npm run test:native
SCRAPE_STARTUP_MODES=fresh,current25,unknown25 npm run test:native-startup
```

#### 4. Documentation & Evidence Updates
- Append the complete Section 6 verification ledger to [`plans/11-implementation-evidence.md`](11-implementation-evidence.md) (exact commit SHAs, commands run, test totals from the actual run, screenshot paths in `plans/assets/11-browser-script-review/`, and explicit separation of deterministic fixtures vs native/live checks).
- Update status headers in [`plans/11-reliable-ingestion-and-ui-repair.md`](11-reliable-ingestion-and-ui-repair.md), [`plans/README.md`](README.md), [`AGENTS.md`](../AGENTS.md), and affected directory `README.md` files (`crates/scrape-core/README.md`, `crates/scrape-worker/README.md`, `src-tauri/README.md`, `src-tauri/migrations/README.md`).
