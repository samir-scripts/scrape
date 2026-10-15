# Bounded ingestion, location targeting, and data management

Status: Sections 2 (bounded ingestion), 3 (geography/workplace controls), and 4 (confirmed Settings data management) are implemented and verified; see [implementation evidence](09-implementation-evidence.md). Section 8 desktop integration acceptance gates remain open. The separately authorized one-off desktop jobs cleanup was completed without a backup, per the user's explicit instruction. Execute one section at a time before finishing Section 8 of [desktop usability](08-desktop-usability-and-extraction.md).

## 1. Decisions and priority

- Default **20 matching postings per workflow per run**, including existing postings refreshed by the run. Test/preview caps are **10**, or the configured limit if lower.
- Invalid, duplicate, rejected, and blocked postings do not consume the budget. Every workflow in Run all has its own budget. This is not a total database capacity limit; existing jobs are untouched until explicit cleanup.
- Add country, state/province, and workplace targeting to workflow matching.
- Settings provides delete all jobs, preserve favorites, preserve tracked applications, and full application-data reset, each with confirmation.
- Implementation does not authorize deleting the user's current records. Never use the live database for tests.

## 2. Bounded ingestion

### 2.1 Research and configuration

Trace create/edit/YAML/preview/manual/scheduled/Run all routes. Extend the authoritative Rust workflow spec with `max_jobs_per_run`; default missing values to 20, accept positive whole numbers up to 10,000, and reject invalid values with path-specific backend and inline UI feedback. Generate boundary TypeScript rather than editing bindings. Show Maximum matching jobs per run and explain that refreshed existing postings count. Preserve through YAML, editing, reopening, source persistence, and effective run configuration. Existing workflows adopt the default without job deletion.

Test legacy defaults, validation boundaries, import/export round trips, editor persistence, and effective values in every execution route.

### 2.2 Adapter stopping and enforcement

Research adapter pagination, record parsing, detail enrichment, timeout, and cancellation points. Replace eager event collection in production with a shared incremental candidate sink that can stop. Keep extraction validation, identity, matching, duplicate handling, and block checks authoritative and shared; do not copy predicates into each adapter. Pass suppression snapshots to workers and recheck at persistence. Stop processing/enrichment and issuing requests once the budget is exhausted. Downloading a provider's single whole-board response may be unavoidable. Existing safety time/request bounds remain; matching limits do not guarantee short scans when few jobs qualify.

Enforce the maximum at persistence too. A concurrent block may produce fewer results; never exceed the maximum to compensate. Preview uses the same path with min(limit,10), persists nothing, and never advances schedules or retention. Report examined/matched/blocked counts, the effective cap, and limit reached; label partial source coverage and avoid invented total counts.

Tests: local HTTP → actual worker → NDJSON → repository; >100 candidates; filtered/invalid/duplicate/blocked records; refresh counts; per-workflow budgets; lower/higher preview caps; no extra detail/page requests after stopping; controlled failure and cancellation. No live providers in required tests.

## 3. Geography and workplace

### 3.1 Research and editor

Reuse existing Rust PostingRules country/region/workplace predicates and source normalizers. Add searchable Country, dependent State/province, and Workplace controls to Matches. Support country alone, country/subdivision, either plus workplace, and workplace alone. Use canonical identifiers with readable names; changing country clears incompatible subdivisions. AND geography with workplace, OR selected workplaces. Keep legacy free-text location in Advanced and describe it as an additional restriction. Unknown geography is excluded when restricted; expose an unchecked inclusion option. All changes invalidate preview.

### 3.2 Normalization and tests

One Rust alias layer handles country/subdivision names/codes and in-person/in office/in-office/onsite/on-site, Remote, and Hybrid. Prefer structured provider evidence; use bounded contextual regex fallbacks, not unrestricted substring matching. Remote US stays both Remote and US; Remote alone does not imply a country. Do not infer eligibility from headquarters or ambiguous abbreviations. Match country/subdivision on the same location entry. Global eligibility requires explicit evidence and preserves exclusions.

Test worker/repository parity, US remote, state hybrid, country-only, multiple locations, unknown values, aliases/case/punctuation, conflicts, abbreviations, editor controls, and YAML round trips.

## 4. Data management

### 4.1 Shared cleanup service

Inventory tables, foreign keys, identities, backups, worker guards, and persisted frontend state. Implement transactional preview/execute with one selection predicate:

| Operation | Preserved |
|---|---|
| Delete all jobs | Workflows, settings, run logs, notifications, suppression |
| Delete except favorites | Above plus qualifying posting identities and associated state |
| Delete except tracked applications | Above plus Applied/Interviewing/Offer/Rejected identities and associated state |
| Reset all application data | Schema, migration history, consistent recovery backup |

Include soft-removed jobs. If any saved copy qualifies for preservation, preserve the identity's copies, notes, favorites, history, and locations. Hard-delete other jobs/dependents; do not create blocks. Full reset also clears workflows/settings/runs/logs/notifications/blocks/legacy tombstones/data caches and frontend preferences. Preserve schema/migration metadata. SQLite uses transactional DELETE in dependency order with foreign keys enabled, not TRUNCATE; no arbitrary sequence reset or VACUUM in the transaction.

Make a consistent SQLite backup first; abort if backup fails. Serialize against ingestion/other writes, cancel and reap workers, and reject new runs during cleanup. Default a visible Pause automatic scraping after cleanup checkbox to checked; full reset always leaves scheduling paused. Recheck counts/selection transactionally and return actual deleted/preserved counts.

### 4.2 Confirmation UI and tests

Add Settings → Data management with shared danger tokens/dialogs. Confirmation shows operation, affected posting/saved-record counts, preserved counts, and which notes/history/workflows/blocks are affected. Explain permanence and possible future rediscovery. Initial focus Cancel, Escape and focus restoration, disabled submission during work. Full reset additionally requires typing RESET. Success shows backup path and recovery instructions; no ordinary Undo claim. Failure shows actionable error and preserves UI data. Invalidate affected lists/counts/metrics and reset paging/selection. Browser mode explains that desktop database cleanup is unavailable.

Tests use populated temporary databases: every preservation mode, protected state, dependents/legacy suppression, full-reset table coverage, backup failure, rollback, concurrency, worker cancellation, scheduling pause, confirmations/cancel/errors, refreshed lists/metrics, and isolated native operation.

## 5. Delivery and acceptance

Commit configuration, incremental adapter enforcement, geography, cleanup service, and cleanup UI separately where coherent; include observable behavior tests/docs. Preserve existing dirty files and follow root AGENTS.md.

Acceptance: 100-posting fixture ingests ≤20 eligible jobs and proves early stopping; preview ≤10 with no writes; geography parity; cleanup preserves exactly selected identities/state; reset preserves migration integrity and yields a paused usable app. Run relevant Rust/frontend/binding/build/browser/native checks. Resolve existing Section 8 screenshot/zoom failures before claiming complete integration, without weakening assertions. Document actual evidence, platform limits, and database recovery. Never perform cleanup on the user's current database as an implementation step.
