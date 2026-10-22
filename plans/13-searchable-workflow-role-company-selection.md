# Searchable workflow role and company selection

Status: the role/company selector interaction work is delivered, but native workflow acceptance was blocked by an unfocused/hidden WKWebView. The role-title data and source pipeline described here are superseded by Plan 14 Section 9; the remaining UI and company behavior contracts remain relevant. See the [evidence ledger](13-implementation-evidence.md) and [Plan 14 role-title evidence](14-implementation-evidence.md).

## Contract

Replace role, company and canned keyword suggestion-button layouts with literal searchable selectors while preserving WorkflowSpec 8, worker protocol 9, catalog version 2, the Rust matcher and preview → save → explicit-run lifecycle.

- Bundle BLS 2018's full 23-group/867-detailed-occupation structure and O*NET 31.0 technology alternate titles. Keep original files, hashes, retrieval date, license and transformation attribution. Catalog titles are suggestions for literal substring filters, not semantic occupation expansion or global coverage.
- Support trimmed custom role/company terms and multi-selection. Company values remain literal employer filters. Existing company templates are the only board directory; no inferred URLs or external lookup.
- Selecting a supported company appends its board in the same draft update, deduplicates by normalized endpoint, preserves existing source identity/configuration, and leaves unavailable/setup/review/script sources disabled. Removing a filter never removes a source. Imported filters do not silently attach boards; the Sources step provides an explicit attach action.
- Preserve the single-source company replacement picker as a separate API and behavior.
- Replace canned keyword suggestions with contextual completions drawn from current workflow role terms and the other keyword groups. Selecting a completion modifies only its current group; ordinary phrase, IME, paste and synchronous flush behavior remains.
- Searching is draft-neutral. Selection changes invalidate preview. Save never launches scraping. Custom and imported values survive YAML and save/reopen.

## Sources and scope

The original BLS/O*NET catalog approach in this plan is retired. Plan 14 Section 9 replaces it with a complete O*NET 31.0 normalized TSV and indexed runtime SQLite search. Current source URLs, hashes, license and attribution are in [`crates/scrape-core/data/occupations/manifest.json`](../crates/scrape-core/data/occupations/manifest.json).

Company selection constraints remain: no live database, network autocomplete, cross-workflow suggestion history, arbitrary company directory or fuzzy company-to-board resolution.

## Verification requirements

Plan 13's catalog checks are historical and no longer runnable because its XLSX/static catalog pipeline was retired. Use Plan 14 Section 9 for current role-title normalization, SQLite seeding/search, migration recovery and native acceptance. The company selector checks and outstanding native UI gates remain as recorded in the [evidence ledger](13-implementation-evidence.md).
