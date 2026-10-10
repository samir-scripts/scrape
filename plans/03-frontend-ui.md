# Phase 3 — Frontend Foundation & Brutalist UI

**Goal:** A single-window, keyboard-first interface that reads like a NieR:Automata system menu rendered on a PS2.

← [Phase 2](./02-background-services.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 4](./04-metrics-dashboard.md)

---

## 3.1 Visual references

| Palette | Dither / halftone | Layout |
|---|---|---|
| ![](./assets/ref-palette.png) | ![](./assets/ref-dither.png) | ![](./assets/ref-nier-menu.png) |

What we take from each:
- **NieR menu:** top tab row with an *inverted solid block* for the active tab; oversized muted sub-category row (`Restorative  Enhancement  … **Key Items**`); 3-column body (list · detail · status panel); hairline rules under every row; huge bold detail title; footer button hints left, contextual help right.
- **Dither image:** dot-matrix halftone clusters (2–4px grid), hard-edged shapes, a vertical **double rule** in a saturated accent along one edge.
- **Palette card:** accent set (Lavender, Obsidian, Cream, Lime, Cherry), small mono-spaced data tables boxed with 1px lines.

## 3.2 Design tokens (`src/styles/index.css`, Tailwind v4)

```css
@import "tailwindcss";

@theme {
  --color-paper:    #F5F5DC;   /* background (brief) */
  --color-paper-2:  #E6E2C8;   /* panel / hover — NieR-style deeper beige */
  --color-ink:      #1A1A1A;   /* text, rules, active block */
  --color-ink-mute: #8C887A;   /* inactive tabs, secondary text */
  --color-obsidian: #262628;
  --color-cream:    #FFF6EE;
  --color-lavender: #C6C2F2;
  --color-lime:     #D6DC82;
  --color-cherry:   #D86072;
  --color-signal:   #1F1FE0;   /* optional: halftone + double-rule accent (from dither ref) */

  --font-sans: "Helvetica Neue", Helvetica, Arial, sans-serif;
  --font-mono: "SF Mono", Menlo, Consolas, monospace;
  --radius-*: initial;         /* nuke every radius token */
}

[data-theme="obsidian"] { /* dark variant: swap paper↔obsidian, ink↔cream via CSS vars */ }

@layer base {
  *, *::before, *::after { border-radius: 0 !important; }
  html { background: var(--color-paper); color: var(--color-ink); font-family: var(--font-sans);
         -webkit-font-smoothing: none; }
  h1, h2, h3, nav, label, th, button { text-transform: uppercase; letter-spacing: -0.02em; }
  img, canvas, svg image { image-rendering: pixelated; }
  ::selection { background: var(--color-ink); color: var(--color-paper); }
  :focus-visible { outline: 1px solid var(--color-ink); outline-offset: 2px; }
}

@utility dither-dots   { background-image: radial-gradient(currentColor 1px, transparent 1.2px); background-size: 4px 4px; }
@utility dither-checker{ background-image: conic-gradient(currentColor 25%, transparent 0 50%, currentColor 0 75%, transparent 0); background-size: 2px 2px; }
@utility dither-lines  { background-image: repeating-linear-gradient(45deg, currentColor 0 1px, transparent 1px 4px); }
@utility rule-double   { border-left: 3px double var(--color-signal); }
```

**Hard rules (lint-reviewed in PRs):** no `rounded-*`, no `shadow-*`, no gradients except dither utilities, no blur, 1px borders only, no UI component libraries. Job titles and data stay **mixed-case** for legibility; all chrome (nav, labels, headings, buttons) is **uppercase** *(confirm)*.

### 3.2.1 Theme-Synchronized Brutalist Scrollbars
Default OS scrollbars (with rounded pill corners, transparent overlays, or OS-native accents) are strictly prohibited. Custom scrollbars must align with the active theme tokens:
- **Geometry:** 8px width / height, 0px border-radius (`border-radius: 0 !important`).
- **Track:** `var(--color-paper-2)` with a 1px solid `var(--color-ink)` boundary.
- **Thumb:** `var(--color-ink-mute)` with a 1px solid `var(--color-ink)` border, inverting to `var(--color-ink)` on `:hover` and `:active`.
- **Corner:** `var(--color-paper-2)` flush intersection.
- **CSS Standards:** Both standard `scrollbar-width: thin; scrollbar-color: var(--color-ink-mute) var(--color-paper-2);` and `::-webkit-scrollbar` pseudo-elements.
- **Theme dynamic:** Updates immediately when toggling between Paper (`#F5F5DC`) and Obsidian (`#262628`).

**Status → color map:** `scraped` lavender · `viewed` cream (outlined) · `applied` lime · `interviewing` signal · `offer` ink (inverted) · `rejected` cherry · `ignored` ink-mute + checker dither.

**Motion:** 80–120ms linear only; selection bar slides between rows; `reduce_motion` setting + `prefers-reduced-motion` disable all.

## 3.3 Shell layout

```
┌──────────────────────────────────────────────────────────────────────────┐
│ DISCOVERED (42)  [■ TRACKED ■]   METRICS   SCRAPER   SETTINGS  ● NEXT 18 │  TopNav
├──────────────────────────────────────────────────────────────────────────┤
│ ROLE: [ ALL ROLES ▾ ]   MIN SALARY: [ ANY ▾ ]        SEARCH: [ /       ]│  SubNav (Dropdown filters)
├───────────────────────┬───────────────────────────────┬──────────────────┤
│ Software Eng, Data  ▸ │ SOFTWARE ENGINEER, DATA       │ STATUS           │
│ STRIPE · $160K-$210K  │ ─────────────────────────────  │ Source  Greenh.  │
│ ▓▓ Platform Eng ▓▓▓▓▓ │ STRIPE · REMOTE · FULL-TIME   │ Posted  10/01    │
│ Backend Intern        │ COMPENSATION: $160K - $210K   │ Added   10/08    │
│ …                     │ ─────────────────────────────  │ Status  SCRAPED  │
│                       │ Notes ▏                        │ ──────────────── │
├───────────────────────┴───────────────────────────────┴──────────────────┤
│ [↵] OPEN  [A] APPLY  [X] IGNORE  [Q/E] TAB  [/] SEARCH                 42│  Footer
└──────────────────────────────────────────────────────────────────────────┘
```

- [x] `components/layout/`: `Shell`, `TopNav`, `SubNav`, `Footer` (context-aware key hints + purely functional status/count), `ThreeColumn`.
- [x] View switching: Zustand `ui.view` (`discovered | tracked | metrics | scraper | settings`) — 5 views with hotkey cycling; persist last view in `localStorage`.
- [x] `components/ui/`: `Panel`, `Rule`, `ListRow` (inverted when selected), `Button` (outlined → inverted on hover/active), `Tag`, `KeyHint`, `Field`/`Input`/`Select`/`Toggle` (square checkbox, `■`/`□`), `Table` (boxed 1px grid like palette card), `Modal` (full-bleed paper overlay with dither backdrop), `Dither` (decorative halftone blob, SVG dots), `EmptyState`.
- [x] Global keyboard map (`useHotkeys` hook): `Q/E` or `[`/`]` cycle across all 5 views; `↑/↓` or `J/K` move; `↵` open URL; `A` applied; `X` ignored; `V` viewed; `U` undo last status change; `/` search; `Esc` back/clear.
- [ ] **No flavor text rule:** Strictly remove vanity/decorative text copy across all views.

## 3.4 Views

### Discovered (Inbox)
- [x] Query `list_jobs({ statuses: ['scraped','viewed'] })`, sorted newest; search box.
- [x] Selecting a row for >1s auto-marks `viewed` (configurable).
- [x] Bulk: `Shift+↑/↓` multi-select → `A`/`X`.
- [x] Right panel: job metadata table + source health; header counter `DISCOVERED (n)`.
- [x] Live: listens to `jobs://changed` / `scrape://*` → invalidate queries; shows `SCANNING…` in TopNav during runs.

### Tracked (Kanban / List)
- [x] Columns: `APPLIED · INTERVIEWING · OFFER · REJECTED` (`TRACKED_STAGES`).
- [x] Responsive CSS Grid architecture: `grid grid-cols-4 min-w-[1120px] w-full h-full divide-x divide-[var(--color-ink)]` inside `overflow-x-auto` viewport (no dead space on wide monitors, no crushing on narrow screens).
- [x] Standardized `h-9 px-3 border-b border-[var(--color-ink)] bg-[var(--color-paper-2)]/40` column headers matching `SubNav` baseline, featuring stage telemetry (`// 01` to `// 04`), `STATUS_CONFIG` status accent pips, and tabular count badges (`[ 03 ]`).
- [x] Gutter stability: uniform `overflow-y-scroll` tracks ensuring card borders and header count badges stay pixel-aligned across all card count variations.
- [x] Native HTML5 drag-and-drop (no library) with delayed ghost snapshot, drag counter flicker protection, and full-height brutalist drop zone targets (`[ + ] DROP TO STAGE`).
- [x] Full 2D keyboard matrix navigation (`src/lib/kanbanNavigation.ts`): in-column vertical (`j`/`k`/`↑`/`↓`), cross-column horizontal (`h`/`l`/`←`/`→`), stage shifting (`Shift+←/→`), inspect drawer (`Enter`), and external URL launch (`Shift+Enter`).
- [x] Toggle `[KANBAN] [LIST]`; list mode = boxed table with sortable columns: `TITLE`, `COMPANY`, `ROLE`, `SALARY`, `LOCATION`, `YOE`, `NEW GRAD`, `SPONSORSHIP`, `CLEARANCE`, `STAGE`, `DATE`.
- [x] Card detail drawer: notes (autosave debounce 500ms), status history timeline from `job_status_events`, with responsive padding shift to prevent 4th column occlusion.

### Metrics
- Real-time pipeline conversions, streaks, role type donut, and salary distribution histogram.

### Scraper (Unified Scraper Hub: left category list, right detail)
- [ ] **SOURCES:** Job boards table (name, kind, enabled, last status/error); `ADD SOURCE` modal supporting Greenhouse, Lever, Ashby, JSON-LD, CSS Selector, and **Custom Scraper (CSS + Regex)** with live preview testing.
- [ ] **SCHEDULE:** Run schedule presets (hourly / every 6h / daily 09:00 / weekdays 09:00), custom 6-field cron with live validation, next 3 run preview, and `RUN NOW` trigger.
- [ ] **FILTERS:** Scraper-level ingestion filtering: role types, include/exclude keywords, locations, and `RECLASSIFY POSTINGS` button.
- [ ] **LOGS:** Chronological scraper execution history, sorted by date and time (newest first). Any run encountering issues or errors is marked with a prominent brutalist red square (`■`). Master-detail log inspector.

### Settings (Application Preferences: left category list, right detail)
- [x] **APPEARANCE:** theme `PAPER / OBSIDIAN`, dither intensity, reduce motion.
- [x] **SYSTEM:** locale/timezone, launch at login, start hidden, notifications, database file path, data folder, `EXPORT CSV / JSON`, and `DATA WIPE`.

## 3.5 Data layer
- [x] `src/lib/api.ts` typed `invoke` wrappers using generated `src/bindings`.
- [x] TanStack Query keys: `['jobs', filter]`, `['settings']`, `['sources']`, `['runs']`, `['metrics', range]`; optimistic updates for status changes with rollback on error.
- [x] `src/lib/events.ts`: subscribe to Tauri events once in `App`, map to query invalidations.

## 3.6 Tests
- [x] Vitest + Testing Library for: hotkey hook, status color map, Kanban move reducer, cron preset mapping, application readiness gate.
- [ ] Theme tests: verify custom brutalist scrollbar classes and CSS variables switch cleanly between Paper and Obsidian themes.
- [ ] Mandatory feature test coverage: every new feature developed on its own `feature/*` branch must include automated unit/component tests covering all state variations before merging into `develop`.
- [x] Manual visual QA checklist against the NieR reference (spacing, rules, inverted states, no radii).

## Acceptance Criteria
- [x] All five views (`DISCOVERED`, `TRACKED`, `METRICS`, `SCRAPER`, `SETTINGS`) reachable by mouse and keyboard only.
- [x] Triage 20 jobs without touching the mouse.
- [x] Zero `border-radius`, `box-shadow`, or third-party UI components in the bundle (grep check in CI).
- [x] Theme switch persists across restarts via `app_settings`.
- [x] Adding a source end-to-end from Scraper tab results in jobs appearing in Discovered after `RUN NOW`.
- [x] SubNav filters implemented via dropdowns (`<Select>`), not horizontal button pills.
- [x] Zero unhelpful vanity flavor text across the entire UI.
- [x] "SCRAPE" logo removed from TopNav; tabs align flush left.
- [ ] OS default scrollbar replaced with theme-synchronized brutalist scrollbar (0px radius, 1px border, ink/paper colors).
- [ ] All user-suggested features developed on dedicated feature branches with passing tests before merge.
- [ ] Extended in [Phase 3.5](./03b-enrichment-filters.md): custom filter panel (state, country, salary range slider + numbers, YOE, New Grad, sponsorship & clearance tri-state `YES`/`NO`/`N/A`), table columns, scraper advanced settings, and scraper logs.
- [ ] Extended in [Phase 3.6](./03c-custom-scrape-jobs.md): custom company scrape jobs registry, per-job cadence/cron scheduling, isolated `[RUN NOW]` execution, dry-run testing, telemetry inspector drawer, and dual form/YAML specification editor.

