# `crates/scrape-core` — Shared Domain Models & Enrichment Library

## Overview

`scrape-core` defines the core data contracts, wire protocols, filtering engine, and text enrichment algorithms used across the Scrape ecosystem. It is consumed by:
1. **The Tauri Host (`src-tauri`)**: For SQLite database mappings (`sqlx::Type`), IPC serialization, and Tauri command interfaces.
2. **The Worker Sidecar (`scrape-worker`)**: For reading worker configurations, parsing ATS payloads into standard events, and filtering jobs.
3. **The React Frontend (`src/`)**: Via auto-generated TypeScript type definitions (`ts-rs`) exported to `src/bindings/`.

---

## Directory Structure

```
scrape-core/
├── Cargo.toml      # Package manifest with optional sqlx features and ts-rs bindings
├── data/           # Pinned ISO country and subdivision catalog (regions.json)
├── examples/       # Reproducible branding asset validator (cache-logo-fixture.rs)
└── src/            # Core domain modules
    ├── branding.rs     # Employer logo evidence resolution, decoding, and PNG normalization
    ├── enrich/         # Heuristic analysis algorithms (salary, location, workplace, HTML, etc.)
    ├── extraction.rs   # Extraction modes, script validation, canonical hashes and shared bounds
    ├── filter.rs       # SQL filter compilation and legacy/query predicate evaluation
    ├── geography.rs    # Country/subdivision catalog and bounded geographic normalization
    ├── identity.rs     # Canonical posting identity and URL normalization
    ├── lib.rs          # Crate root, module exports, and timestamp utilities
    ├── matching.rs     # Authoritative keyword/geography/workplace/partial-evidence matcher & diagnostics
    ├── posting_date.rs # Publication timestamp normalization and timezone-aware date rules
    ├── protocol.rs     # Subprocess IPC contracts (WorkerConfig, WorkerEvent, NDJSON v7)
    ├── role.rs         # Role category classification heuristics (SWE, Data, etc.)
    └── types.rs        # Core enums and domain structures (JobStatus, RoleType, etc.)
```

---

## File Manifest

| File | Responsibilities & Exports |
|---|---|
| [`Cargo.toml`](Cargo.toml) | Declares crate metadata and dependencies (`serde`, `chrono`, `ts-rs`, `regex`, `image`, `resvg`, `isocountry`, optional `sqlx`). |
| [`data/`](data/README.md) | Contains `regions.json` and documentation for the Rust-owned geography catalog exported to `src/bindings/geographyCatalog.ts`. |
| [`examples/cache-logo-fixture.rs`](examples/cache-logo-fixture.rs) | CLI example that runs raw logo bytes through production `branding` validation to reproduce review PNGs. |
| [`src/lib.rs`](src/README.md) | Crate root. Re-exports public submodules and shared utilities (`now_iso()`, `INGESTION_LIMITS`, `PROTOCOL_VERSION`). |
| [`src/types.rs`](src/README.md#typesrs) | Canonical domain enumerations: `JobStatus`, `RoleType`, `TriState`, `SalaryInterval`, `SalarySource`, `SourceKind`, `SourceCadence`, `ClearanceLevel`, `WorkAuthRequirement`, and `WorkplaceType`. |
| [`src/protocol.rs`](src/README.md#protocolrs) | IPC protocol v6 definitions for host-worker communication: `WorkerConfig`, `WorkerLimits`, `EnrichmentLimits`, `WorkerFilters`, `WorkerSource`, `WorkerEvent`, `WorkerStats`, `SourceScanStats`, `ScanOutcome`, `CoverageTermination`, and `ScrapedJob`. |
| [`src/matching.rs`](src/README.md#matchingrs) | Single authoritative owner of `PostingRules` evaluation, `KeywordRules` (Any/All/Exclude/fields), partial-evidence decisions for bounded enrichment, primary `RejectionReason` categorization, and `RejectionExample` diagnostics. |
| [`src/filter.rs`](src/README.md#filterrs) | SQL-style filter parser and evaluator plus parameterized SQLite `WHERE` clause compilation for saved-job queries and `PostingRules`. |
| [`src/identity.rs`](src/README.md#identityrs) | Canonical `PostingIdentity` derivation across ATS providers, JSON-LD, and custom selectors so deduplication, rescraping, and suppression share one identity rule. |
| [`src/geography.rs`](src/README.md#geographyrs) | ISO alpha-2 country and qualified subdivision (`US-CA`, `CA-ON`, etc.) catalog, alias resolution, and exported `CountryOption`/`RegionOption` bindings. |
| [`src/posting_date.rs`](src/README.md#posting_daters) | Authoritative publication date parsing and calendar-day comparison in an explicit IANA timezone without conflating provider edit timestamps with publication dates. |
| [`src/branding.rs`](src/README.md#brandingrs) | Employer identity precedence, `LogoEvidence`/`LogoStatus`/`LogoMethod`, bounded image decoding (PNG/JPEG/WebP/SVG), and canonical PNG re-encoding. |
| [`src/role.rs`](src/README.md#rolers) | Role classification engine that analyzes job titles and external hints to categorize postings into `RoleType` (`swe`, `data`, `product`, `design`, `seasonal`, `other`). |
| [`src/enrich/`](src/enrich/README.md) | Heuristic parsing engine for unstructured ATS fields: native compensation ranges, multi-location strings, security clearance, visa sponsorship, work authorization, experience/new-grad detection, workplace arrangement, and structured HTML normalization. |

---

## TypeScript Binding Generation

Types marked with `#[derive(TS)]` from `ts-rs` automatically synchronize Rust models with TypeScript types in `src/bindings/` when running the workspace export tests:

```bash
npm run gen:types
# or: cargo test --workspace export_bindings
```

Compensation now includes separate native ranges in fixed-point hundredths of a
currency's major unit, with interval, currency, source evidence, scope and derived
status. `CompensationFilter` evaluates native ranges and compiles parameterized
SQLite predicates. Hourly/annual filters require the original matching interval;
annual equivalents remain compatibility analytics and have explicitly named
`salary_annual_min`/`salary_annual_max` fields at the job boundary. Text without
unit evidence retains an unknown interval. Multiple ranges are never blended.

Phase 6B–6C exports browser-runtime, isolated session (`BrowserSessionStatus`,
`BrowserSessionSummary`), and extractor-test contracts from `protocol.rs`, shared
browser runtime discovery / private session cookie validation / RFC 6455 CDP
WebSocket JSON-RPC client from `browser.rs`, and extraction/script contracts from
`extraction.rs`. Rust owns validation and fingerprints; `npm run gen:types`
generates their TypeScript representations. Custom script execution follows in
Phase 6D.
