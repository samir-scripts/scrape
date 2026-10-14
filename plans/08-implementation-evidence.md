# Desktop usability repair: implementation evidence

Status: Section 1 research and test foundation complete. Native gesture reproduction remains pending and must precede choosing the corresponding implementation. No product code has changed for this plan.

This document records results for [the implementation plan](08-desktop-usability-and-extraction.md). Each new result must include the environment, command or interaction, expected behavior, observed outcome, and evidence. Do not upgrade a hypothesis to a confirmed cause without a reproduction.

## Starting state

- Branch: `implementation/trustworthy-scraping`.
- HEAD when Section 1 began: `6162748` (`docs: expand desktop repair plan into Luna execution workbook`).
- Worktree: clean before this section began.
- OS shown by the supplied screenshots: macOS. The native window's precise WebKit version, app binary build, database schema version, saved theme/font preferences, and user-visible scrollbar OS setting were not measured during this section.
- Native Scrape window: not present in the computer-use app inventory at the start of this section. The existing `127.0.0.1:1420` Vite server was already listening; it was not stopped or reused for native regression testing. No live-profile database was opened.
- Screenshot references supplied by the user are preserved in `plans/assets/08-desktop-review/01`–`09`. The screenshots show Obsidian for the desktop screens, 1280×800 for the main images, a 1552×78 toolbar strip, and an external Cloudflare job page. They do not identify the app commit, WebKit build, or database migration version.

## Issue-to-test reproduction matrix

| ID | Report / expected observable result | Existing coverage and gaps | Section / test layer | Reproduction in Section 1 |
|---|---|---|---|---|
| D1 | Encoded HTML descriptions display readable headings, paragraphs, and lists; raw tags are absent. | `enrich::html::tests::test_to_plain_text` covers ordinary HTML only. `job_inspector_interaction.test.tsx` covers notes/actions, not structured descriptions. | 2A: Rust normalization; worker/repository integration; inspector interaction; native visual. | Source proves entity decoding may create markup after the stripping pass. No new failing test yet. |
| D2 | Remote + US finds the Cloudflare posting; workplace and eligible geography remain separate. | Location parser and matching tests cover ordinary locations and stored-country predicates. No fixture contains Remote plus a labeled Available Locations section; no end-to-end test checks extraction→persistence→filter. | 2B: Rust enrichment and matching; worker→repository; filter UI; native journey. | Official posting corroborates Remote and Available Locations: US; exact raw API response is absent. Evidence envelope at `fixtures/providers/regressions/cloudflare-8024849.json`. |
| D3 | User can resize Discovered list/details panes; sizing persists and keyboard works. | Existing inspector tests cover details interactions; `layout/ThreeColumn.tsx` and `DiscoveredTwoColumn.tsx` have unrelated splitters. No current test proves this inspector is adjustable. | 4: split primitive interaction, keyboard, persistence, native pointer. | Static code confirms 480px inspector width; no native pointer reproduction this section. |
| D4 | Kanban columns are visually distinct and a card move persists to another stage. | `kanban_board.test.ts` (10) passes, but its scope is component behavior and does not prove native drag delivery/persistence. Existing native smoke tests status persistence generally, not pointer drag. | 5: component and IPC tests plus native pointer movement/restart. | Source contains HTML5 drag handlers, so root cause is unknown. No native drag reproduction this section. |
| D5 | Scraper options are compact; YAML import is discoverable as an accessible upload SVG; Run all is explicit. | Workflow interaction test passes and tests URL→preview→save. `telemetry_console_and_sources.test.ts` contains a source-string expectation for Import YAML despite actual button text; no file chooser interaction. | 6: wizard interactions, actual file selection/import, run-all backend parity. | Static code confirms both top disclosures and text import action. |
| D6 | Schedule control reports the actual paused/running/next/no-auto-run/error state. | Scheduler unit tests cover cron math/dispatcher. No UI test covers malformed or absent `next_run_at`; frontend falls back to SCHEDULED if absent. | 7: fake-clock status + UI state tests + native click-through. | Existing backend tests pass; static UI path shows ambiguous fallback. Scheduler failure itself is not established. |
| D7 | One toolbar; SVG pagination at bottom; page number can be entered directly and jumps accurately. | Filter tests cover predicates. No result-toolbar/page-jump interaction test. `App.tsx` currently renders Previous/Next and sort above results. | 4: repository page/query interaction; >2k scale; native keyboard and screen review. | Static code confirms current top-row controls and fixed page size; no jump UI currently exists. |
| D8 | Scrollbars and native controls match Paper/Obsidian surfaces in WKWebView. | Browser contrast/visual tests exist; global standard/WebKit scrollbar CSS exists. They do not establish actual macOS overlay scrollbar appearance. | 3: both themes, OS scrollbar settings, actual WKWebView screenshot. | No native WebView reproduction this section. |
| D9 | All action icons use one legible pixel SVG system; action names and focus remain clear. | Existing SVG controls and font tests exist; coverage is partial and some Unicode/text controls remain. | 3: component interaction/accessibility, visual snapshots, keyboard. | User references preserved; no icon changes this section. |

## Baseline checks run in this section

All commands ran from the project checkout and used existing repository fixtures. No live database or public scrape run was used.

| Command | Result |
|---|---|
| `npx vitest run src/test/job_inspector_interaction.test.tsx src/test/kanban_board.test.ts src/test/filters.test.ts src/test/workflow_editor_interaction.test.tsx src/test/workflow_statuses_and_log_filtering.test.ts src/test/persisted_native_filters.test.ts` | Passed: 44 tests in 6 files. These are baseline checks, not new native regressions. |
| `cargo test -p scrape-core enrich::html::tests` | Passed: 1 ordinary HTML test. Encoded-markup behavior is not asserted. |
| `cargo test -p scrape --lib services::scheduler::tests` | Passed: 7 scheduler calculation/dispatcher tests. These do not prove the Scraper status UI is truthful or the macOS desktop schedule works. |

The frontend command emitted Node's existing experimental localStorage warning; all selected tests passed. No test file was changed or weakened in this section. Add each failing regression immediately before its matching implementation in Sections 2A–7, then keep each committed phase green.

## Provider evidence

Official Greenhouse page: https://job-boards.greenhouse.io/cloudflare/jobs/8024849 . Checked through the public page on 2026-10-14. It identifies external ID 8024849 in the URL, title “Senior Principal, Sales Compensation Design & Strategy,” workplace “Remote,” an “Available Locations” section containing “US,” and an application deadline of 2026-10-13. The page text therefore supports the user-visible expected geography for this regression.

The official rendered page is not the raw Greenhouse API response. The reviewed repo has recorded provider payloads for SEP and Linear but no Cloudflare capture. There is no trustworthy checksum or adapter input for this posting in the repository. Capture the public endpoint response, timestamp it, hash the bytes, and review privacy/provenance before creating the deterministic worker fixture in Section 2B. Until then, the JSON evidence envelope explicitly must not be treated as raw provider input.

## Native reproduction status

The user screenshots are preserved and establish what the reported UI looked like. This section did not reproduce pointer drag, resizing, system scrollbars, or theme controls in a live native window. The computer-use inventory had no Scrape window; the existing Vite server on port 1420 was left untouched to avoid attaching tests to an unrelated development session. Existing scripted native smoke uses temporary directories for database/IPC tests, but it does not exercise these UI gestures. Native interaction reproduction remains pending and is an explicit gate before the corresponding fix is selected.

## Section 1 gate and next boundary

Every report now has an affected path, an existing-coverage assessment, and an assigned regression layer. Reproductions supported by source inspection or official page evidence are identified as such; native-only gestures have an explicit limitation and were not guessed at. The exact Cloudflare title/ID/Remote/US evidence is recorded, with the missing raw API response clearly called out. Baseline tests are green. Section 1 is complete at this documented gate; native interaction reproduction remains a required gate before selecting the drag/splitter fix.

The next authorized unit is Section 2A (description normalization/display and safe historical repair); do not bundle Sections 2B–7 into that change. Section 2A must add and observe its focused failing regression immediately before the repair, as the implementation workbook specifies.
