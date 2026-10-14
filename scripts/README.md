# `scripts/` — Build & Quality Automation Scripts

## Overview

The `scripts/` directory contains Node.js automation scripts used during development, continuous integration, and build packaging.

---

## File Manifest

| File | Language / Runtime | Purpose & Execution Flow |
|---|---|---|
| [`build-sidecar.mjs`](build-sidecar.mjs) | Node.js (ESM) | Compiles the `scrape-worker` sidecar binary via `cargo build -p scrape-worker --release`. Detects the local host target triple via `rustc -vV` (e.g. `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`) and stages the resulting binary into `src-tauri/binaries/scrape-worker-<target>` with executable permissions. |
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
