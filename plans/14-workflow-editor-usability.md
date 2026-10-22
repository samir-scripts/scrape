# Workflow editor usability and selection repair

Status: Section 1 and Section 4 are implemented and evidence-recorded. Criteria/time refinements are in place, but Sections 2, 3 and 7 still have open acceptance gates; Section 2 is next. Execute **one section per implementation pass**. A pass ends only when its acceptance gate and evidence are complete. Do not start the next section in the same pass. Keep each section's changes reviewable and update this plan/evidence before proceeding.

## Contract and boundaries

- Preserve the single workflow draft, Rust-owned validation/matching/scheduling, explicit Preview → Save → Run lifecycle, and existing YAML/import compatibility. Do not change protocol, WorkflowSpec, or schema unless a section demonstrates that the existing contract cannot satisfy the behavior; then document and test the migration/version path before editing.
- Never open or modify the user's live database, run live provider queries, invent provider coverage, or ship an XLSX. Use deterministic fixtures, temporary databases, and actual user-entered terms.
- Keep Simple and Advanced as presentations of the same draft. Simple must not silently discard any saved/advanced field. User-provided values remain usable even when no suggestion exists.
- The role-title request is for selectable search suggestions, not automatic occupation matching or filter broadening. Keep suggestion ranking/provenance separate from Rust's literal matcher.
- At each section gate: run its focused interaction test and a real UI smoke for the changed behavior; then record exact commands/results, limits, and screenshots when relevant. Only proceed after the gate passes. Finish with the full integration gate in Section 12.

## Section 1 — Rename and simplify workflow navigation

1. Rename the visible **SCRAPER** tab and its user-facing navigation affordances to **WORKFLOWS**; preserve internal view IDs and IPC names unless a user-facing label leaks through.
2. Put an accessible Simple/Advanced segmented control at the top of Create/Edit. It changes presentation only and keeps one draft.
3. Widen the editor within viewport bounds; retain a scrollable body, visible actions, and narrow-screen fit.
4. Make **Roles → Criteria → Sources → Schedule → Review** five peer navigation steps. Move readable summary, readiness and preview results into Review; put Preview and Save there.
5. Replace promotional/explanatory filler with short task-specific hints. Keep errors and active hidden Advanced values visible.

**Gate:** real browser interaction proves navigation, backtracking, switching modes without losing edits, Review/preview/save gating, and usable editor bounds at 480/1024/1440 CSS px. Update nav, editor, and affected test/docs call sites. Do not change workflow execution semantics.

## Section 2 — Theme-correct controls and selection feedback

1. Inventory native checkboxes/radios/toggles in the workflow editor and reuse existing themed primitives/tokens; do not add a parallel control system.
2. Replace browser-default checkbox rendering where it breaks Paper/Obsidian themes; preserve keyboard, focus-visible, disabled, indeterminate, and label behavior.
3. Make selected choices visibly distinct and state their selected value in accessible names/semantics. In particular, show whether compensation is Annual or Hourly at all times; this selector now uses theme-token selected styling and `aria-pressed`, so Section 2 should verify it in both themes while covering remaining controls.

**Gate:** control interactions in both themes; screenshot and contrast inspection; keyboard and focus checks. No hard-coded component colors.

## Section 3 — Range-based compensation and experience criteria

1. Inspect the existing Rust filter schema, its units, validation and current range control before changing presentation. Compensation is a minimum/maximum amount range with an explicit annual/hourly unit; do not infer or convert missing bounds. Switching units must retain the established contract and clearly communicate any cleared values.
2. Keep required experience as precise inclusive integer minimum/maximum inputs; either endpoint can be blank and therefore unbounded. This is already a numeric range. Do not substitute a slider: MDN recommends `<input type="range">` only when the exact value is unimportant, and range inputs require bounded endpoints ([MDN](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/input/range)). Preserve unknown-experience behavior and advertised-range overlap semantics.
3. Keep numeric boundary validation authoritative in Rust and expose actionable inline errors.

**Gate:** boundary cases (empty, min-only, max-only, reversed/equal, unit switch, unknown values) through the real controls, preview and persisted workflow; no matcher changes without separate approval.

## Section 4 — Country, state and city/location autocomplete

1. Trace the existing generated geography catalog, normalization, input/commit events, and matching contract. Reproduce the reported typed-country and typed-state misses before editing.
2. Repair suggestion filtering/commit for valid country and subdivision names/codes, including case/space variations. Keep unknown free-text location terms available where currently supported.
3. Cities are not a complete built-in global directory. Provide useful local suggestions only from an existing trustworthy dataset or current workflow context; otherwise treat the typed city/location as a literal term, never claim a comprehensive city list or require a suggestion to continue.
4. Preserve multi-country/imported rules until explicit edits and do not broaden matching.

**Gate:** exact fixture-backed keyboard/pointer tests for a country, state/province, code, city term, no-match text, clearing/changing country, paste and Escape. Verify the outgoing Rust rules and preview.

## Section 5 — Replace flavor copy with concise hints

1. Review Roles, Criteria, Sources and Schedule descriptions against the action directly below each one.
2. Convert filler into short field-adjacent hints that explain format, scope, defaults, units, consequences or bounds only when needed. Do not repeat the same help in section header, field, and Review.
3. Keep coverage limitations, unknown-value behavior, safety/trust requirements and validation errors truthful and visible; hints must not imply unsupported scraping coverage.

**Gate:** content review at narrow and wide layouts plus accessible-name/label checks. Existing behavior tests must test controls and outcomes, not pin incidental copy.

## Section 6 — Sources defaults and government-source research

1. In Simple mode, do not require a source choice. Audit the supported-source catalog and default to every safe built-in cross-employer feed available to this app; include all enabled direct boards already configured for the workflow. Do not imply this searches every employer or ATS when source configuration is required, and never silently mutate imported/saved sources. Make the exact default coverage legible.
2. Research official U.S. government jobs access (including USAJOBS API terms, credentials, rate limits, geography, data provenance, freshness and testability) and comparable public government feeds before proposing an adapter. Do not add a named source based on a brand alone.
3. If a supported source is feasible, implement it through the existing source/extractor contracts, disclose its coverage and setup requirements, and keep it opt-in where credentials are required. If not feasible, record the evidence and leave it explicitly unsupported rather than faking a source.
4. Keep source identity, disabled/script review states, per-source trust, ownership and per-run budget intact.

**Gate:** deterministic local-fixture workflow preview → save → explicit run verifies every enabled configured source is examined within its budget and that failures/partial coverage are truthful. Government source is covered only if a real supported adapter is implemented.

## Section 7 — Reuse the existing themed clock control

1. Locate the existing Clock React component and its consumers. The workflow Schedule field now uses `ClockPicker`; verify its theme, keyboard, locale and 12/24-hour contracts before changing anything further.
2. Preserve the existing hookup, adapting only if the gate exposes a gap. Retain IANA timezone semantics and the same canonical `HH:MM` stored value.
3. Do not implement a second time picker or alter scheduler interpretation.

**Gate:** set/read/save/reopen a time through the actual control in Paper and Obsidian, with keyboard operation and timezone unchanged.

## Section 8 — Make Review a usable decision surface

1. Organize Review into compact, scannable sections for readiness, target roles, criteria, companies, sources, schedule and retention.
2. Give Preview a clear action and truthful results; distinguish examined, matched, rejected, invalid, blocked and partial outcomes using existing canonical diagnostics.
3. Keep full YAML and exhaustive rules as optional disclosure. Never present a stale preview as current; edits invalidate it. Save remains disabled until a current successful preview where the existing contract requires one.
4. Keep explicit Save distinct from Run now; saving never launches the worker.

**Gate:** keyboard and screen-reader-readable Review, stale-preview invalidation, disabled/error/partial states, and a real preview-save-run persistence journey against temporary storage.

## Section 9 — Evaluate and improve selectable role-title data

1. Audit the current bundled BLS/O*NET role catalog, import script, licenses/provenance, query path, payload size and role autocomplete before adding data. The repository already has a generated offline catalog; do not create a second source of truth or a redundant SQLite table by default.
2. Compare the requested BLS `Occupation Data`, `Sample of Reported Titles`, and `Alternate Titles` inputs against actual gaps in selectable titles. Use official versioned files and retain source/version/hash/license attribution. Exclude generic `All Other` titles and deduplicate case-insensitively while retaining canonical SOC mapping and source tier.
3. Prefer extending the current generated catalog/resource if it meets startup and query requirements. Add first-launch SQLite seeding only if repository evidence shows the existing query architecture requires a database table; if needed, use an immutable forward migration, one explicit transaction, prepared inserts, an index, idempotent nonempty check, and populated-upgrade/rollback/retry tests. Measure real cold and warm paths rather than assuming <300ms/0ms.
4. Search suggestions must allow adding a custom title and remain literal filters. Do not use the title catalog to expand jobs or alter matcher behavior. No XLSX in source, generated resources, installer or runtime.

**Gate:** deterministic catalog importer/checks, license/provenance audit, catalog size/query/startup measurement, and actual role search/selection/custom-title/preview/save flow with unchanged frontend contract. Record why a runtime table is or is not justified.

## Section 10 — Repair company autocomplete without claiming a complete directory

1. Reproduce the current dropdown filtering/truncation bug and trace its source: templates, configured workflow companies, or custom values. Fix result clipping, focus/scroll, typed-query preservation, and keyboard selection at the actual list boundary.
2. Define what the directory covers (verified career-board templates vs. employer-name suggestions). Evaluate a broader licensed, maintainable company dataset before promising complete Fortune/local-company coverage; ensure local and unlisted employers remain selectable as literal custom filters.
3. Preserve custom literal company filters such as “Dollar General” even when no career board template exists. Selecting a known template may attach its verified board atomically; unknown/custom values must not infer URLs or create fake sources.
4. Do not claim exhaustive company coverage. If a larger offline directory is justified, document provenance, licensing, size, update and staleness policy first; do not download a live directory or ship a spreadsheet.

**Gate:** tests include a known configured company, Dollar General as an unmatched custom filter, a company with a verified board, duplicate selection, long results/scroll, keyboard operation, and persisted literal matching. Source attachments remain exact and separately removable.

## Section 11 — Add restrained color while preserving the visual system

1. Review Paper and Obsidian workflow surfaces against existing semantic theme tokens and visual references.
2. Add color only through existing semantic tokens (e.g. focus, success, warning, stage accents), for selected states and meaningful status hierarchy. Keep flat surfaces, square geometry, sparse accents and contrast requirements.
3. Do not import the unrelated retro reference palette, hard-code colors, or make color the only indicator.

**Gate:** reviewed screenshots in both themes, status/selection non-color cues, automated contrast check, and no horizontal overflow at the required viewport/zoom sizes.

## Section 12 — End-to-end integration and native acceptance

1. Trace the configured workflow from edit through validation, preview, save/reopen, explicit run, worker output, SQLite persistence and Discovered results. Fix the demonstrated broken contract at its owner; do not paper over errors in UI.
2. Run focused and relevant full frontend/Rust checks, type/binding drift and style/build checks after sidecar rebuild as applicable. Exercise deterministic fixture workflows in a fresh temporary database; never use live application data.
3. Verify actual Chromium journeys in both themes, keyboard, narrow/wide viewports and browser zoom. Then run isolated native Tauri/WKWebView acceptance for editor pointer/focus/file chooser/time control and preview/save/run persistence. Report physical pointer/IME, native file chooser, native 200% zoom and live-provider checks separately if not exercised.
4. Update README/directory READMEs, the applicable plan status, evidence ledger and plan index. Do not mark the plan complete while a required acceptance gate remains blocked.

**Completion gate:** all supported user paths pass, unsupported providers/coverage remain explicit, the clean workflow persists and runs end-to-end, and every performed/unperformed check is accurately recorded.
