# `crates/scrape-worker/tests` — Worker Integration Test Suite

## Overview

The `crates/scrape-worker/tests` directory contains offline and loopback-HTTP integration tests verifying the scraper sidecar's parsing, detection, bounded enrichment, employer branding, and NDJSON subprocess protocol against deterministic fixtures.

---

## File Manifest

| File | Purpose & Covered Scenarios |
|---|---|
| [`adapter_tests.rs`](adapter_tests.rs) | Offline integration tests for all ATS adapters and auto-detection routines. Tests parsing fidelity, field mapping, compensation parsing, multi-location resolution, workplace classification, and HTML entity cleanup against saved fixtures without network requests. |
| [`worker_process.rs`](worker_process.rs) | Subprocess protocol and diagnostic tests (`PROTOCOL_VERSION` enforcement, invalid queries/limits rejection before network access, categorized rejection reasons, overlapping rejection examples, and source scan summaries). |
| [`limit_process.rs`](limit_process.rs) | Loopback-HTTP process tests for per-workflow job caps (`max_jobs_per_run`), bounded Greenhouse detail enrichment (skipping conclusive rejects and duplicates before detail fetches), detail request/time bounds, required vs. optional detail failures, and provenance preservation. |
| [`branding_process.rs`](branding_process.rs) | Loopback-HTTP process tests for employer branding across all five adapters, shared logo asset caching (one fetch per source/employer per run), board-page fallback, custom CSS selectors, and explicit warning diagnostics on missing/invalid artwork. |
| [`coordinator_tests.rs`](coordinator_tests.rs) | Integration tests for multi-source round-robin coordination: fair budget allocation (7/7/6 split across 3 healthy sources at cap 20), starting source rotation, preview non-rotation, capacity redistribution on slow or exhausted sources, workflow-wide deduplication, and explicit loopback runs proving both default public search feeds are examined. |
| [`crawl_process.rs`](crawl_process.rs) | Loopback-HTTP worker-process coverage for bounded same-origin public HTML traversal, JSON-LD/CSS extraction, robots policy, depth/page limits, access denial/login walls, redirects, identity preservation and cross-source fairness. |
| [`browser_proxy.rs`](browser_proxy.rs) | Deterministic CDP proxy/server challenge separation, cancellation after rejected credentials, event retention and secret-free failures. An opt-in installed-Chromium loopback proxy test exercises real authenticated navigation without contacting public sites. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`fixtures/`](fixtures/README.md) | Static mock payloads (JSON API responses, HTML pages, and recorded branding assets) used by offline and loopback-HTTP tests. |

---

Static locator unit tests live with `scrape-core::{locator,xpath}` and
`adapters/selector.rs`. The packaged sidecar → temporary SQLite test
`mixed_xpath_css_fields_survive_worker_preview_refresh_and_persistence` in
[`worker_repository.rs`](../../../src-tauri/tests/worker_repository.rs) verifies
preview isolation, matching/cap counters, source IDs/destinations, hourly cents,
refresh and protected user state. Rebuild the sidecar before running host tests.

## Running Integration Tests

```bash
cargo test -p scrape-worker
```

Proxy transport tests: `cargo test -p scrape-worker --test browser_proxy`.
With Chromium installed (or `SCRAPE_BROWSER_PATH` pointing to its executable),
run the real-browser fixture explicitly:
`cargo test -p scrape-worker --test browser_proxy chromium_authenticated_loopback_proxy -- --ignored --exact`.
The fixture supplies its own loopback proxy and fictional credentials; it does
not use saved user proxy settings.
`client.rs` unit tests also verify absolute-form HTTP proxy requests, Basic proxy
authentication without URL/credential leakage, cross-origin redirect rejection
and no direct-network fallback when the configured proxy is unavailable.

Section 6A adds real-worker pagination/detail request-count tests, late matches,
page limits, cycles, invalid navigation, required/optional unknown evidence,
mixed paginated/single-page 7/7/6 fairness and protocol-v7 preflight bounds.
Controlled-deadline worker unit tests cover source and workflow cutoffs.
`src-tauri/tests/worker_repository.rs` additionally runs three listing pages
(including an empty middle page) and two distinct detail URLs through preview,
insert and refresh, retaining destination/ID/pay/date provenance and protected state.
