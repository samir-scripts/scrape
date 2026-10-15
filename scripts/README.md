# `scripts/` — Build & Quality Automation Scripts

## Overview

The `scripts/` directory contains Node.js automation scripts used during development, continuous integration, and build packaging.

---

## File Manifest

| File | Language / Runtime | Purpose & Execution Flow |
|---|---|---|
| [`build-sidecar.mjs`](build-sidecar.mjs) | Node.js (ESM) | Compiles the `scrape-worker` sidecar binary via `cargo build -p scrape-worker --release`, detects the local host target triple via `rustc -vV`, resolves Cargo's actual target directory from metadata (including `CARGO_TARGET_DIR` overrides), and stages the resulting binary into `src-tauri/binaries/scrape-worker-<target>` with executable permissions. |
| [`test-app-startup.mjs`](test-app-startup.mjs) | Node.js (ESM) | Automated smoke test for CI and local verification. Boots the Vite development server, performs an HTTP GET request against `http://localhost:1420/index.html`, verifies module bundling and HTML transformation without runtime panics, and cleanly terminates the process. |

---

## Usage

```bash
# Compile and stage the scraper sidecar binary
node scripts/build-sidecar.mjs

# Run frontend startup smoke verification
node scripts/test-app-startup.mjs
```

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

This runs the actual WKWebView Source/Matching/Schedule/Review controls against a
bounded local JSON-LD fixture and the actual sidecar. It verifies the latest keyword
rule, manual schedule, preview counts, persisted spec, unchanged jobs and no extra
run on Save. The temporary HTTP server and test process are closed by their owners.
This check does not automate the native file chooser; browser chooser coverage
uses `e2e/workflows.pw.ts` and native chooser/visual acceptance remains manual.

Section 7 scheduling summary regression (isolated current migration-19 fixture):

```sh
SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_SCHEDULE=1 npm run test:native-startup
```

Exercises actual WKWebView schedule controls and backend IPC: create/update/toggle/delete, manual-only/no enabled workflows, timezone/dispatcher instant agreement, pause/resume, focus restoration, and no implicit scrape. `npm run test:native` additionally verifies running while paused, completion, cancellation and recovery using the actual local sidecar. Do not edit files or generate bindings while a native dev check is running: Vite reload interrupts its page-injected script. The harness checks DOM/IPC; it does not prove native pointer gestures or screenshot appearance.
