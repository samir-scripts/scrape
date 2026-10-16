# Reliable ingestion implementation evidence

Revision contract: [plan 11](11-reliable-ingestion-and-ui-repair.md). Work began on a clean tree at `17b13de` after committing the specification. Existing review assets were preserved. The six original screenshots and exact active workflow configuration were not supplied in this context, so the exact reported Cloudflare run remains unverified. Tests never use the live user database.

## Section 1 — Matching diagnostics foundation

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
