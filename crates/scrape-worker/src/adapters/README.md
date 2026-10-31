# `crates/scrape-worker/src/adapters` — ATS & Website Ingestion Adapters

## Overview

The `crates/scrape-worker/src/adapters` directory contains specialized parsers that ingest job listings from diverse web formats and Applicant Tracking Systems (ATS). Each adapter fetches raw data (via REST API endpoints or HTML documents), parses the platform's specific schema, applies the enrichment heuristics from `scrape-core::enrich`, and produces standardized `WorkerEvent::Job` events.

Production uses `coordinator.rs` resumable sessions and shared matching/budgeting; compatibility dispatch uses `scrape_source_with_evidence` and a stopping candidate sink. Greenhouse supplies partial evidence to the shared Rust matching evaluator, skips conclusive rejects/duplicates before bounded details, and fills missing fields without replacing listing identity. Optional enrichment failures emit warnings; missing required evidence remains unresolved. Selector sources now accept mixed CSS/XPath locators with shared Rust field transforms; other adapters retain their complete extraction path. Compatibility `scrape_source_with` and collecting entry points wrap the same dispatch. `PostingGate` owns matching, canonical identity deduplication, the suppression snapshot, and the per-workflow acceptance budget (20 by default; Preview/Test at most 10). All five adapters stop record processing when the sink returns false. Greenhouse stops additional detail enrichment; JSON-LD stops recursive extraction and followed-page requests. Existing vector-returning entry points collect this same path for raw adapter tests. Whole-board JSON response downloads/deserialization can be unavoidable; the cap does not imply provider-side pagination support.

`branding.rs` extracts employer-labelled images and `JobPosting.hiringOrganization.logo`, resolving relative URLs and accepting only HTTP(S) assets without credentials. Ashby theme images are accepted only for matching organization identity and the exact app.ashbyhq.com theme routes; public Ashby data is undocumented and fails closed. Greenhouse, Lever and Ashby make at most one branding-page lookup per source/run, using the first provider-supplied posting URL with robots checks and a four-second total deadline. The resulting logo is reused only for that same employer. JSON-LD and selector sources use metadata already in their fetched HTML. A missing, blocked or failed image remains unknown; platform favicons and guessed company domains are not company branding. Persisted visible jobs can request employer-scoped lookup through native IPC, never from demo or preview controls.

---

## File Manifest

| File | Adapter Target | Data Source & Mechanism | Key Implementation Details |
|---|---|---|---|
| [`mod.rs`](mod.rs) | Adapter Dispatcher | Router | Matches `source.kind` (`Greenhouse`, `Lever`, `Ashby`, `JsonLd`, `Selector`, `Workday`, `Search`, `WebDiscovery`) to its respective asynchronous scraper function. |
| [`branding.rs`](branding.rs) | Shared employer branding | Posting-page HTML and Ashby organization theme data | Bounded lookup and shared logo extraction; validates exact employer association and rejects generic platform favicons. |
| [`greenhouse.rs`](greenhouse.rs) | Greenhouse | JSON REST API | Fetches `https://boards-api.greenhouse.io/v1/boards/{board}/jobs?content=true`. Keeps Remote workplace policy separate from eligible geography; uses office location fields for an explicit hybrid/on-site arrangement and labeled eligible-location sections from normalized descriptions. Fetches provider detail fields and preserves posting provenance. |
| [`lever.rs`](lever.rs) | Lever | JSON REST API | Fetches `https://api.lever.co/v0/postings/{company}`. Parses commitment categories, workplace types, salary ranges, and plain-text descriptions. |
| [`ashby.rs`](ashby.rs) | Ashby | JSON REST API | Fetches `https://api.ashbyhq.com/posting-api/job-board/{board}`. Normalizes compensation tiers, remote/hybrid status, and multi-location lists. |
| [`workday.rs`](workday.rs) | Workday ATS | REST CXS API | Ingests Workday CXS endpoints with session pagination and detail enrichment. |
| [`jsonld.rs`](jsonld.rs) | Schema.org Semantic Web | Embedded HTML `<script>` | Scrapes HTML pages containing `<script type="application/ld+json">` tags. Recursively parses Schema.org `JobPosting` objects, supporting both direct objects and `@graph` array collections. |
| [`selector.rs`](selector.rs), [`selector_session.rs`](selector_session.rs) | Custom Web Pages | CSS/XPath, hydration JSON, bounded HTTP pages/details | Configurable engine for arbitrary career websites. Extracts job items, titles, URLs, locations, and salaries using configurable CSS selectors, regex captures, and URL glob filters (`url_pattern`, `exclude_url_patterns`). |
| [`search.rs`](search.rs) | Cross-employer public discovery | The Muse / Arbeitnow / Adzuna / Jobicy | Resumable strict-schema feed sessions share round-robin matching/caps with direct boards. Adzuna requires execution-only credentials, rejects redirects, paginates from page 1 and never treats predicted/native-unit-unknown pay as annual evidence. Jobicy is remote-only, uses original publication dates and native salary metadata, and validates opaque cursor pagination. Provider queries are discovery hints; canonical Rust filters decide acceptance. |
| [`search_cache.rs`](search_cache.rs) | Jobicy response cache | Hourly bounded JSON pages | SHA-256 keyed, credential-free URL cache with atomic replacement, one-hour expiry and at most 64 pages; cached responses are schema-validated again. |
| [`web_discovery.rs`](web_discovery.rs) | Independent public web | Directory and career-page links, bounded sitemaps, structured and visible HTML | Origin-fair frontier, public-only DNS-pinned requests, robots checks, shared-page requisition identity, and seed/referrer/date provenance. JavaScript rendering is not enabled. |

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

## Static extraction (Section 6A)

`selector.rs` compiles locators and regexes before traversing one HTML5 tree,
reusing that tree for employer branding. CSS containers can use XPath fields and
XPath containers can use CSS fields. Required title/link/configured-ID failures
emit invalid-candidate events; missing optional values remain unknown. A regex
mismatch never falls back to an untransformed field. Existing CSS defaults,
URL patterns, enrichment and the coordinator's acceptance gate remain in use.

The parser lives in `scrape-core::{locator,xpath}` so the host validates exactly
the syntax the worker executes. See [configuration examples](../../../../plans/workflow-v3.md#static-custom-locators-section-6a).
`selector_session.rs` retains candidate/page position across coordinator turns,
fetches at most one new page per turn, and skips details for conclusive rejects.
Details fill missing evidence without changing listing title, destination or ID.
Source/page/workflow limits, cycles and malformed navigation produce truthful
partial coverage, retaining valid jobs. No new requests are scheduled after the
acceptance cap. A source with many rejects yields after 32 events; healthy
paginated and single-page sources retain the shared 7/7/6 distribution at cap 20.
Browser/script execution remains pending.

Hydration extraction now uses `scrape-core::hydration` for shared JSON-pointer, field-map and regex validation, with the same raw posting normalizer as DOM extraction. No embedded scripts are executed. See [configuration](../../../../plans/workflow-v3.md#embedded-hydration-json).

Plan 12 Section 4 evidence: Lever deserializes country and applies it to the primary
location only. Ashby reads primary postalAddress and secondary direct/nested address
forms, retaining raw labels/address components, employmentType and workplaceType
(with the explicit workplace enum taking precedence over isRemote). Greenhouse
keeps ordinary titles Unknown and does not guess custom metadata semantics.
Selector/script evidence delegates to core's per-field resolver; a concrete workplace
can be eligible without a description, while unrelated missing fields stay unresolved.
Primary contracts: [Lever](https://github.com/lever/postings-api),
[Ashby](https://developers.ashbyhq.com/docs/public-job-posting-api),
[Greenhouse](https://docs.greenhouse.io/job-board.html).

Required unresolved matching evidence emits at most five contextual examples per
source, including requested country codes and original location text. The existing
eligibility_incomplete warning flag distinguishes these partial-coverage diagnostics
from optional enrichment warnings in worker summaries and host run accounting.
All unresolved candidates remain counted even after the example bound is reached.
