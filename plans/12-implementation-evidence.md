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
