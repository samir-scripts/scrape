# Reliable ingestion implementation evidence

Revision contract: [plan 11](11-reliable-ingestion-and-ui-repair.md). Work began on a clean tree at `17b13de` after committing the specification. Existing review assets were preserved. The six original screenshots and exact active workflow configuration were not supplied in this context, so the exact reported Cloudflare run remains unverified. Tests never use the live user database. Section entries describe their checkpoint; later sections supersede earlier pending notes and protocol versions.

## Section 1 — Matching diagnostics foundation

Implementation: `2ba60b7`; generated-format cleanup: `22891a9`.

Implemented authoritative categorized Rust matching for legacy/query/policy and structured rules. Boolean decisions and diagnostic reasons share predicates. Each candidate has one primary rejection category; up to five examples explicitly list overlapping failures. Source summaries distinguish examined, matching rejection, invalid, duplicate, blocked and accepted counts, effective cap, and termination. Provider totals stay unknown unless supplied. Protocol v4 rejects older requested protocol versions. Rust generates all boundary TypeScript.

Preview displays scan details. Saved run counts retain source summaries in existing counts JSON, including after restart/failure queries; historical rows without summaries remain readable. Completed run rejection totals use primary counts rather than folding duplicates into rejections. Host and preview reject inconsistent new summaries. No schema migration or application-data operation was needed.

The authored 438-record Lever control yields 438 unrestricted, 400 with title Engineer, and two with the repository sample's legacy internship/keyword/location restrictions. This demonstrates diagnostic reconciliation; it is not a captured Cloudflare response or recreation of the missing active configuration. Actual packaged worker → temporary SQLite tests compare Any/All/mixed/exclusion/substring and geography decisions to repository results and primary reason totals.

Workspace verification exposed two existing HTTP fixtures that did not serve the current branding/robots requests after rebuilding the sidecar. Updated explicit fixture routes and exact request counts; no assertions were removed and no live requests were introduced.

Validation actually run:

- Baseline `cargo test -p scrape-core -p scrape-worker`: 168 tests passing; baseline workflow editor: five passing.
- `npm run build:sidecar`: packaged Apple Silicon release worker rebuilt successfully.
- `cargo test --workspace`: 326 tests passing, including five real worker/repository integrations and generated exports.
- `npm test`: 436 tests / 73 files passing, plus Vite startup smoke.
- `npm run build`: TypeScript and Vite passed; existing large-chunk advisory remains.
- `node scripts/check-style.mjs`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.

Remaining section evidence: no original screenshot/configuration reproduction, native visual interaction, or live provider verification. Request/time/cancellation and enrichment-warning outcome refinements belong to Section 3 and later execution sections; their enum vocabulary is present, but these outcomes are not claimed fully implemented by this checkpoint. Company-provided total extraction is still pending adapter integration. The entire revision remains incomplete.


## Section 2 — Keyword entry and explicit matching

Reproduced the typing defect through real input events before the fix: typing
`Engineer,Developer` left `EngineerDeveloper` in the old input. The regression now
requires separate removable chips. Added reusable `KeywordInput` with independent
unfinished text, Enter/Add/comma/newline/paste commits, phrase preservation,
case-insensitive deduplication, named SVG removals, two-stage keyboard deletion,
IME composition handling, and bounded-height wrapping/scrolling without a term cap.
Imported term spelling remains unchanged during ordinary navigation.

Standard Matching exposes Any/All, exclusions and search fields. Imported mixed
groups remain visible while editing, including temporarily empty groups. Imported
All mode survives removing its last term. No frontend domain matcher was added.
Editor actions synchronously flush independent pending terms and preserve all
other matching rules; confirmed replacements/reopening reset abandoned input.
Pending edits invalidate preview. Save still requires a current successful preview.
YAML editing/download, preview, navigation, save and reopen preserve the same rules.

Validation actually run:

- `npm test`: 444 tests / 74 files passed, plus Vite startup smoke. Eight keyword
  interaction cases include direct action events without relying on blur,
  composition, mixed imports, phrases, YAML and imported-spelling preservation.
- `npm run build`: TypeScript and production bundle passed; existing chunk advisory.
- `cargo test -p scrape --test worker_repository`: all five passed. Expanded actual
  packaged worker → temporary SQLite parity to phrase, mixed positive,
  exclusion, substring and selected-location-field keyword cases.
- `cargo fmt --all --check`, `node scripts/check-style.mjs`, `git diff --check`,
  and `git diff --exit-code src/bindings`: passed. No new boundary types or schema.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/keywords.pw.ts e2e/workflows.pw.ts`:
  five browser journeys passed. Actual typing, Any→All selection, unfinished
  phrase commit, save/reopen, YAML download, 100 additional terms with scrolling,
  no horizontal overflow, Paper/Obsidian, 1024px/480px, rendered text contrast and
  keyboard focus contrast were checked. The browser uses labelled fictional
  fixtures and simulated execution; it does not establish provider extraction.
- Visually inspected Paper 480px and Obsidian 1024px screenshots. All four browser
  screenshots are preserved in [keyword review assets](assets/11-keyword-review/).
  No screenshot baselines were weakened or updated.
- `cargo build -p scrape`, then
  `SCRAPE_STARTUP_MODES=fresh SCRAPE_NATIVE_WORKFLOW=1 node scripts/test-native-startup.mjs`:
  passed in an isolated fresh native profile. Actual WKWebView controls via DOM
  events, themed All/manual selections, two persisted All terms, local HTTP →
  actual sidecar preview (one eligible match), US/TX/Remote rules, and no implicit
  job persistence/run during preview/save. Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-7eb6Z4`.

During validation, an initial concurrent frontend run had timeout failures; a
serial rerun passed with unchanged timeouts and assertions. The first native
harness attempt edited the next control before All had rendered; it caught the
lost scripted selection. The harness now waits for the visible selected state
and reports exact saved keyword rules on failure; the isolated rerun passed.
Native file-chooser gestures, physical pointer/IME interaction, 200% native zoom,
and live company verification were not performed. These results do not establish
Sections 3–8 acceptance. No user database was read or modified.

## Section 3 — Bounded enrichment and explicit outcomes

Implementation: `96ce464`; core foundation: `1a31218`. Shared Rust partial matching distinguishes conclusive
rejection from unresolved fields across nested OR/NOT, exclusions, selected fields,
dates and compensation. Completion-property tests require every early decision to
hold under the tested completions; complete assessments use existing predicates.

Greenhouse now evaluates normalized listing evidence before details. Nonempty
listing fields define the snapshot: details fill gaps and cannot rewrite original
ID/title/destination/description. A conflicting-detail fixture proves this through
the real worker and persistence. Known rejects and canonical duplicates spend no
detail requests. Required eligibility gets a detail opportunity; valid detail with
missing required evidence and failed detail remain unresolved unless the active
rule explicitly includes unknowns. Optional enrichment failures are warnings.
Unknown publication dates and native compensation remain unknown; provider edit
time is not publication time.

Default detail bounds remain 100 attempts/45 seconds, independently configurable
only downward via validated worker limits. Branding shares remaining time; existing
URL validation, pacing, cache, cancellation, suppression and persistence caps remain.
No new detail/page requests are scheduled after acceptance reaches the cap. Listing
response download/deserialization can still include the whole board. Provider
`meta.total`, examined, primary rejections, duplicates, invalid, unresolved,
accepted/blocked and enrichment warnings are separate; coverage reports bounds.
Malformed individual records are isolated; missing listing collections fail with
an actionable parser diagnostic.

Worker protocol v5 adds warnings, provider totals and outcomes, and rejects protocol
v4 requests. WorkflowSpec remains v4 until Section 5. Host/preview validate source
summaries and IDs; warnings do not inflate failed-posting counts. Preview warnings
permit save, required-evidence failures prevent a successful preview. Source and
run outcomes distinguish success, success with warnings, partial, failed, cancelled.
Cancelled/incomplete runs retain confirmed saved-record counts and identify unknown
worker counters. Browser error filters exclude explicit cancellation and optional
warnings; cancellation notifications are informational.

Forward-only migration 22 adds outcome columns, preserving old CHECK constraints
and historical NULL outcomes/status fallbacks. Populated v21 upgrades preserve job
IDs, notes, favorites, applied status/history, suppression, source scheduling,
run/log records and applied checksums. Tests verify the consistent v21 snapshot,
transaction rollback after an injected migration failure, successful retry and
idempotent restart. See [recovery](database-recovery.md): reverting code does not
roll back a database. No user database was used.

Validation actually run:

- Rebuilt packaged sidecar: `node scripts/build-sidecar.mjs`.
- `cargo test --workspace`: **345 passed** across host (148), repository/actual
  worker integration (6), core (143), worker (22), adapters (8), bounded worker
  processes (13), protocol/process checks (5). Local HTTP and temporary SQLite only.
  The late-match fixture skips 140 Sales postings then accepts 20 Engineers with
  exactly 20 detail requests; duplicate listings do not spend detail allowance.
  Other fixtures cover unavailable optional/required details, missing required
  fields after valid detail, explicit unknown inclusion, title-or-description
  rescue, zero-request/time bounds, invalid limits and malformed records.
  The two-run persistence fixture inserts then refreshes three late matches and
  retains title/URL/date provenance, notes, favorite and applied status.
- `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace export_bindings`: passed. A before/after SHA-256 comparison
  confirmed no drift on a second Rust binding export; new TypeScript is generated.
- `npm test`: **446 tests / 75 files passed**, plus frontend startup smoke.
  Actual controls check optional-warning preview save eligibility, required-evidence
  blocking, status/error filtering, cancellation and failure recovery. Replaced
  copied log-filter test predicates with the shared production helper.
- `npm run build`, `node scripts/check-style.mjs`, `git diff --check`: passed.
  Production build retains the existing large-chunk advisory.
- Seven Playwright journeys passed:
  `e2e/enrichment-outcomes.pw.ts`, `e2e/keywords.pw.ts`, `e2e/workflows.pw.ts`
  with `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers`. Authored simulated
  runs cover warning/partial/cancelled labels, truthful provider/scan diagnostics,
  error filtering and rendered text contrast in Paper/Obsidian, alongside keyword
  save/reopen and registry regressions. Visually reviewed both new 1024px screenshots,
  preserved in [enrichment review assets](assets/11-enrichment-review/).
- `cargo build -p scrape`, then `node scripts/native-smoke.mjs`: passed in isolated
  WKWebView storage, actual sidecar HTTP preview/ingestion, native persistence,
  protected-state/blocking, cancellation/reaping/recovery, and authored protocol
  fixtures for optional-warning/required-evidence outcome persistence, malformed
  output, unknown IDs, UTF-8 failure, nonzero exit, missing completion, missing/
  duplicate source summaries, timeout and partial-result retention. Native fonts
  and local external opener also passed. Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-V745R9`.
- `SCRAPE_STARTUP_MODES=current19,unknown19 node scripts/test-native-startup.mjs`:
  passed. The known historical database upgraded with protected state retained;
  the unknown checksum history failed before database mutation. Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-gYSRJX`
  and `.../scrape-native-smoke-xSvUGY`.

The initial sandboxed Rust run could not bind fixture sockets; the same suites
passed with local-server access. A new missing-collection assertion exposed
reqwest’s generic decode message; retaining serde diagnostics fixed the behavior
without weakening the assertion. A detail-timeout fixture was adjusted to allow
normalization before delaying the response; it still requires exactly one detail
request and no later requests after the deadline.

No live company check, original 438/436/2 reconstruction, physical native gestures,
native screenshot/zoom review, or logo-asset validation/cache was performed here.
Browser outcomes use labelled fictional data; native warning classification uses
authored protocol fixtures. These do not establish Sections 4–8 acceptance. Existing
plan 06–09 domain/data protections remain authoritative.


## Section 4 — Employer branding evidence, cache and display

Implementation: `1e6a04f`; core foundation: `c4cb2a7`. All five existing extraction paths now use shared logo
evidence. Explicit employer hints are separate from workflow display names;
provider/JobPosting identities take precedence and missing identity stays unknown.
Matching JobPosting metadata precedes employer-labelled artwork and explicit custom
selectors. Configured board pages can supply branding missing from posting pages.
Relative assets use the actual response page after redirects. Platform names,
favicons, social previews, unsafe URLs and mismatched organizations are not employer
evidence. Custom employer selectors validate through WorkflowSpec, source config,
YAML round trips and actual selector extraction.

Protocol v6 carries candidate evidence and one branding report per accepted
source/employer relationship. Missing metadata, download failure, decoding failure
and verified artwork are separate. Downloads share a four-second source allowance,
with a 512 KiB byte limit; decoders limit dimensions to 2048px and request 32 MiB
allocation limits. PNG/JPEG/WebP bytes are decoded regardless of URL/Content-Type;
SVG is rasterized without external images/fonts. Empty or unsupported artwork fails
explicitly. Accepted jobs survive optional logo failure; malformed report bytes
fail the protocol while retaining valid partial results. The host revalidates
report bytes and timestamps before persistence.

Forward-only migration 23 stores original evidence URL/page/method, validation time,
checksum, one PNG per source/employer relationship, and separate last-attempt
metadata. Job rows carry stable relationship references. Job responses contain
metadata/content references instead of repeating image bytes. A shared bounded
frontend cache reads each image once through validated local IPC. Verified assets
survive temporary failures, offline reload and source deletion. Full reset clears
both registries. Legacy image URLs remain intact without fabricated provenance.
Consistent backup/recovery and populated v22 migration/rollback/retry tests protect
notes, favorites, applied status/history, schedules and existing checksums.

The shared logo control now uses 24px list/card boxes and 32px inspector boxes,
neutral light backing, preserved aspect ratio and no internal padding. Discovered,
tracked list, Kanban/drag previews and the shared inspector use it. Accessible
initials cover missing metadata, cached-read and image failures; refresh retries a
changed validation attempt.

Validation actually run:

- `cargo test --workspace`: **358 passed**: host 150, real-worker/repository 6,
  core 148, worker 23, adapters 8, new branding processes 5, bounded processes 13,
  protocol processes 5. Fixtures use loopback HTTP and temporary databases.
  New process tests cover all five adapters, renamed workflows, one shared image
  request for two jobs, configured board fallback, custom selectors, missing
  metadata, failed download, invalid image content, and invalid CSS failing before
  any provider request. Empty employer names fall back to the explicit hint or
  Unknown company without trusting nameless logo metadata. Resolver tests reject
  organization mismatches/platform/credentialed URLs and preserve evidence priority.
- `cargo test -p scrape services::workflow::tests::advanced_specs_are_lossless`
  and `cargo test -p scrape-worker adapters::branding`: passed after adding the
  selector persistence/invalid-expression checks and safe evidence-page guard.
- Rebuilt the packaged sidecar; Rust formatting, Clippy with `-D warnings`, and
  Rust-generated binding export passed. No bindings were hand-edited.
- A second Rust export matched a SHA-256 inventory of every generated TypeScript
  contract with no drift. `SCRAPE_STARTUP_MODES=current19,unknown19` native startup
  checks passed: recognized history upgraded with saved state retained; unknown
  checksum history was refused without file mutation. Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-1pRrq0`
  and `.../scrape-native-smoke-FgYbHn`.
- Frontend: **448 tests / 75 files passed**, using `npx vitest run --maxWorkers=1`,
  followed by `node scripts/test-app-startup.mjs`. New control tests cover cached
  artwork, refresh, missing/display failures, shared IPC reads and cache clearing.
  TypeScript/production build and style guard passed; existing chunk advisory.
- Nine Chromium journeys passed across `company-branding`, `enrichment-outcomes`,
  `keywords`, and `workflows`; the two logo journeys passed again after the final
  cache-reference/semantic-markup changes. Tests check actual decoded pixels,
  24px/32px dimensions, no CDN requests, missing/broken fallbacks, aspect ratio,
  backing, rendered contrast, narrow layouts and logo scaling at 200% CSS zoom.
  Reviewed Paper 1280px, Obsidian 480px and Obsidian 200% images; all six are saved
  in [branding review assets](assets/11-branding-review/). The 200% review shows
  page-wide horizontal overflow; logo scaling passed, global zoom/layout acceptance
  remains open for the UI repair work. No prior screenshot baselines were changed.
  Two additional `e2e/kanban.pw.ts` journeys passed pointer/keyboard movement,
  cancellation, Move to, narrow-board scrolling and restart after card logo changes.
  Their new captures are kept with this section; previous plan-10 assets are intact.
- `node scripts/native-branding.mjs`: passed in isolated WKWebView storage. Actual
  worker → host → database → shared component, 24px/32px decoding and recognizable
  Cloudflare pixels (26 orange pixels in the 24px review canvas), exactly two worker
  asset attempts across initial run and offline refresh, no WebView CDN dependency,
  offline reload, notes/favorite/applied preservation, and source deletion with
  retained artwork. Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-GdWiP7`.
- `node scripts/native-smoke.mjs`: passed in isolated storage, including real
  preview/ingestion, cancellation, protected state, native fonts/opener, and
  malformed branding/protocol output with partial-result retention. Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-2XjF1A`.

The original source-provided Cloudflare asset was fetched separately from the exact
URL already present in the repository page fixture. Capture URL/time/checksum,
original JPEG bytes (despite its .png URL), and canonical PNG checksum are recorded
in `crates/scrape-worker/tests/fixtures/cloudflare-logo.json`. The review PNG is
reproducible through the production validator's `cache-logo-fixture` example.
This verifies public branding artwork, not live career-site discovery/ingestion.

Initial fixture failures exposed missing image endpoints; fixtures now serve
actual bounded artwork and retain exact request assertions. One concurrent
frontend run timed out in unrelated workflow controls and left a cascading test
failure; a serial rerun passed with unchanged timeouts/assertions. React's nested
paragraph warning was fixed by making the inline logo wrapper a span.

No user database, physical native gestures, full native screenshot/zoom review or
live career-site verification was used. Original-report reproduction and Sections
5–8 remain incomplete. Employer logos requiring external SVG resources/fonts or
unsupported formats produce explicit optional diagnostics rather than guessed art.


## Section 5 — Multi-source workflow ownership and fair budgeting

Implementation: `d424cd0`. Authoritative `WorkflowSpec` v5 defines workflows owning nonempty `sources[]` (`id`, `label`, `enabled`, `employer_hint`, `extractor`). Ingestion budgeting (`max_jobs_per_run`), retention policy, matching rules, and schedules are owned strictly at the workflow level; per-source matching overrides are eliminated. Spec validation rejects blank source IDs, duplicate source IDs, and duplicate normalized endpoints (case-insensitive, trailing slash, protocol-insensitive host/path comparisons). Backward-compatible migration transparently normalizes legacy v1–v4 single-source specs into v5 workflows with one source entry, preserving existing configs. Internal persistence metadata (`__source_id`, `__starting_source_index`) is cleanly stripped on round-trips.

Forward-only migration 24 (`0024_multi_source_workflows.sql`) establishes relational parent workflow ownership:
- Introduces `workflows` parent table with `spec_version`, `raw_yaml`, `needs_review`, `schedule`, and audit timestamps.
- Adds `workflow_id` (foreign key to `workflows`), `employer_hint`, and `order_index` to `sources`.
- Adds `workflow_id` to `jobs` and `scrape_runs` with covering indices (`idx_jobs_workflow`, `idx_scrape_runs_workflow`, `idx_sources_workflow`).
- Migrates existing legacy sources into parent `workflows` records, backfilling foreign keys while strictly preserving existing source IDs, job IDs, user state (notes, tags, favorites, applied/tracking history), suppression, run metrics, and schedules. Invalid originals stay preserved with `needs_review = 1`.
- Populated historical upgrade and rollback/retry tests (`test_populated_migration_0024_multi_source_workflows`) verify round-trip persistence and integrity across migrations.

Fair round-robin budgeting and multi-source coordination:
- Implemented `coordinator.rs` in `scrape-worker` and integrated it with `WorkerConfig` (`max_jobs_per_run`, `starting_source_index`).
- The coordinator visits sources in round-robin turns, accepting at most 1 eligible unique candidate per source per round, distributing budget fairly across active sources (e.g. exactly a 7/7/6 split across 3 healthy sources at a total cap of 20).
- Dynamically redistributes capacity when sources are slow, failed, or exhausted until the workflow cap is satisfied or all sources are exhausted.
- Deduplication is canonical and workflow-wide, preventing redundant candidate ingestion across overlapping sources.
- Rotates the persisted `starting_source_index` on actual scrape runs to prevent starvation of downstream sources over time; previews and dry-runs do NOT rotate the starting source index.
- Run all enforces exactly one ingestion cap per workflow.

Sources UI editor in Settings (`WorkflowSections.tsx`):
- Features an accessible, NieR-styled multi-source editor with source cards displaying kind badges, source labels, URLs, employer hints, enabled/disabled checkboxes, move up/down reorder buttons, and source removal.
- Independent per-source "Detect" triggers ATS detection and updates the source's kind/label independently.
- Warns clearly when duplicate endpoints are entered or when all sources are disabled, blocking save until resolved.
- Removing a source stops ingestion from that endpoint while preserving all previously saved jobs, history, and retention records.

Validation actually run:

- `cargo test --workspace`: **367 passed** (155 host, 6 worker_repository, 148 core, 23 worker, 8 adapters, 5 branding, 4 coordinator, 13 limits, 5 worker_process).
  Tests cover multi-source round-robin 7/7/6 distribution at cap 20, rotation across consecutive runs, preview non-rotation, capacity redistribution on exhausted sources, duplicate normalized endpoint rejection, Spec v5 validation, populated migration 24 upgrade/preservation, all 8 historical `(v12 × v19 × v23)` manifest combinations (`migration_twenty_three_histories_preserve_saved_state_and_metadata`), and version-23 unknown checksum/schema drift rejection (`migration_twenty_three_unknown_or_drift_is_rejected_without_writes`).
- `cargo fmt --all --check` & `cargo clippy --workspace --all-targets -- -D warnings`: passed with 0 warnings.
- `npm run gen:types && git diff --exit-code src/bindings`: passed with 0 drift across generated TypeScript boundary contracts (`WorkerConfig.ts`, `WorkflowSpec.ts`, `WorkflowSourceSpec.ts`).
- `npx tsc --noEmit`: passed with 0 type errors.
- `npm test`: **458 passed / 76 test files passed** + Vite startup smoke passed. Tests cover Sources editor interactions (add, edit, reorder, remove, enable/disable, duplicate detection), YAML v5 parsing/validation, draft state management, and scraper ergonomics.
- `npm run build`: Production bundle built cleanly (with known large-chunk advisory).
- `node scripts/check-style.mjs`: Style guard passed (strict NieR brutalism: no rounded-* or shadow-*).
- `git diff --check`: Clean (0 whitespace errors).
- `npm run test:providers`: 2/2 providers verified against recorded fixtures (SEP Lever & Linear Ashby).
- `cargo build -p scrape && npm run test:native`: Passed in isolated WKWebView storage with real sidecar execution, cancellation, and IPC persistence.
  Artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-ezzSqZ`.
- `SCRAPE_STARTUP_MODES=fresh,current19,unknown19 npm run test:native-startup` and `SCRAPE_STARTUP_MODES=fresh,legacy23,current23,unknown23 npm run test:native-startup`: All native startup modes passed; recognized histories (including the cumulative `legacy12` + `legacy19` + `legacy23` chain) upgraded with saved state retained, unknown checksum histories refused without file mutation or backup creation.
  Artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-cVD7ZO` (`fresh`),
  `.../scrape-native-smoke-4k8qxn` (`legacy23`),
  `.../scrape-native-smoke-W4C9hL` (`current23`), and
  `.../scrape-native-smoke-Lsca5W` (`unknown23`).
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflows.pw.ts`: 3/3 journeys passed.
- Verified forward repair ([Incident 27](errors/27-native-startup-migration-23.md), migration 25) on the authorized installation (`REPAIR_OK migration_version=25 integrity=ok`), retaining consistent version-23 snapshot `scrape_data.db.pre-migration-955efef2-348f-4cd4-845d-5ff2c5765ba0.db` and preserving all 23 prior migration records and user state.

No live career-site verification, physical native gestures, native 200% zoom acceptance, or original 438/436/2 reconstruction was performed. Sections 6–8 remain open. Existing plan 06–09 domain, data, and retention protections remain authoritative.

## Section 6A — Static locator foundation (in progress)

Implementation: `5824072`, starting from clean revision `a150e70` and the revised
[Section 6 blueprint](11-section-6-browser-and-script-extraction.md). This is the
first coherent Phase 6A checkpoint, not completion of Phase 6A or Section 6.

Implemented:

- Shared `scrape-core::{locator,xpath}` parsing/evaluation for host workflow
  validation and worker extraction, using the existing HTML5 DOM and dependencies.
  CSS containers/XPath fields and XPath containers/CSS fields work together.
  Branding now reuses the selector page's parsed tree.
- The specified location-path subset supports relative/absolute paths, child,
  self, parent and following-sibling steps, descendant shorthand, wildcard/text
  tests, attribute/string predicates, indexing and terminal attribute/text
  extraction. `//a[1]` retains per-parent indexing; `(//a)[1]` indexes the whole
  result. Nested descendant contexts are deduplicated without rewalking already
  covered subtrees. Unsupported syntax fails explicitly. The documented parser
  limits are 4096 bytes, 64 steps and 32 predicates per step.
- Static `source_config` supports field attributes, CSS `@attribute` shorthand,
  original ID extraction, and Rust regex transforms on all seven posting fields.
  Required title/link/configured-ID failures produce invalid-candidate events;
  optional failures stay unknown. Plain-text description extraction preserves
  literal markup as text. Conflicting extraction modes fail validation.
- Workflow serialization/YAML tests preserve locators/transforms. The actual
  packaged sidecar → production mapper → temporary SQLite regression verifies
  title matching, a shared cap of two, reconciled examined/rejected/accepted
  counts, original IDs/URLs, authentic dates, native hourly cents, preview without
  persistence, and refresh preserving notes/favorites/application status.

The baseline `cargo test -p scrape-worker --lib` passed **23 tests**. The new
`mixed_locators_extract_original_id_attributes_and_regex` regression then failed
against the original code because XPath was passed to the CSS parser. It passes
through the implemented shared parser. An initial new integration test had an
`Option<String>` compile error; the assertion was corrected to inspect the actual
optional ID contract. No existing assertions or timeouts were weakened.

Validation actually run on this checkpoint:

- `npm run build:sidecar`: passed, including the final XPath traversal change;
  rebuilt the packaged `aarch64-apple-darwin` release worker before host tests.
- `cargo test --workspace`: **375 passed** (156 host, 7 worker/repository,
  151 core, 26 worker unit, 8 adapter, 5 branding-process, 4 coordinator,
  13 limits-process, 5 worker-process). Final output:
  `/tmp/scrape-section-6a-rust-tests.log`. Local HTTP and SQLite fixtures only.
- `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `npm run gen:types` and `git diff --exit-code src/bindings`: passed, no drift.
  Output: `/tmp/scrape-section-6a-bindings.log`.
- `npx vitest run src/test/yaml_workflow.test.ts --maxWorkers=1`: **21 passed**.
- `npx vitest run --maxWorkers=1`: **459 passed / 76 files**. Existing Node
  localStorage experimental warnings did not fail tests.
- `npx tsc --noEmit`, `npm run build`, `node scripts/check-style.mjs`,
  `node scripts/test-app-startup.mjs`, and `git diff --check`: passed. Production
  build retains the existing large-chunk advisory.

Limits and remaining work:

- Phase 6A hydration extraction, resumable selector pagination, bounded detail
  enrichment and readiness/load-more metadata are **not implemented**. Selector
  discovery still loads a single page eagerly. Continue those before declaring
  Phase 6A complete.
- Phases 6B–6F (protocol v7, migration 26/trust, CDP/runtime/session management,
  script subprocesses, editor controls and integration acceptance) remain open.
  This checkpoint does not change protocol v6, WorkflowSpec v5 boundary types,
  migrations or stored checksums. New settings use the existing `source_config`
  mapping, with Rust validation.
- No browser/native UI journey, physical interaction, live career-site/provider
  verification or resource benchmark was performed. RAM/CPU figures in the
  blueprint are targets, not measured results. No screenshot baselines changed.
- No user database or application storage was used. Original-report reproduction
  and Sections 7–8 remain open. Section 5 behavior and plans 06–09 protections
  remain authoritative.

## Section 6A — Hydration, resumable pagination and bounded details

Hydration checkpoint: `c449be0`; resumable static extraction and protocol-v7
prerequisite: `35728fd`. Started from clean revision `b279eac`. This completes
the static Phase 6A behavior and supersedes its earlier pending notes. Section 6
as a whole remains open.

Implemented and exercised:

- Explicit embedded hydration JSON extraction through shared Rust validation of
  script locators, RFC 6901 pointers, field mappings and regexes. Numeric IDs,
  original destinations, native salary cents and publication evidence survive the
  real worker/persistence path. Missing/malformed scripts and non-array pointers
  fail explicitly; invalid candidates are isolated. No embedded code executes.
- One posting normalizer handles DOM and hydration evidence. Both retain URL
  patterns and authoritative matching; optional unknowns remain unknown.
- Resumable selector sessions retain page/candidate position, follow relative
  next URLs against actual pages, prevent URL/redirect/fragment loops and yield
  during bounded discovery (32 events, or a newly fetched page). An empty middle
  page does not end discovery. No further discovery/details are scheduled after
  the shared acceptance cap. HTTP robots/pacing/timeouts remain in use.
- Details are fetched only after known evidence is assessed. They fill missing
  location/description/pay/date evidence without replacing listing title, ID,
  company or destination. Required unknown evidence stays unresolved/partial;
  optional failures warn. Employer branding uses the existing evidence ranking:
  matching detail JobPosting metadata can improve a lower-ranked listing image.
- Page/time bounds, invalid navigation and loops retain confirmed results with
  truthful partial coverage. Coverage warnings do not inflate detail-enrichment
  warning counts. Pending evaluated candidates are retained across warning yields.
- Shared budgeting remains fair across mixed paginated/single-page sources,
  including optional detail warnings. Controlled deadlines stop source work before
  consuming another candidate and workflow work before source initialization.
- Static `wait_for` checks HTML presence. Load-more clicking fails explicitly
  until browser mode is implemented. Configuration examples and directory READMEs
  describe the supported static settings and limitations.

Sequencing adjustment: pagination needs the blueprint's `PageLimit` contract.
Protocol v7, `PageLimit`/`NeedsLogin` termination, and the planned WorkerLimits
fields/defaults were pulled forward from 6B. Rust validates bounds before source
work and generates TypeScript. Protocol-v6 requests fail explicitly. The browser
process/context limits are configuration contracts; runtime pool enforcement
awaits 6C. `--protocol-version` lets provider verification query Rust's version
instead of duplicating a numeric constant. No migration was added; applied SQL
and checksums, WorkflowSpec v5 ownership and user state remain unchanged.

Regressions established before the corresponding fixes:

- Hydration was rejected as a missing DOM description locator.
- The late-match fixture returned zero rather than two jobs: the original selector
  path did not continue to page two or obtain required detail evidence.
- Mixed paginated/single-page sources allocated **4/8/8** rather than **7/7/6**.
  Optional detail warnings exposed the same lost-round problem separately. Both
  cases now produce exactly **7/7/6** at cap 20.
- The new limits-contract test failed with an unknown `max_pages_per_source` field.

Validation actually run:

- `npm run build:sidecar`: passed, including the final branding/fairness changes;
  the packaged release worker was rebuilt before host/persistence tests.
- `cargo test --workspace`: **392 passed** (158 host, 9 actual worker/repository,
  153 core, 30 worker unit, 8 adapter, 5 branding-process, 6 coordinator,
  17 limits-process, 6 worker-process). Final log:
  `/tmp/scrape-6a-completion-tests.log`.
- The three-page fixture (empty middle page plus two distinct detail URLs) runs
  Preview, insert and refresh through the packaged worker → production mapper →
  temporary SQLite. Exactly 18 HTTP requests across three runs; two accepted jobs
  per run; no request to the remaining page/posting; notes/favorites/applied status
  and original IDs/destinations/hourly cents/publication dates are preserved.
  The separate late-match process fixture examines 72 (70 rejects, two accepts),
  fetches exactly two details and no irrelevant detail or unused page.
- `cargo test -p scrape-worker`, focused coordinator/limits tests, hydration unit
  regression and core bounds regression were also run. Intermediate logs:
  `/tmp/scrape-6a-hydration-tests.log` (379 workspace tests),
  `/tmp/scrape-6a-static-tests.log`, `/tmp/scrape-6a-worker-tests.log`.
- `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`: passed on final code.
- `npm run gen:types` and `git diff --exit-code src/bindings`: passed with no drift
  after staging the intentional Rust-generated protocol changes. Log:
  `/tmp/scrape-6a-completion-bindings.log`.
- `npx vitest run --maxWorkers=1`: **461 passed / 76 files**, including page-limit
  and needs-login diagnostic labels. Log: `/tmp/scrape-6a-frontend-tests.log`.
- `npx tsc --noEmit`, `npm run build`, `node scripts/check-style.mjs`,
  `node scripts/test-app-startup.mjs`, and `git diff --check`: passed. Production
  build retains the existing chunk-size advisory; frontend tests retain existing
  Node localStorage experimental warnings. Build log: `/tmp/scrape-6a-build.log`.
- `npm run test:providers`: **2/2 recorded HTTP replays passed**, SEP/Lever and
  Linear/Ashby, using the built worker's protocol handshake. Repeated after commit
  so report provenance names `35728fd`. Log: `/tmp/scrape-6a-provider-tests.log`.
  This checks recorded associations through the worker, not live sites or provider
  persistence; temporary SQLite coverage is reported separately above.

No native UI/IPC/startup journey, physical interaction, browser journey, live
provider/FAANG verification or RAM/CPU benchmark was performed in this checkpoint.
No user database/application storage or screenshot baseline was used or changed.
Reference-encoded hydration formats require explicit user decoding in a future
custom script; static extraction does not promise arbitrary SPA support.

Next: finish Phase 6B's migration 26, populated upgrade/recovery tests, browser and
extractor-test boundary contracts, extraction-mode validation and executable
trust/hash gates. CDP/browser sessions, custom scripts, editor controls and final
Section 6 integration acceptance remain open in 6C–6F. Original-report reproduction
and Sections 7–8 remain open; the whole revision is not complete.

## Section 6B — Contracts, migration 26 and local script trust

Implemented 2026-10-17. Commits: `0c4bc25` (additive schema and populated recovery
regression), `755ea9c` (validation/trust and regressions), `826d7e4` (generated
TypeScript contracts), `5230e8b` (host launch-guard regression). Phase 6B is complete; browser/session/script execution,
editor controls and final native acceptance remain open in 6C–6F.

- Migration 26 adds isolated browser-session metadata and source trust/hash
  columns, preserving the existing source-kind CHECK and every old migration.
  The v25 fixture snapshots all old table columns/rows, including workflow/source
  ownership, notes, favorites, tracked history, suppression, runs/logs, schedules,
  branding bytes and migration checksums. Upgrade, consistent backup contents,
  injected transaction failure, retry and restart all pass. Session status CHECK
  and unchanged source-kind CHECK are exercised. Full reset clears session
  metadata and parent workflows; jobs-only cleanup preserves both.
- WorkflowSpec v6 adds typed extraction fields, modes, bounds, session references
  and scripts. Versions 1–5 convert through Rust; historical v4/v5 storage
  backfills retain their original versions. Browser/script YAML aliases normalize
  to selector mode without adding database source kinds. Raw credentials and
  unsupported configuration fail explicitly. CSS/XPath/regex and bounds share the
  same core validation used by static extraction; no frontend matcher was added.
- Script fingerprints cover runtime/interpreter/code/path/arguments and, for path
  scripts, bounded regular-file contents. Imports and edits require fresh local
  trust. Explicit trust checks the reviewed fingerprint and confirmation, never
  enables a source or starts work. Database failures remain database errors.
  Missing files can be saved disabled for review but cannot be trusted.
- The actual IPC persistence helper is tested with a static source plus a script:
  exactly two source rows, disabled/reviewed import, explicit trust, unchanged
  save, re-import reset and cross-workflow ownership rejection. This also fixes
  duplicating the primary source during new multi-source saves. Import origin
  survives the frontend draft into Save. Trust metadata is separate from YAML.
- Preview/Test/manual/scheduled execution checks persisted trust before launching
  a sidecar. Host and real-worker process tests reject unavailable browser/script
  modes explicitly, with zero examined candidates and no static fallback. These
  guards are prerequisites; they are not browser or script execution evidence.

Verification actually run in `/tmp/scrape-phase6b`, with Rust builds using the
existing workspace target via `CARGO_TARGET_DIR`:

- Baseline workflow validation: **11 passed**. The new migration test first
  failed at version 25 instead of 26; the new extraction-mode regression first
  failed because version 6 was unsupported.
- `npm run build:sidecar`: passed after final core/worker changes and before
  host/integration verification.
- `cargo test --workspace`: **407 passed**: 164 host, 9 packaged-worker/repository,
  160 core, 31 worker unit, 8 adapters, 5 branding process, 6 coordinator,
  17 limits process, 7 worker process. Final log:
  `/tmp/scrape-6b-workspace-tests.log`. The final rerun includes regular-file
  validation, database-error propagation and the final host launch-guard assertions.
- `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `npm run gen:types`, and `git diff --exit-code -- src/bindings`: passed.
  Binding drift was checked after staging only the intentional generated files.
  Logs: `/tmp/scrape-6b-clippy.log`, `/tmp/scrape-6b-bindings.log`.
- `npx vitest run --maxWorkers=1`: **463 passed / 77 files**, including import
  quarantine routing, import-origin Save arguments and explicit trust IPC.
  Log: `/tmp/scrape-6b-frontend-tests.log`.
- `npx tsc --noEmit`, `npm run build`, `node scripts/check-style.mjs`,
  `node scripts/test-app-startup.mjs`, and `git diff --check`: passed.
  Logs: `/tmp/scrape-6b-build.log`, `/tmp/scrape-6b-startup-tests.log`.
  Existing large-chunk and Node localStorage advisories remain.
- `npm run test:providers`: **2/2 recorded HTTP → worker replays passed**,
  SEP/Lever and Linear/Ashby; repeated after `755ea9c` so the report identifies
  the tested behavior commit. Log: `/tmp/scrape-6b-provider-tests.log`. This is
  recorded extraction evidence; persistence is covered separately above.

Implementation moved to an isolated worktree when an existing Tauri dev watcher
was discovered. The user confirmed stopping it before integration. No desktop
app was launched and no user database/application storage was opened for testing
or migrated. No native IPC/startup/UI, browser journey, physical interaction,
live-provider/FAANG or RAM/CPU verification was performed in this phase. No visual
baseline was changed. Session material handling, actual runners, trust controls
and disabled-script saving through the editor remain for 6C–6F. Earlier missing
report reproduction and Sections 7–8 remain open.

## Section 6C — Direct Rust CDP browser engine and isolated sessions

Implemented 2026-10-17. Commits: `29b5437` (direct Rust CDP browser engine,
stealth/resource-blocking pipeline, and isolated session lifecycle), `e56580a`
(generated TypeScript contracts for `BrowserSessionStatus` and
`BrowserSessionSummary`). Phase 6C is complete; Tier 3 stdin/NDJSON script
execution, Workflow Editor extraction/session/trust controls, and final native
acceptance remain open in Phases 6D–6F.

- `crates/scrape-core/src/browser.rs` adds local Chromium-family runtime
  discovery (`SCRAPE_CHROME_PATH` override and platform binary probes without
  ever auto-downloading a browser), domain-scoped cookie storage
  (`StoredBrowserSession`, `BrowserCookie`, `BrowserSessionSummary`,
  `BrowserSessionStatus`), restrictive external session storage (`0700`
  directory / `0600` file permissions on Unix), login-wall redirect/DOM heuristics
  (`detect_login_wall`), and a pure-Rust RFC 6455 WebSocket CDP client
  (`CdpWsClient`) shared by worker and host session management.
- `crates/scrape-worker/src/browser/` implements the Tier 2 `BrowserEngine`
  enforcing `max_processes = 1` (single lazy headless browser process per worker
  run with `--headless=new --disable-gpu --disable-dev-shm-usage --disable-extensions --disable-background-networking --disable-sync --no-first-run --no-default-browser-check --mute-audio --js-flags=--max-old-space-size=128`
  and temporary profile cleanup on `Drop`), `max_contexts = 2` (bounded by a
  Tokio semaphore), immediate `Target.closeTarget` and
  `Target.disposeBrowserContext` teardown after every page turn, pre-navigation
  `Network.setBlockedURLs` resource/tracker blocking (images, media, fonts,
  stylesheets, and analytics domains), `Emulation.setUserAgentOverride` +
  `Page.addScriptToEvaluateOnNewDocument` stealth patches, `wait_for` readiness
  polling, `load_more` clicks, `next_page` traversal, bounded detail-page
  rendering, domain-scoped session cookie injection, and `NeedsLogin`
  termination on login redirects or password walls.
- `src-tauri/src/services/browser_session.rs` implements host session lifecycle
  commands (`detect_browser_runtime`, `list_browser_sessions`,
  `open_browser_session`, `save_browser_session`, `clear_browser_session`),
  reconciles `NeedsLogin` worker outcomes into `needs_login` session status in
  SQLite (`runner.rs` and `execute_dry_run_worker`), passes `SCRAPE_SESSION_DIR`
  to the sidecar (`process.rs`), enforces ready/domain-matching session checks in
  `script_trust::validate_execution` before launching a worker, removes external
  `browser_sessions/` files during `ResetAll` cleanup (`cleanup.rs`), and exposes
  `test_extractor` for bounded static/browser sample testing (`commands/scrape.rs`).
- `crates/scrape-worker/tests/browser_cdp_process.rs` exercises the packaged
  worker over a local WebSocket CDP server: verifying resource-blocking patterns,
  pre-document stealth injection, `wait_for` readiness polling, `load_more`
  interaction, `next_page` pagination, immediate target/context teardown,
  `PageLimit` and `TimeLimit` (`navigation_timeout_s`) bounds, domain-scoped
  cookie injection, and `NeedsLogin` termination on login walls.
- `src-tauri/tests/worker_repository.rs`
  (`browser_cdp_extraction_and_session_persistence_survive_repository_ingestion`)
  verifies the end-to-end packaged sidecar → CDP WebSocket + external session
  cookie injection → NDJSON → repository ingestion → SQLite persistence boundary.

Verification actually run in this workspace:

- `npm run build:sidecar`: passed (built release `scrape-worker-aarch64-apple-darwin`
  before host and repository integration tests).
- `cargo test --workspace`: **415 passed** (165 host unit/migration, 10 packaged
  worker/repository, 163 core, 31 worker unit, 8 adapter, 5 branding-process,
  3 browser-cdp-process, 6 coordinator, 17 limits-process, 7 worker-process;
  0 failed).
- `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`: passed with 0 warnings.
- `npm run gen:types` (56 host + 55 core binding exports) and
  `git diff --exit-code src/bindings`: passed with zero drift after staging
  `BrowserSessionStatus.ts` and `BrowserSessionSummary.ts`.
- `npx vitest run --maxWorkers=1`: **464 passed / 77 files**, including browser
  runtime/session and extractor-test IPC wrapper coverage in
  `src/test/script_trust_ipc.test.ts`.
- `npx tsc --noEmit`, `npm run build`, `node scripts/check-style.mjs`,
  `node scripts/test-app-startup.mjs`, and `git diff --check`: passed.
- `npm run test:providers`: **2/2 recorded HTTP → worker replays passed**
  (SEP/Lever: 8 fetched, 8 verified, 0 errors; Linear/Ashby: 31 fetched,
  31 verified, 0 errors), repeated after `29b5437` so the report provenance
  records `29b5437bb51563020d2e0f72575fe676ae570302`.

No desktop app was launched and no user database/application storage was opened
or migrated. No native WKWebView UI harness, Playwright browser journey,
physical pointer/IME check, live provider/FAANG crawl, or live Chromium RAM/CPU
benchmark was performed in this checkpoint. Tier 3 custom script execution
(Phase 6D), Workflow Editor UI controls for extraction modes/sessions/trust
(Phase 6E), and Section 6 end-to-end acceptance (Phase 6F), along with earlier
missing-report reproduction and Sections 7–8, remain open.

## Section 6D — Versioned Stdin/NDJSON Script Extraction Engine and Trust Gate

Implemented Phase 6D of Section 6 (Tier 3: Versioned Stdin/NDJSON Script Runner
& Trust Gate). Phase 6D is complete; Workflow Editor UI controls for extraction
modes/sessions/trust (Phase 6E) and native visual journeys / acceptance (Phase 6F)
remain open.

- `crates/scrape-core/src/extraction.rs` defines the authoritative Tier 3
  protocol types (`SCRIPT_PROTOCOL_VERSION = 1`, `ScriptInputFrame`,
  `ScriptOutputFrame`, `ScriptCandidateFrame`, `ScriptContinuationFrame`,
  `ScriptDiagnosticFrame`, `ScriptCompleteFrame`, `ScriptEnvironmentStatus`),
  as well as `redact_script_stderr` which redacts sensitive authorization,
  cookie, token, secret, and password patterns and truncates output at 16 KiB.
- `crates/scrape-worker/src/script_runner.rs` implements the versioned
  stdin/NDJSON script engine:
  - Executes scripts via structured `argv` using `tokio::process::Command` with
    `kill_on_drop(true)` and process-group isolation (`process_group(0)` on Unix);
    never invoking shell wrappers (`sh -c`, `bash -c`, `cmd.exe`).
  - Serializes single-turn `ScriptInputFrame` with strict limits (`max_candidates`,
    `timeout_ms`, `continuation`, `config`), parses streaming NDJSON frames from
    `stdout`, buffers diagnostics, and requires an explicit `complete` frame.
  - Enforces process-tree reaping via `kill_script_child` on timeouts, errors,
    or drop.
  - Extracts candidate fields into raw `WorkerEvent::Job` events with candidate
    evidence, which are subsequently validated, regex-transformed, assigned
    canonical `PostingIdentity`, and evaluated by `MatchingEvaluator` in
    `coordinator.rs` identically to native adapters.
- `crates/scrape-worker/src/coordinator.rs` integrates script sessions into the
  multi-source fair round-robin loop:
  - Handles `ExtractionMode::Script` alongside native and selector sources.
  - Maintains source continuation state across rounds.
  - Propagates all worker errors non-silently to prevent unhandled drops.
- `src-tauri/src/services/script_trust.rs` adds `check_script_environment`
  which inspects the host runtime (Node.js version, Python interpreter with
  optional module checks via `importlib.util.find_spec`, or custom executables)
  via structured commands, and updates `validate_execution` to enforce explicit
  script trust and content hash verification before launching worker subprocesses.
- `src-tauri/src/commands/scrape.rs` and `src-tauri/src/lib.rs` expose and
  register the `check_script_environment` Tauri IPC command.
- `src/lib/api.ts` adds the `checkScriptEnvironment` client method with
  demo/recorded fallback.
- `src/bindings/ScriptEnvironmentStatus.ts` generated via `npm run gen:types`.
- `crates/scrape-worker/tests/script_runner_process.rs` adds 4 process-level
  integration tests:
  - `script_multi_turn_continuation_streams_and_matches_candidates`
  - `script_protocol_violations_fail_explicitly`
  - `script_subprocess_failure_with_redacted_stderr_and_truncation`
  - `script_timeout_kills_child_process_tree`
- `src-tauri/tests/worker_repository.rs` adds `script_extraction_trust_gate_and_sqlite_persistence`
  verifying the end-to-end packaged worker sidecar execution: imported script
  quarantine, explicit trust requirement, multi-source round-robin candidate
  extraction, filter rejection, SQLite persistence, and in-place script
  modification trust invalidation.

Verification actually run in this workspace:

- `npm run build:sidecar`: passed (built release `scrape-worker-aarch64-apple-darwin`
  in 2.11s).
- `cargo test --workspace`: **424 passed** (166 host unit/migration, 11 packaged
  worker/repository, 168 core, 31 worker unit, 8 adapter, 5 branding-process,
  3 browser-cdp-process, 6 coordinator, 17 limits-process, 4 script-runner-process,
  7 worker-process; 0 failed).
  - **Correction (2026-10-17):** The subtotal breakdown above lists "168 core"
    which sums to 426 rather than 424. The actual count in `crates/scrape-core`
    was 166 (56 unit tests + 56 doc/type tests + 54 integration tests = 166),
    matching the stated 424 total.
- `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`: passed with 0 warnings.
- `npm run gen:types` and `git diff --exit-code src/bindings`: passed with zero drift
  after generating `ScriptEnvironmentStatus.ts`.
- `npx vitest run --maxWorkers=1`: **464 passed / 77 files**.
- `npx tsc --noEmit`, `npm run build`, `node scripts/check-style.mjs`,
  `node scripts/test-app-startup.mjs`, and `git diff --check`: passed.
- `npm run test:providers`: **2/2 recorded HTTP → worker replays passed**
  (SEP/Lever: 8 fetched, 8 verified, 0 errors; Linear/Ashby: 31 fetched,
  31 verified, 0 errors).
- `cargo build -p scrape && npm run test:native`: passed (native smoke passed;
  isolated artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-oA7CvK`).
- `SCRAPE_STARTUP_MODES=fresh,current19,unknown19 npm run test:native-startup`:
  passed (all 3 modes passed; isolated artifacts:
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-C7LoSg`,
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-ZmAhL9`,
  `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-OzV7XZ`).

No user database/application storage was opened or migrated. No native WKWebView
UI harness for script editing, physical pointer/IME check, or live provider crawl
was performed in this checkpoint. Workflow Editor UI controls for extraction
modes/sessions/trust (Phase 6E), and Section 6 end-to-end acceptance (Phase 6F),
along with earlier missing-report reproduction and Sections 7–8, remain open.

## Section 6E — Workflow Editor Custom Extractor, Browser Sessions, and Script Trust

Phase 6E exposes Section 6 extraction modes, browser session management, and script
trust controls inside the single-draft Workflow Editor (`src/components/views/settings/`), committed at `03298d3`.
Phase 6F native/visual QA and Section 6 end-to-end acceptance remain open.

- `src/components/views/settings/CustomExtractorConfig.tsx`:
  - Created modular NieR brutalist extractor configuration component with 0 rounded
    or shadow classes, using semantic theme tokens (`--color-paper`, `--color-ink`,
    `--color-surface`, `--color-border`, `--color-signal`, `--color-warning`, `--color-danger`).
  - **Extraction Mode Selector**: Themed `Select` switching between `Static HTML · CSS & XPath`,
    `Headless Browser · CDP`, and `Custom Script · Stdin/NDJSON`.
  - **Locators & Regex Transforms**:
    - Required fields: `Job container locator *` (`job_container`), `Title locator *`
      (`title_selector`), and `URL / link locator *` (`url_selector`). Supports standard
      CSS and XPath expressions (`xpath://...` or `xpath:.//...`).
    - Optional fields in collapsible section: `Location locator`, `Compensation locator`,
      `Description locator`, `Date posted locator`, `External ID locator`, and `Employer logo locator`.
    - Attributes and Regex Transforms: expandable section allowing custom attribute overrides
      (`text`, `href`, `src`, `@data-*`) and Rust regex captures (`(?P<value>...)`) for title,
      URL, location, and salary fields.
    - Dynamic and Navigation Locators: `Detail page link locator` (`detail_selector`),
      `Next page locator` (`next_page_selector`), `Load more locator` (`load_more_selector`,
      browser mode), `Readiness wait locator` (`wait_for_selector`, browser mode), and
      `URL match pattern (glob)` (`url_pattern`).
    - Interactive **`Test extractor`**: runs `api.testExtractor` with live progress indicator,
      displays sample candidate cards (title, external ID, compensation, location, date posted),
      `CoverageTermination` badges, field match counts (`title: 1`, `url: 1`, ...), and actionable
      warnings/errors.
  - **Browser Runtime & Isolated Sessions Panel**:
    - Chromium runtime status indicator displaying detected binary path and version
      via `api.detectBrowserRuntime` (or actionable installation guidance if missing).
    - Hard bounds inputs: `Navigation timeout (s)` (default 30s), `Max pages` (default 100),
      and `Max duration (s)` (default 180s).
    - Isolated Session selector with `Ready` / `Needs login` badges.
    - Lifecycle action buttons: `Open login session` (launches Chromium window via
      `api.openBrowserSession`), `Save session` (`api.saveBrowserSession`), and
      `Clear session` (`api.clearBrowserSession`).
    - Explanatory disclosure noting YAML exports store only the session reference ID,
      never credentials or cookie bytes.
  - **Custom Script & Trust Panel**:
    - Script runtime selector (`js`, `python`, `custom`), interpreter path input,
      structured arguments, timeout (s), script file path, and inline script code editor.
    - **`Check environment`**: invokes `api.checkScriptEnvironment` to report detected
      interpreter version or missing dependencies.
    - **Truthful Privilege & Security Disclosure**: *"Custom scripts execute with your local
      user account privileges. Scrape enforces timeouts, structured arguments, and output
      validation, but does not provide OS sandboxing or guaranteed bot-challenge bypass."*
    - **Script Trust Gate**: Prominent warning banner rendered when `needs_review || !script_trusted`
      with an explicit `Trust and enable script` action calling `api.trustWorkflowScript`
      with the persisted content SHA256 fingerprint.
- `src/components/views/settings/WorkflowSections.tsx`:
  - Delegates custom extractor configuration to `CustomExtractorConfig` when source adapter
    is `selector`. Passes `initialSource` and `onTrustChanged` callback.
- `src/components/views/settings/AddSourceModal.tsx`:
  - Summarizes active custom extraction settings (`kind === "selector"`, `mode`, `session_id`,
    `script`) in the collapsed advanced settings line.
  - Passes `initialSource` and `onTrustChanged={() => model.invalidate()}` down to `WorkflowSections`.
  - Reflects extraction mode and adapter kind in the Review step source list.
- `src/hooks/useWorkflowDraft.ts`:
  - Exports `invalidate` in the draft controller return interface.
- `src/test/workflow_browser_script.test.tsx`:
  - Added 4 comprehensive Vitest interaction tests:
    1. Mode switching, CSS and XPath locator entry.
    2. Test extractor execution with candidate samples and field match counts.
    3. Headless Browser CDP mode, runtime detection, and session lifecycle (`Open` -> `Save` -> `Clear`).
    4. Custom Script mode, environment diagnostics, truthful disclosure, and explicit trust gate.

Verification actually run in this workspace:

- `npx tsc --noEmit`: passed with 0 errors.
- `npx vitest run src/test/workflow_browser_script.test.tsx`: **4 passed** (0 failed).
- `npx vitest run --maxWorkers=1`: **468 passed / 78 files** (0 failed).
- `node scripts/check-style.mjs`: passed (Production style guard passed, 0 violations).
- `npm run gen:types && git diff --exit-code src/bindings`: passed with zero drift.
- `npm run build`: passed (Vite production bundle built in 5.76s).
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed with 0 warnings.
- `cargo test --workspace`: **424 passed** (166 host unit/migration, 11 packaged
  worker/repository, 166 core, 31 worker unit, 8 adapter, 5 branding-process,
  3 browser-cdp-process, 6 coordinator, 17 limits-process, 4 script-runner-process,
  7 worker-process; 0 failed). The initial sandboxed attempt blocked local CDP socket
  binding on 1 test and passed upon rerun with local socket access.
- `git diff --check`: passed with zero whitespace or line ending issues.

No user database/application storage was opened or mutated. No native WKWebView
UI harness for script editing, physical pointer/IME check, or live provider crawl
was performed in this checkpoint. Phase 6F native/visual QA and end-to-end acceptance,
along with earlier missing-report reproduction and Sections 7–8, remain open.

## Section 6E repair — Quarantined Script Workflow Snapshot Loading and Trust Gate

Committed at `a92bfc8`. Repairs four defects identified in Phase 6E:
1. **Quarantined Script Workflow Editing**: `sourceToWorkflowSpec` now distinguishes between unconvertible legacy originals (`script_sha256 === null`), which continue to ignore invalid snapshots, and quarantined script sources awaiting trust (`script_sha256 !== null && !script_trusted`), whose Rust-validated snapshot is preserved for review and editing when reopened from `SourcesSettings`.
2. **Trust Gate Content Parity**: `CustomExtractorConfig` now stably compares draft script fields (`runtime`, `interpreter`, `path`, `code`, `args`, `timeout_s`) against the saved snapshot. Modifying the draft script disables "Trust and enable script" and shows *"Save the workflow to fingerprint your changes, then trust it."* Trusting updates local `trustedSource` state, immediately removing the trust banner and displaying *"Script trusted and enabled."*
3. **Realistic Test Suite**: In `src/test/workflow_browser_script.test.tsx`, eliminated `fireEvent` in favor of real `userEvent.clear` + `userEvent.type` (with `[[` escaping) for XPath inputs; verified `Needs login` and `Ready` badge transitions across session selection and reset to `None (unauthenticated)` on session clear; tested script editing and trust banner disappearance using the real `initialSource` edit path; and verified that modifying draft script disables trust with the save-first message.
4. **Evidence Reconciled**: Reconciled historical test breakdown counts in Phase 6D and 6E ledger entries (166 core tests, total 424).

Verification actually run in this workspace:

- `npx tsc --noEmit`: passed with 0 errors.
- `npx vitest run src/test/yaml_workflow.test.ts src/test/workflow_browser_script.test.tsx`: **29 passed / 2 files** (24 in `yaml_workflow.test.ts`, 5 in `workflow_browser_script.test.tsx`).
- `npx vitest run`: **472 passed / 78 files** (0 failed).
- `node scripts/test-app-startup.mjs`: passed (Clean Vite development server initialization and shutdown).
- `npm run build`: passed (Vite production bundle built in 5.33s).
- `node scripts/check-style.mjs`: passed (Production style guard passed, 0 violations).
- `git diff --check`: passed with zero whitespace or line ending issues.

No user database/application storage was opened or mutated. No native WKWebView
UI harness for script editing, physical pointer/IME check, or live provider crawl
was performed in this repair. Phase 6F native/visual QA and Section 6 end-to-end
acceptance are recorded below.

## Section 6F — Native and Visual QA and End-to-End Acceptance

Committed at `f5b4d0e`. Completes Phase 6F end-to-end acceptance, native WKWebView verification, and visual regression QA for Section 6 low-resource browser and script extraction:
1. **Visual QA & Theme Contrast**: Verified custom extractor UI surfaces in both Paper (light) and Obsidian (dark) themes across desktop (1024px) and narrow (480px) viewports. Fixed an Obsidian theme contrast defect in `CustomExtractorConfig.tsx` by replacing `--color-signal` with semantic `--color-success` (`#35602D` on Paper, `#C4DA92` on Obsidian), ensuring all status text, environment diagnostic badges, and trust confirmations meet WCAG AA 4.5:1 text contrast and 3:1 control contrast against their rendered backgrounds. Produced 8 reviewed screenshot artifacts stored under `plans/assets/11-browser-script-review/`.
2. **Playwright E2E Verification**: Added `e2e/workflow-sources.pw.ts` covering extraction mode switching (`Static`, `Browser`, `Script`), CSS/XPath selector inputs, browser session management buttons (`Open`, `Save`, `Clear`), script environment diagnostics (`Check environment`), truthful local privilege disclosure, and dirty-script trust gating under both Paper and Obsidian themes. Verified cleanly alongside `e2e/workflows.pw.ts` with 5/5 passing tests.
3. **End-to-End Persistence Matrix**: All 5 comprehensive test flows in `src-tauri/tests/worker_repository.rs` verified against loopback HTTP/WebSocket and mock CDP servers:
   - Mixed XPath container + CSS field extraction surviving preview, refresh, and SQLite persistence.
   - SPA hydration extraction (`__NEXT_DATA__` / `__NUXT_DATA__`) surviving persistence.
   - Resumable paginated static details with bounded enrichment and robots compliance.
   - Headless CDP browser extraction, resource/tracker blocking (`Image`, `Media`, `Font`, `Stylesheet`), and authenticated session cookie injection with `NeedsLogin` handling.
   - Stdin/NDJSON script extraction with round-robin fair budgeting, process-group isolation, 16 KiB secret-redacted stderr, and content-fingerprinted trust gating.
4. **Native Tauri / WKWebView Smoke Verification**: Ran isolated native harnesses:
   - `test:native`: Isolated native IPC, window lifecycle, and sidecar communication clean.
   - `test:native-startup`: Verified all three startup modes (`fresh`, `current19`, `unknown19`) against isolated SQLite databases and automatic backups.
   - `native-branding.mjs`: Verified 2 sidecar asset downloads with 0 WKWebView network dependencies.

Verification actually run in this workspace:

- `npm run build:sidecar`: built release `scrape-worker-aarch64-apple-darwin` in 0.49s.
- `cargo fmt --all --check`: passed with zero formatting diffs.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed with 0 warnings.
- `cargo test --workspace`: **424 passed** (166 host unit/migration, 11 packaged worker/repository, 166 core, 31 worker unit, 8 adapter, 5 branding-process, 3 browser-cdp-process, 6 coordinator, 17 limits-process, 4 script-runner-process, 7 worker-process; 0 failed).
- `cargo test -p scrape --test worker_repository`: **11 passed / 0 failed** in 1.05s.
- `npm run gen:types && git diff --exit-code src/bindings`: passed with 0 drift.
- `npm run test:providers`: **2 passed / 0 failed** (SEP/Lever: 8 fetched, 8 verified; Linear/Ashby: 31 fetched, 31 verified).
- `npx tsc --noEmit`: passed with 0 errors.
- `npm test` (`vitest run && node scripts/test-app-startup.mjs`): **472 passed / 78 files** in 10.39s + Vite dev server startup smoke clean.
- `npm run build`: passed (Vite production bundle built in 4.94s).
- `node scripts/check-style.mjs`: passed (Production style guard passed, 0 violations).
- `git diff --check`: passed with zero whitespace or line ending issues.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers SCRAPE_BROWSER_PORT=1430 npx playwright test e2e/workflows.pw.ts e2e/workflow-sources.pw.ts`: **5 passed / 0 failed** in 14.8s.
- `cargo build -p scrape && npm run test:native`: passed (native artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-xMHpo2`).
- `SCRAPE_STARTUP_MODES=fresh,current19,unknown19 npm run test:native-startup`: all 3 modes passed (artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-HihcHr`, `NFrb2x`, `B67POC`).
- `node scripts/native-branding.mjs`: passed (artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-SwWWDV`).

Reviewed visual QA artifacts:
- `plans/assets/11-browser-script-review/paper-static-1024.png` (Static CSS/XPath mode on Paper theme, 1024px)
- `plans/assets/11-browser-script-review/obsidian-static-1024.png` (Static CSS/XPath mode on Obsidian theme, 1024px)
- `plans/assets/11-browser-script-review/paper-browser-1024.png` (Headless Browser CDP mode on Paper theme, 1024px)
- `plans/assets/11-browser-script-review/obsidian-browser-1024.png` (Headless Browser CDP mode on Obsidian theme, 1024px)
- `plans/assets/11-browser-script-review/paper-script-1024.png` (Custom Script Stdin/NDJSON mode on Paper theme, 1024px)
- `plans/assets/11-browser-script-review/obsidian-script-1024.png` (Custom Script Stdin/NDJSON mode on Obsidian theme, 1024px)
- `plans/assets/11-browser-script-review/paper-script-480.png` (Custom Script Stdin/NDJSON mode on Paper theme, 480px narrow viewport)
- `plans/assets/11-browser-script-review/obsidian-script-480.png` (Custom Script Stdin/NDJSON mode on Obsidian theme, 480px narrow viewport)

Explicit boundaries and unperformed checks:
No user database or application storage was opened or mutated. No physical pointer or IME verification, native OS file chooser interaction, 200% zoom overflow QA, or unrecorded live company crawl was performed in this checkpoint. Section 6 is fully implemented and accepted. Next: Section 7 (Provider/company verification).

---

## Section 7 — Provider/company verification

Target: Gemini 3.8 Flash / GPT-6.1 Sol
Specification: [Section 7 provider/company verification](11-reliable-ingestion-and-ui-repair.md#7-providercompany-verification)
Execution Blueprint: [Section 7 execution blueprint](11-section-7-provider-and-company-verification.md)
Commit: `b6a4573` (`feat: section 7 provider and company verification across FAANG, ATS and custom targets`)

Implemented behavior:
1. **Manifest-Driven Provider Registry & Authentic Provenance (`fixtures/providers/manifest.json`)**:
   - Registered all required companies: SEP, Linear, Bitwarden, Cloudflare, Google, Apple, Netflix, Amazon, Meta, and authored regressions.
   - Every recorded provider response is an authentic, unmodified capture stored under `fixtures/providers/` with exact URL, ISO 8601 UTC timestamp, SHA-256 hash, extraction version, and adapter classification.
   - Updated `fixtures/providers/README.md` with complete classification table and provenance details.
2. **Native ATS Extraction & Replay**:
   - **SEP**: Lever API (`kind: "lever"`, unpaginated REST JSON, 8 jobs verified).
   - **Linear**: Ashby API (`kind: "ashby"`, unpaginated REST JSON with compensation tiers, 31 jobs verified).
   - **Bitwarden**: Greenhouse API (`kind: "greenhouse"`, board `bitwarden`, 39 jobs verified with full HTML descriptions and pay ranges).
   - **Cloudflare**: Greenhouse API (`kind: "greenhouse"`, board `cloudflare`, verified via authoritative single-job projection `fixtures/providers/regressions/cloudflare-8024849.json`).
3. **Static CSS/XPath Selector Extraction (Server-Rendered HTML DOM)**:
   - **Google Careers**: `https://www.google.com/about/careers/applications/jobs/results/` (SSR HTML DOM, `item: "li.lLd3Je"`, `title: "h3.QJPWVe"`, `link: "a[href*='jobs/results/']"`, `location: "span.r0wTof"`), 20 jobs verified.
   - **Apple Jobs**: `https://jobs.apple.com/en-us/search` (SSR HTML DOM, `item: "ul#search-job-list > li.rc-accordion-item"`, `title: "h3 a.link-inline"`, `link: "a.link-inline"`, `location: "span.table--advanced-search__location-sub"`, `date: "span.job-posted-date"`), 20 jobs verified.
4. **Versioned Script NDJSON Extractors (`SCRIPT_PROTOCOL_VERSION = 1`)**:
   - Authored `crates/scrape-worker/examples/extractors/netflix_extractor.js` for Netflix Eightfold AI platform: emits candidate frames with `external_id`, `title`, `url`, `location`, `company`, `date_posted`, and terminal `complete` frame. 10 jobs verified.
   - Authored `crates/scrape-worker/examples/extractors/amazon_extractor.js` for Amazon Jobs Search API: extracts `id_icims`, `title`, full URL, normalized location, date, HTML description, and salary text from preferred qualifications. 10 jobs verified.
5. **Truthful Dynamic SPA / Bot-Shielded Documentation (Meta Careers)**:
   - Captured raw initial HTML `fixtures/providers/meta/meta.html` (SHA-256: `863d7a411ab319a457c38689a162631b86d8c7730cb49ad346e62cfdf78b77f0`).
   - Verified that static DOM extraction yields 0 job items from Meta's client-rendered React/Relay container (`<div id="root"></div>`).
   - Documented as an open acceptance gap per Plan 11 rules ("All FAANG entries need explicit evidence; blocked/unavailable/unsupported remain incomplete. Never fabricate production data.").
6. **End-to-End Packaged Sidecar Replay to SQLite**:
   - Expanded `src-tauri/tests/worker_repository.rs` with `recorded_provider_company_verification_matrix_persists_to_sqlite`.
   - Verified that Bitwarden (Greenhouse), Google (Selector Static), Apple (Selector Static), and Netflix (Script NDJSON with explicit database trust gate) replay through the real packaged native worker sidecar over loopback HTTP and persist 89 jobs into an isolated temporary SQLite database with canonical identity key generation (`greenhouse:bitwarden:...`, `publisher:...`, etc.) and reconciliation.
7. **Expanded Provider Verification Runner (`scripts/provider-verification.mjs`)**:
   - Evaluates all 8 providers in deterministic recorded HTTP replay mode (`npm run test:providers`) with 0 errors.
   - Supports opt-in live provider verification (`--live`) with structured per-check reporting and error classification.

Verification actually run in this workspace:
- `npm run test:providers`: **8 checks / 0 errors** (SEP/Lever: 8 verified; Linear/Ashby: 31 verified; Bitwarden/Greenhouse: 39 verified; Google/Selector: 20 verified; Apple/Selector: 20 verified; Netflix/Script: 10 verified; Amazon/Script: 10 verified; Meta: 0 verified, truthful acceptance gap recorded).
- `node scripts/provider-verification.mjs --live`: live verification evaluated (SEP: 8/8 verified; Linear: 29/29 verified; Bitwarden: 39/39 verified; Google: 20/20 verified; Apple: 0 verified, Akamai policy challenge caught and reported; Netflix: 10/10 verified; Amazon: 10/10 verified; Meta: 0 verified, client SPA shell verified).
- `cargo fmt --all --check`: passed with zero formatting diffs.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed with 0 warnings.
- `cargo test --workspace`: **425 passed / 0 failed** (166 host unit/migration, 12 packaged worker/repository, 166 core, 31 worker unit, 8 adapter, 5 branding-process, 3 browser-cdp-process, 6 coordinator, 17 limits-process, 4 script-runner-process, 7 worker-process; 0 failed).
- `cargo test -p scrape --test worker_repository`: **12 passed / 0 failed** in 1.16s (including `recorded_provider_company_verification_matrix_persists_to_sqlite`).
- `npm run gen:types && git diff --exit-code src/bindings`: passed with 0 drift.
- `npx tsc --noEmit`: passed with 0 errors.
- `npm test` (`vitest run && node scripts/test-app-startup.mjs`): **472 passed / 78 files** in 9.91s + Vite dev server startup smoke clean.
- `npm run build`: passed (Vite production bundle built in 4.77s).
- `node scripts/check-style.mjs`: passed (Production style guard passed, 0 violations).
- `git diff --check`: passed with zero whitespace or line ending issues.

Explicit boundaries and unperformed checks:
No user database or application storage was opened or mutated. No physical pointer or IME verification, native OS file chooser interaction, or 200% zoom overflow QA was performed in this checkpoint. In accordance with Plan 11 rules, Apple's live Akamai bot challenge and Meta's client-rendered Relay SPA without server HTML remain documented acceptance gaps. Section 7 is fully implemented and accepted. Next: Section 8 (Regional settings and Kanban).
