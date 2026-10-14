# Interface simplification and real-source verification

Status: researched implementation handoff; application changes are not implemented by this document.
Date: 2026-10-13. Implementer: Sol or Luna.

This updates the presentation decisions in [06-trustworthy-scraping.md](06-trustworthy-scraping.md), particularly its separate Simple workflow steps. Preserve that plan's data protection, canonical workflow specification, filtering, scheduling, identity, removal, and migration contracts. Do not reopen the database repair or redesign the backend merely to rearrange controls.

## 1. Findings and scope

| Report | Evidence in current implementation | Consequence |
| --- | --- | --- |
| Font selection is incomplete | `src/styles/index.css`: JetBrains, Geist, and Fira change `--font-mono`, while `--font-sans` remains Helvetica. `App.tsx` and AppearanceSettings also apply settings separately. `index.html` fetches fonts remotely. | Headings, descriptions, controls, and labels use different families after one global selection; offline fallback is invisible. |
| Workflow creation feels convoluted | `AddSourceModal.tsx` renders Import YAML, Export YAML, Advanced, and three navigation buttons above the form. Source detection, preview, and save appear in different step states. | Expert tools compete with the task; navigation hides a short configuration and makes progress unclear. |
| Demonstration does not test extraction | `src/lib/api.ts` generates `demo-*` IDs and example.com destinations and simulates execution. | Clicking Run in browser demo proves no live extraction. Existing explicit labeling is honest, but does not meet the user's need for real verification. |
| Triage is cluttered | `discovered/JobDetail.tsx` has a header action strip followed by a boxed triage toolbar, duplicates favorite, and passes a text label to RemoveJobButton. | Action hierarchy competes with the actual job; the removal control differs from cards. |
| Details change after applying | `tracked/JobDetailDrawer.tsx` is a separate fixed 384px layout with a stage grid, raw URL, nested description scrolling, notes, history, and bottom removal action. | The same posting acquires a different information hierarchy and interaction pattern. |
| Applied List is too dense | `tracked/TrackedList.tsx` has 14 columns, including description and multiple eligibility columns. | The title, stage, and timing information compete with rarely needed fields. |
| Kanban feels hollow | `KanbanColumn.tsx` uses the board background for unselected cards, small single-line titles, internal rules and low-opacity metadata. `KanbanBoard.tsx` forces four columns into a minimum 1120px grid; opening details adds 384px padding. | Cards lack surface separation, long titles disappear, and details compress the board. Dragged cards additionally use opacity 30%; verify reset separately. |
| Tests missed visual problems | `e2e/fixtures.ts` blocks remote font requests. Existing appearance coverage does not cover the complete discovered → applied → list → board → details journey. | Passing screenshots can validate fallback typography and omit the user's problematic states. |

The spacing problem is also a hierarchy problem: repeated borders, action bars, tiny labels, and unrelated padding values give similar visual weight to very different content. Fix shared layout rules before adding decoration.

Rendered evidence: at 1440×960 the applied List squeezed the sample title into seven lines while allocating much more width to description. Triage displayed repeated actions and several boxed data sections before its description. Kanban showed compact cards next to nearly full-height dashed empty-state boxes in both themes. The selected card had a filled surface, so the report of a persistently faded card was not reproduced; test drag-state cleanup rather than asserting that defect exists.

## 2. Research and resulting design choices

These are app-specific recommendations, not a claim of a universally optimal interface. Validate with the task walkthroughs below.

- [NN/g progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/): keep common work visible and reveal secondary complexity on demand. Apply to matching options, YAML, evidence, and history. Active hidden settings must remain summarized; errors must never be hidden in collapsed sections.
- [Carbon data-table guidance](https://carbondesignsystem.com/components/data-table/usage/): use a coherent toolbar, consistent sizing, and secondary detail surfaces for dense information. Apply to a smaller default application table with optional columns. Adopt the principles, not Carbon's visual styling or another component framework.
- [WAI modal-dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/): modal focus stays inside the dialog, Escape closes it, and focus returns appropriately. Apply to workflow/removal dialogs and narrow-window job overlays; a docked desktop inspector is nonmodal and must not trap focus.
- [MDN CSS Font Loading API](https://developer.mozilla.org/en-US/docs/Web/API/CSS_Font_Loading_API): inspect actual font loading, not only selected CSS family names. Bundle fonts so offline native use and deterministic screenshots exercise the same assets.
- [Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html): public posting identifiers and provider-returned destinations offer an authoritative basis for source verification. Test extracted identity/title/URL associations against captured responses and timestamped live responses; do not construct believable replacements.
- [WCAG text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) and [non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html): verify actual text and essential control boundaries on their rendered backgrounds, including selected, hover, and drag states.

Retain [NieR alignment](assets/ref-nier-menu.png), [sparse dither](assets/ref-dither.png), and the [palette family](assets/ref-palette.png). Keep square corners, flat surfaces, restrained accents, and thin rules. Remove the dense dither curtain behind dialogs: a quiet scrim provides separation without competing with the form.

## 3. Step-by-step implementation

### Step 1 — Establish reviewable visual baselines and shared layout rules

Before editing, inspect status and preserve unrelated changes. Record current workflow, triage, applied List, Kanban, and job inspector states in both themes with long titles and mixed metadata. Use isolated fixture storage. Capture screenshots at 1440×900, 1024×768, and 480×900; include 200% browser zoom.

Define a small shared spacing scale: 4, 8, 12, 16, 24, 32px. Start with 24px desktop page gutters, 16px narrow gutters, 16px panel/card padding, 24px between sections, and 8px between related controls. Use tokens/primitives rather than mass search-and-replace. Typography: 14–16px body, at least 12px supporting metadata, 16px card titles, 20–24px detail titles. Remove 9px helper labels in touched views. Use 36px standard controls and at least 32px icon targets; enlarge for touch where appropriate. Treat these as layout targets and allow content-driven heights at zoom.

Create/reuse a small set of primitives: IconButton, Field, Section, Toolbar, JobSummary, JobActions, and JobInspector content. Keep one border around a meaningful surface; do not outline every metadata fragment. Keep pagination at the table/list boundary and one contextual toolbar for search, filters, and view mode. Show active filters as removable summaries with a reliable reset. Avoid adding a second navigation system or a framework migration.

Acceptance: alignment is consistent across affected screens; no clipped controls, accidental page-wide overflow, or decorative texture behind text. One main action is visually dominant per task region.

### Step 2 — Make the chosen interface font genuinely global

Use one authoritative `--font-ui` selected family for navigation, titles, body text, forms, buttons, tables, dialogs, tooltips, toasts, and portal content. Migrate current presentation uses of font-sans/font-mono to that token. Retain a separate fixed-width token only for actual YAML/code, and independently styled font-choice specimens. Explain that limited code exception in Appearance; do not silently retain a second UI family.

Bundle licensed WOFF2 assets and the weights actually used, including license files. Remove runtime Google Fonts dependency. Consolidate setting application/hydration so cached startup values and persisted settings do not fight. Apply selection immediately, persist it, and expose failure with retry or rollback. Use one compact family picker and one realistic preview containing a title, paragraph, number, and button instead of six oversized competing specimens.

Files: `src/styles/index.css`, `index.html`, `src/App.tsx`, `AppearanceSettings.tsx`, shared primitives and affected views. Do not hand-edit generated settings bindings.

Acceptance: each family changes representative elements in every view and portal immediately, persists across restart, and works with network disabled. Assert font assets load through `document.fonts`; inspect rendered glyph fonts where supported (Chromium platform-font inspection), not just computed font-family. Include native WebView offline smoke and save-failure behavior.

### Step 3 — Replace workflow step buttons with one focused form

Use a single medium-width dialog, approximately 720–840px maximum, bounded by viewport gutters, with a sticky title and footer and one scrolling form body. Keep all three common sections in one continuous form:

1. Source: URL first; detect supported adapter after paste/blur, show a compact source summary, suggest an editable name. Surface detection errors next to the URL and offer manual Advanced configuration. Avoid a redundant Detect button on the happy path; provide explicit retry when needed.
2. Matching (optional): title keywords and location. An Advanced matching disclosure reveals existing field selectors, any/all/exclude, pay, workplace, eligibility, dates, expert queries and other supported controls. Summarize any active advanced rules when collapsed.
3. Schedule: Manual or Daily, time, explicit timezone, next-run summary. Preserve daily 09:00 in the device timezone for new workflows. Advanced schedule reveals weekday/timezone options and inheritance. Retention stays in a labeled disclosure with inherited/effective behavior visible.

Inline preview sits below these sections. Footer: Cancel and Preview matches initially; after a successful current preview, Save workflow becomes primary and Refresh preview becomes secondary. Invalid input has inline errors. A legitimate zero-match preview can be saved with an explanation; an extraction/validation failure is distinguished from zero matches. Show a compact match count and a few real job summaries, with an expandable breakdown of fetched/matched/rejected/blocked/invalid counts and truncation.

Keep one draft model and backend validator. Any configuration change invalidates the prior preview according to the existing contract; cancel superseded work and ignore stale responses. Collapse/expand never changes configuration. Place Import/Export/Edit YAML in Advanced tools; preserve every supported field through round trips. Save does not scrape again. Success offers an explicit Run now action.

Files: `AddSourceModal.tsx`, `WorkflowSections.tsx`, workflow draft helpers and tests. Keep canonical Rust preview/execution untouched unless a demonstrated integration issue needs repair.

Acceptance: URL → optional edits → preview → save needs no step navigation. Test create/edit/reopen, zero matches, errors, cancellation, stale preview, preserved advanced settings, YAML round trips, explicit empty overrides, keyboard traversal and narrow reflow.

### Step 4 — Make scraper verification real and clearly distinguish its evidence

The current browser generator is a fictional UI demonstration. It should not remain the primary way to demonstrate scraper correctness. Use three deliberately separate paths:

| Path | Data and behavior | Honest claim |
| --- | --- | --- |
| Test live source | Native desktop preview invokes the actual Rust adapter/worker for the entered public URL, with bounded requests and no persistence | This source was fetched at this timestamp; show actual counts, errors and destinations. |
| Recorded example | An unmodified, versioned provider-response snapshot is replayed through the real adapter/worker in tests; browser sample displays a generated extracted artifact | This is a recorded posting from a named source and capture date; it may no longer be open. |
| Synthetic edge cases | Private deterministic test fixtures for missing salary, long titles, unsafe URLs, etc. | Test-only data; no public/live verification claim. |

Remove generated fictional job cards and simulated Run from the normal browser showcase. Replace with an explicitly labeled recorded-example mode; disable live execution there with a clear Open desktop to test live source action. Do not add an insecure local scraping bridge or silently attempt direct browser fetches. Keep source capture URL, capture time, provider ID, raw response checksum, extraction version, and stored destination with each recorded fixture. Never edit title or URL independently. Keep fixtures isolated from user storage.

Add timestamped live smoke checks for supported adapters against maintained public sources. Compare actual worker output to current provider responses, including ID/title/URL association, source-returned custom URLs, native compensation where present, and unknowns. Follow bounded request/rate policies. Live zero jobs is a reported result, not automatic extraction success. Record source, app commit, run time, outcomes, and failure details. Live availability checks complement deterministic required CI; they cannot replace it. Do not label every destination verified from a sampled check.

Extend existing local HTTP → actual worker → NDJSON → repository tests with real recorded provider payloads and native IPC preview checks. Use temporary databases only. Review `src/lib/api.ts`, `src-tauri/tests/worker_repository.rs`, worker adapter tests, native smoke scripts, and E2E fixture loading.

Acceptance: every displayed example has traceable provenance; a live check demonstrably crosses the production extraction boundary. UI demo success is never reported as live scraper success. In the handoff report, separate recorded-fixture, live-provider, browser, and native results.

### Step 5 — Give discovered and tracked jobs one consistent inspector

Use a shared content hierarchy in both contexts:

1. Title and company, then location/workplace and native pay when known.
2. A compact action strip: primary Open posting, contextual Mark applied or current-stage selector, favorite SVG, remove SVG, close SVG. Ignore stays a secondary action in discovered; mark-viewed and uncommon actions can move to a labeled More menu. Preserve existing keyboard shortcuts without triggering them during text entry.
3. Description, followed by Notes. Keep notes directly reachable and show Saving / Saved / Failed accurately.
4. Collapsible More details, Source evidence, and Activity history. Include dates with their distinct meanings and eligibility here; show a concise known-value summary above when useful. Unknown location/workplace remains unknown.

Remove duplicate favorite/open controls, full-width triage button walls, the raw URL box from the main hierarchy, and the separate bottom deletion section. Use the same visible trash SVG in the header everywhere, with accessible name “Remove [title] at [company]”, tooltip, theme danger token, and 32px minimum target. Preserve the named confirmation, initially unchecked blocking option, atomic backend behavior, Undo, and restoration. Icon-only applies to the trigger; confirmation remains explicit text.

Desktop: one nonmodal docked panel around 440–520px where space permits. Narrow windows and zoom: one full-width modal overlay using the shared dialog primitive. Do not always reserve 384px regardless of viewport. Description should share the panel's main scroll, not sit in a second small scrollbox. Retain selection and scroll position when switching List/Kanban; after applying, offer View in Tracked rather than unexpectedly moving the user.

Files: `discovered/JobDetail.tsx`, `tracked/JobDetailDrawer.tsx`, view containers, shared job components, RemoveJobButton. Audit actual usage before changing the older JobMetaPanel. Guard note persistence and history loading against job-switch races while consolidating.

Acceptance: same job retains recognizable hierarchy and all data/actions before and after applying. Remove never opens the posting. Escape, focus restoration, typing shortcuts, notes saving/failure, selection, and keyboard-only stage changes work.

### Step 6 — Simplify the applied List around application tracking

Default columns: Job (title + company; favorite inline), Stage, Location/workplace, Pay, Stage updated, Actions. Use a 64px target row for the two-line identity, expanding for zoom/content. Keep titles in normal case and long text available through the inspector. Default sort should prioritize recent stage activity if a truthful repository sort exists; add a typed repository sort only if needed, never sort a fetched page and claim global ordering.

Move description, experience, new-grad, sponsorship, green-card, clearance, source, and discovery date to the inspector or an optional Columns menu with persisted preferences and Reset columns. Show “Applied on” only from a reliable applied transition; do not relabel discovery date as application date. Stage-updated time can use its existing actual field. Narrow layout keeps job and stage primary, with supplementary fields below rather than forcing all 14 columns across the window.

Keep server pagination/filter totals and stable selection. Search, Filters, Columns, and List/Kanban switch share one coherent toolbar. Do not add a second expanded-row detail system alongside the shared inspector.

Acceptance: a long-title applied job is identifiable without horizontal scrolling at the desktop baseline; optional information remains discoverable; backend sorting/pagination remain correct beyond 2,000 records.

### Step 7 — Give Kanban cards a distinct surface and readable hierarchy

Use a subdued board/column background and a solid card surface that differs in both themes. Keep a subtle border and a stronger focus/selected indicator; avoid shadows and rounded corners. Card contents: title first (up to two lines at 14–16px), company second, location/workplace and pay when known, then a compact note/favorite indicator and actions. Do not repeat stage in every card when the column already states it. Remove decorative column indices and excessive internal dividers.

Use content-aware columns around 280–320px, explicit horizontal board scrolling, and compact empty states instead of giant dashed boxes. At narrow widths offer a stage selector showing one column, while keeping List available. Opening the inspector must not make all columns unusably narrow.

Use an explicit drag handle or equivalent affordance; a visible destination highlight and placeholder show the move. Keep resting cards fully opaque. Audit drag-end, drop, cancellation, and failed-save cleanup so no card remains faded. Support the same movement through the stage selector without dragging; errors restore the prior stage and announce failure. Do not solve contrast by tinting all text with a stage color.

Acceptance: populated/empty/selected/focused/dragging cards are legible in Paper and Obsidian; long titles survive; pointer and keyboard movement preserve data and recover from failures.

Also make board counts truthful under pagination: the current board groups the fetched page into columns. Do not label page-local lengths as total stage counts. Use repository-backed per-stage totals and explicit load-more/pagination where necessary, or clearly label the loaded subset until that support exists. A job on another page must not make a populated stage appear globally empty.

### Step 8 — Validate complete tasks, then finish the remaining spacing cleanup

Implement each step in small coherent commits, with the relevant behavior tests and documentation. Frontend changes can be reviewed independently of the real-source verification work, but both are required for completion. Do not sweep unrelated backend work into this revision.

Required task walkthroughs:

- Select every interface font → navigate all screens → open workflow and removal dialogs → reload offline. Check actual loaded fonts and persistence errors.
- Paste a real supported source → refine matches → preview actual extracted jobs → save → explicitly run → open stored destination. Record live time/source and keep native verification separate from browser tests.
- Open discovered job → inspect details → write note → mark applied → find in List → switch to Kanban → change stage → reopen details. Verify identity, notes, status/history and selection through every transition.
- Remove with and without blocking → Undo → restore/unblock through Settings. Keep cross-workflow semantics intact.
- Repeat with long titles, unknown metadata, no salary, multiple compensation ranges, zero results, slow/error responses, and more than one page of jobs.
- Verify Paper/Obsidian, all selected-font loading tests, default plus widest-font visual baselines, desktop/narrow widths, 200% actual zoom, keyboard-only use and reduced motion. Cover active hover/focus/selection and note-save errors. Test 4.5:1 normal text and 3:1 essential indicators against actual backgrounds.

Review screenshots before approving baseline updates; passing snapshots alone do not establish usable spacing. Run TypeScript, frontend tests/build, relevant Rust/integration tests for changed boundaries, binding drift when contracts change, and the required clean-checkout CI. Run actual Tauri development startup and native IPC smoke, not only Vite, before final completion claims. Do not touch the user's live database for tests.

Usability review: ask a reviewer to create a workflow, identify a job's pay/location, mark it applied, find it in List/board, and remove/undo without instructions. Record points of confusion, mistaken actions and unavailable information; revise before declaring the design complete. No invented benchmark or user-study result.

## 4. Handoff completion checklist

- Global typography is visibly and measurably global, including offline native operation.
- Workflow creation has one continuous common path and preserved advanced control.
- Real-source validation uses the actual extraction pipeline; recorded examples and synthetic tests are correctly identified.
- Discovered and tracked details share hierarchy and visible SVG removal.
- Applied List uses six default columns; optional data remains accessible.
- Kanban has distinct solid cards and reliable accessible stage movement.
- Shared spacing, typography, contrast, loading/empty/error states and focus behavior are verified across the complete journey.
- Data protection, preview parity, schedules, canonical workflow round trips and removal semantics remain intact.
- Evidence states exactly which checks were run and which were not. Passing Vite/browser/fixture tests is never described as successful native startup or live scraping.

## 5. Research-session verification limits

This handoff is based on repository inspection, authoritative design/API documentation, and isolated Chromium inspection of current UI demonstration states. Demonstration screenshots are used only to inspect layout. No live scraper, user database, native WebView, complete font matrix, or full regression suite was verified during this planning task. Implementation and its acceptance checks remain outstanding.
