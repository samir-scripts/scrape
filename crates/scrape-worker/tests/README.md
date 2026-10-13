# `crates/scrape-worker/tests` — Worker Integration Test Suite

## Overview

The `crates/scrape-worker/tests` directory contains offline integration tests verifying the scraper sidecar's parsing, detection, and extraction pipelines against static test fixtures.

---

## File Manifest

| File | Purpose & Covered Scenarios |
|---|---|
| [`adapter_tests.rs`](adapter_tests.rs) | Comprehensive offline integration tests for all ATS adapters and auto-detection routines. Tests parsing fidelity, field mapping, compensation parsing, multi-location resolution, workplace classification, and HTML entity cleanup against saved real-world fixtures without network requests. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`fixtures/`](fixtures/README.md) | Static mock payloads (JSON API responses and HTML pages) captured from real-world career sites. |

---

## Running Integration Tests

```bash
cargo test -p scrape-worker --test adapter_tests
```
