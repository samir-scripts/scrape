# Plan 14 implementation evidence

## Section 1 — Rename and simplify workflow navigation

**Commit:** `2cf79a7` (`feat: simplify workflow editor navigation`)

**Implemented:** visible navigation now reads **WORKFLOWS**; the Simple/Advanced selector is at the top of Create/Edit and preserves the same draft; the dialog is 1200px wide on desktop and viewport-bounded on narrow screens; Roles, Criteria, Sources, Schedule and Review are peer steps; Review owns readable summary, readiness, preview results, Preview and Save; source coverage copy is concise and factual. Updated affected navigation callers, tests, directory READMEs, workflow contract and plan index. No protocol/schema/matcher changes.

**Checks run:**

- `npx vitest run --maxWorkers=1 src/test/workflow_editor_interaction.test.tsx src/test/workflow_selection_interaction.test.tsx src/test/workflow_run_journey_interaction.test.tsx src/test/workflow_browser_script.test.tsx src/test/workflow_import_interaction.test.tsx src/test/keyword_entry_interaction.test.tsx src/test/matching_compatibility_interaction.test.tsx src/test/enrichment_outcomes_interaction.test.tsx src/test/template_catalog_interaction.test.tsx src/test/app_startup.test.ts src/test/scraper_tab.test.ts` — 11 files, 56 tests passed.
- `npx vitest run --maxWorkers=1 src/test/sources_editor_interaction.test.tsx src/test/workflow_editor_interaction.test.tsx` — 2 files, 13 tests passed.
- `npx tsc --noEmit` — passed.
- `node scripts/check-style.mjs` — passed.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflows.pw.ts e2e/workflow-selection.pw.ts e2e/workflow-scroll.pw.ts e2e/zoom.pw.ts` — 10 tests passed (17.3s), including both themes, desktop/narrow scrolling and actual 200% browser zoom.
- `git diff --cached --check` — passed before commit.

**Real UI smoke:** managed Chromium at `http://127.0.0.1:1432/?demo=1&fixtures=1`. Switched Advanced → Simple → Advanced after entering `vector databases`; the term remained selected. Navigated to Review and observed its heading, Preview and Save actions. Measured the open dialog and document at 480/1024/1440 CSS px: dialog widths 464/1008/1200px, left offsets 8/8/120px, and no document horizontal overflow. The visible source hint was reviewed after shortening.

**Reviewed captures:** `plans/assets/29-workflow-scroll-review/paper-desktop.png`, `paper-narrow.png`, `obsidian-desktop.png`, and `obsidian-narrow.png`. All four reflect the five-step Review state and were visually inspected before inclusion.

**Limits:** browser journeys used `?demo=1&fixtures=1`; their previews and saves are demonstrations, not live scraping or production persistence. No native WKWebView workflow interaction, physical pointer/IME, native file chooser, live provider or user's database was exercised in this section. Plan 14 remains incomplete; Section 2 is next and must be executed separately.

## Criteria and local-time refinements

**Implemented:** retained exact inclusive experience min/max fields with either endpoint unbounded; documented why a slider is inappropriate for precise/unbounded years. Annual/Hourly now show a distinct semantic-token selected state and accessible pressed state. Workflow schedule now reuses the existing themed `ClockPicker`, follows the system 12/24-hour preference and converts to/from the unchanged canonical `HH:MM` schedule value. No matcher, workflow schema, or scheduler behavior changed.

**Checks run:**

- `npx vitest run src/test/workflow_editor_interaction.test.tsx src/test/native_salary_interaction.test.tsx src/test/experience_filter.test.ts --maxWorkers=1` — 3 files, 21 tests passed.
- `npx tsc --noEmit` — passed.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflows.pw.ts` — 3 tests passed, including keyboard minute adjustment and review/save time round-trip.

**Real UI smoke:** managed Chromium at `http://localhost:1432/?demo=1&fixtures=1`; navigated to Schedule and observed the existing ClockPicker, editable hour/minute controls, AM/PM controls and timezone. Computed styles showed Paper and Obsidian token values on the actual clock surface, then restored Paper. Browser screenshot calls timed out; no capture is claimed.

**Limits:** this is fixture/demo mode, not live scraping or production persistence. Does not close Plan 14 Section 2's complete checkbox/radio inventory or Section 7's native save/reopen acceptance; those gates remain open. Physical pointer/IME, native WKWebView, and native file chooser were not exercised.

## Section 4 — Geography autocomplete repair

**Implemented:** shared `SearchCombobox` now matches every query token against token prefixes in the label/value/search metadata, so typed multiword country/state names work without substring collisions (`US` no longer matches an unrelated word such as “Australia”). Geography options expose canonical name/code search text. City input now reuses the existing eight common-city suggestions while keeping any user-entered location term valid; no complete city database or coverage claim was added.

**Checks run:**

- `npx vitest run src/test/workflow_geography_interaction.test.tsx --maxWorkers=1` — 4 tests passed, including pasted country name, multiword country/state selection, state code, city suggestion, custom city, unknown country, Escape, country change and imported multiple-country preservation.
- `npx vitest run src/test/workflow_geography_interaction.test.tsx src/test/company_picker_interaction.test.tsx src/test/workflow_selection_interaction.test.tsx src/test/multi_select_combobox_interaction.test.tsx --maxWorkers=1` — 4 files, 15 tests passed.
- `npx tsc --noEmit` — passed.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflows.pw.ts` — 3 tests passed; workflow preview, save and reopen retained country, state, city and workplace rules.

**Limits:** browser journey used fixture/demo data only; no live location service or global city directory is used. Native WKWebView, physical pointer and user's database were not exercised.

## Section 9 — O*NET role-title seed and indexed search

**Implemented:** replaced the incomplete static BLS/O*NET title catalog and workbook inputs with one normalized O*NET 31.0 TSV (45,716 case-insensitively unique titles; 1,535,832 bytes), with source URLs, downloaded-file SHA-256 digests, CC BY 4.0 terms, attribution and output digest in the data manifest. No XLSX, legacy static catalog or generated role-title projection remains. Migration 30 creates indexed SQLite role and token tables; startup seeds only an empty table in one prepared-batch transaction. The Tauri search command feeds the existing multi-select's async suggestions and preserves literal custom titles and the unchanged role-filter contract.

**Checks run:**

- `python3 scripts/prepare-role-title-seed.py --occupation-data /tmp/onet-31.0-occupation-data.csv --reported-titles /tmp/onet-31.0-sample-of-reported-titles.csv --alternate-titles /tmp/onet-31.0-job-titles.csv --check && python3 -m unittest scripts.test_role_title_seed` — deterministic output check passed; 3 normalizer tests passed.
- `cargo test --workspace` — 535 tests passed across 16 suites, including migration 30 population/recovery, role seed rollback/retry and indexed search.
- `cargo test -p scrape --lib db::role_title_tests -- --nocapture` — 2 database tests passed; debug seed 696ms, populated-table skip 1ms.
- `cargo test -p scrape --lib --release db::role_title_tests::role_title_seed_search_is_indexed_idempotent_and_atomic -- --nocapture` — 1 test passed; optimized cold seed 231ms for 45,716 rows, populated-table skip 0ms.
- `npm run gen:types`, `npx tsc --noEmit`, `npm run build`, and `node scripts/check-style.mjs` — passed. Production build transformed 789 modules; existing large-chunk advisory remains.
- `npx vitest run src/test/workflow_editor_interaction.test.tsx src/test/multi_select_combobox_interaction.test.tsx --maxWorkers=1` — 2 files, 14 tests passed, including asynchronous suggestion selection and custom literal values.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflows.pw.ts --workers=1` — 3 workflow browser journeys passed.
- `npm run build:sidecar`, `cargo build -p scrape`, `rustfmt --edition 2021 --check` on changed Rust files, `node --check scripts/native-workflow.mjs`, `python3 -m json.tool crates/scrape-core/data/occupations/manifest.json`, and `git diff --check` — passed.
- Spreadsheet search `**/*.xlsx;**/*.xls;**/*.xlsm` — no files found.

**Native limit:** `SCRAPE_STARTUP_MODES=fresh SCRAPE_NATIVE_WORKFLOW=1 npm run test:native-startup` failed before the WKWebView emitted its first workflow log (`NATIVE_SMOKE_WINDOW ... focused=false`). The native role picker, preview, save and run journey did not execute. The Plan 14 Section 9 gate remains open until isolated native acceptance succeeds.

**Unrelated check blockers:** `cargo fmt --all --check` and workspace Clippy are blocked by formatting and `clippy::iter_overeager_cloned` in the already-modified `src-tauri/tests/worker_repository.rs`; that user-owned file was left untouched. Changed Rust files pass targeted rustfmt checking. No live or user database was opened.
