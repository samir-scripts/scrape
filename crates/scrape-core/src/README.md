# `crates/scrape-core/src` — Core Engine Implementation

## Overview

The `crates/scrape-core/src` directory contains the core source modules that define the data contracts, classification engines, filtering mechanisms, and IPC wire protocol for the Scrape application.

---

## File Manifest

### [`lib.rs`](lib.rs)
- **Role**: Crate entry point and root module declaration.
- **Exports**: Re-exports all public submodules (`types`, `protocol`, `role`, `filter`, `enrich`).
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
- **Role**: IPC message contracts exchanged between the Tauri host (`src-tauri`) and the scraper sidecar (`scrape-worker`).
- **Key Types**:
  - `WorkerConfig`: JSON payload sent via `stdin` to configure a scrape run (sources, global limits, rate limits, filters).
  - `WorkerLimits`: Concurrency caps, per-host request delays, and global timeouts.
  - `WorkerFilters`: Title keywords, location constraints, role filters, and raw SQL filter expressions.
  - `WorkerSource`: Configuration for an individual job board source (ID, kind, name, identifier URL, custom options).
  - `WorkerEvent`: Streaming NDJSON events emitted over `stdout`:
    - `Started`: Signals run initialization and total sources count.
    - `SourceStarted` / `SourceCompleted`: Source-level progress and metrics.
    - `Job`: A fully scraped and enriched job posting.
    - `SourceError` / `Error`: Non-fatal or fatal error events.
    - `Completed`: Final summary statistics.
  - `WorkerStats`: Aggregated run telemetry (jobs found, jobs filtered, duration).
  - `ScrapedJob`: Transport representation of a job listing emitted by the worker.

### [`role.rs`](role.rs)
- **Role**: Rule-based role categorization engine.
- **Key Functions**:
  - `classify_role(title: &str, hint: Option<&str>) -> RoleType`: Analyzes job titles and ATS categories using keyword heuristics, prioritizing explicit hints (e.g., department name) before matching title patterns (e.g., "Software Engineer", "Data Scientist", "Product Manager", "Summer Intern").

### [`filter.rs`](filter.rs)
- **Role**: SQL-like query evaluation and structured filter engine.
- **Key Functions & Types**:
  - `FilterMatcher`: Evaluates whether a scraped job meets keyword, location, and role criteria before emitting events.
  - `parse_sql_query(query: &str)`: Parses simplified SQL `WHERE` clauses (e.g., `role = 'swe' AND location LIKE '%NY%'`) to dynamically filter job streams during headless or background execution.

---

## Subdirectories

| Directory | Description |
|---|---|
| [`enrich/`](enrich/README.md) | Natural language and heuristic parsing routines for extracting structured salary, location, eligibility, experience, and workplace types from raw job text. |
