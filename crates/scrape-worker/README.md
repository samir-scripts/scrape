# `crates/scrape-worker` — Scraper Sidecar Binary & ATS Extraction Engine

## Overview

`scrape-worker` is the execution engine responsible for fetching, detecting, parsing, and streaming job listings from external career pages and Applicant Tracking Systems (ATS). It is designed to run both as an asynchronous library within test harnesses and as an isolated sidecar child process managed by the Tauri application host.

---

## Architecture & Subprocess Protocol

When spawned as a sidecar:
1. **Invocation**: The Tauri host (`src-tauri/src/services/runner.rs`) executes `scrape-worker` with optional arguments (`--source-id`, `--filter-sql`, or `--detect`).
2. **Configuration**: The worker reads a serialized `WorkerConfig` JSON object from standard input (`stdin`).
3. **Execution**: Tokio tasks concurrently fetch job boards with per-host polite rate limiting and robots.txt compliance.
4. **Streaming Output**: As jobs are parsed and enriched, the worker outputs Line-Delimited JSON (`WorkerEvent` NDJSON) to standard output (`stdout`), enabling real-time UI updates before the full crawl completes.
5. **Logging**: Diagnostic logs are emitted to standard error (`stderr`) via `tracing-subscriber`.

```
┌─────────────────────────┐
│     Tauri Host Process  │
└────────────┬────────────┘
             │ stdin: WorkerConfig
             ▼
┌─────────────────────────┐
│  scrape-worker (Sidecar)│
│  - HttpClient / Robots  │
│  - Adapters & Parsers   │
│  - Data Enrichment      │
└────────────┬────────────┘
             │ stdout: WorkerEvent (NDJSON stream)
             ▼
┌─────────────────────────┐
│ Tauri ScrapeRunner Loop │
└─────────────────────────┘
```

---

## Directory Structure

```
scrape-worker/
├── Cargo.toml      # Sidecar dependencies (reqwest, scraper, tokio, clap, etc.)
├── src/            # Core worker modules and binary entry point
│   ├── adapters/   # Target-specific parsers (Greenhouse, Lever, Ashby, JSON-LD, Selector)
│   ├── client.rs   # Rate-limited HTTP client with robots.txt enforcement
│   ├── detect.rs   # ATS auto-detection from arbitrary URLs
│   ├── filter.rs   # Worker-side job pre-filtering
│   ├── lib.rs      # Library exports for testing and integration
│   └── main.rs     # CLI binary entry point (stdin/stdout NDJSON loop)
└── tests/          # Integration tests and ATS fixture responses
    ├── adapter_tests.rs  # End-to-end adapter verification tests
    └── fixtures/         # Mock payloads from real ATS platforms
```

---

## CLI Usage

The worker binary supports standalone manual execution for debugging and dry-run testing:

```bash
# Detect ATS platform for a URL
cargo run -p scrape-worker -- --detect https://stripe.com/jobs

# Run with config passed through stdin
cat config.json | cargo run -p scrape-worker -- --stdin

# Stop after 20 matching jobs from one source (the default)
cargo run -p scrape-worker -- --source lever:example --max-jobs-per-run 20

# Test source extraction with an output cap of 10
cargo run -p scrape-worker -- --source jsonld:https://example.com/careers --dry-run
```

## Extraction integrity

Invalid queries fail worker startup before extraction. `--source` splits only the
first colon, so JSON-LD and selector identifiers can be complete URLs.
Provider destinations are retained; Greenhouse URLs are not rewritten. Its update
time is not a posting date: publication stays unknown when `first_published` is
absent. Detail requests are bounded and cached. ATS board slugs are not company domains.
JSON-LD relative destinations resolve against their extraction page; missing or
unsafe destinations emit errors rather than borrowing the listing URL. Worker
HTTP fixtures and process tests are deterministic and do not verify live providers.

Worker protocol version 4 adds explicit primary rejection categories, overlapping examples, acceptance counts, and coverage termination to source summaries. Older protocol configurations fail explicitly. Rebuild the packaged sidecar with `npm run build:sidecar` after updating host/worker code. The authoritative workflow parameter is `max_jobs_per_run` (default 20, range 1–10,000); Preview/Test accepts at most 10. Each workflow stops after its own eligible budget, including refreshed records. Rejected/invalid/duplicate/blocked postings do not consume that budget. Summary counts cover examined records only and identify limit-reached coverage. The shared Rust constants also generate frontend defaults and bounds during binding export.
