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
├── data/           # Pinned ISO geography and the normalized O*NET role-title seed
├── examples/       # Reproducible branding asset validator (cache-logo-fixture.rs)
└── src/            # Core domain modules
    ├── branding.rs     # Employer logo evidence resolution, decoding, and bounded lossless WebP/PNG normalization
    ├── browser.rs      # Local browser runtime discovery, isolated session storage, and RFC 6455 CDP WebSocket client
    ├── enrich/         # Heuristic analysis algorithms (salary, location, workplace, HTML, etc.)
    ├── extraction.rs   # Extraction modes, script validation, canonical hashes and shared bounds
    ├── filter.rs       # SQL filter compilation and legacy/query predicate evaluation
    ├── geography.rs    # Country/subdivision catalog and bounded geographic normalization
    ├── hydration.rs    # Bounded Next.js and Nuxt framework hydration state extraction
    ├── identity.rs     # Canonical posting identity and URL normalization
    ├── lib.rs          # Crate root, module exports, and timestamp utilities
    ├── locator.rs      # Unified CSS and XPath locator grammar, parsing, and attributes
    ├── matching.rs     # Authoritative keyword/geography/workplace/partial-evidence matcher & diagnostics
    ├── posting_date.rs # Publication timestamp normalization and timezone-aware date rules
    ├── protocol.rs     # Subprocess IPC contracts (WorkerConfig, WorkerEvent, NDJSON v11)
    ├── role.rs         # Role category classification heuristics (SWE, Data, etc.)
    ├── search_source.rs # Shared public search provider identity, endpoint validation and bounds
    ├── static_source.rs # Bounded static selector source configuration and transforms
    ├── types.rs        # Core enums and domain structures (JobStatus, RoleType, etc.)
    └── xpath.rs        # Pure-Rust XPath 1.0 query execution over html5ever DOM trees
```

---

## File Manifest

| File | Responsibilities & Exports |
|---|---|
| [`Cargo.toml`](Cargo.toml) | Declares crate metadata and dependencies (`serde`, `chrono`, `ts-rs`, `regex`, `image`, `resvg`, `isocountry`, optional `sqlx`). |
| [`data/`](data/README.md) | Contains the Rust-owned geography catalog and versioned O*NET title seed provenance. |
| [`examples/cache-logo-fixture.rs`](examples/cache-logo-fixture.rs) | CLI example that runs raw logo bytes through production `branding` validation to reproduce review PNGs. |
| [`src/lib.rs`](src/README.md) | Crate root. Re-exports public submodules and shared utilities (`now_iso()`, `INGESTION_LIMITS`, `PROTOCOL_VERSION`). |
| [`src/types.rs`](src/README.md#typesrs) | Canonical domain enumerations: `JobStatus`, `RoleType`, `TriState`, `SalaryInterval`, `SalarySource`, `SourceKind`, `SourceCadence`, `ClearanceLevel`, `WorkAuthRequirement`, and `WorkplaceType`. |
| [`src/protocol.rs`](src/README.md#protocolrs) | IPC protocol v14: WorkerConfig, candidate decisions, ordered commit feedback, limits, filters, sources, NDJSON events and reconciled coverage/outcome statistics. User-supplied proxy configuration is execution-only; custom script protocol remains v2. |
| [`src/matching.rs`](src/README.md#matchingrs) | Single authoritative owner of `PostingRules` evaluation, `KeywordRules` (Any/All/Exclude/fields), partial-evidence decisions for bounded enrichment, primary `RejectionReason` categorization, and `RejectionExample` diagnostics. |
| [`src/filter.rs`](src/README.md#filterrs) | SQL-style filter parser and evaluator plus parameterized SQLite `WHERE` clause compilation for saved-job queries and `PostingRules`. |
| [`src/identity.rs`](src/README.md#identityrs) | Canonical `PostingIdentity` derivation across ATS providers, JSON-LD, and custom selectors so deduplication, rescraping, and suppression share one identity rule. |
| [`src/geography.rs`](src/README.md#geographyrs) | ISO alpha-2 country and qualified subdivision (`US-CA`, `CA-ON`, etc.) catalog, alias resolution, and exported `CountryOption`/`RegionOption` bindings. |
| [`src/search_source.rs`](src/README.md#search_sourcers) | Canonical public provider identities and fixed endpoint validation for The Muse, Arbeitnow, Adzuna and Jobicy; bounded provider hints never replace local matching. |
| [`src/branding.rs`](src/README.md#brandingrs) | Employer identity precedence, `LogoEvidence`/`LogoStatus`/`LogoMethod`, bounded image decoding (512 KiB input, 5120px dimensions, 64 MiB raster allocation), lossless asset caching at most 128px, and the exact legacy Ashby-platform-favicon filter. |
| [`src/role.rs`](src/README.md#rolers) | Role classification engine that analyzes job titles and external hints to categorize postings into `RoleType` (`swe`, `data`, `product`, `design`, `seasonal`, `other`). |
| [`src/enrich/`](src/enrich/README.md) | Heuristic parsing engine for unstructured ATS fields: native compensation ranges, multi-location strings, security clearance, visa sponsorship, work authorization, experience/new-grad detection, workplace arrangement, and structured HTML normalization. |

---

## TypeScript Binding Generation

Types marked with `#[derive(TS)]` from `ts-rs` automatically synchronize Rust models with TypeScript types in `src/bindings/` when running the workspace export tests:

```bash
npm run gen:types
# or: cargo test --workspace export_bindings
```

Compensation uses fixed-point native ranges in hundredths of a currency's major
unit, retaining currency, interval, source evidence and derived status. Filters
support `range_overlap` (the default for existing stored rules) and
`advertised_amount`. Advertised mode compares only evidenced endpoints within the
requested native currency and interval: a range with an upper endpoint above the
threshold qualifies, while unknown units/currencies and derived annual values do
not. Worker evaluation and SQLite compilation share the same per-range rule;
hourly/monthly pay is never annualized for filtering.

Phase 6B–6C exports browser-runtime, isolated session (`BrowserSessionStatus`,
`BrowserSessionSummary`), and extractor-test contracts from `protocol.rs`, shared
browser runtime discovery / private session cookie validation / RFC 6455 CDP
WebSocket JSON-RPC client from `browser.rs`, and extraction/script contracts from
`extraction.rs`. Rust owns validation and fingerprints; `npm run gen:types`
generates their TypeScript representations. Custom script execution follows in
Phase 6D.
