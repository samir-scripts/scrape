# Searchable workflow role and company selection

Status: implementation delivered; native workflow acceptance is blocked by an unfocused/hidden WKWebView. See the [evidence ledger](13-implementation-evidence.md); do not treat Plan 13 as fully verified until that gate passes.

## Contract

Replace role, company and canned keyword suggestion-button layouts with literal searchable selectors while preserving WorkflowSpec 8, worker protocol 9, catalog version 2, the Rust matcher and preview → save → explicit-run lifecycle.

- Bundle BLS 2018's full 23-group/867-detailed-occupation structure and O*NET 31.0 technology alternate titles. Keep original files, hashes, retrieval date, license and transformation attribution. Catalog titles are suggestions for literal substring filters, not semantic occupation expansion or global coverage.
- Support trimmed custom role/company terms and multi-selection. Company values remain literal employer filters. Existing company templates are the only board directory; no inferred URLs or external lookup.
- Selecting a supported company appends its board in the same draft update, deduplicates by normalized endpoint, preserves existing source identity/configuration, and leaves unavailable/setup/review/script sources disabled. Removing a filter never removes a source. Imported filters do not silently attach boards; the Sources step provides an explicit attach action.
- Preserve the single-source company replacement picker as a separate API and behavior.
- Replace canned keyword suggestions with contextual completions drawn from current workflow role terms and the other keyword groups. Selecting a completion modifies only its current group; ordinary phrase, IME, paste and synchronous flush behavior remains.
- Searching is draft-neutral. Selection changes invalidate preview. Save never launches scraping. Custom and imported values survive YAML and save/reopen.

## Sources and scope

BLS SOC 2018: https://www.bls.gov/soc/2018/ . O*NET database 31.0: https://www.onetcenter.org/database.html . O*NET-derived titles are credited under CC BY 4.0; exact stored sources, version, retrieval metadata, hashes and license details are in [`crates/scrape-core/data/occupations/manifest.json`](../crates/scrape-core/data/occupations/manifest.json). The catalog is generated offline by [`scripts/import-occupation-catalog.py`](../scripts/import-occupation-catalog.py) and loaded by Rust; generated TypeScript is updated only by `npm run gen:types`.

No migration, IPC endpoint, matcher behavior, workflow version or worker protocol change is in scope. No live database, network autocomplete, cross-workflow suggestion history, arbitrary company directory or fuzzy company-to-board resolution is introduced.

## Verification requirements

Run the checks recorded in the [evidence ledger](13-implementation-evidence.md): catalog import/check and isolated importer tests; Rust catalog tests and generated-binding drift; full relevant Vitest suite; TypeScript/build/style checks; Chromium journeys at both themes and actual browser zoom; actual UI inspection for selector states, responsive widths, focus, contrast and overflow; isolated WKWebView workflow composition/save/preview/explicit-run against local fixtures and temporary SQLite. Physical pointer, native file chooser, physical IME, native zoom, and live-provider checks remain separate and must be reported as unperformed unless actually exercised.
