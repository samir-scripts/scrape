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
└── src/            # Core modules (types, protocol, role, filter, enrich)
    ├── enrich/     # Heuristic analysis algorithms (salary, location, workplace, etc.)
    ├── filter.rs   # SQL query evaluation and worker-side filtering logic
    ├── lib.rs      # Crate root, module exports, and timestamp utilities
    ├── protocol.rs # Subprocess IPC contracts (WorkerConfig, WorkerEvent, NDJSON)
    ├── role.rs     # Role category classification heuristics (SWE, Data, etc.)
    └── types.rs    # Core enums and domain structures (JobStatus, RoleType, etc.)
```

---

## File Manifest

| File | Responsibilities & Exports |
|---|---|
| [`Cargo.toml`](Cargo.toml) | Declares crate metadata and dependencies (`serde`, `chrono`, `ts-rs`, `regex`, optional `sqlx`). Contains the `sqlx` feature flag to toggle database trait derivations without bloating the worker binary. |
| [`src/lib.rs`](src/README.md) | Crate root. Re-exports public submodules (`types`, `protocol`, `role`, `filter`, `enrich`) and provides shared utilities such as `now_iso()` for UTC ISO-8601 millisecond timestamping. |
| [`src/types.rs`](src/README.md#typesrs) | Canonical domain enumerations: `JobStatus`, `RoleType`, `TriState`, `SalaryInterval`, `SalarySource`, `SourceKind`, `SourceCadence`, `ClearanceLevel`, `WorkAuthRequirement`, and `WorkplaceType`. |
| [`src/protocol.rs`](src/README.md#protocolrs) | IPC protocol definitions for host-worker communication: `WorkerConfig`, `WorkerLimits`, `WorkerFilters`, `WorkerSource`, `WorkerEvent`, `WorkerStats`, and `ScrapedJob`. |
| [`src/role.rs`](src/README.md#rolers) | Role classification engine that analyzes job titles and external hints to categorize postings into `RoleType` (`swe`, `data`, `product`, `design`, `seasonal`, `other`). |
| [`src/filter.rs`](src/README.md#filterrs) | SQL-style filter parser and evaluator for pre-filtering job records before emitting events. Supports `WHERE` clauses, keyword exclusions, and token matching. |
| [`src/enrich/`](src/enrich/README.md) | Heuristic parsing engine for unstructured ATS fields: compensation ranges, multi-location strings, security clearance, visa sponsorship, work authorization, experience/new-grad detection, and workplace arrangement (onsite/remote/hybrid). |

---

## TypeScript Binding Generation

Types marked with `#[derive(TS)]` from `ts-rs` automatically synchronize Rust models with TypeScript types in `src/bindings/` when running tests:

```bash
cargo test -p scrape-core -- test_bindings
```

Compensation now includes separate native ranges in fixed-point hundredths of a
currency's major unit, with interval, currency, source evidence, scope and derived
status. `CompensationFilter` evaluates native ranges and compiles parameterized
SQLite predicates. Hourly/annual filters require the original matching interval;
annual equivalents remain compatibility analytics and have explicitly named
`salary_annual_min`/`salary_annual_max` fields at the job boundary. Text without
unit evidence retains an unknown interval. Multiple ranges are never blended.
