# Bounded ingestion and data management: implementation evidence

## Section 2 — Bounded ingestion (2026-10-15)

Implemented against [the approved addition](09-bounded-ingestion-and-data-management.md), on `implementation/trustworthy-scraping`.

### Delivered behavior

- Optional workflow `max_jobs_per_run`: 20 by default, positive integer up to 10,000. Existing stored workflows receive the default without schema changes or deletion. Create/edit/import/export/source persistence preserve it. Review describes the effective cap; invalid text blocks preview/continuation and edits invalidate stale previews.
- Every workflow has its own budget for manual, scheduled, and Run all execution. Preview and Test use min(configured limit, 10). A refresh of an existing posting counts. Duplicate, invalid, rejected, and blocked records do not consume the acceptance budget.
- One Rust `PostingGate` applies canonical identity, filters, duplicate detection, suppression, and acceptance counting. All five production adapters use a stopping sink; Greenhouse stops detail requests and JSON-LD stops extraction/followed-page requests when it returns false. Raw adapter collector functions delegate to this same extraction path for tests.
- Provider responses containing the entire board still download and decode as one response; processing/enrichment stops at the limit. Existing request/time safety bounds remain. Matching follows provider order; this does not implement rotation or a total database capacity limit.
- Workers receive a consistent suppression snapshot; persistence retains its transactional suppression recheck. The host independently rejects excess worker job events using per-workflow budgets. A concurrent block can leave a run below the requested limit.
- Worker protocol is now version 3. Generated `SourceScanStats` reports examined/matched/blocked/duplicate/invalid counts, effective cap, and limit reached. Run logs record partial coverage; preview labels its potentially incomplete coverage. The packaged sidecar was rebuilt.
- Rust owns default/preview/maximum constants and exports `ingestionLimits.ts` during normal binding generation. Frontend consumers share those generated constants. No generated binding was hand-edited.

### Verification

| Command / boundary | Observed result |
|---|---|
| `cargo test --workspace` | 288 passed, zero failures, including generated binding tests and 4 packaged-worker/repository integration tests. |
| `cargo test --workspace export_bindings` | Passed; generated the updated worker/workflow contracts and limit constants. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| `cargo fmt --all --check` | Passed. |
| `npm test` | 409 tests across 65 files passed, plus Vite startup/HTML transformation smoke. This is the shared worktree and includes the pre-existing user interaction test. |
| `npm run build` | TypeScript and production build passed; existing Vite large-bundle advisory remains. |
| `node scripts/check-style.mjs` | Passed. |
| `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers SCRAPE_BROWSER_PORT=1441 npx playwright test e2e/workflows.pw.ts --workers=1` | Both browser workflow interactions passed; the limit defaults to 20 and a saved value of 7 reopens correctly. This uses explicitly labeled browser fixtures. |
| `npm run build:sidecar` | Rebuilt and copied the native aarch64 macOS packaged worker. Binary is a build artifact, not committed source. |
| `npm run test:providers` | Recorded HTTP replay at 2026-10-15T14:48:29Z verified Lever 8/8 and Ashby 31/31, zero errors. Verification intentionally requests a 10,000-posting cap to cover the captured responses. No live-provider check. |

Actual worker-process tests use 120-posting local HTTP responses for every adapter. They prove default/configured early stopping; Greenhouse listing plus exactly three detail requests for a limit of three; no JSON-LD followed-page request after inline matches fill the budget; invalid/filtered/blocked/duplicate records not consuming quota; min(limit,10) previews; and independent Run all budgets. CLI invalid limits fail before extraction.

The packaged-sidecar → actual NDJSON → production mapper → temporary SQLite test ingests 20 records from a 120-posting page, previews 10 without writing, refreshes the same 20 with zero new inserts, honors a lower override of seven, and preserves notes, favorites, tracked status and history after database reopening. The persistence budget separately rejects surplus and unknown workflow IDs.

One initial Rust run exposed an accepted-socket `WouldBlock` race in the existing Greenhouse fixture server on macOS. Explicitly returning accepted sockets to blocking mode fixed the harness; the final complete suite passed. Initial sandboxed socket binds were denied; loopback fixture suites ran with the required local-listener permission.

### Limits and preservation

No native pointer/visual QA or live database testing was performed for this section. The Section 8 no-update screenshot and browser-extension zoom failures remain unresolved and were not relabeled as passing. Its manual-native acceptance gates also remain open. No clean dependency installation or remote CI run is claimed.

The four prior edits in `IconButton.tsx`, `RemoveJobButton.tsx`, discovered `JobCard.tsx`, and `job_card_interaction.test.tsx` remain unchanged and unstaged; their diff matches `/tmp/scrape-pre-section7.patch`. No merge into develop was performed.

Commits: `412d51d` approved priority plan; `7afd5ff` runtime/contracts/worker/repository tests; `0efb8e6` editor/preview/import/review controls and interactions. Next implementation section is geography/workplace targeting; confirmed data management follows it. Neither section has been implemented by this change, and the user's existing jobs have not been deleted.
