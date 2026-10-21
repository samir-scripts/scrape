# Desktop usability repair: implementation evidence

Status: Sections 1–7 are implemented. Automated workspace, frontend, recorded-provider, browser, isolated native startup/migration, and WKWebView integration checks pass. Section 8 remains in progress for native pointer/visual/file-chooser acceptance; the exact Cloudflare posting has now been identified and its API fields are captured and covered by adapter and worker/repository regressions.

This document records results for [the implementation plan](08-desktop-usability-and-extraction.md). Each new result must include the environment, command or interaction, expected behavior, observed outcome, and evidence. Do not upgrade a hypothesis to a confirmed cause without a reproduction.

## Starting state

- Branch: `implementation/trustworthy-scraping`.
- HEAD when Section 1 began: `6162748` (`docs: expand desktop repair plan into Luna execution workbook`).
- Worktree: clean before this section began.
- OS shown by the supplied screenshots: macOS. The native window's precise WebKit version, app binary build, database schema version, saved theme/font preferences, and user-visible scrollbar OS setting were not measured during this section.
- Native Scrape window: not present in the computer-use app inventory at the start of this section. The existing `127.0.0.1:1420` Vite server was already listening; it was not stopped or reused for native regression testing. No live-profile database was opened.
- Screenshot references supplied by the user are preserved in `plans/assets/08-desktop-review/01`–`09`. The screenshots show Obsidian for the desktop screens, 1280×800 for the main images, a 1552×78 toolbar strip, and an external Cloudflare job page. They do not identify the app commit, WebKit build, or database migration version.

## Original Section 1 issue-to-test triage (historical)

The following matrix records the initial evidence and gaps at the start of Section 1. It is intentionally retained as history; the current acceptance reconciliation at the end of this file supersedes its “current” and “no test” statements.

| ID | Report / expected observable result | Existing coverage and gaps | Section / test layer | Reproduction in Section 1 |
|---|---|---|---|---|
| D1 | Encoded HTML descriptions display readable headings, paragraphs, and lists; raw tags are absent. | Baseline covered ordinary HTML only. Section 2A now adds core, actual-worker/repository, and inspector-interaction regressions. | 2A: Rust normalization; worker/repository integration; inspector interaction; native visual. | Encoded markup regression added and observed against the prior implementation; it failed because markup tags leaked into output. The fixed path now passes focused Rust, integration, and UI checks. Native visual check is pending. |
| D2 | Remote + US finds the Cloudflare posting; workplace and eligible geography remain separate. | Section 2B added labeled-section, prose-negative, office-precedence, incomplete-refresh, filter-control, label-display, and worker→SQLite→restart tests. Section 8 follow-up replaced the positive stand-in with captured Cloudflare API fields. | 2B and Section 8: Rust enrichment and matching; worker→repository; filter UI; native journey. | Captured Greenhouse ID `8024849` passes adapter extraction and actual worker→SQLite→restart→Remote+US filtering. The authored negative control still proves general company prose does not establish eligibility. Native UI click-through remains separate. |
| D3 | User can resize Discovered list/details panes; sizing persists and keyboard works. | Existing inspector tests cover details interactions; `layout/ThreeColumn.tsx` and `DiscoveredTwoColumn.tsx` have unrelated splitters. No current test proves this inspector is adjustable. | 4: split primitive interaction, keyboard, persistence, native pointer. | Static code confirms 480px inspector width; no native pointer reproduction this section. |
| D4 | Kanban columns are visually distinct and a card move persists to another stage. | `kanban_board.test.ts` (10) passes, but its scope is component behavior and does not prove native drag delivery/persistence. Existing native smoke tests status persistence generally, not pointer drag. | 5: component and IPC tests plus native pointer movement/restart. | Source contains HTML5 drag handlers, so root cause is unknown. No native drag reproduction this section. |
| D5 | Scraper options are compact; YAML import is discoverable as an accessible upload SVG; Run all is explicit. | Workflow interaction test passes and tests URL→preview→save. `telemetry_console_and_sources.test.ts` contains a source-string expectation for Import YAML despite actual button text; no file chooser interaction. | 6: wizard interactions, actual file selection/import, run-all backend parity. | Static code confirms both top disclosures and text import action. |
| D6 | Schedule control reports the actual paused/running/next/no-auto-run/error state. | Scheduler unit tests cover cron math/dispatcher. No UI test covers malformed or absent `next_run_at`; frontend falls back to SCHEDULED if absent. | 7: fake-clock status + UI state tests + native click-through. | Existing backend tests pass; static UI path shows ambiguous fallback. Scheduler failure itself is not established. |
| D7 | One toolbar; SVG pagination at bottom; page number can be entered directly and jumps accurately. | Filter tests cover predicates. No result-toolbar/page-jump interaction test. `App.tsx` currently renders Previous/Next and sort above results. | 4: repository page/query interaction; >2k scale; native keyboard and screen review. | Static code confirms current top-row controls and fixed page size; no jump UI currently exists. |
| D8 | Scrollbars and native controls match Paper/Obsidian surfaces in WKWebView. | Section 3 adds explicit app color-scheme, shared scrollbar tokens, and native computed-style checks at wide/Always and narrow/Automatic settings. Native screenshot review remains pending. | 3: both themes, per-process scrollbar preferences, actual WKWebView styles, browser visual regression. | Browser regression first confirmed color-scheme was normal in both themes. Native fallback now reports Paper rgb(103,99,87) and Obsidian rgb(185,181,175) thumbs. |
| D9 | All action icons use one legible pixel SVG system; action names and focus remain clear. | Section 3 introduces the shared family and migrates job, filter, dialog, notification and navigation controls. Unit/browser tests exercise keyboard actions, disabled semantics, target sizes, names, fonts and contrast. Later workflow/pagination/drag sections consume the same family. | 3: component interaction/accessibility, reviewed visual snapshots, keyboard, native styles. | Existing duplicated paths and Unicode action glyphs were replaced; no icon font or stock artwork was introduced. |

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

At the time of the original Section 2B work, the official rendered page was the only Cloudflare evidence available; the authored regression was correctly labeled and not treated as raw provider input. The Section 8 follow-up below captured the public API response and supersedes that limitation for posting `8024849`.

## Native reproduction status

The user screenshots are preserved and establish what the reported UI looked like. This section did not reproduce pointer drag, resizing, system scrollbars, or theme controls in a live native window. The computer-use inventory had no Scrape window; the existing Vite server on port 1420 was left untouched to avoid attaching tests to an unrelated development session. Existing scripted native smoke uses temporary directories for database/IPC tests, but it does not exercise these UI gestures. Native interaction reproduction remains pending and is an explicit gate before the corresponding fix is selected.

## Section 1 gate and next boundary

Every report now has an affected path, an existing-coverage assessment, and an assigned regression layer. Reproductions supported by source inspection or official page evidence are identified as such; native-only gestures have an explicit limitation and were not guessed at. The exact Cloudflare title/ID/Remote/US evidence is recorded, with the missing raw API response clearly called out. Baseline tests are green. Section 1 is complete at this documented gate; native interaction reproduction remains a required gate before selecting the drag/splitter fix.

Section 2A is implemented and its code-level checks pass. The user subsequently directed work to Section 2B. The Section 2A native visual gate remains open: `npm run test:native` timed out before its injected page script ran, so it is not evidence of a pass.

## Section 2A implementation evidence

**Research and design.** Traced the Greenhouse, Ashby, Lever, JSON-LD, and selector adapters through the worker event, ingestion, repository, and shared inspector. The workspace already had the maintained `scraper` HTML parser available, so the normalizer uses that parser instead of adding another HTML parser. Input encoding is explicit (`html`, `plain_text`, `mixed`); only recognized single-layer encoded tags are decoded before parsing. HTML is bounded to 512 KiB input and 20,000 parsed nodes, production adapter output is bounded to 32 KiB, and truncation is Unicode-safe and reported. Text content that merely contains angle brackets remains text. Script/style and other non-display content is excluded; the output contract contains paragraphs, headings, lists, emphasis, and safe absolute HTTP(S) links.

**Regression and implementation.** Added a Rust regression for an entity-encoded paragraph that failed against the old strip-then-decode path, as well as cases for script elements with attributes, literal comparison text, double encoding, plain-text input, structure, unsafe links, and UTF-8 truncation. Adapter events now carry the structured description, original source, and input format. Canonical plain text remains the searchable/snippet representation. The React inspector renderer creates elements from the restricted structure and routes safe links through the existing checked opener; it never inserts provider markup as HTML.

**Persistence and repair.** Added forward migration 0019 with nullable source, structured JSON, and source-format fields. Repository reads expose the structured contract while retaining canonical text. Historical re-enrichment reads preserved source when available, otherwise uses the existing description as legacy HTML/unknown input; it processes at most 200 records per invocation, backs up a file-backed database with SQLite `VACUUM INTO` before a nonempty repair batch, and leaves user state columns/history/removals/blocks untouched. It is idempotent and retries remaining old-version rows on a later startup. The populated-record regression checks repair twice and verifies notes, favorite, and applied status survive.

**Verification.** On 2026-10-14, from the implementation checkout:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed after final Rust edits. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| `cargo test --workspace` | Passed; full Rust workspace suite, including binding export tests. |
| `cargo test -p scrape --test worker_repository` | Passed: 2 tests, including local HTTP → actual worker → NDJSON → SQLite description/source/format round trip. The harness drains process pipes concurrently. |
| `npm run gen:types` | Passed; Rust-derived TypeScript bindings generated. |
| `npm test` | Passed: 366 tests across 52 files; included development HTML-transformation startup smoke. |
| `npm run build` | Passed; Vite reported its existing large-chunk advisory. |
| `npm run test:native-startup` | Passed: fresh, legacy, current, and unknown schema startup cases in isolated temporary profiles. |
| `npm run test:native` | **Not verified:** exceeded its deadline before the injected page script ran. It did not exercise description rendering. |

The native limitation is explicit: startup fixtures demonstrate database/core startup handling, not actual macOS WKWebView rendering. No claim is made that the description was visually inspected in the native application. Do not advance this plan's section until that check is completed or the user explicitly directs work to another section.

## Section 2B implementation evidence

**Research and source assessment.** Traced Greenhouse location and office selection through the worker event, shared matcher, ingestion, `job_locations`, and repository SQL. The adapter now keeps Remote workplace policy separate and extracts eligible geography only from labeled description sections. Greenhouse documents `content=true` as including the full description, department, and office data ([Job Board API](https://docs.greenhouse.io/job-board.html)); an office address alone does not establish eligible geography for a Remote role. The original attempt to retrieve this posting failed at that time; Section 8 later captured its public API response and recorded the checksum in the regression fixture.

**Regression fixture and implementation.** Added `fixtures/providers/regressions/remote-us-availability-authored.json`, explicitly marked as an authored case with no source capture, and documented that distinction in `fixtures/providers/README.md`. The positive posting is Remote with US only under “Available Locations”; the negative posting mentions the United States only in company prose. The shared Rust location extractor consumes bounded normalized description blocks only under explicit location labels, retains the label in `ParsedLocation.raw`, and recognizes exact country abbreviations such as `US`. Greenhouse keeps Remote independent from office addresses and prefers the provider's actual office `location` field for hybrid/on-site postings. All adapters call the same labeled-section extractor.

The worker event now includes `location_data_complete`. This transient status distinguishes a complete empty extraction from a source that did not expose eligible geography. Repository upserts replace stored geography only when this status is true; incomplete Remote-only refreshes preserve prior eligible locations, while a complete empty response clears stale entries. No schema migration was needed. A shared TypeScript formatter now displays workplace policy and evidenced geography consistently in discovered cards/table, tracked list/Kanban, and the inspector.

**Verification.** On 2026-10-14:

| Check | Result |
|---|---|
| `cargo test -p scrape-worker remote_job_uses_only_labeled_eligible_locations_from_its_description` | Passed after first observing the new regression fail because US eligibility was absent. |
| `cargo test -p scrape --lib db::jobs::tests::test_upsert_scraped_preserves_user_triage_state_and_updates_enrichment` | Passed; incomplete geography preserves existing evidence and a complete empty refresh clears it. |
| `cargo test -p scrape --test worker_repository` | Passed: 3 tests. The authored Remote + US case runs through local HTTP, the actual rebuilt Greenhouse sidecar, ingestion, SQLite persistence, filter query, and database reopen; company-prose-only US does not match. |
| `cargo test --workspace` | Passed: Rust workspace suites, including 121 core, 19 worker, and existing Tauri tests. |
| `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` | Passed. |
| `npm run gen:types` | Passed; protocol and repository TypeScript contracts were regenerated. |
| `npm test` | Passed: 368 tests across 53 files plus development HTML transformation smoke. |
| `npm run build` | Passed; existing large-bundle advisory remains. |
| `npm run build:sidecar` | Passed for the local aarch64 macOS target. |

Native macOS pointer/visual and OS file-chooser verification remain outstanding. The original authored case was later supplemented by captured Cloudflare API fields exercised through the actual worker and persistence boundary; the complete response is retained locally, while only a minimal projection and checksum are committed.

## Branch merge decision

Before Section 2B implementation, local `develop` was the merge base and `git rev-list --left-right --count develop...HEAD` reported `0 60`: even then, a fast-forward would have brought in 60 branch commits covering broader in-progress work, rather than just this section. Tests are green, but native visual/interaction acceptance is still open and Sections 3–7 remain. Do not merge this branch into `develop` yet; revisit after the remaining acceptance and review gates.

## Startup regression discovered before Section 3 (2026-10-14)

The user's migration-19 recovery screen is logged in [incident 26](errors/26-native-startup-migration-19.md).
Read-only inspection confirmed the exact two-column historical SQL by its SHA-384
checksum and schema. Added immutable historical manifests and forward migration 20;
kept all applied SQL/checksums unchanged. Fixed form-encoded spaces on the recovery
screen. The new version-19 regression first reproduced the reported failure.
Populated upgrades pass for all four recognized version-12/version-19 combinations;
unknown version-19 history and schema drift remain untouched on refusal.

The reviewed repair helper upgraded the authorized existing database to version 20,
created the backup recorded in incident 26, and reported integrity `ok`. All original
column values across 13 tables and prior migration metadata were fingerprinted
before/after and remained unchanged. These were repair checks, not live-profile tests.

`npm run test:native-startup` passed all seven actual Tauri dev fixtures: fresh,
legacy/current version 12, unknown version 12, early/current version 19, and unknown
version 19. Fixtures use temporary directories and isolated local Vite ports. The
startup harness now explicitly shows its test window so WKWebView loads and emits
an IPC completion result. This proves actual native startup/IPC readiness or controlled
failure; it does not by itself prove every visual/interaction acceptance criterion.


## Section 3 — shared icons, controls, fonts and native scrolling

**Research.** Inventoried action glyphs and duplicated SVG paths, `IconButton`,
`Button`, `Input`, `Select`, `Modal`, font selectors and scrollbar rules. Existing
scrollbar colors were present, but the new browser regression failed because both
app themes reported `color-scheme: normal`. [MDN color-scheme](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/color-scheme)
confirms that this property controls native form and scrollbar appearance;
[WebKit scrollbar documentation](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/::-webkit-scrollbar)
explains the fallback and interaction with supported standard scrollbar properties.
The tested macOS 15.6 WKWebView uses the WebKit fallback for thumb colors.

**Implementation.** Added one original SVG family on a 24-unit grid with shared
14/16/20px sizes and `currentColor`. Shared icon buttons own 32px minimum targets,
forwarded refs, semantic variants, disabled behavior and accessible names. Reused
them for job/card/table/inspector controls, global and panel search, filter chips,
workflow matching chips, dialogs, toasts, notifications and the terminal launcher.
Discovered table sort headers are named keyboard-operable buttons with `aria-sort`.
Selected fonts remain inherited by controls; code keeps its separate fixed-width
font. Removed an empty subnavigation strip on non-job screens and a duplicate
filter-search reset control. Toolbar/pagination consolidation, split panes, Kanban
movement, workflow breadcrumbs and schedule-status work remain in Sections 4–7.

Shared control heights and scrollbar roles now live in CSS tokens. Paper uses
light native controls and Obsidian uses dark controls. Standard and WebKit scrollbar
rules consume the same semantic values; vertical scroll surfaces use stable gutters
where supported, and tables retain horizontal scrolling. No custom JavaScript
scrollbar was introduced. Updated primitive and test-harness documentation.

**Test adaptation.** Replaced brittle polygon/path/class/glyph expectations with
shared-control and favorite keyboard interactions, a real terminal-button click,
rendered dimensions, decorative SVG semantics, accessible labels and focus checks.
Extended bundled-font coverage to icon controls and shared indicator-contrast checks.
Reviewed 35 changed macOS Chromium screenshots (settings, workflow, jobs, removal,
inspector, applied list and board across both themes and relevant widths) before
accepting baselines. Screenshot expectations remain failures; soft assertions simply
allow all visual checkpoints to be collected in a single failing review run.

**Native verification.** Isolated actual Tauri dev runs passed at 1280×600 with
per-process Always scrollbars and 480×600 with Automatic scrollbars. All five bundled
font families loaded and applied to navigation/fields; the workflow dialog retained
the selected IBM Plex font. Both themes produced the expected native select/color
scheme, named SVG controls of at least 32px, and WebKit thumb colors:
Paper `rgb(103, 99, 87)`, Obsidian `rgb(185, 181, 175)`. No global macOS preference was
changed. Successful artifacts include `scrape-native-smoke-YqQAv0` and
`scrape-native-smoke-GsYEuV` in the system temporary directory.

The older full-native smoke assumed a fixed Vite URL despite per-test URLs being
embedded in the executable, and its default window was hidden. Both native scripts
now share a launcher owning a visible temporary-profile window, isolated port,
actual Tauri dev build, process group and completion log. `npm run test:native`
passed with explicit IPC, packaged sidecar, persistence, block/restore, cancellation,
malformed output/nonzero exit/missing completion/timeout, and external-destination
completion (`scrape-native-smoke-VDYF2j`). Refactored startup checks also passed for
current19 appearance and unknown19 refusal.

Native screenshot capture/manual gesture review remains pending: the UI automation
surface could not attach to the development binary, and an optional delayed visual
capture attempt exceeded the native deadline. Its temporary delay code was removed.
The successful checks above are programmatic WKWebView checks; the visually reviewed
screenshots are Chromium. The prior native description/Remote+US full journey gates
remain open.

**Final checks for this section (2026-10-14).**

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed. |
| `cargo test --workspace` | Passed with loopback enabled for deterministic HTTP fixtures. The initial restricted run could not bind those servers. |
| `npm test` | Passed: 370 tests in 55 files, plus Vite HTML-transformation startup smoke. |
| `npm run build` | Passed; existing large-bundle advisory remains. |
| `node scripts/check-style.mjs` | Passed. |
| `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers SCRAPE_BROWSER_PORT=1437 npm run test:browser -- --workers=1` | Passed: all 18 browser tests, including 200% zoom, both-theme contrast/focus, font loading, workflows, removal and job-stage journeys. |
| `npm run test:native-startup` | Seven startup fixtures passed before the shared-launcher refactor; current19 appearance and unknown19 refusal passed again through the shared launcher. |
| `npm run test:native` | Passed with explicit IPC/worker/opener completion. |
| Native appearance commands in `scripts/README.md` | Wide/Always and narrow/Automatic runs passed for both themes and all bundled fonts. |

A source edit during one browser run triggered Vite reload and invalidated a
screenshot; that blank image was rejected. The final full browser run used stable
source files and all 18 tests passed. No unreviewed screenshot was accepted.

Implementation commits: `ea9c3dc` (migration-19 incident/repair), `3b229d8`
(shared SVG family and interaction regressions), `e814a8b` (isolated native launcher
and appearance checks), and `1ebede4` (screen migration, shared styles, adapted tests,
reviewed baselines and CI wiring). Generated bindings have no drift. The branch
remains unmerged because the later sections and native visual/manual gates are open.


## Section 4 — Toolbar, pagination and adjustable details (2026-10-14)

Scope is section 4 only. Sections 5–7 remain unimplemented and this branch remains
unmerged. No live profile/database was opened or changed for these checks.

Research traced App query keys, page offsets, SubNav, inspector drafts and cleanup.
The historical layouts exist but are not a reusable adjustable split. The new
shared primitive follows the [WAI window splitter pattern](https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/)
and [pointer capture](https://developer.mozilla.org/en-US/docs/Web/API/Element/setPointerCapture).
Page state could revive an old offset after switching filters back; conditional
inspector shells remounted pending drafts across the narrow breakpoint.

One toolbar now contains filters, favorites, search and sorting. The controlled
bottom paginator offers SVG first/previous/next/last controls and direct page entry.
App alone owns fetching, stale-response isolation, total/clamp and selection reset.
Filter/sort/page changes close the inspector and flush notes to the original job;
failed saves expose retry. A transient page draft accepts blank typing, validates
bounds, and restores on Escape. The existing 2,105-job repository regression checks
search, notes and offsets beyond the former 2,000-row assumption.

Discovered and both Tracked views share one measured split with minimum widths,
pointer capture, cancel, keyboard bounds and a reset control. One versioned ratio
persists only completed changes. Corrupt preferences use the default. Below 812px,
the same inspector content becomes a native modal dialog and preserves its editor,
draft and scroll; no hidden duplicate notes editor is mounted.

Regressions were observed failing before implementation (missing direct-page
control and separator). Native testing then found an additional WebKit issue:
showModal's deferred focus scroll moved Notes into view even after synchronous
scroll restoration. Dialog open state now has one imperative owner, with a bounded
next-task scroll restoration after the native focus task. The same-node, exact
scroll and persisted-note native assertions passed after this correction. Earlier
native failures/deadlines were rejected; they are not acceptance evidence.

Validation:

- `npm test`: 387 tests across 58 files and the Vite HTML startup smoke passed.
- `npm run build`, style guard, Rust formatting, Clippy with warnings denied, and
  Rust workspace tests passed. The existing bundle-size advisory remains.
- Three new real Chromium interaction tests cover actual pointer capture, completed
  vs cancelled drags, keyboard limits/reset, reload, both themes, long content,
  narrow/wide drafts, direct page jumps and pending-note persistence. Unit tests
  additionally cover reversed responses, shrinking totals and save retry.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_LAYOUT=1 npm run test:native-startup`
  passed using a temporary 105-discovery fixture, actual WKWebView controls/window
  resizing and IPC note retrieval. Result: ratio 58, page reset 1, notes persisted.
  Artifacts: `scrape-native-smoke-eZ6kXf` under the OS temporary directory.
- Browser screenshots reviewed in both themes at 480/1024/1280/1440 widths include
  jobs, inspector, applied list, board, removal and workflow. Existing keyboard,
  contrast, font, reduced-motion and 200% zoom assertions remain required.

The native script uses keyboard separator input and programmatic native window
resizing. Actual native pointer dragging and native visual screenshot review remain
pending; Chromium pointer/screenshot checks do not close those gates. CI includes
the new browser layout checks and isolated native layout command.

Implementation: `1e64e25` controlled paginator, `d41f0fb` split primitive,
`3ee6103` toolbar/query integration, `2cda158` responsive inspector and native tests.

Final Section 4 verification: the complete browser suite passed **21/21** on stable
source against 31 reviewed updated baselines (`9d89d82`). A blank intermediate
board capture was rejected and independently recaptured/reviewed before acceptance;
no screenshot tolerances were relaxed. The full `npm run test:native` smoke also
passed, including worker, persistence, cancellation/failure handling and the
external destination (`scrape-native-smoke-cJds7r`). Rust binding-export tests ran
with the workspace suite and left `src/bindings` unchanged. The native pointer and
visual/manual gates above remain explicitly open. Clean-checkout CI was wired but
was not executed remotely during this section.


## Section 5 — Kanban movement (2026-10-14)

Implementation is scoped to section 5. Sections 6–7 are untouched; the branch is
unmerged. Four pre-existing card/icon/test modifications were preserved unchanged
and excluded from commits. Their diff was checkpointed at
`/tmp/scrape-pre-section5.patch`. Visual verification used a separate checkout to
exclude those unrelated edits.

Research traced whole-card HTML5 dragging, data-transfer drop handlers, the App
mutation and the repository's atomic status/history transaction. An isolated
historical fixture captured trusted `dragstart` and `dragend` events with no
`dragenter`/`dragover`/`drop`; the posting remained applied. Diagnostic artifacts:
`scrape-native-smoke-1JYBAr` in the OS temporary directory. This trace records no completed
transition; it does not establish the exact gesture or underlying OS cause. Temporary instrumentation
was not added to production.

The [Tauri configuration reference](https://v2.tauri.app/reference/config/)
documents HTML5 drag interception on Windows. No Tauri configuration or file import
behavior was changed. The board instead uses one handle-only
[pointer-capture implementation](https://developer.mozilla.org/en-US/docs/Web/API/Element/setPointerCapture),
with coordinate hit testing, threshold, edge scrolling, Escape/capture-loss/blur
cancellation and keyboard pick-up/arrow/confirm. The old HTML5 system was removed.
Native delivery of the replacement still needs actual pointer acceptance.

Visible column borders, shared SVG handles and Move to selects make the four
stages explicit. Narrow layouts retain all drop zones in a horizontal scroller;
the stage picker scrolls to a stage. Cards preserve selectable text and independent
controls. Page-local stage counts remain labeled. Drag, select, inspector,
shortcuts and Undo all reach one per-job transition queue. Cards stay in their
confirmed stage until saving succeeds. Failure keeps that stage and offers retry;
no stale optimistic snapshot can overwrite a newer move. Same-stage choices are
no-ops. Queue snapshots expire when drained so refreshed repository state wins.

Tests were first observed failing for the missing Move to control and handle.
New interaction tests cover keyboard cancellation, independent controls, failed
IPC placement and visible retry. Queue tests cover rapid requests, no-op history,
failure followed by success/retry and refreshed state after a queue drains. A
populated temporary SQLite test injects a real history-write failure and proves
atomic rollback, no-op history, notes/favorite protection and reopen persistence.

Native checks use `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_KANBAN=1 npm run
test:native-startup`. Actual WKWebView select and keyboard controls passed, including
cancellation, two exact history events, notes/favorite preservation and reload to
offer. Final artifacts: `scrape-native-smoke-uRh9wV`. These are not synthesized
pointer tests and do not establish physical dragging.

Native pointer/visual acceptance is still open. The UI automation surface rejected
the standalone debug executable as an invalid app; macOS's
`CGPreflightPostEventAccess()` returned false. No permission setting was changed.
A successful alternative control does not close the pointer acceptance gate.

Rust verification exposed a separate deterministic-fixture defect: nonblocking
listeners accepted streams that could remain nonblocking on macOS, causing immediate
`WouldBlock` HTTP reads. Sequential tests passed, while standard parallel runs
failed at different reads. Fixture streams now explicitly select blocking mode
before their existing bounded read timeout; no assertion or timeout was relaxed.
See [Rust TCP stream modes](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.set_nonblocking).

Implementation commits: `a009274` stage queue/atomic history test, `1440a5e` refreshed
queue state, `4265623` board controls and browser/native regressions.

Final checks for this section:

| Check | Observed result |
|---|---|
| Shared-workspace frontend suite | 394 tests in 60 files and Vite startup smoke passed, including the pre-existing added card test. |
| TypeScript and production build | Passed; existing bundle-size advisory remains. |
| Rust fmt / Clippy with warnings denied | Passed. |
| Standard parallel Rust workspace tests | Passed after explicitly setting accepted fixture streams to blocking mode (`39fe3fe`). Prior failures and the sequential diagnostic run are described above. |
| Complete browser suite, isolated checkout | 23 passed, including both-theme real pointer input, Escape cancellation, Move to, edge scrolling, reload, contrast/focus, reduced motion, fonts and 200% zoom. Six reviewed board baselines accepted (`dad9e09`); other screen baselines were unchanged. |
| Native Kanban select/keyboard/restart | Passed, exact two persisted history events and saved notes/favorite state. |
| Full native IPC/worker/persistence/cancellation/opener smoke | Passed; artifacts `scrape-native-smoke-gTcBaN`. |
| Native physical pointer and screenshots | Still pending; no claim of native drag acceptance. |

The initial isolated font test returned NetworkError with dependencies symlinked
to the shared workspace. The final isolated run used copied local dependency assets and
all font assertions passed. No font assertion was removed or weakened. Generated
binding-export tests ran with the Rust workspace suite and left bindings unchanged.
No remote CI run or merge was performed. The four pre-existing diffs were compared
to the initial checkpoint and remained byte-for-byte unchanged.

The isolated checkout also passed 393 frontend tests plus startup smoke (the shared
workspace count includes one pre-existing added test), TypeScript, production build
and style guard. Verification checkout: `/tmp/scrape-step5-verify`.


## Section 6 — Workflow wizard, YAML import and Scraper layout (2026-10-15)

This section implements only step 6. Step 7 and the outstanding native
pointer/visual acceptance gates remain open. The four pre-existing card/icon/test
diffs were checkpointed in `/tmp/scrape-pre-section6.patch`, compared byte-for-byte,
and left unchanged. Visual verification used `/tmp/scrape-step6-verify` with local
copied dependencies so those unrelated changes were excluded from baselines.

Research traced AddSourceModal, WorkflowSections, SourcesSettings, ScheduleSettings,
YAML conversion, generated v3 contracts, canonical workflow validation and the
preview/run entry points. The old continuous editor combined request state,
validation, YAML parsing and rendering. Field ownership and all YAML-only supported
fields are recorded in [the v3 field manifest](08-workflow-fields.md). No schema
migration, boundary type copy or new orchestration framework was introduced.

Implementation:

- `useWorkflowDraft` owns the complete draft, step/visited state, operation,
  configuration revision, preview revision, cancellation, validation errors and
  saving. Source, Matching, Schedule and Review share one footer and ordered step
  navigation (`aria-current=step`). Backtracking preserves hidden controls and
  advanced configuration. Navigation alone does not write settings or save.
- Simple source detection happens on Continue; Advanced configures explicit
  adapters/selectors. Schedule includes collapsed retention. Review exposes the
  configured rules, preview results and next run. A current successful preview is
  required for Save; zero matches is valid, errors are not. Save never runs a scrape.
- Registry and editor use the same SVG upload component, YAML parser and canonical
  validator. Invalid imports retain the current draft. Replacement of edited input
  uses an in-app confirmation with Keep editing / Replace draft and focus return.
  Late validation considers the latest edits and ignores closed/disabled importers.
  Resetting the input allows selecting the same file again; empty selection is a
  no-op. Expert YAML fields and explicit empty overrides remain intact.
- Run all moved to the registry header; truly global controls moved into an
  explicit settings dialog. Source actions share active-run/progress/cancel state
  and post-run refreshes. Partial/failed results remain visible, including useful
  persisted results. Maintenance is secondary; inherited values are explicit.
- Recorded examples now reject saved and transient Test APIs as well as Run and
  Preview; the UI explains that live execution requires desktop. Explicit fixture
  Run all iterates every enabled source and never falls back to a disabled one.
  Its regression exposed timestamp-colliding fixture IDs; fixture workflow/run IDs
  now use UUIDs. Production provider/posting identity was not changed.

The behavior-preserving controller extraction passed the original interactions
before navigation expectations were adapted. Tests retain zero-match save, hidden
hourly cents, expert keyword/YAML preservation, stale preview and interrupted
source detection assertions. New coverage adds import-before-save, invalid version,
replacement rejection/acceptance, same-file selection, edits during validation,
empty selection, manual Run all while paused, partial/error/cancel feedback,
enabled-source dispatch and explicit inherited schedule preservation with no global
writes. A duplicate inherited option found during review was corrected.

Verification:

| Check | Observed result |
|---|---|
| Isolated complete frontend suite | 400 tests across 63 files plus Vite HTML startup smoke passed. The shared workspace additionally contains the pre-existing card test. |
| TypeScript / production build / production style guard | Passed. Existing large-bundle advisory remains. |
| Rust fmt, Clippy warnings denied, workspace tests | Passed, including canonical version/unknown-field validation, preview/single/global rules and protected-state regressions. Generated binding tests left bindings unchanged. |
| Complete isolated browser suite | 24 passed. Create, edit/reopen, real browser file chooser, invalid import, replacement decision/focus, fonts, Paper/Obsidian contrast, keyboard, reduced motion, 200% zoom and existing job flows were exercised. |
| Reviewed screenshots | Five workflow baselines reviewed at wide/narrow sizes in both themes and bundled mono. Other screen baselines remained unchanged; no tolerances were relaxed. |
| Actual native wizard and extraction | Passed: local HTTP JSON-LD → actual sidecar → current-rule preview → manual workflow save. One eligible match; no discovered jobs persisted and no additional run started during Preview/Save. Final artifact profile: `scrape-native-smoke-qdnA7d`. |
| Unknown migration history | Controlled refusal passed without panic or database mutation (`scrape-native-smoke-XQfUBd`). No live database was opened. |
| Full native IPC/worker/persistence/cancellation/opener smoke | Passed (`scrape-native-smoke-SJkLGY`). |

The native wizard command is documented in `scripts/README.md` and wired into CI.
The native test does not synthesize a file chooser or claim live-provider
verification. An actual native file-chooser gesture and native visual/manual review
remain pending, as do the earlier real native pointer gates. No remote CI run or
merge was performed.

Commits: `7738245` controller extraction, `945456b` shared importer, `ba878bc`
wizard, `f9f0c34` registry/guards, `bcaf391` native worker wizard, `6887233` optional
fixture rendering, `69950c3` in-app replacement/partial feedback and `6bbc929`
inherited schedule regression.

## Section 7 — Truthful scheduling control

Implemented 2026-10-15 on `implementation/trustworthy-scraping`; Section 8 remains separate.

- Wrote the [state/priority table](section7-scheduling-states.md) before implementation. One Rust response now includes enabled-workflow count and automatic workflow names, IANA timezones and normalized dispatch instants. Global cadence uses the dispatcher's global instant only when an enabled global workflow exists. Disabled/manual workflows contribute no next run; missing/invalid instants remain unknown. No migration or user database change was required.
- One frontend formatter owns header/summary presentation and absolute date/time formatting. It never calculates calendar occurrences. Loading, failed IPC, paused, no enabled workflows, manual-only, invalid schedules, next run, and active-run states are distinct. Running remains compact; running while paused displays both facts. Stale IPC responses cannot substantiate a next-run label. Browser examples explicitly require desktop.
- The focusable header button opens a shared Modal summary with workflow times/timezones, refresh/retry, pause/resume, active-run cancellation and a Scraper route. Errors stay visible; a rejected cancellation does not claim completion. Existing modal Escape/focus restoration is retained.
- Source create/update/toggle/delete and schedule settings now refresh the canonical query through backend events. Run start, source finish and terminal events refresh the same query. Pause IPC emits settings changes; no component timer was added. Manual Run all remains available while paused.
- Found and fixed completion ordering separately: terminal notifications previously preceded release of the worker lock, allowing status refresh to retain Running. Every existing terminal branch now releases its guard before notifying. A regression observes the released lock inside completion notification; actual native success/cancellation status also passes.

| Verification | Result |
|---|---|
| Rust workspace tests | 279 passed, including generated bindings, controlled-clock status/dispatch agreement, manual/disabled exclusion, global timestamp normalization, paused+running, missing timestamps and existing DST/catch-up tests. |
| Rust formatting / Clippy warnings denied | Passed. Generated status bindings committed; no generated drift remains. |
| Shared-tree frontend suite | 407 passed across 65 files, including one pre-existing user card test. |
| Isolated frontend `npm test` | 406 passed across 65 files, plus Vite HTML transformation startup smoke. |
| TypeScript / frontend production build / style guard | Passed. Existing bundle-size advisory remains. Final compact-running-label refinement also passed targeted interaction/event tests and TypeScript. |
| Isolated browser comparisons | 26 passed after reviewing 35 images (33 changed header baselines and two new summary baselines). Both themes, narrow/wide layouts, keyboard/focus, contrast, reduced motion, fonts, workflow/import flows, real pointer resize/drag, removal and 200% zoom pass. No screenshot tolerance was relaxed. |
| Actual desktop scheduling summary | Passed in isolated current-19 profile `scrape-native-smoke-73JBTZ`: actual WKWebView summary, create/update/toggle/delete, pause/resume, manual-only/no enabled workflows, Pacific/Auckland instant agreement with backend, focus restoration, no implicit scrape. |
| Full actual native IPC/sidecar smoke | Passed in isolated profile `scrape-native-smoke-eP90fc`: running while paused, completion, cancellation/reaping, protocol failures/timeouts, persistence/restoration and external opener. |

The browser verification used isolated checkout `14faf06` plus the new scheduling browser test and reviewed screenshots. The subsequent header refinement only shortens desktop Running text; it has targeted test/TypeScript coverage and the native smoke ran afterward. Logs: `/tmp/section7-*`. The first sandboxed integration attempt could not bind localhost; the complete suite passed with local fixture access. Early native attempts were interrupted by concurrent file edits/binding generation causing Vite reloads; rerunning without edits passed. The native harness now documents that constraint.

Browser screenshots demonstrate browser fixture rendering, not live-provider verification or native visual QA. Native checks operate actual WebView controls and IPC programmatically; native visual/manual pointer gates from earlier sections remain open. No live database was tested, no remote CI run was claimed, and no merge into develop was performed. The four pre-existing user changes are byte-for-byte unchanged against `/tmp/scrape-pre-section7.patch`.

Behavior commits: `440fd01` canonical dispatch-derived status; `240a7f4` completion ordering; `14faf06` summary/refresh wiring; `8d6500b` native/browser regressions and CI.


## Section 8 — Final integration and handoff

Section 8 is still in progress because it requires actual native pointer/visual and file-chooser evidence. The exact posting in the original report is now identified, captured, and covered by a real-provider regression; the automated verification records below do not substitute for the remaining native manual gates.

### Current issue-to-test reconciliation

| Report | Current automated coverage | Remaining acceptance evidence |
|---|---|---|
| Encoded HTML descriptions | Rust structured normalization; actual worker/repository and inspector interaction cases. | Native visual review of the captured Cloudflare posting in the desktop inspector. |
| Remote + US | Captured Cloudflare Greenhouse ID `8024849` traverses the actual worker, SQLite, restart, and shared geography filter. Raw response SHA-256: `20e0e48e9e2ca890b690bcf5232ad8ca7a1f5ef5cd05c372104ca6f32eef8bcc`; the repository stores a minimal field projection. | Native click-through/visual review remains pending; source extraction and repository filtering pass. |
| Resizable Discovered panes | Keyboard/persistence component checks; browser pointer split tests; isolated native layout fixture resized the real window, changed pane ratio to 58, navigated pages, and preserved notes. | Real pointer gesture and visual inspection in native WKWebView. |
| Kanban separation and movement | Outlined columns and visible Move to; browser pointer drag; isolated native status change reached `offer`, wrote the expected history, and preserved notes/favorite state through restart. | Real native pointer drag through macOS WebView, including populated and empty columns. |
| Scraper workflow UI/import/run | URL→preview→save interactions; browser file chooser; isolated native local JSON-LD→sidecar preview matched one US/TX remote job without persisting; recorded-mode execution guard. | Native OS file chooser operation and manual workflow visual inspection. |
| Schedule control | Rust dispatch-derived status tests; frontend loading/error/paused/manual/invalid/active states; isolated native create/update/toggle/delete, pause/resume, Pacific/Auckland next-run agreement, and no implicit run; native cancellation/completion. | Native screenshot review remains part of final visual gate. |
| Pagination/search | Repository query returns 2,105 rows and searches beyond row 2,000; page jumps, stale responses, reset and native keyboard/window-reflow checks. | Native screen review at required widths and 200% zoom (browser zoom is covered separately). |
| Theme/scrollbars/fonts | Isolated actual WKWebView checks passed at 1280 and 480 widths for Paper/Obsidian, five bundled fonts, font inheritance, and app-scoped Always/Automatic scrollbar modes. Reported thumb colors: Paper `rgb(103, 99, 87)`, Obsidian `rgb(185, 181, 175)`. | Human visual review in native WKWebView; no Windows/Linux evidence. |
| SVG/action controls | Accessible icon interactions, keyboard coverage, named trash actions, visual browser baselines; workflow deletion failure feedback. | Review native icon/target appearance in both themes. |

### Section 8 audit and targeted cleanup

- Confirmed the Scraper workflow registry still displayed a text `DEL` control. Replaced it with the existing contextual, danger-colored SVG icon button and catches delete errors into the shared alert toast. The interaction test verifies the button's accessible name/SVG, confirmation text, persistence call, and visible failure message. Replaced the remaining production action arrows and square favorite indicator with shared SVGs; keyboard shortcut glyphs and decorative wizard separators remain text because they represent key names or hierarchy, not action icons.
- Updated stale component/project status notes. The old 2,000-row setting is limited to browser demonstration fixture retrieval; production listing, filtering, count, sorting and paging use the repository contract. Actual SQLite coverage searches/counts 2,105 records.
- Captured the official Greenhouse response for the exact screenshot posting (ID `8024849`) and verified its title, provider URL, Remote label, `first_published`, update time, and labeled US eligibility. Replaced the adapter's authored positive stand-in with a minimal field projection from that response; the worker/repository restart test now uses the captured posting plus the separate authored company-prose negative control.
- CI already runs recorded provider replay, generated-binding drift, workspace and frontend checks, both-theme/browser interaction tests, and isolated native startup/workflow/Kanban/layout/schedule paths. This audit did not weaken or bypass CI checks.

| Section 8 verification | Result |
|---|---|
| Recorded provider replay | Passed at 2026-10-15: Lever 8/8 and Ashby 31/31 verified, zero worker errors. These are timestamped captured responses, not live postings. |
| Shared frontend suite | Follow-up on 2026-10-15: `npm test` passed 421 tests across 67 files and the Vite startup smoke. `npm run build` passed TypeScript checking and the production bundle; Vite reported its existing large-chunk advisory. |
| Targeted workflow deletion/import and inspector interactions | Passed after SVG/error handling: 9 tests across 2 files; the broader tooltip/workflow suite also passed 23 tests before the final decorative/action-icon refinements. |
| Playwright | Follow-up on 2026-10-15: reviewed and refreshed Paper/Obsidian screenshots for the current settings category, discovered-job actions, inspector, and schedule summary. Replaced the zoom test's timing-dependent `onUpdated` assumption with an explicit native tab zoom after navigation. Full no-update browser suite: 30/30 passed, including actual 200% browser zoom. |
| Actual Tauri startup and native integration | Passed on 2026-10-15 with temporary profiles: all seven startup/migration fixtures (fresh, legacy/current/unknown at versions 12 and 19); sidecar smoke including preview, execution, block/restore, cancellation, malformed output and recovery; plus isolated appearance at 1280/480, layout, Kanban, workflow, schedule, and data-management checks. Result JSON and database audits were retained under the temporary test directories. |
| Captured Cloudflare remote-US regression | Passed: `cargo test -p scrape-worker` (35 tests across unit/process/adapter suites) and `cargo test -p scrape --test worker_repository` (5 integration tests). The exact posting `8024849` preserves its `gh_jid` URL, uses `first_published` rather than `updated_at`, extracts US from “Available Locations,” and matches the Remote+US filter after SQLite restart. |
| TypeScript, production build, style guard and Rust formatting | Passed in the isolated committed checkout after using the committed `danger` IconButton variant. Build retains its existing large-bundle advisory. |
| Previously completed Section 7 checks | Rust workspace 279 tests; browser 26 tests; full sidecar native smoke and isolated actual Tauri schedule control check passed. Section 8 changes do not touch Rust/scheduling behavior. |

Native rerun details from 2026-10-15 (each creates its own profile under `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-*`):

- `npm run test:native` — the first cold build hit the harness's 120-second timeout while Cargo was waiting on the shared target lock; a retry passed. Its result records real Tauri/sidecar preview and run, block/restore, cancellation and recovery, malformed/nonzero/missing-completion/timeout worker cases, and external opening. Result: `scrape-native-smoke-ITopJP/result.json`.
- `npm run test:native-startup` — all seven migration/startup fixtures passed: `fresh`, `legacy`, `current`, `unknown`, `legacy19`, `current19`, `unknown19`. Unknown checksums failed safely without mutation. Logs/results are in the corresponding fixture directories, including `scrape-native-smoke-720JJ2` for `unknown19`.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_APPEARANCE=1 SCRAPE_SCROLLBAR_MODE=Always npm run test:native-startup` and `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_APPEARANCE=1 SCRAPE_NATIVE_WIDTH=480 SCRAPE_SCROLLBAR_MODE=Automatic npm run test:native-startup` — passed at 1280 and 480 CSS-pixel windows; results in `scrape-native-smoke-K062B6/result.json` and `scrape-native-smoke-4rN9mI/result.json`.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_LAYOUT=1 npm run test:native-startup` — passed; `scrape-native-smoke-1X3IHR/result.json` records pane ratio 58, real window resizing, page navigation and notes persistence.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_KANBAN=1 npm run test:native-startup` — passed status/history/restart/state-preservation checks; `scrape-native-smoke-texmaC/result.json`. Input was native select/keyboard; this is not pointer-drag evidence.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_WORKFLOW=1 npm run test:native-startup` — passed actual local JSON-LD worker preview and save with one US/TX remote match and no persisted preview jobs; `scrape-native-smoke-5cuYaY/result.json`.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_SCHEDULE=1 npm run test:native-startup` — passed; `scrape-native-smoke-Xh556l/result.json` records pause/resume, CRUD, no implicit run and Pacific/Auckland next-run agreement.
- `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_DATA_MANAGEMENT=1 npm run test:native-startup` — passed four cleanup modes, cancellation, and favorite/tracked-state preservation; temporary database/backups audited; `scrape-native-smoke-qwZG5w/result.json`.

`npm ci` and the complete CI sequence have not been repeated from a newly installed clean checkout in this section; the isolated worktree used copied, already installed dependencies. No remote CI run was performed. The native harness did not synthesize OS pointer input or open the native file chooser, and no human native screenshot review was performed; those gates remain open. The full Cloudflare API response is retained locally at `/private/tmp/cloudflare-job-8024849.json` for this verification and identified by its committed checksum/projection; it is not stored in the repository. This work uses the macOS WKWebView host; results do not establish behavior on Windows/Linux. Never use or mutate the live database to complete these gates.

The four pre-existing user edits in `IconButton.tsx`, `RemoveJobButton.tsx`, discovered `JobCard.tsx`, and its interaction test remain uncommitted and unchanged by this section. Section 8 must remain in progress until the real native manual gates above are recorded or explicitly accepted as outstanding for handoff.

### 2026-10-20 — Current-tree native acceptance rerun

**Section 8 remains in progress: two requested feature harnesses passed and four failed.** The 2026-10-15 results above remain historical; they do not establish acceptance of the current working tree. Before execution, `git status --short` reported 33 modified and 11 untracked files. Existing Plan 12 Sections 1–8 changes were preserved; this acceptance update changes documentation only. No tests, harnesses, timeouts, screenshot baselines, or application implementation were modified.

Prerequisites: `cargo metadata --no-deps --format-version 1` resolved the repository `target` directory through `scripts/cargo-target-dir.mjs`; `CARGO_TARGET_DIR` was unset. `npm run build:sidecar && cargo build -p scrape` passed. The six commands below ran sequentially on macOS Darwin 24.6.0 arm64 using the actual Tauri/WKWebView application. Each launcher also rebuilt the sidecar and ran the development host.

All artifact paths below are relative to `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/`; each contains `result.json`, `dev-startup.log`, the injected `smoke.js`, and its isolated SQLite profile.

| Exact command | Observed result | Artifact directory |
|---|---|---|
| `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_LAYOUT=1 node scripts/test-native-startup.mjs` | Exit 1; `Native layout deadline: first page`. Pane resizing/persistence acceptance was not reached. | `scrape-native-smoke-37Euhj` |
| `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_KANBAN=1 node scripts/test-native-startup.mjs` | Exit 1; `Native Kanban deadline`. The generic error does not identify the failed wait; current transition/restart acceptance is not established. | `scrape-native-smoke-Ng6Z20` |
| `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_APPEARANCE=1 SCRAPE_SCROLLBAR_MODE=Always node scripts/test-native-startup.mjs` | Exit 1 at 1280×600; `Native UI deadline: apply inter`. Subsequent font/theme/scrollbar assertions were not reached. | `scrape-native-smoke-pTcXJV` |
| `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_APPEARANCE=1 SCRAPE_NATIVE_WIDTH=480 SCRAPE_SCROLLBAR_MODE=Automatic node scripts/test-native-startup.mjs` | Exit 1 at 480×600; `Native UI deadline: apply inter`. Subsequent font/theme/scrollbar assertions were not reached. | `scrape-native-smoke-3Cnf0l` |
| `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_SCHEDULE=1 node scripts/test-native-startup.mjs` | Passed: summary, create/update/toggle/delete, pause/resume, no implicit run, Pacific/Auckland next-run agreement. Result next run: `2026-10-20T20:00:00.000Z`. | `scrape-native-smoke-eR9Xke` |
| `SCRAPE_STARTUP_MODES=current19 SCRAPE_NATIVE_DATA_MANAGEMENT=1 node scripts/test-native-startup.mjs` | Passed: four cleanup modes, actual worker cancellation, favorite/tracked-state preservation, reset paused. Post-run SQLite audit passed integrity/foreign-key checks, migration history, empty reset tables except settings/migrations, and four backup databases with job counts `[0,1,1,1]`. | `scrape-native-smoke-Z4YOXA` |

The failures are observed acceptance failures, not diagnosed application defects. No rerun was used to turn them into passes. In particular, one wide appearance probe reported hidden/unfocused state, but the narrow run was visible/focused and failed at the same font step; focus alone is not an established cause.

#### Automated evidence versus outstanding native gates

- **Physical separator and Kanban dragging: pending.** `native-job-layout.mjs` dispatches a synthetic `ArrowLeft` keyboard event; `native-kanban.mjs` dispatches select changes and synthetic keyboard drag/cancel/commit events. Neither supplies physical OS pointer gestures. Their current failures additionally leave their automated acceptance paths unverified.
- **Native OS YAML chooser: pending.** None of these six runs opens or exercises the operating-system file chooser. Browser chooser coverage and injected events do not satisfy this gate.
- **Native Paper/Obsidian screenshots at 200% zoom: pending.** The appearance helper checks DOM/computed styles and does not capture screenshots or set native zoom. Both runs failed before theme iteration. The inspected narrow-run artifact directory contains no screenshot; no native screenshot review, font/scrollbar visual approval, or 200% zoom approval is claimed.
- **Scope and safety:** temporary `current19` profiles only; no live database or user application data was opened for testing. No Windows/Linux verification, live provider checks, clean-checkout full CI sequence, or physical/manual acceptance was performed in this rerun.

Next acceptance work is to diagnose the layout/Kanban/font-control deadlines without weakening verification, then rerun the affected native journeys. Separately record real pointer drags (including populated/empty Kanban columns), native YAML chooser interaction, and visually inspected Paper/Obsidian native 200% screenshots with OS/WebView, font, viewport, and zoom metadata. Until those gates and the required final integration sequence are satisfied, Section 8 is not complete.
