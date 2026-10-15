# UI refinement implementation evidence

This ledger records checks actually run for [plan 10](10-ui-workflow-refinement.md). Worktree: `implementation/trustworthy-scraping`.

## Section 0 — Starting point

- Inspected the four existing card/unread edits and checkpointed them in commit `3347e84` (`Refine discovered card unread and action treatment`). `npm test` passed before new behavior changes: 421 tests across 67 files plus the Vite startup smoke.
- The initial source inventory found native selects in workflow forms, settings, filters, the inspector, Kanban, and the log view. Geography matching also used browser datalists. Section 1 replaced these with the shared Listbox and searchable Combobox. The native `<select>` inventory now returns no matches under `src/`.
- Initial allocated `target/` size was 5.3 GB; free space was 151 GB. Later measurement remained 5.3 GB with 150 GB free. Cargo build artifacts were not used for Section 1. Process inspection failed because this environment's `sysmond` process-list service is unavailable, so no claim is made that the repository target was idle or cleaned. The existing target remains untouched.
- `scripts/build-sidecar.mjs` now resolves Cargo's actual metadata target directory, so `CARGO_TARGET_DIR` overrides are honored when locating the built worker. `node scripts/test-cargo-target-dir.mjs` checks absolute, relative, and malformed metadata.
- A pre-change open-menu screenshot set was not captured. Existing plan 08 screenshots were not treated as matching this checkpoint. No application database was opened or changed during investigation.

## Section 1 — Menus, unread marker, and date controls

- Added the pinned Headless UI 2.2.10 dependency and shared themed `Select`/Listbox, searchable Combobox, and ActionMenu controls. Options render in the nearest dialog or the document body and are positioned against their trigger. First Escape closes an open popup; a later Escape reaches its dialog. Portaled choices do not dismiss the filter panel. All app-owned native selects/datalists were removed.
- Replaced the date buttons with formatter-generated examples in a responsive Listbox. Time selection is now a labeled 12/24-hour radio group. The unread cue is a fixed-gutter 6px SVG circle using a lavender semantic token in both themes, shown for unread discoveries only.
- Added action-menu focus/keyboard coverage, Listbox disabled-option/Escape/focus coverage, all-five-date-format state updates, and Chromium journeys for open date/country choices. Captures at 480px in both themes are saved in [10-ui-review](assets/10-ui-review/). Chromium also covered 1280px, a 240px 200%-equivalent viewport, typeahead, option selection, Escape, and focus return.
- `npm test`: **425 passed across 70 files**, including Vite startup smoke. `npm run build`: passed; Vite reports the existing large-chunk advisory, with the generated JS bundle at 937 kB. `node scripts/check-style.mjs`: passed. `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers SCRAPE_BROWSER_PORT=1451 npx playwright test e2e/themed-selects.pw.ts --workers=1`: **4 passed** across both themes and 480/1280 widths, including the 240px viewport check.
- `npm install` reported 3 dependency audit findings (1 moderate, 2 critical). `npm audit` could not reach the npm advisory service because network DNS is unavailable; affected package details were not verified. No force-upgrade was run.
- Visual/browser checks used the deterministic `?demo=1&fixtures=1` fixture UI and the local Chromium cache. Actual macOS Tauri dropdown/pointer QA, clean-checkout screenshot verification, and browser download-independent dependency installation remain unperformed.
