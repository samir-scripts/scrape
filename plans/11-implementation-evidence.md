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

Core foundation: `c4cb2a7`. All five existing extraction paths now use shared logo
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
