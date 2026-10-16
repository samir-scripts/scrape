# `crates/scrape-core/src` — Core Engine Implementation

## Overview

The `crates/scrape-core/src` directory contains the core source modules that define the data contracts, classification engines, filtering mechanisms, and IPC wire protocol for the Scrape application.

---

## File Manifest

### [`lib.rs`](lib.rs)
- **Role**: Crate entry point and root module declaration.
- **Exports**: Re-exports public submodules (`branding`, `enrich`, `filter`, `geography`, `identity`, `matching`, `posting_date`, `protocol`, `role`, `types`) and shared ingestion/protocol constants (`PROTOCOL_VERSION`, `INGESTION_LIMITS`).
- **Utilities**: Defines `now_iso() -> String`, generating deterministic UTC timestamps in ISO-8601 millisecond format (`YYYY-MM-DDTHH:MM:SS.sssZ`) used across event logging, run reporting, and database storage.

### [`types.rs`](types.rs)
- **Role**: Primitive domain types and enumerations.
- **Key Types**:
  - `JobStatus`: State of a tracked application (`discovered`, `applied`, `interviewing`, `offer`, `rejected`).
  - `RoleType`: High-level role classification (`swe`, `data`, `product`, `design`, `seasonal`, `other`).
  - `TriState`: Three-state boolean enum (`yes`, `no`, `unknown`) used for visa sponsorship, security clearance, and new-grad eligibility.
  - `SalaryInterval`: Frequency of pay (`hourly`, `monthly`, `yearly`).
  - `SalarySource`: Origin of salary data (`structured`, `parsed_title`, `parsed_body`).
  - `SourceKind`: Ingestion adapter type (`greenhouse`, `lever`, `ashby`, `jsonld`, `selector`).
  - `SourceCadence`: Scraping frequency (`hourly`, `six_hours`, `twelve_hours`, `daily`, `manual`).
  - `ClearanceLevel`: Government clearance requirements (`none`, `secret`, `top_secret`, `ts_sci`, `polygraph`, `public_trust`).
  - `WorkAuthRequirement`: Work authorization status (`citizen_only`, `us_person`, `open`, `unknown`).
  - `WorkplaceType`: Working arrangement (`onsite`, `hybrid`, `remote`, `unknown`).
- **Derivations**: Derives `Serialize`, `Deserialize`, `TS` (for TypeScript bindings), and conditionally `sqlx::Type` for database storage.

### [`protocol.rs`](protocol.rs)
- **Role**: Versioned IPC message contracts (`PROTOCOL_VERSION = 6`) exchanged between the Tauri host (`src-tauri`) and the scraper sidecar (`scrape-worker`).
- **Key Types**:
  - `WorkerConfig`: JSON payload sent via `stdin` to configure a scrape run (protocol version, sources, limits, filters, suppression keys).
  - `WorkerLimits` & `EnrichmentLimits`: Concurrency caps, per-host request delays, global timeouts, and bounded detail enrichment limits (`max_requests` 0–100, `timeout_ms` 1–45,000).
  - `WorkerFilters`: Structured `PostingRules`, legacy keyword/location/role constraints, and SQL filter expressions.
  - `WorkerSource`: Configuration for an individual job board source (ID, kind, name, identifier URL, custom options).
  - `WorkerEvent`: Streaming NDJSON events emitted over `stdout`:
    - `Started`: Signals run initialization and total sources count.
    - `SourceStarted` / `SourceCompleted`: Source-level progress and `SourceScanStats` summary (examined, accepted, rejected by primary reason, duplicates, invalid, blocked, unresolved, warnings, provider total, `CoverageTermination`, `ScanOutcome`).
    - `Job`: A scraped, matched, and enriched job posting (`ScrapedJob`).
    - `Branding`: Verified or failed employer logo report per accepted source/employer relationship.
    - `Warning`: Non-fatal optional detail or branding enrichment warning.
    - `SourceError` / `Error`: Source-level or fatal error events.
    - `Completed`: Final `WorkerStats` summary statistics.

### [`matching.rs`](matching.rs)
- **Role**: Authoritative matching engine and diagnostic reason classifier.
- **Key Types & Functions**:
  - `PostingRules` & `KeywordRules`: Structured matching rules for Any/All/Exclude keywords across selected fields (`Title`, `Department`, `Location`, `Description`), workplace arrangements, countries/subdivisions, native compensation, experience, eligibility, and posting dates.
  - `MatchingEvaluator`: Evaluates both complete postings and partial listing evidence (`PartialEvidenceDecision`), distinguishing conclusive rejections from unresolved fields that merit bounded detail enrichment.
  - `RejectionReason`, `RejectionCount`, & `RejectionExample`: Assigns one deterministic primary rejection category per rejected candidate while recording all overlapping failure categories in bounded representative examples.

### [`filter.rs`](filter.rs)
- **Role**: SQL-like query evaluation and SQLite query compiler.
- **Key Functions & Types**:
  - `FilterMatcher`: Evaluates legacy/expert SQL `WHERE` queries and structured `PostingRules` using shared predicates with `matching.rs`.
  - Parameterized SQLite compilation: Compiles `PostingRules` and `JobFilter` into bound SQLite predicates for repository queries.

### [`identity.rs`](identity.rs)
- **Role**: Canonical posting identity and URL normalization.
- **Key Types & Functions**:
  - `PostingIdentity`: Computes stable canonical keys across Greenhouse, Lever, Ashby, JSON-LD, and CSS selector postings so duplicate detection, rescraping, soft removal, and cross-workflow blocking agree.

### [`geography.rs`](geography.rs)
- **Role**: ISO country and qualified subdivision normalization.
- **Key Types & Functions**:
  - Loads `data/regions.json` and `isocountry` alpha-2 codes, exports `CountryOption`, `RegionOption`, and `geographyCatalog.ts`, and resolves structured address fields and bounded location aliases without guessing ambiguous abbreviations.

### [`posting_date.rs`](posting_date.rs)
- **Role**: Publication timestamp parsing and date-filter evaluation.
- **Key Types & Functions**:
  - Normalizes authentic publication timestamps (never substituting provider `updated_at` edit times or scrape discovery times) and evaluates relative-day or calendar-date bounds in the workflow's IANA timezone.

### [`branding.rs`](branding.rs)
- **Role**: Employer identity and logo asset validation.
- **Key Types & Functions**:
  - `LogoEvidence`, `LogoMethod`, `LogoStatus`: Prioritizes explicit provider/JobPosting organization metadata over employer-labelled artwork and custom selectors while rejecting platform favicons, social previews, and mismatched organizations.
  - Decodes PNG/JPEG/WebP/SVG within strict byte (512 KiB), dimension (2048px), and memory (32 MiB) bounds, rasterizing standalone SVGs and re-encoding verified artwork into portable PNGs.

### [`role.rs`](role.rs)
- **Role**: Rule-based role categorization engine.
- **Key Functions**:
  - `classify_role(title: &str, hint: Option<&str>) -> RoleType`: Analyzes job titles and ATS categories using keyword heuristics, prioritizing explicit hints (e.g., department name) before matching title patterns.

---

## Subdirectories

| Directory | Description |
|---|---|
| [`enrich/`](enrich/README.md) | Natural language and heuristic parsing routines for extracting structured salary, location, eligibility, experience, workplace types, and normalized descriptions from raw job text. |
