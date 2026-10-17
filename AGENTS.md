# Project instructions

Scrape is a local-first Tauri v2 desktop job scraper and application tracker: React 19 + TypeScript WebView → Tauri/Rust host with SQLite (SQLx, WAL) → an isolated Rust worker sidecar that speaks versioned NDJSON over stdin/stdout. Prior implementers: GPT-6 Astra, Sol, and Gemini 4 Argon. This file is the onboarding entry point for the next implementer (Gemini 3.8 Flash / GPT-6.1 Sol).

## Current state and next task

- **Active revision:** [Reliable ingestion, multi-source workflows, and UI repair](plans/11-reliable-ingestion-and-ui-repair.md), with evidence in [plan 11 evidence](plans/11-implementation-evidence.md). Sections 1 (matching diagnostics), 2 (keyword chips), 3 (bounded Greenhouse enrichment/outcomes, migration 22, protocol v5), 4 (employer branding, migration 23/25, protocol v6), and 5 (multi-source workflow ownership and fair budgeting, migration 24, WorkflowSpec v5) are implemented and verified. The branch is `implementation/trustworthy-scraping`, Section 5 evidence is at `d424cd0`, and Incident 27 (migration 23→25 forward repair) is at `7c1fd52`.
- **Next: Section 6D, bounded stdin/NDJSON script runner.** Follow the step-by-step execution blueprint in [plans/11-section-6-browser-and-script-extraction.md](plans/11-section-6-browser-and-script-extraction.md) (Phases 6A–6F: Tier 1 single-pass CSS + XPath 1.0 over `html5ever`, Tier 2 direct Rust CDP over WebSocket with resource blocking & stealth, and Tier 3 bounded stdin/NDJSON script runner). After Section 6, do Sections 7–8 one at a time.
- **Sections 6A–6C implemented:** shared static CSS/XPath fields, attributes/regexes, explicit hydration JSON, resumable pages, bounded detail enrichment and readiness checks. Protocol v7 adds page/login termination and extraction bounds as a prerequisite pulled forward from 6B. Phase 6B adds migration 26 with populated recovery tests, WorkflowSpec v6 (v1–v5 compatibility), browser/extractor-test boundary contracts and explicit local script trust with content fingerprints/import quarantine. Phase 6C adds the direct Rust CDP browser engine (`max_processes = 1`, `max_contexts = 2`, immediate tab/context teardown, pre-navigation resource/tracker blocking, pre-document stealth injection, `wait_for`/`load_more`/`next_page` traversal, domain-scoped session cookie injection, and `NeedsLogin` detection) plus isolated host browser session lifecycle commands (`Open`/`Save`/`Clear`) and `test_extractor`. Continue 6D–6F. Script execution and Workflow Editor extraction/session/trust controls remain unimplemented; configured script mode fails explicitly before subprocess launch. See the evidence ledger for checks and limitations.
- **Still open from earlier plans:** reproducing the original 438/436/2 report, which is impossible without the missing screenshots and configuration, so do not claim it. Also open: plan 08 Section 8 native acceptance (physical pointer, file chooser, native visual/zoom QA) and plan 10's final native verification.
- **Plan precedence:** plan 11 > plan 10 > plans 09/08 > plan 07 > plan 06 > older plans. A later plan supersedes earlier ones only where it says so. The domain, data-protection, identity, provenance, migration, and retention contracts from [Trustworthy scraping](plans/06-trustworthy-scraping.md), [Interface simplification](plans/07-interface-simplification.md), and [Bounded ingestion](plans/09-bounded-ingestion-and-data-management.md) stay authoritative. Read [Workflow v3](plans/workflow-v3.md) and [database recovery](plans/database-recovery.md) before touching specs or migrations. Visual references: [NieR menu](plans/assets/ref-nier-menu.png), [dither](plans/assets/ref-dither.png), [palette](plans/assets/ref-palette.png).
- Keep directory READMEs synchronized with the code and newest plans whenever behavior or files change; trust code and the newest plans over older narrative notes.

## Project rules

1. Protect the working tree. Inspect changes before editing; never discard unrelated work or use destructive reset/clean as routine recovery.
2. Commit early and often. Checkpoint existing work first; commit each coherent behavior with relevant tests. Separate mechanical formatting.
3. Keep commits reviewable. Stage explicit files, inspect staged diffs, and record behavior and validation.
4. Prefer reverting individual commits. Database rollback requires a compatible backup or tested forward repair; Git revert alone is insufficient.
5. Never fabricate production data. Keep unknowns unknown and isolate visibly labeled demonstrations.
6. Preserve source identity and provenance of titles, URLs, IDs, dates, and compensation throughout the pipeline.
7. Apply DRY to domain rules: one authoritative implementation for salary conversion, identity, matching, scheduling, validation, and theme semantics.
8. Generate TypeScript boundary contracts from Rust. Never hand-edit generated bindings or create competing interfaces.
9. Fail explicitly with actionable errors for invalid filters/configuration, malformed worker output, and persistence failures. Never broaden matches silently.
10. Make schema changes forward-only. Never rewrite applied migrations or stored checksums. Test populated historical upgrades.
11. Preserve notes, favorites, tracking state, and history during rescraping, retention, source deletion, and migration.
12. Follow the linked visual references: square geometry, flat surfaces, thin rules, restrained accents, sparse dithering, shared primitives and semantic tokens.
13. Make destructive actions clear with recognizable SVG icons, readable danger colors, accessible names, and specified reversible behavior.
14. Test observable behavior through controls, commands, worker processes, and persistence boundaries. Static markup/constants do not prove end-to-end behavior.
15. Keep tests deterministic with local HTTP fixtures, temporary databases, controlled clocks, and isolated storage. Never test against the user's live database.
16. Never weaken checks to get green results. Replace misleading tests with tests proving the intended contract.
17. Validate before declaring completion. Report only checks actually run; distinguish fixtures and unperformed native QA from live verification.
18. Keep documentation truthful and update affected specifications/setup with implementation. Mark superseded plans clearly.

## Directory map

| Path | Purpose |
|---|---|
| `crates/scrape-core/` | Shared Rust domain library and the **single owner of domain rules**: `matching.rs` (keyword/geography/workplace predicates, categorized rejection reasons, partial-evidence decisions), `filter.rs` (SQL filter compilation), `identity.rs` (canonical posting identity), `locator.rs`/`xpath.rs` (shared static locator grammar/evaluation), `hydration.rs`/`static_source.rs` (hydration maps and bounded static configuration), `extraction.rs` (mode/script contracts, fingerprints and shared bounds), `browser.rs` (local browser runtime discovery, isolated session/cookie storage, login-wall detection, and RFC 6455 CDP WebSocket client), `enrich/` (salary, location, workplace, eligibility, experience, HTML/description normalization), `posting_date.rs`, `geography.rs` + `data/regions.json`, `branding.rs` (logo evidence/validation), `protocol.rs` (worker NDJSON protocol, `PROTOCOL_VERSION` = 7), `types.rs`. `examples/cache-logo-fixture.rs` reproduces branding review PNGs. |
| `crates/scrape-worker/` | Sidecar binary that runs crawls. `client.rs` (rate-limited HTTP, robots, URL safety), `browser/` (direct Rust CDP browser engine, resource blocking, pre-document stealth, runtime discovery), `coordinator.rs` (multi-source round-robin coordinator and fair budget manager), `detect.rs` (ATS detection), `filter.rs`, `adapters/` (Greenhouse, Lever, Ashby, JSON-LD, CSS/XPath/browser selector, shared branding). `tests/` has process-level tests (`worker_process`, `limit_process`, `branding_process`, `browser_cdp_process`, `coordinator_tests`, `adapter_tests`) against loopback HTTP/WebSocket and recorded `fixtures/`. |
| `src-tauri/` | Tauri host. `src/commands/` holds IPC endpoints. `src/db/` holds SQLite repositories (`jobs.rs` is the largest, plus postings, runs, sources, branding, cleanup, history, settings, metrics). `src/services/` holds `workflow.rs` (authoritative WorkflowSpec validation/versioning), `script_trust.rs` (local script trust/import quarantine and session readiness validation), `browser_session.rs` (isolated browser session Open/Save/Clear lifecycle and runtime detection), `runner.rs` (spawns the worker, validates output, persists), `ingestion.rs`, `scheduler.rs`, `calendar.rs`, `retention.rs`, `process.rs` (process-tree reaping), `startup.rs` (backup + migrations), `tray.rs`, and the debug-only `native_smoke.rs`. `migrations/` holds immutable forward-only SQL (0001–0026). `historical-migrations/` holds original SQL kept for upgrade tests (12/18, 19/20, 23/25). `tests/worker_repository.rs` covers the real worker → temporary SQLite path. `examples/repair_database.rs`, `capabilities/` (ACL), `binaries/` (built sidecar, git-ignored), `gen/` (generated schemas), `icons/`. |
| `src/` | React frontend. `bindings/` is **generated by ts-rs; never edit**. `components/ui/` holds shared primitives (Button, IconButton, Icon, Modal, Select, KeywordInput, CompanyLogo, Tag, Toast, Tooltip, Dither, …). `components/layout/` holds the shell, nav, split panes, and schedule control. `components/filters/` holds the Filters & sort panel. `components/jobs/` holds the shared JobInspector, description, and paginator. `components/views/{discovered,tracked,scraper,metrics,settings}/` holds the screens; workflow editing lives in `settings/` (AddSourceModal, WorkflowSections, KeywordMatching, GeographyMatching, ScanDiagnostics). `lib/` holds the IPC client `api.ts` (including explicit `?demo=1` demonstration mode), Zustand `store.ts`, `companyLogoCache.ts`, YAML/import helpers, and formatters. `hooks/` holds `useWorkflowDraft` (the single draft controller), hotkeys, and status changes. `styles/index.css` holds **all semantic theme tokens**. `test/` holds Vitest + Testing Library suites. |
| `e2e/` | Playwright (Chromium) journeys `*.pw.ts`, with fixtures (`fixtures.ts`, `jobFixtures.ts`), the `contrast.ts` helper, the zoom extension, and reviewed macOS screenshot baselines (`*-snapshots/`). |
| `scripts/` | Node automation: `build-sidecar.mjs`, `check-style.mjs` (bans rounded/shadow utilities), `test-app-startup.mjs`, `test-native-startup.mjs` and `native-*.mjs` (isolated real Tauri/WKWebView harnesses sharing `tauri-dev-test.mjs`), `provider-verification.mjs` (recorded and `--live` provider checks), and `cargo-target-dir.mjs`. |
| `fixtures/providers/` | Recorded public provider captures with `manifest.json` provenance (URL/time/checksum). Never edit titles or URLs independently. |
| `workflows/` | Sample YAML workflow specs. |
| `plans/` | Specifications and evidence ledgers (`NN-*.md` plans, `NN-implementation-evidence.md`). `assets/` holds visual references and reviewed screenshots per section. `errors/` holds troubleshooting playbooks. |
| `public/fonts/` | Bundled licensed fonts for offline use. |
| `.github/` | CI (`ci.yml`: Linux checks and macOS native/visual jobs), Dependabot, PR template. |
| `.cargo/config.toml` | Sets `TS_RS_EXPORT_DIR=src/bindings`. |
| `target/`, `dist/`, `node_modules/`, `test-results/` | Build and test output. Do not commit these. |

## UX/UI practices

- **Visual language:** NieR-inspired brutalism with square corners (no `rounded-*`/`shadow-*` other than `-none`; `scripts/check-style.mjs` enforces this), flat surfaces, 1px rules, restrained accents, and dithering only as sparse decoration, never behind text or as a dialog scrim. Keep one border per meaningful surface.
- **Tokens only:** use the semantic CSS variables in `src/styles/index.css` (`--color-surface`, `--color-text`, `--color-danger`, `--color-stage-*`, spacing `--spacing-page/panel/section/control`, `--control-height(-sm/-lg)`). Paper (light) and Obsidian (dark) themes must both work. Palette: Paper #F5F5DC, cream #FFF6EE, ink #1A1A1A, lavender #C6C2F2, lime #D6DC82, cherry #D86072. Do not hard-code colors in components.
- **Typography:** one global `--font-ui` drives every surface, including portals, dialogs, and toasts. `--font-code` is only for YAML/code. Body text is 14–16px, metadata at least 12px, card titles 16px, detail titles 20–24px. Uppercase is reserved for short labels and actions.
- **Spacing and targets:** use a 4/8/12/16/24/32px scale. Standard controls are 36px. Icon targets are at least 32px no matter the SVG size.
- **Shared primitives first:** extend `src/components/ui` before writing new controls. Use the Headless UI–based themed Select/Combobox/ActionMenu, not native selects, and `Icon` SVGs with `IconButton` (accessible name, `aria-hidden` icon). Use one shared JobInspector for discovered and tracked jobs, and the one `CompanyLogo` component (24px list/card, 32px inspector, light backing, initials fallback).
- **Destructive actions:** use the trash SVG, the danger token, an accessible name such as "Remove [title] at [company]", and an explicit confirmation dialog. Optional blocking starts unchecked. Undo and restoration are required. Removal is soft.
- **Workflow editor:** use one continuous Source → Matching → Schedule form backed by one draft. Advanced settings and YAML share that draft. Any edit invalidates the preview. Save requires a current successful preview and never scrapes; Run now is explicit. Use progressive disclosure, but summarize active hidden settings and never hide errors in a collapsed section.
- **Truthful UI:** unknown stays unknown. Label page-local counts (e.g., "0 loaded"), keep provider totals separate from examined counts, and visibly label demonstration/recorded data (`?demo=1`). Never present browser demo results as live scraping.
- **Accessibility:** follow WAI dialog patterns (focus trap, Escape, focus restore; docked inspectors are nonmodal). Provide keyboard equivalents for pointer interactions (e.g., Kanban Move to), respect reduced motion, and require text contrast of 4.5:1 and control/focus contrast of 3:1 against the actual rendered background. Layouts must work at 480px, 1024px, 1440px, and 200% zoom without page-wide horizontal overflow (200% zoom overflow is a known open item for plan 11 Section 8).

## Development practices

- **Section loop:** for each section, do research (trace all callers and routes) → write a failing regression → implement → run targeted verification → write evidence → commit. Check `git status` first, checkpoint unrelated work, and stage explicit files. Use conventional messages (`feat:`, `fix:`, `docs:`, `style:`), and commit generated or formatting-only changes separately.
- **Evidence:** append a section entry to `plans/11-implementation-evidence.md` with the commit IDs, the exact commands run, the results and totals from *this* run (never copied), reviewed screenshot paths, and the explicit limits (no live, native, or physical checks). Update the plan status line, `plans/README.md`, and the root `README.md` when behavior changes.
- **Domain ownership:** Rust is authoritative for matching, identity, salary, geography, schedules, workflow validation, limits, and accounting. The frontend must not reimplement a matcher, parser, or scheduler. It consumes canonical values only.
- **Boundary contracts:** add `#[derive(TS)]` in Rust, run `npm run gen:types` (`cargo test --workspace export_bindings`), and commit `src/bindings` changes. Drift is checked with `git diff --exit-code src/bindings`.
- **Versioning:** if the worker protocol changes, bump `PROTOCOL_VERSION` and reject older requests. WorkflowSpec changes need a new version, with older versions converted by the single Rust resolver and errors that include paths. Invalid originals stay preserved as Needs review, and matches are never broadened.
- **Migrations:** add a new numbered file in `src-tauri/migrations/` and never edit applied SQL or checksums. Startup takes a `VACUUM INTO` backup before migrating. Test populated upgrades from the previous version (preserving IDs, notes, favorites, status/history, suppression, schedules, runs, and checksums), rollback after an injected failure, retry, and idempotent restart. Document the new migration in `plans/database-recovery.md`.
- **Process safety:** the worker uses structured argv, bounded and redacted stderr, cancellation, and process-tree reaping. The host revalidates all worker output before persistence. Malformed output fails explicitly but keeps valid partial results.
- **Build hygiene:** rebuild the sidecar (`npm run build:sidecar`) after worker or core changes before host, integration, or native tests. Resolve the Cargo target directory via `scripts/cargo-target-dir.mjs` and `CARGO_TARGET_DIR`. Do not run parallel builds against one target. Stop only processes you started.
- **Do not implement features outside the active section.** Do not add frameworks, redesign unrelated areas, or touch the user's live database or application data.

## Testing practices

- **Layers:** Rust unit/property tests in crates; worker process tests over loopback HTTP; `src-tauri/tests/worker_repository.rs` for the actual packaged worker → temporary SQLite path; Vitest interaction tests that drive real controls (Testing Library, `user-event`); Playwright journeys for both themes, widths, zoom, contrast, keyboard use, and screenshots; native harnesses for real Tauri IPC, WKWebView, and the sidecar in isolated temp profiles; and recorded provider replay. Live provider checks are opt-in and reported separately; CI never uses them.
- **Rules:** use local HTTP fixtures, temporary databases, controlled clocks, and labelled fictional or recorded data with provenance. Assert exact request counts where bounds matter. Do not compute expected values with the code under test, and keep separate negative cases for parity tests. Never delete, skip, or loosen assertions, raise timeouts to get green, or regenerate screenshot baselines without visually reviewing them. Store new review captures under `plans/assets/11-<topic>-review/`.
- **Commands** (record which ones you actually ran):

```bash
npm ci
npm run build:sidecar                         # required before host/integration/native tests
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                        # needs local socket binding for fixtures
npm run gen:types && git diff --exit-code src/bindings
npx tsc --noEmit
npm test                                      # vitest + Vite startup smoke; use `npx vitest run --maxWorkers=1` if concurrent runs time out
npm run build
node scripts/check-style.mjs
git diff --check
npm run test:providers                        # recorded HTTP → actual worker
PLAYWRIGHT_BROWSERS_PATH=/tmp/scrape-playwright-browsers npx playwright test e2e/<journey>.pw.ts
cargo build -p scrape && npm run test:native  # isolated native IPC/sidecar smoke
SCRAPE_STARTUP_MODES=fresh,current19,unknown19 npm run test:native-startup
# Feature harnesses: SCRAPE_NATIVE_WORKFLOW|SCHEDULE|KANBAN|LAYOUT|APPEARANCE|DATA_MANAGEMENT=1, node scripts/native-branding.mjs
```

- **Known quirks:** the sandbox may block fixture socket binding, so rerun with local-server access rather than changing tests. Concurrent frontend runs can time out spuriously; rerun serially without changing timeouts. Native harness artifacts go to OS temp dirs; cite their paths in evidence. The production build has a known large-chunk advisory.
- **Reporting:** keep deterministic fixtures, browser QA, native QA, live checks, and unperformed checks (physical pointer/IME, native file chooser, native 200% zoom, live company verification) in separate categories. A required check that is blocked or was not performed prevents claiming completion.
