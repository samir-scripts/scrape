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
