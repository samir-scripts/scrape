# Phase 2 — Background Services (Tray, Scheduler, Sidecar)

**Goal:** Scrape silently and reliably on a schedule, with the app living in the system tray.

← [Phase 1](./01-database-config.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 3](./03-frontend-ui.md)

---

## 2.1 Tray-first lifecycle (`services/tray.rs`)

- [x] `tauri.conf.json`: main window `"visible": false`, `1280×800`, min `960×600`. Enable `tray-icon` feature.
- [x] `TrayIconBuilder` with monochrome template icon (macOS `icon_as_template(true)`).
  - Left-click → show + focus main window (toggle if already focused).
  - Menu: `OPEN SCRAPE` · `RUN SCRAPE NOW` · `PAUSE / RESUME SCHEDULE` · `LAST RUN: 12:00 · 14 NEW` (disabled label) · `QUIT`.
- [x] Window `CloseRequested` → `api.prevent_close()` + `window.hide()` (only `QUIT` exits).
- [x] macOS: `ActivationPolicy::Accessory` while hidden (no Dock icon); switch to `Regular` when window is shown.
- [x] Plugins: `tauri-plugin-single-instance` (second launch focuses window), `tauri-plugin-autostart` (bound to `app.launch_at_login`), `tauri-plugin-notification`, `tauri-plugin-opener` (open job URLs in default browser).

## 2.2 Scheduler (`services/scheduler.rs`)

- [x] On startup: read `scraper.enabled` + `scraper.cron`; build `JobScheduler`, add a `Job::new_async_tz(cron, tz, ...)` using `system.timezone` so "9 AM" means local 9 AM.
- [x] Store the job UUID in `AppState.scheduler: Mutex<SchedulerHandle>`; `reschedule(cron)` removes + re-adds on settings change (hooked in `update_setting`).
- [x] Each tick calls `ScrapeRunner::trigger(Trigger::Schedule)`.
- [x] `run_on_startup` → trigger once ~10s after launch.
- [x] Commands: `validate_cron(expr) -> { ok, next_runs: [ISO; 3] }` (for Settings preview), `get_scheduler_status() -> { enabled, next_run_at, running }`, `set_scheduler_paused(bool)`.

## 2.3 ScrapeRunner (`services/runner.rs`)

- [x] **Overlap guard:** `AtomicBool` / `try_lock` — if a run is in progress, skip (log + return `AlreadyRunning`).
- [x] Flow (see master sequence diagram):
  1. `runs::start(trigger)`; load enabled sources + filters.
  2. `app.shell().sidecar("scrape-worker")?.args(["--stdin"]).spawn()` → `(rx, child)`.
  3. Write `WorkerConfig` JSON to stdin, then close it.
  4. Read `CommandEvent::Stdout` lines → parse `WorkerEvent`; buffer jobs and flush via `jobs::upsert_scraped` every 50 jobs or 500 ms.
  5. Forward `progress`/`error` as `scrape://progress` events; capture stderr to log.
  6. On `Terminated`/`done`: finalize `scrape_runs`, update `sources.last_*`, emit `scrape://finished` + `jobs://changed`.
  7. If `jobs_new > 0` and notifications enabled → "SCRAPE COMPLETE — 14 NEW POSTINGS".
- [x] Hard timeout (default 10 min) → `child.kill()`, mark run `failed`.
- [x] Command `run_scrape_now()` and `list_scrape_runs(limit)`.

### Upsert semantics
```sql
INSERT INTO jobs (...) VALUES (...)
ON CONFLICT(dedupe_key) DO UPDATE SET
  title = excluded.title, location = excluded.location, url = excluded.url,
  role_type = CASE WHEN jobs.role_type = 'unknown' THEN excluded.role_type ELSE jobs.role_type END,
  last_seen_at = excluded.last_seen_at;
-- status, notes, date_added are never touched by scrapes
```
New-vs-existing counted via `RETURNING (date_added = last_seen_at)` or pre-select of keys.

## 2.4 Worker protocol (`crates/scrape-core/src/protocol.rs`)

**stdin → worker**
```json
{
  "version": 1,
  "sources": [
    { "id": "0192…", "kind": "greenhouse", "name": "Stripe", "identifier": "stripe", "config": {} },
    { "id": "0193…", "kind": "jsonld", "name": "Acme", "identifier": "https://acme.com/careers", "config": { "follow_links": "a[href*='/jobs/']", "max_pages": 25 } }
  ],
  "filters": { "include_keywords": ["engineer"], "exclude_keywords": ["senior staff"], "locations": ["remote","new york"], "role_types": [] },
  "limits": { "concurrency": 4, "per_host_delay_ms": 750, "request_timeout_s": 20 }
}
```

**worker → stdout (NDJSON, one object per line)**
```json
{"type":"job","source_id":"0192…","external_id":"5012345","title":"Software Engineer, Data","company":"Stripe","role_type":"full_time","location":"Remote","url":"https://…","date_posted":"2026-10-01T00:00:00Z"}
{"type":"progress","source_id":"0192…","fetched":120,"matched":14}
{"type":"error","source_id":"0193…","message":"HTTP 403 from acme.com","retryable":false}
{"type":"done","stats":{"sources_ok":5,"sources_failed":1,"jobs_emitted":63,"duration_ms":8123}}
```
- [x] Versioned; host rejects unknown major versions. Logs go to **stderr only**.

## 2.5 `scrape-worker` adapters (`crates/scrape-worker`)

Deps: `reqwest` (`rustls-tls`, `json`, `gzip`), `scraper`, `tokio`, `futures`, `clap`, `url`, `texting_robots`.

| Kind | Endpoint / method | Field mapping notes |
|---|---|---|
| `greenhouse` | `GET https://boards-api.greenhouse.io/v1/boards/{token}/jobs` | `id`, `title`, `absolute_url`, `location.name`, `first_published`/`updated_at`; role type via heuristic. |
| `lever` | `GET https://api.lever.co/v0/postings/{slug}?mode=json` | `id`, `text`, `hostedUrl`, `categories.location`, `categories.commitment` → role hint, `createdAt` (ms). |
| `ashby` | `GET https://api.ashbyhq.com/posting-api/job-board/{name}` | `id`, `title`, `jobUrl`, `location`, `employmentType` (`FullTime/PartTime/Intern/Contract/Temporary`), `publishedAt`. |
| `jsonld` | Fetch page(s), parse `<script type="application/ld+json">`, walk objects/arrays/`@graph` for `@type: JobPosting` | `title`, `hiringOrganization.name`, `employmentType`, `jobLocation`, `datePosted`, `url`/`identifier`. Optional `follow_links` selector (depth 1, `max_pages`). |
| `selector` | Fetch page, apply user CSS selectors | `config: { item, title, link, location? }`; external_id = absolute link URL. |

- [x] **Auto-detect** (`scrape-worker --detect <url>`): fetch a careers page and look for `boards.greenhouse.io/{t}`, `job-boards.greenhouse.io/{t}`, `jobs.lever.co/{s}`, `jobs.ashbyhq.com/{n}` → suggest an API source; else check for JSON-LD → `jsonld`; else suggest `selector`. Exposed to UI via `detect_source(url)` command.
- [x] **Dry run** (`scrape-worker --stdin --dry-run`) used by `test_source(id)` command to preview first 10 results in Settings.
- [x] Filtering in-worker (case-insensitive keyword/location match on title + location) so the DB only stores relevant jobs.
- [x] Politeness: UA `Scrape/<ver> (local job tracker)`, robots.txt check for `jsonld`/`selector`, per-host semaphore = 1 + delay, retry 3× with backoff on 429/5xx.
- [x] Unit tests per adapter using recorded JSON/HTML fixtures in `crates/scrape-worker/tests/fixtures/` (no network in tests).

## 2.6 Sidecar bundling

- [x] `tauri.conf.json` → `"bundle": { "externalBin": ["binaries/scrape-worker"] }`.
- [x] `scripts/build-sidecar.mjs`:
  1. Target = `process.env.TAURI_ENV_TARGET_TRIPLE` ?? host triple from `rustc -vV`.
  2. `cargo build -p scrape-worker --release --target <triple>`.
  3. Copy to `src-tauri/binaries/scrape-worker-<triple>[.exe]`.
- [x] Hook into `beforeDevCommand` (`build:sidecar` + `vite`) and `beforeBuildCommand` (`build:sidecar` + `vite build`).
- [x] Capabilities: host spawns from Rust, so no `shell:allow-execute` permission is granted to the webview.

## Acceptance Criteria

- [x] App launches with **no window**; tray icon present on macOS/Windows/Linux; closing the window hides it.
- [x] `scrape-worker --source greenhouse:stripe` prints valid NDJSON standalone.
- [x] Adding a Greenhouse, Lever, Ashby and a JSON-LD source → "Run now" inserts jobs; re-running inserts **0 duplicates** and preserves statuses.
- [x] Changing `scraper.cron` reschedules without restart; overlapping triggers are skipped.
- [x] Killing the worker mid-run marks the run `failed` and the app stays healthy.
- [x] Notification shows count of new jobs.
