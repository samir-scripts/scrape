# Project instructions

Implement [Trustworthy scraping](plans/06-trustworthy-scraping.md) and [Interface simplification](plans/07-interface-simplification.md). Visual references: [NieR menu](plans/assets/ref-nier-menu.png), [dither](plans/assets/ref-dither.png), [palette](plans/assets/ref-palette.png).

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
