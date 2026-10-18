# Workflow ingestion repair — implementation evidence

Contract: [Plan 12](12-workflow-ingestion-repair.md). Diagnosis: [Incident 28](errors/28-empty-seeded-workflows.md). Work started from clean commit `ae2b741` on `implementation/trustworthy-scraping`. Tests use temporary databases and authored/recorded fixture responses; the user's database and application storage were not opened.

## Section 1 — Reproduction and durable execution diagnostics

Implementation commit: `141b113`.

Both seed IPC commands now call `services/sample_workflows.rs`; the deterministic harness exercises this same service. Manual/scheduled/Run all workflow-to-worker construction shares `services/execution_diagnostics.rs::prepare`. This keeps source attribution, effective filter resolution, cap and rotation preparation in one path while preserving existing ingestion semantics. Invalid single-workflow filters still fail before starting a run record.

Before trust/runtime preflight, attempted sidecar execution records an incomplete-examination marker, a redacted effective-config copy, and a per-workflow JSON context in run logs. Context includes parent/child IDs, observed source ownership/existence, saved configuration/filters, effective rules, enabled/review/trust flags, fingerprints, the requested protocol version and suppression count. Missing children are identified as `source_row_missing`; observations never create or enable them. Context logs are associated with the parent so Run all preparations are discoverable through source history. Seed provenance remains unknown because old rows have no authoritative origin marker; no provenance was invented.

Diagnostic copies redact executable code, interpreter/file paths/arguments, credential keys, cookie/session material, URL user information and credential query values. Suppressed identities are replaced by a context count. The actual worker configuration is unchanged. Historical effective-config reads receive the same sanitization without rewriting their stored bytes. Snapshots are inspection data, not executable exports.

Failed batch persistence retains earlier inserted/refreshed counters, records the number of failed batch candidates and an explicit persistence-stage error, and preserves the original database error classification. The regression proves that a batch containing one valid and one missing-source candidate rolls back that batch while leaving a previously committed job saved.

### Observable deterministic coverage

- Six actual templates are seeded through the service behind Load test workflows; repeated seeding is idempotent. Every recorded source-existence observation is compared with the database, and every parent context survives in a Run all history.
- An untrusted executable preflight leaves both its diagnostic snapshot and a pre-discovery error; no executable is trusted or launched by the test.
- Legacy stored filters and the currently resolved rules are retained separately, including legacy locations, separate keyword restrictions and raw SQL. This exposes current conversion differences without endorsing them or changing them here.
- Real packaged worker → production ingestion mapper → diagnostic batch writer → temporary SQLite → Discovered status query covers unrestricted, title-Engineer-only and exact-template settings against the same listing responses. Cloudflare is a visibly labelled **authored three-record control**, yielding **3/2/1**; it is not the earlier live 435-record board. Recorded Bitwarden yields **20/20/0** with 39 examined and 23 primary role rejections in the strict run; recorded SEP yields **8/6/0**, with six primary geography rejections in the strict run. Positive counts of 20 are capped.
- The fixture server serves exact listing routes, explicit robots responses, and 404 for uncaptured optional resources; it never substitutes a listing for an unknown detail route. Each of the three listing routes is requested exactly once for each of its three variants.
- Refreshing accepted results updates existing records; a posting moved to Applied remains outside the Discovered query after refresh.
- Frontend completion-event tests invalidate every cached jobs page and total for success, failure and cancellation, preserving the active view filters.

### Validation actually run

- Initial `cargo test -p scrape --lib db::runs::tests`: five existing tests passed and the new redaction regression failed because a URL token was persisted. After implementation, the six run-repository tests passed.
- `cargo test -p scrape --lib services::execution_diagnostics::tests`: initial three diagnostic tests passed; the final full host run includes the fourth legacy-context/history test.
- The first new worker replay failed due to the test listener's accepted nonblocking socket truncating the large Bitwarden response (`Network error: error decoding response body`). Accepted streams were made blocking and writes now assert success; expectations, payloads, limits and timeouts were not weakened.
- `npm run build:sidecar`: passed; the packaged Apple Silicon worker was rebuilt before integration verification (`/tmp/plan12-sidecar.log`). No core/worker extraction code changed.
- Final `cargo test -p scrape --lib --test worker_repository`: **171 host tests and 13 packaged-worker repository tests passed**, with no skips (`/tmp/plan12-final-rust.log`). This includes the isolated fake-CDP browser-session lifecycle fixture, not native desktop acceptance.
- `cargo clippy -p scrape --all-targets -- -D warnings`: passed (`/tmp/plan12-clippy.log`).
- `cargo fmt --all --check`, `git diff --check`: passed.
- `npx vitest run src/test/scheduler_events.test.ts --maxWorkers=1`: **5 tests passed** (`/tmp/plan12-events.log`; initial two-test baseline also passed).
- `npx tsc --noEmit`: passed (`/tmp/plan12-typescript.log`).
- `node scripts/check-style.mjs`: passed (`/tmp/plan12-style.log`).
- `git diff --exit-code -- src/bindings`: passed after host export tests. No boundary types changed, and no generated files were edited.

### Limits and remaining work

No schema/data migration, user-database repair, live provider request, live workflow run, browser UI journey, native startup/IPC/UI acceptance, or physical QA was performed. The full core/worker workspace suite, full frontend suite, production frontend build and standalone provider verifier were not rerun for this host-only section. Earlier evidence is not substituted for those checks.

This checkpoint implements the shared reproduction/diagnostics foundation of Section 1. Existing missing multi-source rows, template restrictions, unsafe filter normalization, geography parity, Google base URLs, and pagination remain open acceptance defects. Their repair-specific desired-behavior regressions belong with Sections 2–7; the original Incident 28 probes remain archived. The exact user's all-zero saved-run state remains uninspected.

The existing Run all effective-config column and aggregate counters still have historical last-workflow semantics; the new context log retains each workflow preparation but does not claim aggregate accounting repair. Source-level/native launch and portable script-resource verification remain later acceptance work. Next implementation section: **2 — canonical seed/save/import ownership and local trust persistence**.


## Section 2 — Atomic workflow ownership and local script review

Commits: migration `456e2b7`; canonical persistence, repair helper and UI `b29d3a4`; generated report binding `68d1025`; parent-retention/legacy-CRUD follow-up `38a362e`. Started on a clean tree at `f27edb2`. No application database was opened or repaired.

### Delivered behavior

All built-in seed paths, editor Save and imported workflow Save use `workflow::persist` → `workflow_store.rs`. Each workflow is validated before mutation; one transaction checks ownership, assigns canonical UUIDs to new local aliases, writes the parent and every child, and stores those exact IDs in the spec. Existing owned source IDs are retained. Attempts to take another workflow's source or save a child as a different parent fail explicitly. Normal Save cannot leave a half-created/half-edited workflow after a child write fails. Legacy single-source IPC CRUD rejects embedded multi-source configurations and directs callers to Save workflow.

Migration 27 replaces global `(kind, identifier)` uniqueness with workflow-scoped uniqueness. All prior SQL/checksums remain unchanged. Startup's normal consistent backup precedes the forward migration. The replacement preserves every source column and temporarily maps/restores the two `ON DELETE SET NULL` source-reference columns while foreign keys stay enabled. Populated v26 tests compare every historical table, preserved job/run links and migration metadata; check backup contents, failure immediately after the old table is dropped, rollback/retry and restart. Recovery details and primary SQLite references are in [database recovery](database-recovery.md).

Workflow lists and due-source queries show parent records. Children use manual cadence and have no separate schedule; launching one directly is rejected. Parent job filters include owned child postings. Removed sources remain saved, disabled and unscheduled, retaining historical job ownership. Re-adding an endpoint reuses its existing owned ID. Post-run retention resolves children to the parent, deduplicates owners and applies the parent's policy; bulk-cleanup previews count parent workflows. Tracked/favorited/annotated jobs remain protected.

Suite seeding reports created, already-present and needs-review records using Rust-generated `SeedWorkflowReport`. Loading examples again preserves existing edits. Six seeded workflows materialize eight source rows, including independently reviewable Netflix/Amazon script records. Duplicate external boards in separate workflows are allowed; duplicate normalized endpoints inside one workflow remain rejected.

Executable sources are persisted disabled with local fingerprints. Imports reset trust, changed executable content invalidates trust, and unchanged locally trusted content retains it. Trust never enables a source/workflow or creates a next run. Aggregate review covers the currently configured sources, excluding retired historical children. Preflight requires a source row owned by the selected parent before worker launch.

The editor's **Save disabled for review** path validates and saves without a preview or a run, avoiding the trust/preview deadlock. Reopening a static parent with a script child retains the validated multi-source draft. Each script loads its own source row/fingerprint; the review control compares the executable being displayed with that child's actual saved executable, including differences from a stale parent snapshot. The action now says **Trust saved script** and explains that enabling remains explicit.

Existing malformed seeded workflows are not silently rewritten. Normal Save repairs their ownership as an explicit edit. The new `repair_workflow_ownership` example provides an explicit idempotent data-repair path with a separate consistent backup, refusing missing databases, backup failures and foreign ownership conflicts. It preserves the parent ID, name, identifier, filters, schedule, notes, favorites, tracking history and suppression. Failure during repair rolls back; restart and repeated repair add no duplicates or extra ownership backup. The CLI/service starts no extraction or scheduling. Only the isolated service fixtures were exercised; the helper was not run against user data.

### Regressions and verification

- The new seed regression first failed **6 source rows vs 8 expected**, proving the original missing-child defect. A later retry exposed legacy-vs-canonical endpoint duplicate detection; both paths now recognize existing workflows without overwriting them.
- Injected child INSERT failure proves rollback for creation and editing. Identity-preserving reorder/removal/re-add, shared endpoints across workflows, foreign-source conflicts, and saving via a child ID are tested.
- The populated v26 upgrade test preserves all historical table rows and validates `foreign_key_check`, backup version/content, injected failure after DROP, retry and restart. The previous migration-26 test's current-manifest expectation advances to 27 while all its protected-state checks remain intact.
- Exact canonical IDs replace hard-coded local aliases in existing script trust/persistence tests; original emitted-job and persistence assertions remain. The default sample now asserts empty legacy role restrictions plus its unchanged structured role choices, rather than the obsolete duplicated legacy array.
- A real packaged-worker test uses the actual seeded Google/Apple configuration with labelled authored HTML and **unrestricted control filters**: both source IDs persist successfully and both postings are returned through the parent workflow filter. It does not claim the strict templates or live career sites are repaired.
- New UI tests prove disabled Save performs neither Preview nor Run, child trust targets the child's stored fingerprint, and a different saved child executable blocks trust. The initially missing review button exposed the aggregate-review draft bug; the implementation was fixed instead of weakening the fixture.
- The final retention/cleanup regression first failed because two source rows were counted as two workflows. It now also proves a child-only completion applies the parent policy, while retaining the annotated, favorited, interviewing posting.

Actually run:

- `npm run build:sidecar`: passed (`/tmp/plan12-step2-sidecar.log`).
- `cargo test --workspace`: **441** tests passed (`/tmp/plan12-step2-workspace.log`). This includes 179 host tests and 14 worker/repository integrations; no tests were skipped.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed (`/tmp/plan12-step2-clippy.log`).
- `npm run gen:types` and `git diff --exit-code -- src/bindings`: passed (`/tmp/plan12-step2-bindings.log`); the generated binding is committed separately.
- `npx vitest run --maxWorkers=1`: **504 tests / 79 files passed** (`/tmp/plan12-step2-frontend.log`).
- `npm run build`: TypeScript and production build passed; existing large-chunk advisory remains (`/tmp/plan12-step2-build.log`).
- `node scripts/check-style.mjs`: passed (`/tmp/plan12-step2-style.log`).
- `node scripts/test-app-startup.mjs`: Vite startup/entry transform/shutdown smoke passed (`/tmp/plan12-step2-startup.log`).
- `cargo fmt --all --check` and `git diff --check`: passed.

No live provider verification, native app startup/IPC/UI/browser journey, visual/physical QA, or user-data migration/repair was performed. The authored two-source fixture proves ownership/persistence, not Google/Apple coverage. Full company scripts, portable resource setup, matching normalization/evidence and URL/pagination defects remain in later sections. **Next: Section 3 — restore matching compatibility and remove competing normalization paths.**


## Section 3 — Preserve matching compatibility without destructive migration

Commits: Rust compatibility and worker/persistence regressions `95c9453`; generated boundary `a900efa`; editor/API repair `a5e3b16`; browser journeys and visually reviewed captures `72ebfa7`. Started on the clean Section 2 checkpoint `d8c9652`. No user database was opened, modified, migrated or repaired.

### Behavior and compatibility

Removed all three `normalize_into_matching` / `normalizeLegacyWorkflowFilters` implementations. `WorkerFilters::resolve_compatibility` is the single Rust overlay/validation operation. It retains the original independent predicates and raw values; there is no attempted lossless conversion. Legacy keyword OR groups still search title/location with SQL LIKE semantics, independent structured Any/All/exclude groups retain their selected fields, legacy locations remain an OR on original text, and legacy/structured role sets remain intersected. Recognized old sample SQL is preserved just like arbitrary queries. Query, policy, legacy compensation and structured compensation all remain active with AND.

Normal `resolve_worker_filters` now has no global-default argument. Runtime, preview, transient tests, validation, persistence and execution preparation use it. Explicit historical upgrade/import paths alone may opt into defaults; v1–v3 omission/null inheritance and explicit empty-array overrides resolve once through the same core operation. V4–v6 workflows do not acquire global restrictions. The historical application defaults remain limited to their four supported keyword/location/role fields.

The editor replaces the unsafe migration action with **Additional compatibility restrictions**. Active legacy include/exclude/location chips, role values, expert query/policy and compensation are visible in standard Matching. Rust-generated `FilterCompatibilityRestriction` descriptions arrive through `describe_workflow_filters`; browser demonstrations report that authoritative descriptions require desktop instead of implementing inference. Users can explicitly edit/remove restrictions; individual legacy role and compensation edits remain available in YAML. Pending inputs flush using functional updates so separate legacy fields cannot overwrite one another. Expert-field typing retains focus, malformed policy drafts reach authoritative validation, and edits invalidate preview.

This restores the existing v6 predicate semantics; the workflow/worker wire shape and protocol versions are unchanged. No database migration or rewriting of saved restrictions is needed. Previously discarded/unioned restrictions cannot be reconstructed from already-normalized data; review against an original or compatible backup remains necessary. No guessed restoration or live-data repair was attempted.

### Regression evidence

- The first new acceptance regression failed on onsite Austin under legacy `[Remote, Austin]`: expected true, normalized result false (`/tmp/plan12-step3-red.log`). After repair, independent expected decisions pass for all three Incident 28 flips, both recognized sample queries, role intersections, title/location-vs-description search fields, mixed Any/All/exclude groups, arbitrary query/policy and compensation intersections. Exact filter equality and validation idempotence prevent hidden rewrites.
- Import/save/reopen tests cover versions 4, 5 and 6 with populated global defaults, preserving every effective filter. A separate v3 case retains explicit empty keywords and materializes omitted location defaults only once. Invalid query, policy field, filter shape and compensation bounds fail explicitly. The first negative-policy fixture used a legal bare text expression and was corrected to an actually unsupported policy field; the compiler was not weakened.
- Eight independently authored selector scenarios execute through the real import/save service, canonical owned source, preparation, packaged-worker Preview and Run, production event mapper, temporary SQLite and parent Discovered query. Exact accepted IDs, examined/rejected/unresolved/accepted totals, exhausted termination, absence of errors, no preview writes and persisted counts are asserted. The fixture serves exactly 16 listing responses across the preview/run pairs and rejects unexpected routes. These are authored compatibility controls, not claims of provider support.
- Interaction tests drive Preview → YAML → Save, reopen conversion, explicit role/query/policy edits, simultaneous unfinished legacy keyword/location inputs, and ordinary expert typing. The API test proves the untouched filters are sent to Rust IPC and browser mode does not invent a description/compiler.

### Checks actually run

- `npm run build:sidecar`: passed (`/tmp/plan12-step3-sidecar.log`).
- `cargo test -p scrape --lib matching_compatibility`: 3 passed (`/tmp/plan12-step3-targeted.log`).
- `cargo test -p scrape --lib --test worker_repository`: **182 host + 15 integration tests passed**, no skips (`/tmp/plan12-step3-host.log`).
- `cargo test --workspace`: **446 tests passed**, no skips (`/tmp/plan12-step3-workspace.log`). Counts include 182 host, 15 worker/repository, 168 core and 81 worker/unit/process tests. Initial integration compilation exposed incorrect enum/count types in the new test; these were corrected to the existing Scraped/Viewed Discovered contract and i64 persistence counts.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed (`/tmp/plan12-step3-clippy.log`).
- `npm run gen:types` plus `git diff --exit-code -- src/bindings`: passed (`/tmp/plan12-step3-bindings.log`); the generated binding is a separate commit.
- `npx vitest run --maxWorkers=1`: **508 tests / 81 files passed** (`/tmp/plan12-step3-frontend.log`). Targeted editor/keyword/YAML checks also passed.
- `npx tsc --noEmit` and `npm run build`: passed; production build retains the existing large-chunk advisory (`/tmp/plan12-step3-ts.log`, `/tmp/plan12-step3-build.log`).
- `node scripts/check-style.mjs`, `node scripts/test-app-startup.mjs`, `cargo fmt --all --check`, `git diff --check`: passed (`/tmp/plan12-step3-style.log`, `/tmp/plan12-step3-startup.log`). Startup smoke is Vite only.
- Existing Chromium journeys: `SCRAPE_BROWSER_PORT=1449 PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/keywords.pw.ts e2e/workflows.pw.ts e2e/workflow-sources.pw.ts`: **7 passed** (`/tmp/plan12-step3-browser-final.log`). The initial run passed six and lost the page execution context during contrast evaluation while concurrent Rust tests rewrote generated bindings and triggered Vite reload. Rerunning unchanged tests after generation finished passed all seven; no timeouts/assertions were relaxed. Automatically overwritten old Plan 11 review images were archived under `/tmp/plan12-step3-browser-script-review` and restored, rather than silently replacing historical review assets.
- New authored browser journey: same environment with `e2e/matching-compatibility.pw.ts`: **2 passed** (`/tmp/plan12-step3-compatibility-browser.log`). Both themes verify visible retained restrictions, no migration action, explicit edits in exported YAML, narrow-dialog overflow and text contrast. An initial fixture syntax typo was corrected before the test executed. Four captures were visually reviewed: [Paper 1024](assets/12-matching-compatibility-review/paper-1024.png), [Paper 480](assets/12-matching-compatibility-review/paper-480.png), [Obsidian 1024](assets/12-matching-compatibility-review/obsidian-1024.png), [Obsidian 480](assets/12-matching-compatibility-review/obsidian-480.png). They are visibly labelled browser demonstrations, not native Rust execution.

No native Tauri startup/IPC/WebView/physical/200%-zoom acceptance, live provider verification, standalone provider-verifier rerun or user-data repair was performed. Existing workspace recorded-provider tests passed but do not establish the six strict templates or live career-site coverage. Field evidence/geography parity, template defaults, actual URLs/pagination and exact user saved-run reproduction remain open. **Next: Section 4 — field evidence and geographic predicate parity.**
