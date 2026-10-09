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

**Status → color map:** `scraped` lavender · `viewed` cream (outlined) · `applied` lime · `interviewing` signal · `offer` ink (inverted) · `rejected` cherry · `ignored` ink-mute + checker dither.

**Motion:** 80–120ms linear only; selection bar slides between rows; `reduce_motion` setting + `prefers-reduced-motion` disable all.

## 3.3 Shell layout

```
┌──────────────────────────────────────────────────────────────────────────┐
│ DISCOVERED (42)  [■ TRACKED ■]   METRICS   SETTINGS        ● NEXT 18:00 │  TopNav
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
- [x] View switching: Zustand `ui.view` (`discovered | tracked | metrics | settings`) — no router needed for 4 views; persist last view in `localStorage`.
- [x] `components/ui/`: `Panel`, `Rule`, `ListRow` (inverted when selected), `Button` (outlined → inverted on hover/active), `Tag`, `KeyHint`, `Field`/`Input`/`Select`/`Toggle` (square checkbox, `■`/`□`), `Table` (boxed 1px grid like palette card), `Modal` (full-bleed paper overlay with dither backdrop), `Dither` (decorative halftone blob, SVG dots), `EmptyState`.
- [x] Global keyboard map (`useHotkeys` hook, custom ~40 LOC): `Q/E` or `[`/`]` prev/next tab; `↑/↓` or `J/K` move; `↵` open URL; `A` applied; `X` ignored; `V` viewed; `U` undo last status change; `/` search; `Esc` back/clear.
- [ ] **No flavor text rule:** Strictly remove vanity/decorative text copy (e.g. "KEYBOARD-FIRST TRIAGE ACTIVE", "COMPUTED IN REAL-TIME FROM SQLITE TRANSITIONS", "SYSTEM ONLINE"). Chrome and status labels must be purely functional and informative.

## 3.4 Views

### Discovered (Inbox)
- [x] Query `list_jobs({ statuses: ['scraped','viewed'] })`, sorted newest; search box.
- [ ] Filter via compact dropdowns (`<Select>` components):
  - **Role Filter Dropdown:** `ALL ROLES`, `FULL-TIME`, `CONTRACT`, `INTERNSHIP`, `PART-TIME`, `OTHER`.
  - **Salary Filter Dropdown:** `ANY SALARY`, `$100K+`, `$120K+`, `$150K+`, `$180K+`, `$200K+`, `$250K+`.
- [ ] Job row & detail display parsed salary (e.g., `$160K–$210K/YR` or raw string) alongside company and location.
- [x] Selecting a row for >1s auto-marks `viewed` (configurable).
- [x] Bulk: `Shift+↑/↓` multi-select → `A`/`X`.
- [x] Right panel: job metadata table (including salary row) + source health; header counter `DISCOVERED (n)`.
- [x] Live: listens to `jobs://changed` / `scrape://*` → invalidate queries; shows `SCANNING… ▓▓▓░░ 3/6 SOURCES` in TopNav during runs.

### Tracked (Kanban / List)
- [x] Columns: `APPLIED · INTERVIEWING · OFFER · REJECTED`; column header = inverted block with count.
- [x] Native HTML5 drag-and-drop (no library) + keyboard `Shift+←/→` to move card between columns.
- [x] Toggle `[KANBAN] [LIST]`; list mode = boxed table with sortable columns (applied date, company, salary, stage).
- [x] Card detail drawer: notes (autosave debounce 500ms), status history timeline from `job_status_events`.

### Metrics
- Placeholder grid in Phase 3; implemented in [Phase 4](./04-metrics-dashboard.md). Zero vanity/preview subtitle text.

### Settings (NieR "SYSTEM" style: left category list, right detail)
- [x] **SCHEDULE:** enable toggle; presets (hourly / every 6h / daily 09:00 / weekdays 09:00) + custom 6-field cron with live `validate_cron` preview of next 3 runs; `RUN NOW`; recent runs table.
- [x] **SOURCES:** boxed table (name, kind, enabled, last status/error); `ADD SOURCE` → paste URL → `DETECT` → confirm kind/identifier → `TEST` (dry run shows first 10) → save. Selector config form for `selector` kind.
- [x] **FILTERS / SCRAPER:** include/exclude keyword chips, locations, role types (extended with advanced scraper pre-filters in Phase 3.5).
- [ ] **LOGS:** Chronological scraper execution history, sorted by date and time (newest first). Any run with issues needing addressing (e.g. source HTTP 403/429 errors, timeouts, parsing/adapter failures) is marked with a prominent brutalist red square (`■` / red block). Master-detail view: left run list with status/timestamp/trigger/duration, right inspector with per-source events and diagnostic logs.
- [x] **APPEARANCE:** theme `PAPER / OBSIDIAN`, dither intensity, reduce motion.
- [x] **SYSTEM:** locale/timezone (detected, overridable), launch at login, start hidden, notifications, `OPEN DATA FOLDER`, `EXPORT CSV / JSON`.

## 3.5 Data layer
- [x] `src/lib/api.ts` typed `invoke` wrappers using generated `src/bindings`.
- [x] TanStack Query keys: `['jobs', filter]`, `['settings']`, `['sources']`, `['runs']`, `['metrics', range]`; optimistic updates for status changes with rollback on error.
- [x] `src/lib/events.ts`: subscribe to Tauri events once in `App`, map to query invalidations.

## 3.6 Tests
- [x] Vitest + Testing Library for: hotkey hook, status color map, Kanban move reducer, cron preset mapping.
- [x] Manual visual QA checklist against the NieR reference (spacing, rules, inverted states, no radii).

## Acceptance Criteria
- [x] All four views reachable by mouse and keyboard only.
- [x] Triage 20 jobs without touching the mouse.
- [x] Zero `border-radius`, `box-shadow`, or third-party UI components in the bundle (grep check in CI).
- [x] Theme switch persists across restarts via `app_settings`.
- [x] Adding a source end-to-end from Settings results in jobs appearing in Discovered after `RUN NOW`.
- [x] SubNav filters implemented via dropdowns (`<Select>`), not horizontal button pills.
- [x] Zero unhelpful vanity flavor text across the entire UI.
- [x] "SCRAPE" logo removed from TopNav; tabs align flush left.
- [ ] Extended in [Phase 3.5](./03b-enrichment-filters.md): custom filter panel (state, country, salary range slider + numbers, sponsorship & clearance tri-state `YES`/`NO`/`N/A`), table columns, and scraper advanced settings.
