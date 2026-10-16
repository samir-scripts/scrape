# Reliable ingestion implementation evidence

Revision contract: [plan 11](11-reliable-ingestion-and-ui-repair.md). Work began on a clean tree at `17b13de` after committing the specification. Existing review assets were preserved. The six original screenshots and exact active workflow configuration were not supplied in this context, so the exact reported Cloudflare run remains unverified. Tests never use the live user database.

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

## Section 3 — In progress

Shared Rust partial matching now distinguishes a known rejection from unresolved
fields, including nested OR/NOT, exclusions and multiple search fields. Complete
matching predicates remain authoritative. Deterministic completion-property tests
prove that any decision made from known fields holds under every tested completion
of the missing fields. Core tests and `cargo clippy -p scrape-core --all-targets -- -D warnings`
passed for this foundation. Adapter, outcome, migration and integration acceptance
are still in progress; this checkpoint does not claim Section 3 completion.
