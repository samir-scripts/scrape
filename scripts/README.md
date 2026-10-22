# `scripts/` — Build & Quality Automation Scripts

## Overview

The `scripts/` directory contains Node.js automation scripts used during development, continuous integration, and build packaging.

---

## File Manifest

| File | Language / Runtime | Purpose & Execution Flow |
|---|---|---|
| [`generate-template-catalog.mjs`](generate-template-catalog.mjs) | Node.js + Rust | Runs the Rust catalog exporter and writes the picker JSON/YAML projections. `--check` fails on drift; does not touch any database or execute scraping. |
| [`prepare-role-title-seed.py`](prepare-role-title-seed.py) | Python 3 standard library | Normalizes official O*NET 31.0 Occupation Data, Sample of Reported Titles, and Job Titles CSVs into the one bundled TSV; `--check` validates deterministic output. Tests: `python3 -m unittest scripts.test_role_title_seed`. |
| [`test-app-startup.mjs`](test-app-startup.mjs) | Node.js (ESM) | Automated smoke test for CI and local verification. Boots the Vite development server, performs an HTTP GET request against `http://localhost:1420/index.html`, verifies module bundling and HTML transformation without runtime panics, and cleanly terminates the process. |

---

## Usage

```bash
# Compile and stage the scraper sidecar binary
node scripts/build-sidecar.mjs

# Run frontend startup smoke verification
node scripts/test-app-startup.mjs
```

```bash
python3 scripts/prepare-role-title-seed.py --occupation-data /path/to/occupation_data.csv --reported-titles /path/to/sample_of_reported_titles.csv --alternate-titles /path/to/job_titles.csv
python3 scripts/prepare-role-title-seed.py --occupation-data /path/to/occupation_data.csv --reported-titles /path/to/sample_of_reported_titles.csv --alternate-titles /path/to/job_titles.csv --check
python3 -m unittest scripts.test_role_title_seed

```

The source CSV URLs, hashes, license and attribution are documented in
[`crates/scrape-core/data/occupations/manifest.json`](../crates/scrape-core/data/occupations/manifest.json).

`check-style.mjs` checks relevant production class strings rather than comments or
fixtures. `native-smoke.mjs` drives real Tauri IPC with a temporary database, local
HTTP fixture, packaged sidecar and controlled failing processes. Build the sidecar
and debug host first. It verifies cancellation, persistence, block/restore,
protocol failures, timeouts and the external opener without normal app startup
side effects. Playwright browser checks live in `e2e/`.

`test-native-startup.mjs` runs the actual configured Tauri development pipeline
with isolated ports and temporary database fixtures. It verifies native readiness
on fresh and both migration-12 histories, and controlled nonzero refusal of an
unknown checksum without a panic or database mutation. Run `npm run test:native-startup`.

### Isolated native startup and appearance

`test-native-startup.mjs` and `native-smoke.mjs` share `tauri-dev-test.mjs`.
It reserves a local port, launches the actual Tauri dev pipeline with that URL,
shows a temporary-profile window, records completion/logs, and cleans up only its
owned process group. It does not reuse another dev server or use the live database.

The startup matrix covers fresh, recognized version-12/version-19 histories and
unknown checksums. Select specific fixtures with `SCRAPE_STARTUP_MODES`.
`SCRAPE_NATIVE_APPEARANCE=1` additionally exercises real WKWebView theme controls,
all five local font families, native selects, scrollbar fallback colors, icon names,
target sizes, and workflow-dialog font inheritance. For example:

```sh
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_APPEARANCE=1 SCRAPE_SCROLLBAR_MODE=Always npm run test:native-startup
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_APPEARANCE=1 SCRAPE_NATIVE_WIDTH=480 SCRAPE_SCROLLBAR_MODE=Automatic npm run test:native-startup
```

Scrollbar preference arguments are scoped to the launched macOS test process;
the scripts do not change global OS preferences. Appearance results include the
actual WebView user agent and computed theme/scrollbar values in `result.json`.
Computed-style checks do not replace a native screenshot or pointer/keyboard QA.

### Native pagination and responsive inspector

```sh
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_LAYOUT=1 npm run test:native-startup
```

This seeds 105 authored discoveries in a temporary SQLite profile, exercises the
actual toolbar and direct-page controls, resizes the real window through its
compact modal threshold, and verifies the same notes node, draft and scroll
position survive. A reload checks pane preference and notes through actual IPC.
`native-job-layout.mjs` owns these assertions. Debug-only resize controls accept
bounded sizes and are installed only for an explicitly isolated smoke profile.
This check uses keyboard resizing; real native pointer dragging remains manual QA.

### Native Kanban controls

```sh
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_KANBAN=1 npm run test:native-startup
```

`native-kanban.mjs` operates the saved-posting fixture through actual WKWebView
select/keyboard controls and verifies SQLite status/history through IPC. It checks
cancellation and restart, including notes/favorite preservation. It does not
synthesize pointer events or claim to validate OS pointer delivery. Real native
pointer dragging remains an acceptance gate; browser pointer tests live in
`e2e/kanban.pw.ts` and share the pointer helper with the full job journey.

### Native workflow wizard

```sh
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_WORKFLOW=1 npm run test:native-startup
```

This drives actual WKWebView **Roles → Criteria → Sources → Schedule** controls
against bounded local JSON-LD and The Muse-schema feeds using the packaged
sidecar and isolated SQLite. It verifies legacy keyword/YOE/New Grad/geography
rules, preview invalidation, protected status on refresh, and role-first creation
without a career URL: Google/Amazon Software Engineer, Past 12 hours, two accepted
jobs and three explained rejections. Preview/Save persist no jobs or run records;
explicit Run now creates exactly one execution. The selected feed endpoint is
edited through YAML; actual public endpoints are not contacted by this harness.
The HTTP server and process are owned and closed by the launcher. DOM/IPC
automation does not establish physical typing/IME, native chooser gestures or
native visual/zoom acceptance.

Section 7 scheduling summary regression (isolated current migration-19 fixture):

```sh
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_SCHEDULE=1 npm run test:native-startup
```

Exercises actual WKWebView schedule controls and backend IPC: create/update/toggle/delete, manual-only/no enabled workflows, timezone/dispatcher instant agreement, pause/resume, focus restoration, and no implicit scrape. `npm run test:native` additionally verifies running while paused, completion, cancellation and recovery using the actual local sidecar. Do not edit files or generate bindings while a native dev check is running: Vite reload interrupts its page-injected script. The harness checks DOM/IPC; it does not prove native pointer gestures or screenshot appearance.


Plan 11 keyword verification: `SCRAPE_STARTUP_MODES=fresh SCRAPE_NATIVE_WORKFLOW=1 node scripts/test-native-startup.mjs`
uses an isolated profile and local fixture to check two All keywords, themed
schedule selection, real preview and saved configuration. This harness dispatches
DOM events inside WKWebView; it does not establish physical typing/IME or native
file-chooser acceptance. See `plans/11-implementation-evidence.md`.


Plan 11 bounded-enrichment verification uses `cargo test --workspace` with local
HTTP fixtures and temporary databases. `e2e/enrichment-outcomes.pw.ts` checks
Paper/Obsidian outcome displays and contrast using labelled simulated runs.
`npm run test:native` additionally drives optional-warning, required-evidence and
cancelled outcomes through the actual native host/worker IPC and verifies saved
run/source outcomes after reload. These are authored fixtures, not live company
verification. Native scripts use isolated storage; never point them at user data.


Plan 11 employer-logo verification: `node scripts/native-branding.mjs` uses a
temporary native profile, authored local JSON-LD, and the separately captured public
Cloudflare logo asset. It verifies real worker ingestion, one image request per
source/employer/run, 24px/32px WKWebView image decoding and visible orange pixels,
local IPC asset references, offline refresh/reload, notes/favorite/tracking
preservation and saved artwork after source deletion. It does not establish live
career-site ingestion, physical native gestures, or native screenshot acceptance.

`e2e/company-branding.pw.ts` uses labelled synthetic jobs plus the recorded logo
for browser review in both themes at wide/narrow sizes and 200% CSS zoom. Artwork
review does not establish page-wide zoom/layout acceptance. To reproduce the PNG
review fixture through production validation, use
`cargo run -p scrape-core --example cache-logo-fixture -- INPUT OUTPUT.png`.
Capture URL/time/checksums and the original JPEG-format bytes are preserved next
to `crates/scrape-worker/tests/fixtures/cloudflare-logo.json`.

Plan 12 Section 7 delivers manifest-driven provider workflow verification (`npm run test:providers` via `verify_provider_workflows`): exact routes (HTTP 404 on unlisted paths), raw capture SHA-256 checksum verification, full canonical workflow lifecycle (seed, atomic persist, script trust, launch, budget accounting, SQLite persistence, and Discovered queries), and truthful status classification (`passed`, `partial`, `unsupported`, `failed`). Meta remains an explicit unsupported client-side React SPA gap. Live workflow evaluation is opt-in via `--live` and separately reported.
