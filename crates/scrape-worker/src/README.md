# `crates/scrape-worker/src` — Scraper Worker Implementation

## Overview

The `crates/scrape-worker/src` directory contains the source code for the scraper sidecar binary and its underlying asynchronous scraping pipeline.

---

## File Manifest

### [`main.rs`](main.rs)
- **Role**: CLI binary entry point.
- **Responsibilities**:
  - Parses command-line arguments using `clap` (`--detect <URL>`, `--filter-sql <QUERY>`, `--source-id <ID>`, and proxy-aware `--branding`).
  - Initializes tracing subscriber to output diagnostics on `stderr`.
  - Reads `WorkerConfig` from standard input (`stdin`), including execution-only HTTP proxy settings.
  - Streams newline-delimited JSON (`WorkerEvent` NDJSON) to `stdout` in real-time.
  - Reads a proxy configuration JSON line from stdin only for host-launched lazy branding lookups.
  - Handles graceful cancellation upon termination signals.

### [`lib.rs`](lib.rs)
- **Role**: Crate library interface.
- **Responsibilities**:
  - Re-exports `client`, `adapters`, `detect`, and `filter` modules for use in unit and integration test harnesses (`tests/adapter_tests.rs`).

### [`client.rs`](client.rs)
- **Role**: High-performance, polite HTTP networking layer.
- **Key Structs & Methods**:
  - `HttpClient`: Thread-safe HTTP wrapper around `reqwest::Client`.
  - `fetch_text(url, check_robots)`: Fetches raw HTML/JSON with timeout enforcement and optional `robots.txt` compliance checking using `texting_robots`.
  - `fetch_text_same_origin_at`: Traverses static public HTML with checked same-origin redirects and per-hop robots/pacing checks.
  - `with_proxy`: Applies an explicit user-supplied HTTP(S) proxy with Basic credentials; no direct fallback. WebSocket-driven Chromium uses the same proxy with CDP challenge handling.
  - Rate limiting: Manages per-host request spacing; the default is 750 ms.

### [`public_web.rs`](public_web.rs)
- **Role**: Isolated, public-only independent-discovery transport with DNS pinning, four-request concurrency, per-origin serialization and bounded response bodies.
- **Origin admission**: Validates destinations before atomically reserving source origins; scheduler reservations and ordinary redirect destinations share the same authoritative origin cap.
- **Robots policy**: Follows at most five same-origin redirects (six robots requests), resolving relative/query locations and rejecting loops, credentials, private destinations and all origin changes, including HTTP-to-HTTPS. The returned policy is cached for the original origin and enforced before content fetches. Missing or unsuccessful robots responses remain fail-closed; source/global budget exhaustion keeps its original error and is not cached as a denial.
- **Request accounting**: Every contacted robots hop uses the existing pacing, DNS pinning, timeout and source/global request charges under the same origin lock and scheduler permit. Ordinary content redirects independently check the destination origin's policy.

### [`detect.rs`](detect.rs)
- **Role**: Career page ATS auto-detection engine.
- **Key Functions**:
  - `detect_ats(client, url) -> Result<Option<DetectedSource>>`: Probes an arbitrary company website or careers page. Analyzes HTTP redirects, inline `<script>` tags, canonical links, and DOM structures to discover whether the target uses Greenhouse (`boards.greenhouse.io`), Lever (`jobs.lever.co`), Ashby (`ashbyhq.com`), or Schema.org `JobPosting` JSON-LD markup.

### [`coordinator.rs`](coordinator.rs)
- **Role**: Multi-source round-robin coordinator and fair budgeting engine.
- **Responsibilities**:
  - Distributes crawl budget across multiple sources in a round-robin schedule, taking at most 1 unique eligible candidate per source per round.
  - Dynamically redistributes capacity when sources exhaust early or encounter transient errors.
  - Rotates the persisted starting source index across actual runs while leaving preview runs unrotated.
  - Performs workflow-wide canonical deduplication and enforces the authoritative total job cap.
  - Resumes The Muse/Arbeitnow search sessions one page/candidate per turn alongside direct boards, with shared acceptance caps and truthful page/time/unknown-evidence partial outcomes.
  - Validates geographic evidence before emitting candidate diagnostics or jobs. Malformed or oversized evidence is reported as an invalid candidate, without truncating it or terminating other source work.

### [`filter.rs`](filter.rs)
- **Role**: Worker-side stream filtering.
- **Key Structs**:
  - `WorkerFilterEvaluator`: Applies title keyword exclusions, required role categories, and optional SQL query expressions directly in the worker process before streaming events to the host, minimizing IPC overhead.

---

## Subdirectories

| Directory | Description |
|---|---|
| [`adapters/`](adapters/README.md) | Ingestion adapters specialized for individual ATS platforms (Greenhouse, Lever, Ashby, JSON-LD, CSS Selectors). |

Static sources now use `adapters/selector_session.rs` for resumable CSS/XPath or
hydration discovery, bounded next pages and lazy detail enrichment. Coordinator
turns retain one eligible unique acceptance and yield during bounded discovery.

Independent discovery retains literal publication values, their actual page/locator
and observation time. Compatible structured/visible evidence keeps the narrower
real interval; conflicting or timezone-dependent unresolved evidence does not
establish eligibility. Listing dates can reach only the fetched exact detail URL
with matching real requisition IDs, or matching titles when both IDs are absent;
employer/geography fields are always extracted from the detail. Posting-block
closure text and timezone-resolved date-only expiry remain rejection evidence.

Protocol v14 adds final candidate evaluations for persisted runs while preserving ordered commit feedback; proxy configuration and bounded static crawling remain supported. Per-host pacing and robots checks remain authoritative.
`--protocol-version` reports the Rust-owned contract version for external harnesses.
