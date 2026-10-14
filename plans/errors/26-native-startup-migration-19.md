# Incident 26 — desktop startup refuses migration 19 (2026-10-14)

User-visible error:

```text
Scrape+could+not+start
Validation+error:+Migration+history+mismatch+at+version+19.+No+checksums+were+changed.
```

The remaining message directs preservation of the database/WAL and links to
`plans/database-recovery.md`. No panic or reset was reported.

## Research and confirmed cause

Read-only inspection of the affected installation found successful migration 19
with SHA-384 `46b957723ddf2e3742c025811cfbadeaf39e6f0496fd2636757c4efb22049e2bd717ddf1f850fbe0e4321fedd826b666`.
The jobs schema contains `description_source` and `description_json`, but lacks
`description_format`. The checksum exactly matches the first two statements of
migration 19. The committed three-column file has SHA-384
`c63b3f7604683b974990f71ede7ee76ea57cc6b3364e05b79a18f7bc210c76be669f303f911efe320a19d9f6d922090d`.
Thus the installation applied an intermediate two-column migration before the
file gained the third statement. Git contains only the committed variant; the
stored checksum and exact schema verify the earlier SQL independently.

The plus signs are a second defect: startup used form-urlencoding for a data URL.
Form encoding represents spaces as `+`; a data URL needs percent-encoded spaces.
See [MDN data URLs](https://developer.mozilla.org/en-US/docs/Web/URI/Reference/Schemes/data).

## Repair

Retain the committed migration 19 unchanged and archive the exact earlier SQL in
`src-tauri/historical-migrations`. Select it only by its known checksum, then
validate the full successful history and independently reconstructed schema using
the existing startup guard. Migration 20 adds `description_format` on that history;
the committed three-column history uses an immutable no-op migration 20. Both
converge without changing old checksums. Unknown histories/schema drift still stop.
The recovery data URL now encodes spaces as `%20` and literal plus signs as `%2B`.

## Verification and existing installation

The new populated version-19 regression first failed with the reported error.
After repair it passes for all combinations of recognized version-12/version-19
histories, retaining saved notes, favorites, status history, settings, source data,
description evidence and all prior migration records. Unknown version-19 checksums
and recognized checksums with schema drift are refused without file mutation or
new backups. A unit regression covers recovery URL spaces, literal plus and Unicode.

Applied the reviewed helper to the authorized existing installation after recording
protected-state fingerprints. It created consistent backup
`scrape_data.db.pre-migration-2f3f55eb-9e4f-4cd9-b591-9c704b3c9db1.db` beside the database.
It reported migration 20 and integrity `ok`. Fingerprints of every original column
across 13 tables, including migration records through version 19, were unchanged.
These are recovery checks, not tests against the live database. Required tests use
temporary databases.

Actual Tauri dev startup with a temporary early-version-19 database passed. The
startup harness now makes its isolated window visible, allowing WKWebView page
loading and the injected IPC completion check to run. Full native startup matrix
results are recorded in the implementation evidence after completion.
