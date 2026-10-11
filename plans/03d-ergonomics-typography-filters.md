# Phase 3.8 — Workflow Ergonomics, System Formats, Typography & Anchored Filter Dropdowns

**Goal:** Refine application ergonomics and UI interaction design across time formatting, modal workflows, typography, tooltip collision systems, and filter mechanics. Eliminate Fitts's law interaction penalties by replacing off-canvas drawers with trigger-anchored dropdowns, provide multi-font typography selection with isolated previews, and guarantee seamless reactive settings synchronization.

← [Phase 3.7](./03c-custom-scrape-jobs.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 4](./04-metrics-dashboard.md)

---

## 3.8.0 Summary of Capabilities & Ergonomic Architecture

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        PHASE 3.8 ERGONOMIC & UX IMPROVEMENTS                           │
├────────────────────────────────────────┬───────────────────────────────────────────────┤
│ 1. TRIGGER-ANCHORED FILTER DROPDOWN    │ • Tethered directly beneath [FILTERS [F]]     │
│    (Eliminates 750-1200px mouse trip)  │ • Viewport-clamped (left, top, maxHeight)     │
│                                        │ • React Portal rendering (no parent clipping) │
│                                        │ • Light dismiss & zero blocking backdrop      │
├────────────────────────────────────────┼───────────────────────────────────────────────┤
│ 2. CLOCK MANUAL TYPING & SYSTEM FORMATS│ • Manual HH:MM input parser without defaults  │
│    (12H vs 24H, MM/DD vs DD/MM)        │ • Locale, timezone & date/time format sync    │
│                                        │ • Reactive SQLite setting mutations           │
├────────────────────────────────────────┼───────────────────────────────────────────────┤
│ 3. WORKFLOW ERGONOMICS & YAML IMPORT   │ • Simple vs Advanced scoped strictly to modal │
│    (Cadence exclusivity)               │ • Preset vs Clock time mutual exclusivity     │
│                                        │ • Direct YAML import/export & validation      │
├────────────────────────────────────────┼───────────────────────────────────────────────┤
│ 4. CONTAINS & HOURLY COMPENSATION      │ • Query-level CONTAINS text filter            │
│    (Discovered & Tracked views)        │ • Annual ($0-$500k+) vs Hourly ($0-$250/hr)   │
│                                        │ • Clean removal of obsolete flavour copy      │
├────────────────────────────────────────┼───────────────────────────────────────────────┤
│ 5. ZERO-INTERSECTION TOOLTIP ENGINE    │ • Strict 6px separation gap                   │
│    (Transform contact-edge pinning)    │ • CSS transform translate(-50%, -100%/0)      │
│                                        │ • Global UI toggle (ui.tooltips_enabled)      │
├────────────────────────────────────────┼───────────────────────────────────────────────┤
│ 6. TYPOGRAPHY & ISOLATED SPECIMENS     │ • 6 curated typefaces (Inter, JetBrains, etc) │
│    (Settings > Appearance)             │ • Reactive data-font cascading                │
│                                        │ • Isolated .font-specimen-${id} previews      │
└────────────────────────────────────────┴───────────────────────────────────────────────┘
```

---

## 3.8.1 Trigger-Anchored Filter Dropdown Popover

### Problem & Fitts's Law Analysis
The legacy design opened a full-height slide-out drawer on the far right edge of the screen when clicking the `[FILTERS (F)]` button in the top-left toolbar (`SubNav`). This violated Fitts's Law by requiring users to move their cursor across 750px–1,200px of horizontal distance, created cognitive discontinuity by drawing their gaze to the opposite side of the display, and obscured the rightmost Kanban board columns (`OFFER`, `REJECTED`) with an opaque `bg-black/40` modal backdrop.

### Anchored Popover Engine
- **Trigger Element**: Attached `id="subnav-filters-trigger"` to the `[FILTERS]` button in `SubNav.tsx`.
- **Coordinate Calculation (`calculateFilterDropdownCoordinates`)**:
  - `top`: `triggerRect.bottom + 4`
  - `left`: `Math.max(12, Math.min(triggerRect.left, window.innerWidth - panelWidth - 12))`
  - `maxHeight`: `Math.max(220, windowHeight - top - 16)`
  - `width`: `420px`
- **Portal Rendering**: Mounted into `document.body` via `createPortal`, completely eliminating clipping from parent toolbar overflow rules.
- **Light Dismiss**: Added document `pointerdown` listener that closes the dropdown when clicking outside, while safely exempting clicks on the trigger button itself to prevent double-toggle race conditions.
- **Live Filtering**: Removed the dark modal backdrop curtain, allowing users to see the job board and table update in real time as filter criteria are modified.

---

## 3.8.2 Clock Manual Typing & System Format Integration

### Problem
Manual input into the clock time fields (`HH` and `MM`) was buggy, frequently snapping back to `12` or failing to parse valid typed numbers due to premature state overrides during focus. Additionally, selections in **Settings > System** (12H vs 24H and Date Format) did not reactively persist or influence the clock displays.

### Resolution
- **Input State Machine**: Separated raw input text state from committed numerical hour/minute state in `ClockPicker.tsx`. Added blur and Enter handlers that parse, clamp (`0–23` for 24H, `1–12` for 12H, `0–59` for minutes), and format with zero-padding.
- **System Settings Persistence**: Wired `system_time_format` (`12h` | `24h`) and `system_date_format` (`MM/DD/YYYY` | `DD/MM/YYYY`) through `src-tauri/src/db/settings.rs`, TypeScript bindings, and `api.ts`.
- **Reactive Clock Formatting**: `ClockPicker` respects the user's active system time format, automatically switching between 12-hour AM/PM dials and 24-hour military dials.

---

## 3.8.3 Workflow Ergonomics & Dual-Mode Modal

### Problem
Previously, mode toggling (`SIMPLE` vs `ADVANCED`) sat in the top-level Scraper view header, creating confusion about whether the entire tab mode or just the creation modal was affected. Cadence selection also allowed contradictory states (both clock time and preset selected simultaneously).

### Resolution
- **Modal-Scoped Mode Switch**: Scoped the `[ MODE: ■ SIMPLE | ADVANCED ]` switch strictly inside the `AddSourceModal` window.
- **Cadence Exclusivity**:
  - **Simple Mode**: Explicit mutual exclusivity between Preset Cadence (e.g. `Hourly`, `Every 6 Hours`, `Daily`) OR Clock Time. Selecting one clears the other.
  - **Advanced Mode**: Choice between Clock Time OR Custom CRON expression with live 6-field tokio-cron validation.
- **Direct YAML Import/Export**: Added tab inside the modal allowing power users to directly paste, import, or export raw workflow YAML v2 with live validation.

---

## 3.8.4 Query-Level Contains Filter & Hourly Rate Toggle

### Enhancements
- **Contains Keyword Filter**: Added a `CONTAINS (KEYWORDS / TEXT)` text search input in `FilterPanel.tsx` for both Discovered and Tracked views. Evaluates against job title, company, description text, notes, and skills.
- **Annual vs. Hourly Salary Interval**: Added an interval toggle (`[ANNUAL]` vs `[HOURLY]`) to `SalaryRangeFilter.tsx`. Switches slider and numerical input bounds seamlessly between Annual (`$0 – $500,000+/yr`) and Hourly (`$0 – $250+/hr`) with appropriate step increments.
- **Copy Polish**: Removed obsolete flavour text (e.g., `QUERY-LEVEL CRITERIA HOTKEY [ESC]`) in favor of standard NieR brutalist labels and tooltips.

---

## 3.8.5 Zero-Intersection Tooltip Engine & Global Tooltip Control

### Problem
Tooltips near the right edge of the viewport suffered from clipping. Furthermore, certain tooltips overlapped their trigger buttons on hover.

### Resolution
- **Transform Contact-Edge Pinning**:
  - `top`: `translate(-50%, -100%)` with top offset `rect.top - 6px`
  - `bottom`: `translate(-50%, 0)` with top offset `rect.bottom + 6px`
  - Horizontally clamped with `12px` viewport safe margins.
  - Strict zero-intersection invariant verified across arbitrary element heights.
- **Universal Tab Hints**: Added bottom-anchored hints across all primary navigation tabs (`TopNav`), view toggles (`SubNav`), and settings categories (`SettingsView`).
- **Global Toggle**: Added `ui.tooltips_enabled` to SQLite settings and `AppearanceSettings.tsx`. When disabled, hover timers and portals are suppressed immediately.

---

## 3.8.6 Typography & Font Family Switcher

### Curated Typeface System
Added 6 curated fonts in `src/lib/constants.ts`:
1. **SF Mono / System (`default`)**: Classic NieR brutalist monospace typography.
2. **Inter (`inter`)**: Readable sans-serif with tall x-height and high legibility for reading dense job descriptions without eye fatigue.
3. **JetBrains Mono (`jetbrains`)**: Developer favorite monospace with open counters and distinct code glyphs.
4. **Geist Mono (`geist`)**: Modern minimalist developer typography by Vercel.
5. **Fira Code (`fira_code`)**: Iconic programming font with high-clarity punctuation and ligatures.
6. **IBM Plex Mono (`ibm_plex`)**: Engineered grotesque monospace blending machine precision with human warmth.

### Reactive Cascade & Isolated Specimen Previews
- **Google Fonts Loading**: Linked in `index.html` and `@import url(...)` at the very top of `index.css`.
- **Dynamic Cascade**: Controlled via `[data-font="..."]` on `document.documentElement` overriding `--font-mono` and `--font-sans`.
- **Isolated Specimen Boxes**: To prevent preview boxes from inheriting the active application font, each specimen box uses dedicated `.font-specimen-${id}` and `.font-specimen-${id} *` CSS rules with `!important` declarations. Displays character shapes (`AaBbCc 0123456789`), indicator badges, and realistic code/posting text in that exact typeface.

---

## 3.8.7 Automated Test Coverage Matrix

| Test Suite | File | Tests | Validates |
|---|---|---|---|
| Filter Coordinates & Viewport Clamping | `src/test/filters.test.ts` | 16 | Anchor geometry, viewport edge clamping, unmeasured fallback |
| Contains & Hourly Compensation | `src/test/contains_and_hourly_filter.test.ts` | 13 | Keyword matching, annual vs hourly interval switching |
| Tooltip Engine & Zero-Intersection | `src/test/tooltip.test.ts` | 18 | Transform contact-edge, zero-overlap invariant, delay state machine |
| Typography & Font Settings | `src/test/font_settings.test.ts` | 4 | Font options, reactive data-font attribute, settings API round-trip |
| Clock Schedule & Manual Typing | `src/test/clock_picker.test.ts` | 18 | HH:MM typing, clamping, dial angle calculations, AM/PM transitions |
| YAML Workflow Spec & Retention | `src/test/yaml_workflow.test.ts` | 9 | Spec v2 serialization, validation errors, toast feedback |
| Settings API & Persistence | `src/test/api.test.ts` | 5 | Setting updates for tooltips, font family, date/time formats |
| Rust Settings Repository | `src-tauri/src/db/settings.rs` | 3 | SQLite persistence, defaults, migration integrity |
