# Desktop usability and extraction repair

Status: proposed implementation handoff; application code has not changed for this plan.

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

## Research references

- [Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html): encoded content and structured detail/office fields justify adapter-specific decoding and evidence-based geography.
- [WAI window splitter](https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/): focusable separator, accessible values, keyboard resizing.
- [WCAG dragging movements](https://www.w3.org/WAI/WCAG22/Techniques/failures/F108.html): retain a single-pointer alternative such as Move to.
- [Tauri WebView API](https://v2.tauri.app/reference/javascript/api/namespacewebview/): native drag-drop/HTML5 interaction is documented for Windows; test the actual macOS failure independently.
- [MDN color-scheme](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/color-scheme) and [scrollbar-color](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/scrollbar-color): theme platform controls and verify WebView support/fallbacks.
- [WAI breadcrumb pattern](https://www.w3.org/WAI/ARIA/apg/patterns/breadcrumb/): distinguish hierarchy from workflow progress while using the requested compact visual trail.
