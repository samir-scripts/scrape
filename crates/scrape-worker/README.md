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

Worker protocol version 5 added enrichment warnings, unresolved candidates, provider totals, and explicit source outcomes to the rejection/coverage summaries introduced in version 4. Older protocol configurations fail explicitly. Rebuild the packaged sidecar with `npm run build:sidecar` after updating host/worker code. The authoritative workflow parameter is `max_jobs_per_run` (default 20, range 1–10,000); Preview/Test accepts at most 10. Each workflow stops after its own eligible budget, including refreshed records. Rejected/invalid/duplicate/blocked postings do not consume that budget. Summary counts cover examined records only and identify limit-reached coverage. The shared Rust constants also generate frontend defaults and bounds during binding export.


## Bounded Greenhouse enrichment

The listing request uses `content=true`. Its original ID, title and destination
identify the provider snapshot; detail responses fill missing fields without
replacing that identity or a nonempty listing description. `updated_at` remains
provider edit time, never publication time. This precedence makes title/role
rejection safe before detail requests. Field availability follows the
[Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html), with authored
HTTP fixtures proving the extraction contract rather than claiming live coverage.

The shared Rust matching evaluator distinguishes conclusive rejection from
unresolved evidence. Known rejects and repeated canonical identities spend no
detail allowance. Missing fields receive a detail opportunity before final
matching; failed required evidence remains unresolved and is not persisted.
Explicit include-unknown rules still apply after the enrichment opportunity ends.
Accepted candidates receive optional details/branding within the remaining bound.
The sink stops new processing/requests immediately at the acceptance cap.

`WorkerLimits.enrichment` optionally sets `max_requests` (0–100, default 100) and
`timeout_ms` (1–45,000, default 45,000). The deadline begins after listing discovery,
includes normalization and optional branding, and bounds each detail request.
Invalid limits fail before network access. Request counts cover detail attempts;
branding keeps its existing bounded lookups and shares the time deadline. A
whole-board download may still occur before candidate processing stops.

Optional detail/branding failures emit `warning` events, not failed-posting errors.
Valid details missing required fields yield partial coverage. Source summaries
reconcile accepted + blocked + rejected + duplicates + invalid + unresolved with
examined, group rejections by one primary reason, and keep overlapping examples
explicit. Provider totals are separate from examined counts. Outcomes are
`success`, `success_with_warnings`, `partial`, `failed`, and `cancelled` (the host
owns explicit cancellation). Neither missing date/pay nor absent logo metadata
is fabricated. Preview warnings permit saving; required-evidence failures prevent
a successful preview. Rebuild the sidecar after changing protocol or adapters.


## Employer branding (protocol 6)

Every extraction path uses shared employer-logo evidence. `__company` is an explicit
employer hint, separate from the workflow display name. Provider names and matching
JobPosting organizations take precedence; missing identity stays Unknown company.
Page lookup accepts matching JobPosting logos, employer-labelled images, then an
explicit custom `employer_logo` selector. Relative assets resolve against the actual
page after redirects. A configured board page can supply missing posting branding.
Platform favicons/social previews and guessed employer domains are not substitutes.

Protocol 6 adds logo evidence to candidate records and one `branding` report per
accepted source/employer relationship. Older protocol requests fail explicitly.
Accepted relationships share a four-second asset-download deadline and at most
512 KiB per asset. PNG/JPEG/WebP are decoded by content, with dimensions bounded to
2048px and a decoder allocation allowance of 32 MiB. SVG artwork is rasterized
without external images or fonts; unsupported/empty artwork fails explicitly.
The portable cache stores PNG only. Asset caching cannot discover additional jobs
or consume acceptance slots; detail/page discovery still stops at the job cap.
Missing metadata is distinct from download and decoding failure. Only failures
emit optional enrichment warnings; they do not invalidate accepted jobs.

The desktop host validates reports again and stores evidence URL/method, check
time, content checksum and one local image per source/employer relationship. Job
responses contain references, not duplicate image data. The shared UI reads each
reference through local IPC once, and retains the last verified asset during a
failed refresh. Source deletion preserves artwork for saved postings; full reset
clears the registry and frontend cache. No logo is inferred for historical records
without evidence. Refresh supplies new evidence without changing user annotations.
