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
