# Plan 13 implementation evidence

**Status: implementation complete; native workflow acceptance blocked.** Frontend, Rust, catalog, and browser checks passed. The required real WKWebView workflow journey did not complete because the isolated native window remained unfocused/hidden; see the native section below. Do not treat Plan 13 as fully verified until that gate passes.

**Supersession:** the BLS/XLSX-backed role catalog and static generated title projections recorded below have been removed. Plan 14 Section 9 now owns role-title data, embedded seed, runtime SQLite search and current source attribution. The historical checks below describe the earlier implementation only.

## Behavior delivered

- Pinned BLS 2018 SOC detailed occupations and O*NET 31.0 alternate titles, with original files, hashes, per-dataset license, attribution, importer, and Rust-owned catalog bindings. Role terms remain literal substring filters.
- Searchable, keyboard-operable multi-selects for target roles and company filters, with custom literals. Options close after selection so the following field remains reachable; query-only text does not alter the workflow.
- Exact catalog-company selection attaches a usable board in the same draft update, deduplicates normalized endpoints, preserves customized source identity/configuration, and leaves unready sources disabled. Removing filters does not delete sources. Imported workflows require the explicit Sources action.
- Contextual keyword completions replace canned suggestions. Existing one-source company replacement remains separate.
- WorkflowSpec 8, worker protocol 9, matcher, and schema remain unchanged.

## Checks passed

| Check | Result |
|---|---|
| `python3 scripts/import-occupation-catalog.py --check` | Passed; checked derived catalog against pinned source files. |
| `python3 -m unittest scripts/test_occupation_catalog.py` | 7 tests passed. |
| `cargo test -p scrape-core occupations` | 7 tests passed. |
| `npm run gen:types && git diff --exit-code src/bindings` | Passed; Rust-generated TypeScript bindings current. |
| `cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` | Passed; 537 workspace tests, Clippy with warnings denied. |
| `npx vitest run --maxWorkers=1` | 86 files, 535 tests passed. The initial parallel `npm test` had one 5-second timeout; serial rerun passed without changing timeouts. |
| `node scripts/test-app-startup.mjs` | Passed; Vite app startup and HTML transform smoke. |
| Targeted selector tests (`multi_select_combobox_interaction`, `company_picker_interaction`, `workflow_selection_interaction`) | 3 files, 11 tests passed after close-on-selection behavior and focus handling. |
| `npm run build && node scripts/check-style.mjs` | Passed; TypeScript production build transformed 788 modules; style guard passed. Existing large-chunk advisory remains. |
| `npm run build:sidecar && cargo build -p scrape` | Passed; packaged aarch64-apple-darwin sidecar and host binary. |
| `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflow-selection.pw.ts e2e/workflows.pw.ts e2e/keywords.pw.ts e2e/appearance.pw.ts e2e/workflow-sources.pw.ts e2e/themed-selects.pw.ts e2e/zoom.pw.ts --workers=1` | 17 browser tests passed, including both themes, selector composition, save/reopen, and actual 200% browser zoom. |
| `python3 -m json.tool crates/scrape-core/data/occupations/manifest.json` | Passed after dataset-specific BLS public-domain and O*NET CC BY 4.0 metadata was recorded. |

Historical catalog implementation artifacts have been retired. The current seed, source manifest and data notes are [`seed_role_titles.tsv`](../crates/scrape-core/data/occupations/seed_role_titles.tsv), [`manifest.json`](../crates/scrape-core/data/occupations/manifest.json), and [`data notes`](../crates/scrape-core/data/occupations/README.md).

## Visual evidence

Updated and reviewed workflow snapshots:

- [Paper, 480px](../e2e/appearance.pw.ts-snapshots/paper-480-workflow-chromium-darwin.png)
- [Obsidian, 480px](../e2e/appearance.pw.ts-snapshots/obsidian-480-workflow-chromium-darwin.png)
- [Paper, 1280px](../e2e/appearance.pw.ts-snapshots/paper-1280-workflow-chromium-darwin.png)
- [Obsidian, 1280px](../e2e/appearance.pw.ts-snapshots/obsidian-1280-workflow-chromium-darwin.png)

Browser inspection also exercised 480px, 1024px, and 1440px layouts without document-wide horizontal overflow. The Chromium journey verified actual 2× zoom and selector keyboard/focus behavior. These are browser checks, not physical pointer, native file chooser, or IME checks.

The existing source-mode journey also refreshed and reviewed the nearby contextual captures under [`plans/assets/11-browser-script-review/`](assets/11-browser-script-review/): Paper/Obsidian static and browser panels at 1024px, and script panels at 480px. The captures show the role-first workflow surrounding the unchanged extractor controls; they were rewritten by the existing browser journey, not manually substituted.

## Native acceptance blocker

The required feature journey was attempted with `SCRAPE_STARTUP_MODES=fresh SCRAPE_NATIVE_WORKFLOW=1 npm run test:native-startup`. The Tauri window was reported visible but `focused=false`; WKWebView reported `visibility=hidden`. Runs terminated in the harness before completion of the role/company selection and SQLite persistence assertions (one run stopped at `O*NET occupation role`; the latest foreground retry stopped while waiting for the new workflow control). Retrying `current19`, running in a PTY, and requesting macOS foreground activation did not produce a stable focused window. The native feature acceptance gate remains open; no live database was opened.

`SCRAPE_NATIVE_WORKFLOW=1 npm run test:native` separately reported `Native smoke passed; external destination reached`, but that generic smoke does not replace the role/company workflow acceptance journey.

Physical pointer, native file chooser, physical IME, native 200% zoom, and live-provider verification were not performed.
