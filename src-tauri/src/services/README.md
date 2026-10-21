# `src-tauri/src/services` — Background Services & System Integrations

## Overview

The `src-tauri/src/services` directory contains long-running background tasks, process managers, scheduling daemons, and operating system integration services.

---

## File Manifest

| File | Service Domain | Core Responsibilities & Implementation |
|---|---|---|
| [`mod.rs`](mod.rs) | Service Root | Consolidates service modules and exports public service constructs. |
| [`template_catalog.rs`](template_catalog.rs) | Built-in Templates | Versioned Rust-validated catalog; title-only seed defaults, separately labelled restrictive examples, embedded protocol-2 scripts and generated picker/YAML/extractor projections. |
| [`sample_workflows.rs`](sample_workflows.rs) | Built-in Seeding | Shared callable service behind both seed commands; canonical atomic saves and created/already-present/needs-review reports. |
| [`execution_diagnostics.rs`](execution_diagnostics.rs) | Run Preparation & Diagnostics | Shared workflow-to-worker preparation, redacted context/config snapshots before preflight, and durable batch-persistence failure evidence. |
| [`workflow_store.rs`](workflow_store.rs) | Atomic Workflow Ownership | Validates and saves parent/child records in one transaction, canonicalizes IDs, preserves owned identities/trust, retires removed sources, and performs explicit backed-up ownership repair. |
| [`workflow.rs`](workflow.rs) | Authoritative Workflow Specification | Validates WorkflowSpec v8; converts v1–v7 without broadening stored filters; prepares public search and direct-board inputs through the same resolver. |
| [`script_trust.rs`](script_trust.rs) | Local Script Trust | Quarantines imports, hashes executable inputs/file bytes, persists explicit confirmation, rechecks trust before previews/runs and reconciles workflow review state. Trust never enables execution automatically. |
| [`runner.rs`](runner.rs) | Worker Process Orchestration | Spawns worker protocol 9, revalidates NDJSON, enforces write coordination and broadcasts scrape events. |
| [`ingestion.rs`](ingestion.rs) | Ingestion & Revalidation | Validates worker-emitted job records, source scan summaries, and employer branding reports before persisting them to SQLite. |
| [`scheduler.rs`](scheduler.rs) | Automated Crawl Dispatcher | `SchedulerService`: Unified background dispatcher that evaluates per-workflow schedules and startup triggers, coalesces due runs, respects the global automatic-scraping pause, and coordinates with manual runs. |
| [`calendar.rs`](calendar.rs) | IANA Calendar Scheduling | Computes next-run UTC instants for `CalendarSchedule` (local `HH:MM`, ISO weekdays, and IANA timezone), handling DST gaps and overlaps deterministically. |
| [`retention.rs`](retention.rs) | Automatic Data Retention | `RetentionService`: Runs after startup and every 24 elapsed hours, independent of scraping pause and workflow enablement. Shares the fixed-clock, protected-record predicate with post-run and explicit retention routes; defers without cancelling work when the shared writer is busy and records the last result. |
| [`process.rs`](process.rs) | Subprocess Lifecycle & Reaping | Structured argument handling, bounded/redacted `stderr` capture, cancellation, and process-tree termination/reaping for sidecar workers. |
| [`startup.rs`](startup.rs) | Safe Database Startup | Coordinates pre-migration read-only history checks, `VACUUM INTO` backups, and migration execution during host initialization. |
| [`tray.rs`](tray.rs) | System Tray Integration | Manages the native OS menu bar / taskbar tray icon, context menus, and window visibility toggling. |
| [`system.rs`](system.rs) | OS Interop & Notifications | Manages desktop system notifications via `tauri-plugin-notification` and coordinates external URL navigation via `tauri-plugin-opener`. |
| [`native_smoke.rs`](native_smoke.rs) | Debug-Only Native Test Harness | Provides isolated temporary-profile IPC, sidecar, window-resize, and startup smoke verification hooks (compiled/enabled only for explicit debug smoke runs, never affecting release builds or user data). |

---

## Service Lifecycle & Architecture

```
Application Boot (lib.rs)
        │
        ├──► startup.rs (Read-only history check, VACUUM INTO backup, migrations)
        ├──► SchedulerService::start()
        │         │ (Evaluates per-workflow CalendarSchedule & pause state)
        │         ▼
        │    ScrapeRunner
        │         │
        │         ├──► Spawns scrape-worker Sidecar (Protocol v9)
        │         ├──► Revalidates NDJSON Events (ingestion.rs)
        │         ├──► Upserts Jobs, Branding & Outcomes into SQLite
        │         └──► Emits Tauri Window Events
        ├──► RetentionService (Startup + 24h safe cleanup sweep)
        └──► TrayService::init() (Maintains background presence)
```

Each workflow owns an explicit WorkflowSpec v8 (sources, role/company/keyword criteria, dates, calendar schedule, job cap and retention) without global inheritance. Legacy v1–v7 imports resolve through `workflow.rs`; invalid stored originals remain preserved and marked Needs review. Empty collections are unrestricted.

Calendar scheduling (`calendar.rs`) computes next runs from local time, ISO weekdays, and IANA timezone across creation, edits, and post-run updates. DST gaps run at the first valid instant; overlaps use their first occurrence once. One dispatcher queue in `scheduler.rs` coalesces due workflows and waits through manual-run overlap; missed occurrences become one catch-up run before the next future date. All automatic paths share the persisted global pause flag, while explicit manual runs remain available.

`PostingRules` is shared by worker evaluation and repository SQL compilation, including literal target-role/company substrings and dynamic publication-date presets. One run-start instant anchors relative windows; Today resolves the date filter's local midnight each run. Missing publication dates are never replaced with discovery/provider edit time. Protocol 9 enforces host/sidecar compatibility. `execution_diagnostics::prepare_search_sources` derives the same canonical context for preview/manual/scheduled/Run all execution; providers are not assumed to support arbitrary query fields.

## Seeded-workflow diagnostics (Plan 12 section 1)

`sample_workflows.rs` is the service used by both seed IPC commands and isolated
regressions. It uses canonical atomic workflow persistence for each template and reports
created/already-present/review states without overwriting existing workflows.

`execution_diagnostics.rs` shares workflow-to-worker preparation between manual,
scheduled and Run all execution. Before trust/runtime validation, each attempted
execution stores a redacted worker-config snapshot and a per-workflow context log
with persisted rules, effective rules, child ownership and enabled/review/trust
state. Missing source rows are labelled `source_row_missing`. These observations
do not repair or enable sources. Historical seed origin remains unknown.

Executable bodies, file paths/arguments, credentials, URL authentication/query
secrets and cookie/session material are omitted from diagnostic copies. Suppressed
identities are represented by a count in context. The actual worker configuration
is unchanged. Historical effective-config rows are sanitized when read without
rewriting their stored bytes. Per-workflow context logs retain all preparations
within a Run all run; the historical `effective_config` field still represents the
last prepared child workflow, and aggregate Run all counters are not repaired here.

Batch persistence uses the shared diagnostic writer to retain earlier committed
counters and record the failure stage if a later transaction rolls back. Database
errors retain their database error classification. See
[Plan 12](../../../plans/12-workflow-ingestion-repair.md) for remaining repairs.

## Atomic ownership (Plan 12 section 2)

`workflow::persist` delegates to `workflow_store.rs`. One transaction validates
ownership, assigns collision-safe IDs, writes the parent and all children, and
stores the canonical IDs in the spec. Existing owned IDs and protected job data
are retained. Removed children are disabled and unscheduled; re-adding the same
endpoint reuses its owned ID. Foreign ownership and saving a child as a new parent
are rejected. Only parent records appear in workflow lists and scheduling; children
have manual cadence and cannot be launched as independent workflows. Parent job
filters include jobs from every owned child.

Executable sources are saved disabled and need explicit local trust. Imports and
executable edits reset trust; unchanged local scripts retain it. Trust does not
enable execution. The editor can save disabled without a preview, then reopen each
child's exact saved script for trust. Parent aggregate review does not hide the
validated multi-source draft. The UI checks the child's saved executable, not a
stale parent snapshot, before offering trust.

Existing malformed records are repaired only by explicit Save or the backed-up
`repair_workflow_ownership` helper documented in database recovery. This revision
does not run that helper against application data.

Retention resolves completed child IDs to their parent and deduplicates by
workflow before applying the parent's retention policy. Bulk-cleanup previews
count workflow parents rather than extraction-source rows. The regression keeps
tracked/favorited/annotated jobs while expiring an eligible child posting.

Matching compatibility delegates to `WorkerFilters::resolve_compatibility` in
scrape-core. Normal `resolve_worker_filters` has no global argument; only the
explicit legacy upgrade/import paths may inherit old defaults. Validation,
preview, save and execution preserve the independent predicates and raw values;
there is no host normalization/compiler copy. `matching_compatibility_tests.rs`
checks independent acceptance expectations and import/save/reopen/global isolation.

Ingestion revalidates the core location tuple bounds and country/subdivision
consistency before persistence. Script protocol 2 supplies employment/workplace/
location evidence through existing WorkerEvent fields; worker protocol stays 7.
Existing script fingerprints/trust checks remain authoritative.

JavaScript runtime preflight requires an explicit absolute Node.js 18+ executable; missing setup never falls back to a developer shell. File scripts require absolute readable paths before trust/execution; embedded scripts need no repository checkout. See [template setup](../../../workflows/README.md).
