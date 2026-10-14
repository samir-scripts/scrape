# Desktop usability and extraction repair

Status: Sections 1, 2A, and 2B are implemented with code-level tests. Native verification for the description and Remote + US journey remains pending. Sections 3–7 are not started.

**Luna: execute the implementation workbook below in order.** Sections 1–8 specify behavior; the workbook specifies how to research, implement, test, and deliver it. Do not treat a passing unit suite or a completed visual change as completion of a phase whose native acceptance remains unverified.

This follows the user's nine screenshots in [the review assets](assets/08-desktop-review/). It supersedes plan 07's continuous workflow form and fixed details-pane layout where they conflict with this request. Preserve plan 06's data protection, identity, matching, and scheduling contracts. Prior passing browser tests do not establish that these desktop interactions work.

The message ends with “all SVG.” Assumption: standardize action icons as original pixel-style SVGs inspired by images 8–9. Keep primary action labels where they explain the task. Reference 9 is a watermarked reference, not a production asset to copy or trace.

## Findings and uncertainty

| Report | Evidence in current implementation | Investigation still required |
|---|---|---|
| HTML appears in descriptions | `crates/scrape-core/src/enrich/html.rs` decodes entities directly into output, so encoded tags bypass tag stripping. `JobInspector.tsx` displays this as text. | Capture the affected provider payload and distinguish newly extracted from historical malformed records. |
| Remote US jobs disappear under US filters | Greenhouse uses location/offices only for geography; it does not extract the description's Available Locations section. Office name currently takes precedence over its location. | Identify the exact posting ID/URL from the screenshot, capture API/detail evidence, and trace matching end to end. Never infer the country from company headquarters. |
| Details cannot resize | `JobInspectorPanel` hardcodes 480px with a 45vw cap. | Audit the existing `ResizableSplitPane` before reuse; verify sizing in native WebView. |
| Board looks undivided | Column and board share a surface; columns have a header rule but no full-height outline. | Native screenshots with populated and empty columns. |
| Moving a card fails | `KanbanColumn` already uses HTML5 drag events; its apparent handle is a decorative span. | Reproduce with actual macOS pointer input, then inspect event delivery, status IPC, repository mutation, and query refresh. Root cause is not yet established. |
| Scheduled indicator is unhelpful | `App.tsx` returns SCHEDULED when enabled but `next_run_at` is absent; `TopNav` renders static text. | Distinguish missing schedule, loading/error, paused, and genuinely due work. This alone does not prove the scheduler is broken. |
| Toolbar duplication | `SubNav` owns filters/search, while `App` renders another top row for pagination/sort. | Preserve hotkeys, totals, selection, and stale-response protections when consolidating. |
| Scrollbars ignore theme | Standard and WebKit scrollbar rules already exist in `index.css`. | Inspect actual WKWebView computed styles, OS overlay settings, and app color-scheme; do not assume another identical rule fixes it. |
| YAML import placement | A visible Import YAML text button is inside advanced options. An existing source-string test still describes an SVG. | Replace that misleading assertion with a file-selection interaction test. |

## 1. Reproduce before changing behavior

1. Inspect status and preserve existing changes; make small commits by behavior. Record the current native build, OS/WebView, theme, font, window size, and database migration version.
2. Use an isolated temporary profile and populated historical fixture, never the live database. Reproduce screenshots at 1280×800, plus narrow and wide windows.
3. Add failing observable regressions for encoded descriptions, Remote + US, native card movement, split resizing, pagination, and schedule state. Add an empty/populated/filtered state for each affected screen.
4. Capture the affected Cloudflare posting with URL, external ID, timestamp, payload checksum, description, and detail response. If unavailable, record that limitation and use a clearly labeled deterministic regression fixture. The screenshot alone cannot prove API contents or a title/URL mismatch.

## 2. Repair descriptions and geographic extraction first

### Description contract

1. Replace the handwritten stripping loop with a parser-based normalization path in Rust. Decode the provider's encoded HTML at the adapter boundary with explicit bounded rules; parse the resulting document. Preserve literal comparisons, Unicode, entities, paragraphs, headings, and list items; exclude script/style content, including elements with attributes. Enforce safe truncation at Unicode boundaries and report truncation.
2. Preserve raw source evidence separately. Produce canonical searchable plain text for snippets/filtering and a restricted structured description for display (paragraphs, headings, lists, emphasis, safe links). Generate boundary types from Rust if this adds a contract.
3. Render those structures as React elements. Do not put untrusted provider markup directly into `dangerouslySetInnerHTML`. Permit HTTP(S) links only and use the application's checked opener; omit remote embeds, scripts, styles, and tracking images.
4. Show real headings and lists in the inspector; card snippets use plain text. Add `min-width: 0` and safe overflow wrapping so descriptions cannot push controls or the close icon offscreen.
5. Repair existing records through a versioned, idempotent re-enrichment operation using stored evidence. Back up consistently before persistent repair. Records without sufficient evidence remain recoverable and may require an explicit refresh; never delete and re-import jobs to clean text. Preserve notes, history, favorites, removals, and blocks.

### Workplace and eligible geography

1. Keep workplace type independent from eligible locations. Remote does not imply US or worldwide. Display `Remote · United States` when both are evidenced; otherwise explicitly show geography unknown.
2. Read provider structured geography and office location fields first. Retain multiple eligible locations. For missing geography, parse bounded, labeled job-specific sections such as Available Locations and Work Location from normalized content. Recognize US/USA/United States; do not scan all prose for country names or treat “About us” as evidence.
3. Store the evidence field/section and original text. For conflicting sources preserve evidence and flag ambiguity; do not invent a confident combined location. Use bounded cached detail fetching only when required.
4. Carry structured locations through worker NDJSON, ingestion, child rows, API, and cards. Rescraping updates extracted metadata while preserving user state. Distinguish authoritative empty geography from failed enrichment so stale country rows are not silently retained or erased.
5. Use the shared Rust predicate for `workplace=remote AND country=US`. Countries within one selection are OR; workplace and geography restrictions combine with AND. Unknown geography follows the explicit inclusion setting. Keep worker and repository results identical.

Tests: ordinary/encoded/double-encoded provider content, literal angle brackets, entities, malicious tags/URLs, multibyte truncation, heading/list rendering; Remote+US, Hybrid+US, Canada, multiple countries, unknown geography, incidental US company prose, conflicting evidence, refreshed locations. Run captured HTTP → actual worker → repository → filter → rendered inspector. Assert the exact destination remains associated with the same posting ID/title.

## 3. Shared visual primitives and native scrolling

1. Create one original pixel-style SVG component family using a consistent 16/24-unit grid, square geometry, `currentColor`, and tested sizes. Cover upload, download, arrows, close, trash, star, external link, drag handle, board/list, search, filter, bell, and terminal. Replace text/Unicode substitutes consistently. Keep Add workflow, Run all, Save, and Apply labeled.
2. Use existing `IconButton` with accessible names, tooltips, keyboard focus, and minimum 32×32 targets. Destructive icons retain semantic danger colors. No decorative icon fonts, copied stock artwork, or font-dependent glyphs.
3. Apply shared spacing and control heights across Discovered, Tracked, Scraper, and dialogs. Keep readable user-selected fonts and 14px body/12px supporting text. Remove redundant titles, generic technical subtitles, and empty navigation strips.
4. Set explicit light/dark `color-scheme` from the selected app theme. Coordinate standard scrollbar colors and WebKit fallbacks with surface/thumb/hover tokens; test both overlay and always-visible macOS settings. Keep native scroll behavior and visible thumbs. Use stable gutters where supported and prevent accidental horizontal scrolling in descriptions. Actual table/board horizontal scrolling remains available.
5. Verify native selects as well as scrollbars; the screenshot shows platform control styling that needs theme alignment.

Tests: both themes and all bundled fonts in WKWebView; keyboard focus; rendered text/control contrast; 200% zoom; reduced motion; long descriptions and scrollbar corners. Metrics receives shared control/scrollbar fixes; a dashboard redesign is outside this request.

## 4. Discovered: one toolbar, resizable panes, bottom pagination

Target layout:

```text
Tabs
Filters · Favorites                 Search…       Sort
Active filters (only when present)
Job results                 │ draggable separator │ Job details
1–50 of 437                 |‹  ‹   Page [1] of 9   ›  ›|
```

1. Consolidate Filters, Favorites, Search, and Sort into one shared toolbar. It may wrap at narrow widths; do not clip controls to enforce one physical row. Remove redundant Discovered label and top Close window button; the inspector retains an SVG close control and Escape.
2. Extract a shared bottom paginator above the hotkey footer. Use first/previous/next/last SVG buttons plus editable page number, total pages, and results range. Enter commits a valid page; invalid input receives inline feedback. Disable impossible navigation at boundaries and show a true empty state for zero results.
3. Retain repository pagination and totals. Filters/search/sort reset to page 1; deletion or stage change clamps out-of-range pages; older responses cannot replace newer pages. Page navigation changes the result pane, with explicit selection handling so an off-page inspector is not silently stale.
4. Replace fixed inspector width with a persistent horizontal split. Use pointer capture and a visible full-height separator with generous hit area; clamp both panes to readable minima and restore the previous ratio on reopen. Arrow keys resize; Home/End reach bounds; provide Reset pane width. Follow ARIA separator values and labeling.
5. At widths insufficient for two readable panes, switch to the existing full-width details dialog/view and restore the split preference when widening. Preserve notes, focus, list position, and inspector scroll through resizing.

Tests: drag/keyboard resize and persistence, rapid job selection, resize while editing notes, long content without horizontal overflow; page jump, first/last, invalid input, zero/one/many pages, >2,000 searchable records, changing totals and race responses. Verify controls in actual desktop WebView.

## 5. Tracked: clearly separated, operable stages

1. Add continuous column borders and distinct gutter/surface treatment. Keep stage headers pinned, cards solid, and the empty stage itself a usable drop target. Use readable stage labels and restrained empty messages.
2. Reproduce and fix native movement before changing drag technology. Inspect HTML5 events, source ID, target stage, native file-drop handling, and mutation errors. Tauri documents a Windows HTML5 drag/drop interaction with native drag-drop configuration; this is a hypothesis to test, not a confirmed explanation for macOS.
3. Use an actual visible SVG drag handle and sufficient drop zones, hover indication, edge auto-scroll, cancellation, and cleanup. Avoid dragging when selecting description text or operating card buttons. If native HTML5 remains unreliable, use a maintained pointer/keyboard implementation after reviewing compatibility; choose one implementation rather than stacking two systems.
4. Add a visible `Move to…` stage control on every card, usable without opening details or dragging. Route this, inspector stage changes, keyboard shortcuts, and drag through the same status mutation.
5. Show pending movement, rollback with an actionable error on persistence failure, and refresh affected counts/list/history. A same-column drop is a no-op; avoid duplicate history events. Confirm persistence after restart.
6. Keep current page-local stage counts explicitly labeled until a separate per-stage paging API exists. Show global filtered total in the bottom paginator. Do not present an empty loaded column as proof there are no jobs in that stage anywhere. Preserve board position after successful moves.

Tests: actual native pointer drag from applied to interviewing and back; empty/populated target; offscreen target; Move to with mouse/keyboard; failed IPC rollback; same-stage and rapid actions; restart persistence; active filters and page-boundary moves. Browser synthetic drag dispatch alone is insufficient.

## 6. Scraper and workflow configuration

Target Scraper header: `Workflows                       [upload SVG] [Run all] [Add workflow]`.

1. Remove the two large scheduling/defaults disclosures from the top. Put Run all in a dedicated labeled button; it runs enabled workflows with their effective rules and remains available while automatic scheduling is paused. Show active progress and Cancel during execution.
2. Put Import workflow YAML in an SVG upload control at the top of the registry and at the top of the workflow editor. Both use one importer: file selection → parsing → canonical backend validation → draft review. Import never silently saves or runs.
3. On Add workflow, use a compact breadcrumb-like step trail: **Source → Matching → Schedule → Review**. This deliberately replaces plan 07's continuous form. Use an ordered progress navigation with `aria-current="step"`; true breadcrumbs represent hierarchical navigation, so avoid incorrect breadcrumb semantics for a wizard.
4. Keep one draft state. Back/Next and revisiting completed steps preserve configuration. Source includes detection and editable name; Matching includes workplace, eligible geography, keywords, and real preview; Schedule includes timezone, cadence, and collapsed retention; Review shows effective rules, current preview counts, next run, and Save workflow. Saving does not run; provide explicit Run now afterward.
5. Allow advanced settings and YAML without losing hidden fields. Changes invalidate dependent preview results; saving requires a current successful preview or displays the reason it cannot proceed. Zero matches remains a valid reviewed outcome.
6. Place per-workflow defaults/inheritance explanations inside the relevant steps. Move truly global defaults and global automatic pause into a compact Scraper settings popover/dialog. Editing one workflow must not change defaults for every workflow. Preserve existing inherited global schedules during migration and make their effective values clear.
7. Replace jargon-heavy registry captions with useful metadata: source, next run or Manual, last result, enabled state. Put advanced logs/cleanup in secondary actions, retaining discoverability and confirmation semantics.

Tests: complete create/edit/reopen flow; backtracking; latest-draft preview; invalid URL/YAML/query; cancel file picker; import preserves all supported fields; explicit empty overrides/inheritance; no silent save/run; run-all filter parity and manual execution during global pause. Adapt existing continuous-form tests to this intended behavior rather than deleting coverage.

## 7. Replace the ambiguous Scheduled indicator

1. Remove the generic SCHEDULED label. A compact schedule control opens the Scraper schedule summary; running state shows progress/cancel. No green success dot for an unknown next run.
2. Derive truthful states from backend data: Loading, Paused, Manual only/no enabled schedules, Next [date/time], Running, or Schedule unavailable with retry/error detail. Validate timestamps explicitly with a finite parsed time; invalid Date does not necessarily throw.
3. Summary lists the next workflow(s), explicit timezone, enabled state, and global pause/resume. Invalidate/refresh status after workflow save, edit, enable/disable, pause, completion, and cancellation.
4. Verify next occurrence comes from actual due-workflow dispatch, not a fabricated UI value. Preserve DST, missed-run coalescing, pause, and manual-only rules.

Tests: fake-clock backend states, timezone/date boundary, malformed/missing timestamp, no enabled workflows, manual-only, paused/running, scheduler failure, and native click-through. Do not claim a scheduler repair from changing the label alone.

## 8. Delivery and verification gates

Commit order: regression fixtures/evidence; description normalization and historical repair; geographic extraction/parity; icons/theme; toolbar/pagination/split; native Kanban movement; workflow wizard/import; schedule control; final native review and documentation. Each behavior commit includes relevant tests and passing targeted checks. Keep broad formatting separate.

Required final checks: Rust formatting, Clippy, workspace tests, generated-binding drift, TypeScript, complete frontend tests, production build, style/accessibility checks, recorded provider replay, browser E2E, native smoke and native startup fixtures. Run from a fresh checkout with sidecar built first. Add native QA for the exact reported interactions: moving/resizing via real pointer input, scrolling, loading the affected description, Remote+US filtering, workflow import/save/run, and bottom page jumps. Exercise Paper/Obsidian at 1280×800 and 1440px, narrow windows, 200% zoom, keyboard-only and reduced-motion. Test all fonts for overflow.

Acceptance requires a temporary-database full journey: extract evidenced Remote+US posting → filter → inspect formatted content → resize → apply → move stages → restart → verify persisted state → reopen original destination. Save before/after native screenshots and record failures/limitations honestly. Browser screenshots and successful dev-server startup are not native acceptance. Update the recovery document for any schema/data repair, and never rewrite migration checksums.

No implementation tests were run while preparing this plan. Findings above come from source inspection, user screenshots, and documentation; native reproduction remains the first implementation gate.

## Luna implementation workbook

### Working rules and DRY ownership

1. Read root `AGENTS.md`, this entire plan, and affected directory instructions before editing. Re-read the actual implementation: paths below are starting points, not a substitute for tracing callers.
2. Work one bounded behavior at a time. For every phase follow **research → failing regression → implementation → targeted verification → evidence → commit**. An expected failing test proves the reproduction; fix it before committing the behavior. A fixture-only preparatory commit must leave required CI passing.
3. Record decisions and evidence in `plans/08-implementation-evidence.md` when implementation begins. For each task record commit, actual command, outcome, environment, and pending native checks. Never copy prior test totals as current results.
4. Search existing implementations before introducing helpers. Prefer extending a useful existing primitive; remove superseded implementations and update all callers in the same change. Avoid generic frameworks for a single feature.
5. Domain ownership must remain explicit:

| Rule or behavior | Single authoritative owner | Consumers |
|---|---|---|
| HTML normalization and description structure | Rust core enrichment, with adapter-specific input encoding metadata | Worker, historical repair, generated UI contract |
| Location normalization/evidence | Rust core location enrichment | Each adapter and historical repair |
| Workplace/geography matching | `crates/scrape-core/src/matching.rs` | Worker predicates and SQL compilation |
| Workflow validation/effective defaults | Existing Rust workflow service/contracts | Create, edit, import, preview, manual and scheduled runs |
| Stage changes | Existing status command/repository transaction and one frontend mutation | Drag, Move to, inspector, hotkeys |
| Schedule computation | Existing backend scheduler | Dispatcher, status response, UI summary |
| Pagination state | One controlled state owner in the jobs view/query layer | Top results summary and bottom paginator |
| Pane sizing | One reusable split primitive and persisted preference | List/details layouts |
| Icons and control states | Shared SVG components plus `IconButton`/`Button` | All screens |
| Theme/scrollbar geometry | Shared semantic CSS tokens | Every scroll container and control |
| Workflow draft/import | One draft controller and one import handler | Wizard steps, YAML, registry upload |

6. Do not implement a second HTML cleaner, country parser, schedule calculator, or matching engine in TypeScript. UI formatting may consume canonical values; it must not invent missing domain data. Generate shared types; never hand-edit bindings.
7. Tests should reuse fixture builders and normal harness setup while asserting independent expected outcomes. Do not compute expected results with the implementation being tested. Keep separate negative cases so a shared bug cannot make both sides of a parity test pass.
8. If a phase cannot meet its gate, document the exact blocker and continue only independent work. Do not mark the overall plan complete. Do not silently expand into deduplication, a new scheduler, a new design framework, or a metrics redesign.

### Step 1 — Reproduction and test foundation

**Research**

1. Run `git status --short --branch`; inspect existing diffs and preserve unrelated work. Read `scripts/native-smoke.mjs`, `scripts/test-native-startup.mjs`, `src-tauri/src/services/native_smoke.rs`, and `playwright.config.ts` to learn supported isolation and launch mechanisms.
2. Read `plans/07-interface-implementation-evidence.md` to distinguish prior verification from the new native defects. Inventory existing description, location, drag, workflow, and pagination tests; identify which only inspect source strings or dispatch synthetic events.
3. Reproduce each reported behavior with a temporary profile. Build a table of action, expected behavior, observed behavior, screenshot/log, and suspected layer. Keep hypotheses separate from confirmed causes. Do not run the normal live-profile dev app as a test harness.

**Implementation preparation**

4. Add a review-fixture manifest under the existing fixtures convention. Use provider captures with provenance where available and explicitly labeled authored cases for edge conditions. Include populated historical records carrying notes, favorite state, status history, removals, and blocks.
5. Create the evidence document and an issue-to-test checklist covering all nine screenshot references. Preserve screenshots as reference assets; do not replace them with new baselines.

**Tests and gate**

6. Run the existing relevant tests once to establish the local baseline. Add a focused failing regression immediately before each subsequent fix, rather than accumulating a large unrelated failing suite.
7. Gate: every reported issue has a reproduction or a precise reproduction limitation, an affected code path, and an assigned test layer. Commit fixture/provenance and evidence setup separately from product changes.

### Step 2A — Description normalization, display, and historical repair

**Research**

1. Trace `greenhouse.rs` and the other adapters → `enrich/html.rs` → worker event → `services/ingestion.rs` → `db/jobs.rs` → `JobInspector.tsx` and card snippets. Inspect actual encoding and stored evidence fields before designing a schema addition.
2. Review existing Cargo HTML-parser/entity dependencies and their primary documentation. Prefer a maintained parser already in the workspace. Record a small decision covering encoding boundaries, supported structure, resource limits, and preservation of literal text; do not blindly decode repeatedly until strings stop changing.

**Create regressions, then implement**

3. Add Rust table-driven tests for the description cases in section 2. Include an encoded `<p>` case that fails the current code, a script with attributes, and text containing mathematical `<`/`>` characters. Assert exact visible content and absence of executable structure.
4. Implement the canonical normalizer and restricted description model. Have adapters state their input format and use the common normalizer. Generate TypeScript contracts if required.
5. Add persistence fields only if existing evidence/storage cannot represent the contract. Use a new forward migration with nullable/backward-compatible fields and tests against populated historical schemas.
6. Implement one React description renderer shared by all inspectors. Use canonical plain text for snippets and search. Keep old records readable through a plain-text fallback until canonical repair is available; do not run a competing frontend sanitizer/parser.
7. Add versioned re-enrichment using available original evidence. Process bounded batches, report repaired/skipped/failed counts, and make retries idempotent. Define what happens after interruption and ensure a partial repair cannot advance the version for unfinished rows. Use the existing consistent backup mechanism before persistent repair.

**Verify and commit**

8. Extend `src/test/job_inspector_interaction.test.tsx` for visible headings/lists, selectable text, safe-link opener behavior, and long-word wrapping. Add worker→repository assertions in `src-tauri/tests/worker_repository.rs`.
9. Test repair twice, interruption/retry, and unavailable evidence. Compare notes/favorites/history/removal/block state before and after. Check migration failure leaves metadata untouched.
10. Gate: affected fixture displays readable structure in the native app, no source tags leak into snippets, no unsafe content executes, and historical user state survives. Split normalization, persistence/repair, and display into reviewable passing commits if independently deliverable.

### Step 2B — Workplace and geographic eligibility

**Research**

1. Inspect raw list/detail responses and the captured job's Available Locations section. Read `enrich/location.rs`, `matching.rs`, adapter office selection, and location replacement in `db/jobs.rs`. Inspect the generated location contract and filter controls.
2. Write an evidence-precedence table for structured job geography, structured office geography, labeled description sections, unknown, and conflicting evidence. Confirm whether each field means an eligible work location or merely an organizational office. An organizational office alone must not be promoted to job eligibility without supporting semantics.

**Create regressions, then implement**

3. Build independent expected-location fixtures, including Remote + US whose country occurs only under Available Locations. Include a US mention only in company prose that must remain geographically unknown.
4. Extend shared location enrichment and provenance; wire adapter inputs to it. Keep workplace classification independent. Prefer actual office location over a display name only when that office is relevant evidence for the posting.
5. Carry the result through generated contracts, ingestion and persistence. Explicitly model success-with-empty, ambiguity, and extraction failure where needed; avoid treating all three as `[]` during refresh.
6. Reuse the canonical matching predicate and existing SQL compiler. Fix only the path shown to lose geography or disagree; do not introduce a UI-only country workaround. Update display formatting once so cards, list, and inspector agree.
7. Use the same versioned repair service for stored location evidence; if a refresh is required, make its scope explicit and preserve saved state.

**Verify and commit**

8. Assert positive and negative expected matches through both worker and repository, including remote-US versus remote-Canada and unknown geography. Test multiple eligible countries, same-location country/region constraints, changed geography, and incomplete refresh.
9. Add a UI interaction that selects Remote and United States, then opens the expected posting and verifies its exact stored destination. Keep live-provider verification optional and timestamped.
10. Gate: the captured or clearly labeled equivalent fixture survives actual-worker ingestion and matches Remote + US after restart, while the negative cases do not. Commit enrichment/parity and presentation changes in coherent slices.

### Step 3 — Icons, spacing, controls, and scrollbars

**Research**

1. Inventory SVGs, Unicode action glyphs, inline colors, duplicated control classes, and theme selectors in `src/components` and `src/styles/index.css`. Review `IconButton`, `Button`, `Input`, `Modal`, and current font ownership.
2. Inspect WKWebView scrollbar behavior in both app themes with OS automatic/always-visible scrollbar settings. Record computed colors and `color-scheme`; restore any OS setting changed during QA. Use the primary references below for compatibility decisions.

**Implement**

3. Define the icon grid and shared sizes once. Implement original SVG paths using `currentColor`; migrate actions incrementally through `IconButton`. Keep accessible labels contextual, such as Remove [title] at [company]. Decorative SVGs must not add duplicate accessible names.
4. Consolidate repeated control sizing/spacing into shared variants and tokens only where there are real repeated callers. Remove superseded styles. Preserve selected global fonts; avoid new hardcoded font families in icons' surrounding controls.
5. Add app-theme `color-scheme`, reconcile standard/WebKit scrollbar rules, and fix containment/overflow. Do not implement JavaScript scrollbars merely to match appearance.

**Tests and gate**

6. Add interaction checks for icon buttons, disabled states, accessible names, and focus. Adapt `e2e/appearance.pw.ts`, `e2e/fonts.pw.ts`, and `e2e/contrast.ts`; verify keyboard use and actual rendered contrast.
7. Review changed screenshots visually before accepting baselines. Gate: both themes have coherent controls and usable scrollbars in native WebView, all bundled fonts remain applied, and no action depends on an undecipherable icon. Commit shared primitives before migrating screens.

### Step 4 — Toolbar, pagination, and adjustable details

**Research**

1. Trace `App.tsx` page state/query keys and filter resets, `SubNav.tsx`, `Shell.tsx`, `DiscoveredView.tsx`, and `JobInspectorPanel`. Search for the historical split-pane component; if absent, document that rather than importing a stale path.
2. Identify focus, hotkey and note-save lifecycles before moving containers. Specify selection behavior: default to closing the off-page inspector on an explicit page change, after safely handing pending notes to the existing save mechanism. A sort/filter change must follow the same documented selection rule.

**Create regressions, then implement**

3. Add page-navigation interaction cases and split-resize cases against controlled data. Use more than 2,000 records for repository completeness, not for every UI test.
4. Extract a controlled toolbar and paginator. Leave query/offset ownership in one place; the paginator emits requested pages and never fetches independently. Centralize page-count/clamp logic. Treat displayed pages as one-based and offsets as zero-based at a single conversion boundary.
5. Move pagination to the view bottom; remove the old row. Keep a local editable page-input string so blank typing is possible; validate on Enter/commit, preserve focus on error, and restore the current page on Escape. Announce loaded range without moving keyboard focus unexpectedly.
6. Reuse or implement one split-pane primitive with pointer capture, keyboard resizing, semantic separator values, and a versioned preference. Persist on drag end rather than every pointer event. Clamp on viewport changes and handle corrupt/out-of-range saved values.
7. Remove fixed inspector widths that compete with the split. Preserve a single inspector instance where practical so resize does not remount notes unnecessarily. Use the narrow-layout fallback when minima cannot fit.

**Tests and gate**

8. Test zero, one, exact-page and partial-last-page totals, invalid/fractional/out-of-range pages, deleting the last item, stage changes reducing totals, rapid jumps with reversed response order, filters resetting, and pending-note behavior.
9. Test pointer and keyboard split controls, min/max, persisted ratio, narrow/wide transitions, scroll preservation, and long URLs/descriptions. Extend the actual native harness/manual script for dragging the separator.
10. Gate: one top toolbar, usable bottom page jump, no orphan fixed-width styling, and resize/paging work in native WebView with persisted notes. Commit paginator/toolbar and split behavior separately.

### Step 5 — Native Kanban movement

**Research**

1. Trace `KanbanBoard.tsx`, `KanbanColumn.tsx`, `TrackedView.tsx`, the status mutation, IPC command, and repository history transaction. Reproduce a move with real pointer input before considering a dependency/configuration change.
2. Instrument drag start/over/drop and mutation outcome in the isolated debug harness. If no event arrives, investigate native/WebView interception; if events arrive, inspect transfer data, target, mutation and refresh. Remove temporary logging once diagnosed.

**Create regressions, then implement**

3. Add a failing test at the actual broken boundary and a persisted status/history integration assertion. Do not infer native support from Playwright `dispatchEvent` success.
4. Fix the diagnosed drag path or adopt one pointer/keyboard implementation if required. Check official compatibility guidance before introducing it. Document why any Tauri drag-drop configuration changes are necessary and test file-import behavior afterward.
5. Add the shared SVG handle, clear column boundaries, hover/drop feedback, and visible Move to control. Route all stage actions through one mutation; keep display state derived from the authoritative job collection.
6. Handle same-stage no-op, pending movement, cancellation, failed persistence, and rapid repeated moves. Serialize per-job transitions or apply version-aware rollback so an older failure cannot overwrite a newer successful move.

**Tests and gate**

7. Exercise mouse drag, keyboard alternatives, click-based Move to, empty/offscreen targets, scrolling, drag cancellation, nested buttons, and text selection. Assert one successful history event per actual transition and no history event for a no-op.
8. Test failure rollback, retry, concurrent transitions, filtered views and restart. Capture native before/after card placement and persistent state.
9. Gate: real native movement and Move to both persist; failure is visible and recoverable. A successful alternative control does not excuse unresolved dragging. Commit native fix, visual separation and alternative control as independently reviewable changes where possible.

### Step 6 — Workflow wizard, import and Scraper layout

**Research**

1. Trace `SourcesSettings.tsx`, `ScraperView.tsx`, `AddSourceModal.tsx`, `WorkflowSections.tsx`, YAML utilities, `services/workflow.rs`, and preview/run commands. List every supported v3 field and its owner; identify fields inherited from global defaults.
2. Audit existing tests in `workflow_editor_interaction.test.tsx`, `yaml_workflow.test.ts`, and `e2e/workflows.pw.ts`. Keep their domain assertions while changing navigation expectations. Record which controls move and where their functionality remains reachable.

**Create regressions, then implement**

3. Add tests for draft preservation through backtracking and import-before-save. Cover advanced fields that are invisible in the simple steps, explicit empty overrides, and edits to existing workflows.
4. Extract a single draft controller from the modal before adding steps. Track step, draft, validation, draft revision, preview revision, request cancellation, save state, and errors there; step components receive state/actions and never maintain competing domain drafts.
5. Implement Source, Matching, Schedule and Review with one footer and compact progress navigation. Permit back navigation without resetting inputs; validate the relevant step before continuing. Error summaries link/focus the field or step that needs attention.
6. Extract one YAML import handler using the existing YAML library and backend validator. Registry upload opens an unsaved draft; editor upload replaces the draft only after successful validation and warns before replacing unsaved changes. Failed import leaves the prior draft intact. Reset the file input so selecting the same file works again.
7. Move workflow settings into the steps; retain global settings in a compact separate dialog. Display inherited effective values and provide explicit overrides. Changing view/navigation cannot write settings or advance the schedule.
8. Add header Run all using the existing backend entry point and common run-state handling. Keep cancel/progress, action errors, and disabled/recorded-mode explanations consistent. Do not simulate live scraping in recorded-example mode, including Test and transient test APIs.

**Tests and gate**

9. Test complete creation, preview change/cancellation/races, zero matches, failed preview, edit/reopen, YAML round trips/import errors/unsupported fields, replacing unsaved input, file-picker cancellation, and no extra run on Save.
10. Test Run all with differing workflow filters, disabled workflows, global pause, worker error and cancellation. Exercise upload using an actual file chooser in browser/native coverage, not source-string assertions.
11. Gate: each supported field survives every representation/navigation path; real preview and execution agree; global changes remain explicit. Commit controller extraction, wizard, importer, and registry organization separately while preserving working entry points.

### Step 7 — Truthful scheduling control

**Research**

1. Trace scheduler status from `services/scheduler.rs` through IPC/query cache to `App.tsx` and `TopNav.tsx`. Determine whether the existing response distinguishes manual-only, no enabled schedules, errors, and pending initialization. Extend the Rust response and generate bindings only where required.
2. Write a state/priority table before coding: active run, loading/error, paused, no automatic work, and valid next run. Consider a run that remains active after automatic scheduling is paused; show both facts rather than a contradictory label.

**Create regressions, then implement**

3. Add backend fake-clock tests for relevant status cases and UI tests for missing/invalid timestamps. Keep existing DST/catch-up tests; extend only where the change affects behavior.
4. Remove SCHEDULED fallback and create one presentation formatter from canonical status. Share it between header and summary. Do not recalculate schedules in JavaScript.
5. Implement the focusable schedule control and summary with timezone, next workflow(s), pause/resume, error/retry, and running/cancel state. Use existing query invalidation/events across all schedule-changing actions rather than ad hoc local timers per component.

**Tests and gate**

6. Exercise create/update/toggle, global pause/resume, manual-only/no workflows, active run, completion/cancellation, malformed data, failed IPC, and date rollover. Assert manual Run all remains available while automatic work is paused.
7. Gate: clicking the indicator opens useful current information; every label follows backend evidence; the next displayed run agrees with dispatch under a controlled clock. Commit presentation separately from any demonstrated scheduler repair.

### Step 8 — Final integration, clean checkout, and handoff

1. Review the issue-to-test checklist and shared-owner table. Search for superseded cleaners, duplicated query state, old top pagination, fixed inspector widths, SCHEDULED fallback, Unicode action glyphs, and obsolete string-based tests. Remove only confirmed obsolete code and update affected docs.
2. Run targeted suites during each phase. Once all phases pass, execute the following from a clean checkout, preparing the toolchain/browser engine as CI does. Commands are sequential; a failed prerequisite must be fixed before its dependent verification.

```sh
npm ci
npm run build:sidecar
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run gen:types
git diff --exit-code -- src/bindings
npx tsc --noEmit
npm test
npm run build
node scripts/check-style.mjs
npm run test:providers
npx playwright install chromium
npm run test:browser
cargo build -p scrape
npm run test:native
npm run test:native-startup
```

3. Use the existing configurable Playwright port to avoid reusing an unrelated running dev server. Native scripts must continue to use temporary profiles and verify explicit completion; successful process startup alone is insufficient. Do not stop the user's running app to simplify tests.
4. Execute the native acceptance journey in section 8 with real pointer input. Record actual OS/WebView, theme, fonts, viewport, and zoom; keep distinct evidence for automated IPC checks and real dragging/resizing. If native automation cannot supply the required gesture, perform and record manual QA or leave that gate explicitly pending.
5. Review screenshot diffs individually. Update baselines only after judging the intended layout and contrast. Report any platform not verified; do not extrapolate macOS evidence to Windows/Linux.
6. Confirm migration/repair backups and recovery instructions, preserved user state, disabled recorded-mode live actions, no live-database test writes, and clean generated bindings. Audit `.github/workflows/ci.yml` so new required tests actually run, including the native interaction coverage that caught the reported defect.
7. Update the evidence document and this plan's status only when all gates are met. Final handoff includes changes, commit IDs, exact checks and results, native screenshots, remaining limitations, and recovery instructions. This planning revision itself requires documentation/diff validation only; it must not be reported as application test execution.

## Research references

- [Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html): encoded content and structured detail/office fields justify adapter-specific decoding and evidence-based geography.
- [WAI window splitter](https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/): focusable separator, accessible values, keyboard resizing.
- [WCAG dragging movements](https://www.w3.org/WAI/WCAG22/Techniques/failures/F108.html): retain a single-pointer alternative such as Move to.
- [Tauri WebView API](https://v2.tauri.app/reference/javascript/api/namespacewebview/): native drag-drop/HTML5 interaction is documented for Windows; test the actual macOS failure independently.
- [MDN color-scheme](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/color-scheme) and [scrollbar-color](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/scrollbar-color): theme platform controls and verify WebView support/fallbacks.
- [WAI breadcrumb pattern](https://www.w3.org/WAI/ARIA/apg/patterns/breadcrumb/): distinguish hierarchy from workflow progress while using the requested compact visual trail.
