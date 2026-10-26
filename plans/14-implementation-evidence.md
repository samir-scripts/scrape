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

**Commit:** `ea58780` (`feat: seed searchable O*NET role titles`)

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

## Approved step 5 continuation — title ranking and migration-30 startup repair

**Commit:** `3ba9e73` (`fix: finish indexed role search and repair migration 30 startup`).

Implemented the remaining audited step-5 gaps: Unicode-casefold title ordering,
same-tier SOC tie-break tests, rejection of malformed CSV/empty titles with source
diagnostics, bounded search (256 UTF-8 bytes / 16 unique words / 50 results),
exact → full-title prefix → other token-prefix ranking, and generated
`RoleTitleSuggestion` title/SOC/tier metadata. The picker now preserves backend
ranking/tokenization rather than alphabetically re-filtering remote results.
Selected workflow values remain literal strings. The reordered seed still
contains 45,716 titles; its current hash is in the source manifest.

The reported startup screenshot is a version-30 history mismatch. A prior SQL
revision removed `WITHOUT ROWID` after the earlier version could have been
applied. Preserved that earlier SQL as a recognized immutable manifest and added
forward migration 31. Startup requires the exact recognized checksum, complete
successful history and matching reconstructed schema; it backs up before repair.
No checksum rewrite, database deletion or user-data reset is performed.

Checks from this continuation:

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.test_role_title_seed` — 5 passed; full pinned CSV regeneration and `--check` passed.
- `cargo test -p scrape --lib role_title -- --nocapture` — 7 passed, including all three migration-30 variants, injected migration failure/rollback/retry, unchanged historical checksums, protected job state, backup and restart, and unknown-checksum refusal without database mutation.
- `cargo test -p scrape --lib` — 215 passed.
- Initial concurrent release role tests measured 347ms cold seeding; this missed the target. Sorting/deduplicating term links in primary-key order removes random index writes. The subsequent isolated `cargo test -p scrape --lib --release db::role_title_tests::role_title_seed_search_is_indexed_idempotent_and_atomic -- --nocapture` passed: cold seed **221ms**, populated skip **0ms rounded**, 45,716 rows, on Apple M4/macOS 15.6, optimized build. These are seed-only measurements, not total schema-plus-startup latency. Warm startup returns before parsing or inserting.
- Same isolated run: exact `Software Developers` 69µs; prefix `software dev` 146µs; broad token `engineer` 885µs; `ware` 214µs. Token-prefix search uses the term-first index; arbitrary within-word substring search is intentionally not offered. The earlier concurrent broad-token measurement was 1,154µs; sub-millisecond lookup is not a universal guarantee.
- `npx tsc --noEmit`; focused workflow-editor/combobox Vitest run — 2 files, 14 tests passed; `npm run build` and `node scripts/check-style.mjs` passed. Existing production chunk advisory remains.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflow-selection.pw.ts e2e/workflows.pw.ts --workers=1` — 4 passed after updating the retired static-catalog dependency to an explicit browser suggestion fixture. This proves browser presentation/selection only; it does not claim native SQLite coverage.
- `npm run gen:types`, targeted changed-file rustfmt checks, and `git diff --check` passed.
- `cargo build -p scrape && SCRAPE_STARTUP_MODES=fresh npm run test:native-startup` — real Tauri search IPC returned canonical title/SOC/tier; artifacts `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-h4jZsU`.
- After adding migration 31, `cargo build -p scrape && SCRAPE_STARTUP_MODES=legacy30,current30,unknown30 npm run test:native-startup` — all three passed. Artifacts respectively `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-qWZx7B`, `.../scrape-native-smoke-aA2Svr`, and `.../scrape-native-smoke-qrzKNo`. The native launcher rebuilt the sidecar.

Limits: the complete native workflow journey was attempted again with the changed
role-search boundary. It reached the first role-picker navigation logs but the
window became hidden/unfocused while waiting for Add workflow; it did not
complete role selection/preview/save. Artifacts:
`/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-ZMrEdh`.
An additional managed-browser visual capture attempt timed out; no new reviewed
screenshot is claimed. Native startup/IPC recovery is proven on isolated
fixtures. Section 9's full native editor selection/preview/save acceptance remains
open. After the user resumed and requested the next step, Section 10 was implemented
and its scoped gate was exercised; Section 10 limits are recorded below.

## Section 10 — Bounded company autocomplete

**Implemented:** native company suggestions now merge exact verified template companies with case-insensitive prefix matches from local job-employer names. Migration 32 adds `idx_jobs_company_nocase`; bounded results rank exact then prefix matches, with local posting counts as tie-breakers. The picker shows local counts without implying a verified career board, keeps any unmatched company as a literal employer filter, and never invents a source URL. More than 50 results are disclosed through the existing “Show more results” control. This is not a complete Fortune/local-company directory.

**Checks run:**

- `cargo test -p scrape --lib company_search -- --nocapture` — 2 tests passed, including populated v31→v32 migration preservation, wildcard escaping, 250-result bound, index query plan, saved-state preservation, exact local employer ranking and template coverage.
- `npx tsc --noEmit` — passed.
- `npx vitest run src/test/company_picker_interaction.test.tsx src/test/workflow_editor_interaction.test.tsx src/test/multi_select_combobox_interaction.test.tsx --maxWorkers=1` — 3 files, 23 tests passed, including custom Dollar General, known board attachment, duplicate behavior, keyboard selection and paging beyond 50 suggestions.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflow-selection.pw.ts` — 1 browser journey passed; typed company values persisted through the workflow preview/save/reopen UI in explicit fixture mode.
- `SCRAPE_STARTUP_MODES=observed30 npm run test:native-startup` — actual isolated Tauri startup applied migrations 31 and 32; native `search_companies` IPC returned the indexed `Fixture Co` posting. Artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-7E3Kpl`.

**Limits:** browser interaction used `?demo=1&fixtures=1`; no live provider or external directory is used. Native IPC exercised a temporary fixture database, not a native company-picker workflow selection/save journey. There is no new screenshot. The autocomplete suggests only supported templates and employers present in local job records; unknown typed terms remain selectable.

**Startup mismatch correction:** a read-only online SQLite backup and checksum/schema inspection identified the user's exact role-term migration-30 variant. The explicit reviewed database-repair helper advanced the user's database from v30 to v31 without rewriting checksums or changing role/user data; an immediate second run was idempotent. Integrity and foreign-key checks passed. The pre-repair snapshot and adjacent v31 repair backup remain preserved. The app's next startup applies additive migration 32. Recovery details are in [`database-recovery.md`](database-recovery.md).

## Section 6 — Default feeds and government-source research

**Implemented:** new workflows enable both safe built-in public search feeds:
The Muse and Arbeitnow. They are separate, enabled source records; a new user
does not have to choose a provider. The Sources hint states their limited
coverage and explicitly says government listings, including USAJOBS, are not
supported. Existing/imported workflows are unchanged; tests preserve their
source records and only new drafts receive these defaults. Selected, supported
company boards remain additional sources.

**Government-source decision:** no government adapter was added. USAJOBS
requires an application API key for search and uses the registered email in the
`User-Agent` header ([authentication](https://developer.usajobs.gov/guides/authentication),
[quick start](https://developer.usajobs.gov/General/Quick-Start)). Its guidance
allows up to 500 results per page and 10,000 per query but publishes no fixed
request-per-minute quota ([rate limits](https://developer.usajobs.gov/guides/rate-limiting)).
The official search contract supplies announcement and application URIs, but
data freshness/completeness and service availability are not guaranteed
([search API](https://developer.usajobs.gov/api-reference/get-api-search),
[terms](https://developer.usajobs.gov/guides/terms-of-service)). This app has no
per-user OS credential-store integration, and workflow source configuration
rejects credentials. Storing the key/email in workflow YAML, SQLite source
configuration, or shared defaults would expose secrets, so USAJOBS cannot be
implemented safely within this section.

The reviewed NYC DCAS [Jobs NYC Postings dataset](https://data.cityofnewyork.us/api/views/kpav-sd4t)
is public and geographically narrow. Its data dictionary separates External
and Internal postings and provides job identifiers and descriptive fields, but
no per-posting application URI. Anonymous Socrata access shares a throttled IP
pool ([API token guidance](https://dev.socrata.com/docs/app-tokens.html)); the
dataset metadata does not establish a dataset-specific reuse license or an
official ID-to-application-URL mapping. It is not added until attribution,
reuse and authentic posting navigation can be verified. No source URL is
constructed from an identifier.

**Checks run:**

- `npx vitest run src/test/workflow_editor_interaction.test.tsx src/test/company_picker_interaction.test.tsx --maxWorkers=1` — 2 files, 19 tests passed. New drafts save two enabled search providers; company board attachment remains additional; imported source lists remain unchanged.
- `cargo test -p scrape-worker --test coordinator_tests default_public_search_feeds_are_both_examined_on_an_explicit_run -- --nocapture` — 1 loopback-HTTP sidecar process test passed; both feeds returned one accepted posting and successful scan outcomes.
- `PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/workflow-selection.pw.ts --workers=1` — 1 browser journey passed; both built-in feeds appeared in the new workflow, remained saved alongside selected direct boards, and were present on reopen. Browser used explicit fixture/demo mode.
- `npx tsc --noEmit` and `node scripts/check-style.mjs` — passed.
- `rustfmt --edition 2021 --check crates/scrape-worker/tests/coordinator_tests.rs` and `git diff --check` — passed.

**Limits:** no government source is supported; no live USAJOBS/NYC requests,
native workflow journey, or physical-device QA was performed. Feed verification
uses local fixtures, not live-provider coverage.

## Approved Software Engineering Preview crawler repair

**Implementation commit:** `7cd4a10` (`fix: bound workflow preview crawler coverage`).

**Implemented:** workflow Preview applies a six-navigation cap only to transient
public-HTML selector configs. The saved Software Engineering workflow remains at
50 pages/source; normal execution retains its source/workflow deadlines. Crawl
timeout diagnostics now identify whichever source or navigation deadline expired
first. Preview marks page/request/time limits as sampled and does not report an
eligibility-evidence error for a partial source with zero unresolved candidates.
Preview breakdown retains per-source termination details; logo decoding remains
separate. The workflow's 24-hour US role/date rules, robots override, and 40-job
persisted-run cap were not changed.

**Checks run:**

- `npm run build:sidecar` — packaged macOS worker built successfully.
- `cargo test -p scrape-worker --test crawl_process --test adapter_tests` —
  26 tests passed. Crawl fixtures proved navigation-specific and source-specific
  timeout warnings, `TimeLimit`, prior posting retention and no later request.
- `cargo test -p scrape --test worker_repository` — 29 tests passed against the
  packaged worker and temporary databases. The exact saved test YAML was
  previewed with a transient six-page cap and max 10 preview jobs; full-run
  configuration remained 50 pages/source and its existing test persisted 40
  eligible fixture jobs with strict date/US evidence.
- `npx vitest run src/test/workflow_preview_results.test.tsx src/test/scan_diagnostics_interaction.test.tsx`
  — 2 files, 4 tests passed.
- `npx tsc --noEmit` — passed.
- `npm run build` — passed; Vite transformed 789 modules. Existing large-chunk
  advisory remains.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `node scripts/check-style.mjs`, and `git diff --check` — passed.
- `cargo build -p scrape` — passed.

**Native limit:** `SCRAPE_NATIVE_WORKFLOW=1 npm run test:native-startup` did not
reach Preview. The isolated WKWebView opened **Create workflow**, then timed out
after 21.3 seconds waiting for the indexed O*NET Software Engineer suggestion.
Startup logged `visible=true focused=false`; subsequent WebView diagnostics
reported `visible` and `focused` true, with the modal still open. No native
Preview surface/result is claimed. Artifacts:
`/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-r2QqFx`.
The separate `SCRAPE_NATIVE_WORKFLOW=1 node scripts/native-branding.mjs` run
passed automatic logo branding only; it does not verify workflow Preview.

**Limits:** all worker data came from loopback fixtures and isolated temporary
SQLite. No live provider, installed app profile, or user's database was used.
Zero current matches remain a valid result when no posting has evidenced
publication within 24 hours and US location.
