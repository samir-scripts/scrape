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

Section 2 handoff (historical): commits `412d51d` approved priority plan; `7afd5ff` runtime/contracts/worker/repository tests; `0efb8e6` editor/preview/import/review controls and interactions. Geography and Settings data management had not yet been implemented, and the user's existing jobs had not been deleted at that point. Subsequent cleanup and Section 3 delivery are recorded below.

## Authorized desktop cleanup before Section 3 (2026-10-15)

The user explicitly requested all saved jobs/favorites be cleared, without a recovery backup, before continuing development. This superseded the proposed backup for this one-off operation. It was a requested desktop data operation, not a test against the live database.

The identified desktop process was closed; no scrape worker was active. With foreign keys enabled, a single SQLite transaction deleted every jobs row and cascaded to locations/history. Favorites are columns on jobs; there is no separate favorites table. No backup was created.

| Data | Before | After |
|---|---:|---:|
| Jobs | 437 | 0 |
| Job locations | 445 | 0 |
| Job status events | 467 | 0 |
| Favorited jobs | 0 | 0 |

Workflow count (1), settings (24), runs (15), logs (32), notifications (1), suppression tables, and all 20 migration records remained unchanged. Migration rows/checksums were compared before commit; foreign-key validation passed. No schema, migration, or unrelated application table was truncated.

## Section 3 — Geography and workplace targeting

Implemented and verified. Section 4 Settings data-management controls remain pending.

- Country/state search and workplace selection are available in Simple Matching. Country changes clear incompatible states. Search drafts invalidate previews; invalid values prevent continuing. Multiple workplaces use OR; geography and workplace use AND. Unknown location is opt-in. Additional free-text location and raw-code controls remain in Advanced.
- Rust owns 249-country metadata via pinned isocountry 0.3.2, a consolidated 92-subdivision catalog, and generated frontend types/catalog values. Supported subdivision coverage is documented in `crates/scrape-core/data/README.md`; this is not a universal geocoder. Legacy/multiple-country rules survive editing and YAML round trips.
- Source aliases use bounded words, punctuation/case normalization, and common Latin accent folding. Explicit codes precede ambiguous region names (including the Washington/DC regression). Bare Remote does not imply geography; ambiguous CA/Georgia remain unknown without context. Structured address fields are normalized before text inference.
- JSON-LD keeps multiple locations/nested country values and applies explicit remote applicant eligibility before office geography. Labeled eligibility sections remain scoped; ordinary company prose is not evidence.
- Worker/SQL parity now includes unknown-only Remote rows, partial legacy rows with qualified subdivisions, known contradictory geography, same-entry country/state matching, and qualified state compatibility. Invalid/impossible geography filters fail explicitly.
- UI interactions cover search, state addition/removal, country changes, workplace OR, unknown inclusion, invalid drafts, imported multi-country preservation, and YAML/source round trips. Browser workflow creation/editing exercises saved geography. Native workflow coverage uses actual WKWebView controls, local HTTP extraction and the sidecar, with geographic rules verified on the persisted workflow and no jobs written by preview/save.

### Section 3 verification

| Boundary/check | Result |
|---|---|
| Rust workspace | 298 tests passed, including five packaged-worker → NDJSON → mapper → temporary SQLite integration tests. |
| Worker/repository geography parity | Ten filter combinations over twelve local Ashby postings, including partial historical country rows, Remote+US, qualified subdivisions, workplace OR, same-entry matching, unknown inclusion, and known contradictions. |
| JSON-LD normalization | Tests preserve all address-array entries, nested Country fields, ambiguous country/state codes using named address fields (including CA/NL), and remote applicant eligibility over a UK office address. |
| Native wizard | `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_WORKFLOW=1 npm run test:native-startup` passed. Actual isolated WKWebView selected US/Texas/Remote, extracted one eligible local posting through the sidecar, and persisted the reviewed rules without writing jobs or creating a run. Final artifacts: `scrape-native-smoke-QaUU2j` in the OS temp directory. |
| Frontend | `npm test`: 412 tests across 66 files passed plus Vite startup smoke. Shared worktree includes the untouched pre-existing user test. |
| Browser workflow | Two Playwright workflow/import checks passed; geography controls were selected, saved and verified after reopening. No screenshot baselines were regenerated. |
| Build/style | TypeScript/production build and `node scripts/check-style.mjs` passed. Existing Vite bundle-size advisory remains. |
| Rust checks | Clippy with `-D warnings` and formatting check passed. After Clippy's equivalent-branch consolidation, all nine workplace classifier tests passed again. |
| Binding generation | Normal Rust binding-export tests generated CountryOption, RegionOption and the static geography catalog; frontend hand-maintained metadata/types were replaced with generated consumers. |
| Recorded provider replay | 2026-10-15T16:28:04Z: Lever 8/8 and Ashby 31/31, zero errors; recorded fixtures, not live-provider verification. |
| Desktop cleanup follow-up | Read-only verification still reports jobs/locations/status-history/favorites at zero; workflow 1, settings 24, migrations 20. No regression test used this live database. |

The broader adapter suite caught Washington/DC being misclassified as Washington state; explicit abbreviation/code priority now fixes it, with a regression entry. An initial native repeat timed out at unbounded animation-frame waits. Test-only frame waits were replaced with event-loop yields; existing 20/60-second deadlines and all assertions remain intact. The final native run passed. Inactive WKWebView frame scheduling is a suspected cause, not a proven production defect.

Section 8's prior no-update screenshot/zoom failures and manual native visual/pointer/file-chooser gates remain open. This section adds actual native programmatic workflow evidence; it does not establish manual pointer/visual QA or Windows/Linux behavior. No remote CI run or clean dependency install is claimed. All four pre-existing user edits remain unchanged and unstaged, verified against `/tmp/scrape-pre-section7.patch`.

Section 3 commits: `06c86c3` shared catalog/extraction/matching parity; `f1640eb` searchable workflow controls plus browser/native interactions. A post-commit binding export and `git diff --exit-code HEAD -- src/bindings` passed with no drift. The next section is confirmed Settings data management; the one-off jobs cleanup above does not substitute for that feature.
