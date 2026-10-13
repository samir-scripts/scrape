# `src-tauri/binaries` — Bundled Sidecar Executables

## Overview

The `src-tauri/binaries` directory houses target-specific pre-compiled sidecar executables required by Tauri's external binary bundling mechanism.

---

## File Manifest

| File | Target Architecture | Description |
|---|---|---|
| [`scrape-worker-aarch64-apple-darwin`](scrape-worker-aarch64-apple-darwin) | Apple Silicon (macOS `aarch64`) | Compiled standalone binary of `crates/scrape-worker`, named with Tauri's target-triple suffix to allow `tauri::process::Command::sidecar("scrape-worker")` execution at runtime. |

---

## Build & Staging Process

During production builds or local testing, the sidecar binary is compiled from source and staged here by [`scripts/build-sidecar.mjs`](../../scripts/build-sidecar.mjs):

```bash
node scripts/build-sidecar.mjs
```

This ensures the sidecar matches the host's current architecture and toolchain prior to packaging.
