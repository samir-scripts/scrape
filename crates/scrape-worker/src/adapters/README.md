# `crates/scrape-worker/src/adapters` — ATS & Website Ingestion Adapters

## Overview

The `crates/scrape-worker/src/adapters` directory contains specialized parsers that ingest job listings from diverse web formats and Applicant Tracking Systems (ATS). Each adapter fetches raw data (via REST API endpoints or HTML documents), parses the platform's specific schema, applies the enrichment heuristics from `scrape-core::enrich`, and produces standardized `WorkerEvent::Job` events.

Production dispatch uses `scrape_source_with_evidence` and a stopping candidate sink. Greenhouse supplies partial evidence to the shared Rust matching evaluator, skips conclusive rejects/duplicates before bounded details, and fills missing fields without replacing listing identity. Optional enrichment failures emit warnings; missing required evidence remains unresolved. Selector sources now accept mixed CSS/XPath locators with shared Rust field transforms; other adapters retain their complete extraction path. Compatibility `scrape_source_with` and collecting entry points wrap the same dispatch. `PostingGate` owns matching, canonical identity deduplication, the suppression snapshot, and the per-workflow acceptance budget (20 by default; Preview/Test at most 10). All five adapters stop record processing when the sink returns false. Greenhouse stops additional detail enrichment; JSON-LD stops recursive extraction and followed-page requests. Existing vector-returning entry points collect this same path for raw adapter tests. Whole-board JSON response downloads/deserialization can be unavoidable; the cap does not imply provider-side pagination support.

`branding.rs` extracts employer-labelled images and `JobPosting.hiringOrganization.logo`, resolving relative URLs and accepting only HTTP(S) assets without credentials. Greenhouse, Lever and Ashby make at most one branding-page lookup per source/run, using the first provider-supplied posting URL with robots checks and a four-second total deadline. The resulting logo is reused only for that same employer. JSON-LD and selector sources use metadata already in their fetched HTML. A missing, blocked or failed image remains unknown; platform favicons and guessed company domains are not company branding. Saved postings acquire missing logos through normal rescraping, preserving existing images when no replacement is supplied.

---

## File Manifest

| File | Adapter Target | Data Source & Mechanism | Key Implementation Details |
|---|---|---|---|
| [`mod.rs`](mod.rs) | Adapter Dispatcher | Router | Matches `source.kind` (`Greenhouse`, `Lever`, `Ashby`, `JsonLd`, `Selector`) to its respective asynchronous scraper function. |
| [`branding.rs`](branding.rs) | Shared employer branding | Posting-page HTML | Bounded optional lookup and shared logo extraction, URL validation and employer association. |
| [`greenhouse.rs`](greenhouse.rs) | Greenhouse | JSON REST API | Fetches `https://boards-api.greenhouse.io/v1/boards/{board}/jobs?content=true`. Keeps Remote workplace policy separate from eligible geography; uses office location fields for an explicit hybrid/on-site arrangement and labeled eligible-location sections from normalized descriptions. Fetches provider detail fields and preserves posting provenance. |
| [`lever.rs`](lever.rs) | Lever | JSON REST API | Fetches `https://api.lever.co/v0/postings/{company}`. Parses commitment categories, workplace types, salary ranges, and plain-text descriptions. |
| [`ashby.rs`](ashby.rs) | Ashby | JSON REST API | Fetches `https://api.ashbyhq.com/posting-api/job-board/{board}`. Normalizes compensation tiers, remote/hybrid status, and multi-location lists. |
| [`jsonld.rs`](jsonld.rs) | Schema.org Semantic Web | Embedded HTML `<script>` | Scrapes HTML pages containing `<script type="application/ld+json">` tags. Recursively parses Schema.org `JobPosting` objects, supporting both direct objects and `@graph` array collections. |
| [`selector.rs`](selector.rs) | Custom Web Pages | CSS Selectors + Regex | Configurable engine for arbitrary career websites. Extracts job items, titles, URLs, locations, and salaries using configurable CSS selectors, regex captures, and URL glob filters (`url_pattern`, `exclude_url_patterns`). |

---

## Adapter Data Flow

Geography uses the shared Rust country/subdivision catalog and bounded alias rules. Structured JSON-LD address fields retain their meaning (for example CA country with NL province is Canada/Newfoundland, not California/Netherlands). All location-array entries survive extraction and nested Country objects are supported. Remote applicant eligibility overrides office geography; labeled eligible-location sections precede physical fallback for remote roles. Explicit workplace fields override weaker location/description hints, with word-boundary aliases for in-person/in-office/on-site, Remote and Hybrid.

```
                      WorkerSource Config
                               │
                               ▼
                        [adapters::mod.rs]
                               │
       ┌───────────────┬───────┴───────┬───────────────┬───────────────┐
       ▼               ▼               ▼               ▼               ▼
[greenhouse.rs]   [lever.rs]      [ashby.rs]      [jsonld.rs]     [selector.rs]
 (REST JSON)     (REST JSON)     (REST JSON)     (HTML Script)    (CSS & Regex)
       │               │               │               │               │
       └───────────────┴───────┬───────┴───────────────┴───────────────┘
                               │
                               ▼
               Enrichment Engine (scrape-core::enrich)
               (Salary, Location, Workplace, Eligibility)
                               │
                               ▼
                      WorkerEvent::Job
```

## Static locator extraction (Section 6A, first checkpoint)

`selector.rs` compiles locators and regexes before traversing one HTML5 tree,
reusing that tree for employer branding. CSS containers can use XPath fields and
XPath containers can use CSS fields. Required title/link/configured-ID failures
emit invalid-candidate events; missing optional values remain unknown. A regex
mismatch never falls back to an untransformed field. Existing CSS defaults,
URL patterns, enrichment and the coordinator's acceptance gate remain in use.

The parser lives in `scrape-core::{locator,xpath}` so the host validates exactly
the syntax the worker executes. See [configuration examples](../../../../plans/workflow-v3.md#static-custom-locators-section-6a).
Resumable selector pagination/details, hydration extraction and browser/script
execution are still pending; this checkpoint does not make selector loading lazy.
