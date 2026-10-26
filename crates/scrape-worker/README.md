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
│   ├── adapters/   # Target-specific parsers (Greenhouse, Lever, Ashby, Workday, JSON-LD, Selector)
│   ├── browser/    # Direct Rust CDP browser engine (stealth, blocking, pagination)
│   ├── client.rs   # Rate-limited HTTP client with robots.txt enforcement
│   ├── coordinator.rs # Multi-source round-robin coordinator and fair budget manager
│   ├── detect.rs   # ATS auto-detection from arbitrary URLs
│   ├── filter.rs   # Worker-side job pre-filtering
│   ├── lib.rs      # Library exports for testing and integration
│   ├── main.rs     # CLI binary entry point (stdin/stdout NDJSON loop)
│   └── script_runner.rs # Versioned stdin/NDJSON script subprocess runner
└── tests/          # Integration tests and ATS fixture responses
    ├── adapter_tests.rs       # Offline adapter and auto-detection verification
    ├── branding_process.rs    # Subprocess tests for employer branding evidence & decoding
    ├── browser_cdp_process.rs # Subprocess tests for direct Rust CDP browser engine
    ├── coordinator_tests.rs   # Multi-source round-robin ingestion & budget tests
    ├── limit_process.rs       # Subprocess tests for bounded ingestion & detail enrichment
    ├── script_runner_process.rs # Subprocess tests for versioned stdin/NDJSON script runner
    ├── worker_process.rs      # Subprocess protocol, diagnostics & error handling tests
    └── fixtures/              # Mock payloads and branding fixtures from real ATS platforms
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

Worker protocol version 11 is current; version 5 introduced enrichment warnings,
unresolved candidates, provider totals, and explicit source outcomes, building on
coverage summaries from version 4. Older protocol configurations fail explicitly.
Rebuild the packaged sidecar with `npm run build:sidecar` after updating host/worker
code. The authoritative workflow parameter is `max_jobs_per_run` (default 20,
range 1–10,000); Preview/Test accepts at most 10. Each workflow stops after its own
eligible budget, including refreshed records. Rejected/invalid/duplicate/blocked
postings do not consume that budget. Summary counts cover examined records only
and identify limit-reached coverage. Shared Rust constants generate frontend
defaults and bounds during binding export.


## Cross-employer search feeds

The search adapter shares the coordinator's candidate turns, source deadlines,
page limits and accepted-job cap with direct boards. The Muse and Arbeitnow keep
their existing feed behavior; Arbeitnow's ingestion timestamp is not an original
publication date.

Adzuna uses only `https://api.adzuna.com/v1/api/jobs/us/search/{page}`, starting at
page 1, with 50 results per page and date ordering. The host hydrates App ID/key
only into execution copies. Its credential-sensitive transport rejects redirects
and emits provider/status or transport-class errors, never request URLs or
response bodies. `what_or`, `where` and whole-day `max_days_old` are discovery
hints; canonical local rules remain authoritative. `created` is advertisement
placement time; returned redirect URLs are preserved. Descriptions are truncated,
not full-ad extraction. Predicted salary is excluded, and bare numeric salary
fields do not prove an annual interval or explicit currency. See the
[Adzuna search contract](https://developer.adzuna.com/docs/search).

Jobicy uses its [public jobs feed](https://jobicy.com/jobs-rss-feed), not the paid
commercial API. This remote feed covers seven days and is delayed three hours;
`Anywhere` does not imply a country. Original `pubDate` and explicit native pay
fields supply date/pay evidence. Requests use `count=100`; opaque cursors are
encoded once and contradictory or repeated pagination metadata produces partial
coverage without discarding valid candidates. Public JSON pages are cached for
one hour beneath the host-provided application cache directory, keyed by the
canonical credential-free URL and bounded to 64 atomic page entries. Missing,
corrupt or expired entries miss; unavailable cache storage fails explicitly
rather than bypassing hourly synchronization. Tests use isolated temporary
caches and loopback replay; none of this establishes live provider availability.

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
Page lookup accepts matching JobPosting logos, employer-labelled images and
Ashby's organization-scoped theme artwork; explicit custom `employer_logo`
selectors remain supported. Ashby theme metadata must match both employer name
and organization ID, and resolves only `app.ashbyhq.com` theme-image paths.
Relative assets resolve against the actual page after redirects. A configured
board page can supply missing posting branding. Platform favicons/social previews
and guessed employer domains are never substitutes.

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
reference through local IPC once. When a persisted job becomes visible, the host
may perform a bounded lookup using identity loaded from that job row; concurrent
lookups and frontend retries are coalesced. Failed refreshes retain the last
verified asset. Source deletion preserves artwork for saved postings; full reset
clears the registry and frontend cache. Legacy Ashby platform favicons are hidden
from read models without rewriting stored rows.


## Multi-source round-robin coordination

When a workflow defines multiple sources (`sources[]`), `scrape-worker` coordinates
ingestion across them using fair round-robin budgeting (`coordinator.rs`).
- The total workflow budget `max_jobs_per_run` is distributed evenly across healthy sources.
- For example, a cap of 20 across 3 healthy sources yields a 7/7/6 split.
- The starting source index rotates across consecutive actual runs (`starting_source_index`)
  so no single source starves downstream boards over time.
- Previews and dry runs do NOT advance the starting source index.
- If a source is exhausted or yields fewer postings than its share, its unused capacity
  redistributes to remaining active sources until the workflow cap is satisfied or all
  sources are exhausted.
- Deduplication is workflow-wide across all sources based on canonical posting identity.

## Static pages and details (protocol 11)

Custom sources use shared CSS/XPath locators or explicit JSON-pointer hydration
mappings. Resumable pages/details use the same matching, identity, suppression
and acceptance coordinator as ATS sources. The default bounds are 100 pages,
180 seconds per static source and 600 seconds per workflow. Global limits cap
per-source overrides. Navigation is bounded to 30 seconds by default; details
also retain the existing maximum-100-request/45-second enrichment allowance.
Page limits, loops, readiness failures and required missing evidence remain
visible outcomes; partial coverage retains confirmed jobs. Source and workflow
limits apply across HTTP, browser and script candidates.

See [configuration examples](../../plans/workflow-v3.md#static-pagination-and-details).

Advanced static selector sources can enable bounded public HTML crawling with
`crawl: { max_pages: 20, max_depth: 2, extraction: "public_html" }`. Seeds are depth
zero; pages are counted as attempted and links are limited to the seed's HTTP(S)
origin. `extraction: "jsonld"` parses structured `JobPosting` data,
`extraction: "selectors"` uses configured locators, and `extraction: "public_html"`
uses automatic page/link heuristics without requiring configured field locators.
All static crawl requests share the source's configured pacing and robots policy.
Robots are respected by default; `ignore_robots: true` bypasses `robots.txt` for that
source only and must remain confined to explicit test/development workflows.
Partial coverage retains confirmed jobs and page/time-limit outcomes. Crawling starts
only from user-provided URLs; it does not discover search-engine results or the web.
An optional secure-store HTTP(S) proxy routes shared HTTP transports and
application-launched Chromium, without enabling a direct-route fallback.
Use `scrape-worker --protocol-version` to query the authoritative version rather
than duplicating it in a harness. Protocol-v10 requests are rejected explicitly.

Browser sources use the direct Rust CDP engine and isolated contexts described
below. Script sources use the versioned NDJSON runner; setup, trust and execution
errors are explicit and never fall back to static scraping.

## Direct Rust CDP browser engine and isolated sessions (Phase 6C)

Sources with `mode: "browser"` use the direct Rust Chrome DevTools Protocol (CDP)
engine (`src/browser/{mod,discovery,cdp,stealth}.rs`). A lazy shared `BrowserPool`
spawns at most one headless Chromium-family process per run (`max_processes = 1`)
with up to two concurrent isolated browser contexts (`max_contexts = 2`) and
immediate tab/context teardown (`Target.closeTarget` / `Target.disposeBrowserContext`)
at the end of each round-robin turn. Before navigation, CDP intercepts and blocks
`Image`, `Media`, `Font`, `Stylesheet` and analytics/tracker domains, injects
pre-document stealth overrides (`Page.addScriptToEvaluateOnNewDocument`), and
injects domain-scoped cookies from `<SCRAPE_SESSION_DIR>/<session_id>.json`.
Wait-for selectors (`wait_for`), repeated `load_more` clicks with DOM-growth
detection, `next_page` links, bounded detail-page rendering, and login-wall
detection (`CoverageTermination::NeedsLogin`) feed into the same authoritative
`SelectorAdapter` and matching pipeline as static extraction.

## Script evidence contract (Plan 12 Section 4)

Custom scripts speak protocol **2**, distinct from host/worker protocol 11. Candidate
frames optionally provide employment_type, a snake-case workplace_type enum and
validated locations tuples; Rust owns matching/acceptance. Unknown frame fields
and version-1 frames fail explicitly. The supported pay text field is salary.
See the [current contract](../../plans/workflow-v3.md#field-evidence-and-script-protocol-2-plan-12-section-4).
Changed bundled/user scripts invalidate local trust and require review; no workflow
is automatically rewritten or enabled. Bundled Netflix workplace mapping remains
limited to recorded evidence; its fixture success is not live coverage proof.

## Bundled script examples (Plan 12 Section 5)

Netflix/Amazon example code is embedded by the independent company presets and
exported inline in YAML. Desktop preflight requires an explicit absolute Node.js
18+ executable and local per-source trust. Recorded route replay verifies only
the published captures and expected outcomes; it does not establish current live
provider availability.
