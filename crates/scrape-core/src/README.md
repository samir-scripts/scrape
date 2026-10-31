# `crates/scrape-core/src` — Core Engine Implementation

## Overview

The `crates/scrape-core/src` directory contains the core source modules that define the data contracts, classification engines, filtering mechanisms, and IPC wire protocol for the Scrape application.

---

## File Manifest

### [`lib.rs`](lib.rs)
- **Role**: Crate entry point and root module declaration.
- **Exports**: Re-exports public submodules (`branding`, `enrich`, `filter`, `geography`, `identity`, `locator`, `matching`, `posting_date`, `protocol`, `role`, `types`, `xpath`) and shared ingestion/protocol constants (`PROTOCOL_VERSION`, `INGESTION_LIMITS`).
- **Utilities**: Defines `now_iso() -> String`, generating deterministic UTC timestamps in ISO-8601 millisecond format (`YYYY-MM-DDTHH:MM:SS.sssZ`) used across event logging, run reporting, and database storage.

### [`types.rs`](types.rs)
- **Role**: Primitive domain types and enumerations.
- **Key Types**:
  - `JobStatus`: State of a tracked application (`discovered`, `applied`, `interviewing`, `offer`, `rejected`).
  - `RoleType`: High-level role classification (`swe`, `data`, `product`, `design`, `seasonal`, `other`).
  - `TriState`: Three-state boolean enum (`yes`, `no`, `unknown`) used for visa sponsorship, security clearance, and new-grad eligibility.
  - `SalaryInterval`: Frequency of pay (`hourly`, `monthly`, `yearly`).
  - `SalarySource`: Origin of salary data (`structured`, `parsed_title`, `parsed_body`).
  - `SourceKind`: Ingestion adapter type (`search`, `web_discovery`, `greenhouse`, `lever`, `ashby`, `workday`, `jsonld`, `selector`).
  - `SourceCadence`: Scraping frequency (`hourly`, `six_hours`, `twelve_hours`, `daily`, `manual`).
  - `ClearanceLevel`: Government clearance requirements (`none`, `secret`, `top_secret`, `ts_sci`, `polygraph`, `public_trust`).
  - `WorkAuthRequirement`: Work authorization status (`citizen_only`, `us_person`, `open`, `unknown`).
  - `WorkplaceType`: Working arrangement (`onsite`, `hybrid`, `remote`, `unknown`).
- **Derivations**: Derives `Serialize`, `Deserialize`, `TS` (for TypeScript bindings), and conditionally `sqlx::Type` for database storage.

### [`web_discovery.rs`](web_discovery.rs)
- Validates canonical seed sets, traversal bounds, and optional imported manifest provenance.

### [`locator.rs`](locator.rs) and [`xpath.rs`](xpath.rs)
- **Role**: One locator/regex parser shared by workflow validation and static extraction.
- **Behavior**: CSS and a documented XPath 1.0 location-path subset operate on the same `scraper::Html` tree. Supports relative/absolute paths, child/self/parent/following-sibling steps, descendant shorthand, attribute/text tests, positional predicates, field attributes and named regex captures. Unsupported syntax fails explicitly. XPath is bounded to 4096 bytes, 64 steps and 32 predicates per step; this is not a full XPath implementation.

### [`protocol.rs`](protocol.rs)
- **Role**: Versioned IPC message contracts (`PROTOCOL_VERSION = 15`) exchanged between the Tauri host and isolated worker. Optional proxy credentials are execution-only and excluded from TypeScript bindings.
- **Key Types**:
  - `WorkerConfig`: JSON payload sent via `stdin` to configure a scrape run (protocol version, sources, limits, filters, suppression keys).
  - `WorkerCommitAck`: Correlates one emitted job with its committed inserted, updated or blocked outcome in persisted runs.
  - `CandidateEvaluation`: One bounded, sequenced final decision per examined posting in diagnostic-enabled runs.
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

### [`search_source.rs`](search_source.rs)
`search_source.rs` owns the shared identity, fixed HTTPS collections, credential-free
endpoint rules, and bounded configuration for The Muse, Arbeitnow, Adzuna and Jobicy.
Blank identifiers retain The Muse; provider-specific discovery hints never replace
the canonical local filters.

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
  - Explicit rounded relative ages from web discovery retain their original text and observation time. The strict 48-hour preset accepts them only when the entire possible publication interval lies within the window; other date filters retain their existing unknown-date behavior.

### [`branding.rs`](branding.rs)
- **Role**: Employer identity and logo asset validation.
- **Key Types & Functions**:
  - `LogoEvidence`, `LogoMethod`, `LogoStatus`: Prioritizes explicit provider/JobPosting organization metadata over employer-labelled artwork and custom selectors while rejecting platform favicons, social previews, and mismatched organizations. `is_legacy_ashby_platform_favicon` matches only the old exact Ashby board/CDN favicon evidence pair for read-time filtering.
  - Decodes PNG/JPEG/WebP/SVG within strict byte (512 KiB), dimension (2048px), and memory (32 MiB) bounds, rasterizing standalone SVGs and re-encoding verified artwork into portable cached assets.

### [`role.rs`](role.rs)
- **Role**: Rule-based role categorization engine.
- **Key Functions**:
  - `classify_role(title: &str, hint: Option<&str>) -> RoleType`: Analyzes job titles and ATS categories using keyword heuristics, prioritizing explicit hints (e.g., department name) before matching title patterns.

---

## Subdirectories

| Directory | Description |
|---|---|
| [`enrich/`](enrich/README.md) | Natural language and heuristic parsing routines for extracting structured salary, location, eligibility, experience, workplace types, and normalized descriptions from raw job text. |

### [`hydration.rs`](hydration.rs)

Validates explicit hydration-script locators, RFC 6901 JSON pointers, posting-field maps and regex transforms for both host configuration and static worker extraction. Numeric IDs are preserved; optional unknowns remain unknown. No script execution or automatic decoding of reference-encoded payloads.

### [`static_source.rs`](static_source.rs)

Shared validation of static next/detail locators, detail field maps, readiness,
page/source bounds and identity immutability. Global `WorkerLimits` in protocol
v7 owns default/hard maxima; source settings can lower them. Browser limit
configuration is published, but browser-process/context enforcement awaits 6C.

`protocol.rs` owns `WorkerFilters::resolve_compatibility`: a validated, non-destructive
overlay with explicitly opt-in historical inheritance. It preserves every legacy
predicate alongside structured matching. `compatibility_restrictions()` emits the
generated `FilterCompatibilityRestriction` descriptions consumed by desktop UI;
no keyword/role union, location inference or sample-query deletion is performed.

`MatchingEvidence::from_event` records individually known structured fields without
pretending description/date/pay evidence exists. `ParsedLocation::validate` and
`location::validate_locations` enforce the shared script/host location contract.
`parse_with_country` uses provider context and the subdivision registry for unlisted
cities, preserving raw labels and ambiguous uncontextualized IN/CA as unknown.
Geographic worker/SQLite predicates share tuple, city exact/raw-literal-substring,
ASCII case and Unicode-whitespace semantics; aggregate location text is never borrowed.
Custom script frames are version 2; the worker event/config protocol is 14.
Numeric experience rules expose `ExperienceMatchMode`: omitted domain/filter modes
retain `advertised_range_overlap`, while workflow v7+ omissions resolve to
`required_minimum`. Workflow versions 1–6 explicitly retain overlap on upgrade.
Required-minimum rules compare inclusive bounds against `experience_min`;
maximum-only evidence follows the unknown policy after detail opportunities end.
Native compensation exposes `CompensationMatchMode`: omitted modes retain inclusive
`range_overlap`, while explicit `advertised_amount` compares only evidenced native
endpoints. Rust and SQLite share the same per-range comparisons without unit or
currency conversion.
