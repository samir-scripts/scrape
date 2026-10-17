# Incident 27 — desktop startup refuses migration 23 (2026-10-16)

User-visible error:

```text
STARTUP_FAILED: Validation error: Migration history mismatch at version 23. No checksums were changed. Close the application, preserve the database and its WAL files, and restore a compatible pre-migration backup or use a reviewed forward repair. See plans/database-recovery.md.
```

No panic, abort, or database reset occurred; the read-only preflight guard in `src-tauri/src/db/mod.rs` refused writes and preserved the database and its WAL files untouched.

## Research and confirmed cause

Read-only inspection of an isolated copy of `~/Library/Application Support/dev.scrape.app/scrape_data.db` and its active write-ahead log (`scrape_data.db-wal`, 374,952 bytes) established:

1. **Uncheckpointed WAL state**: The main `.db` file header was last checkpointed at migration 21, while migrations 22 (`execution outcomes`) and 23 (`employer branding`) were committed in `scrape_data.db-wal`.
2. **Intermediate six-column migration 23 applied before commit**:
   - Migration 23 was applied to the installation at `2026-10-16 18:20:36 UTC` with SHA-384 checksum `90753B5B2EFD184E3AB2E818FCE7D8E874353BE16BEAD3B0CF767DB72FFD3B84C5E3830360F410CD150706B58357F368`.
   - At `2026-10-16 18:55:48 UTC` (commit `1e6a04f`), `src-tauri/migrations/0023_employer_branding.sql` was committed with a seventh column (`asset_data TEXT,`), changing its SHA-384 checksum to `6CB9797FFBCDD231EDA0AC42C41752D8B2F55502E054E461516785D7AF80EFFE4F9C510118E749CB00EFDE24FC768F92`.
   - Removing `    asset_data TEXT,\n` from the committed file reproduces the exact stored SHA-384 (`90753B5B...`) and the installation's six-column `employer_branding` table definition (`key`, `source_id`, `company`, `logo_json`, `last_attempt_json`, `asset_reference`, `updated_at`).

| Migration 23 | SHA-384 |
|---|---|
| Stored/applied six-column historical bytes | `90753B5B2EFD184E3AB2E818FCE7D8E874353BE16BEAD3B0CF767DB72FFD3B84C5E3830360F410CD150706B58357F368` |
| Committed seven-column file (`1e6a04f`) | `6CB9797FFBCDD231EDA0AC42C41752D8B2F55502E054E461516785D7AF80EFFE4F9C510118E749CB00EFDE24FC768F92` |

## Why existing tests missed this

1. `scripts/test-native-startup.mjs` only synthesized historical databases up to version 12 (`legacy`, `current`, `unknown`) and version 19 (`legacy19`, `current19`, `unknown19`), and always read `0023_employer_branding.sql` from `src-tauri/migrations/` during upgrade. It lacked `legacy23`, `current23`, and `unknown23` startup modes.
2. `src-tauri/src/db/history.rs` and `src-tauri/src/db/mod.rs` only tested combinations of version-12 and version-19 historical manifests (`12 → 18` and `19 → 20`), with no test covering the six-column version-23 variant.

## Forward repair

1. Retain the committed `src-tauri/migrations/0023_employer_branding.sql` and `0024_multi_source_workflows.sql` unchanged.
2. Archive the exact six-column SQL in `src-tauri/historical-migrations/0023_employer_branding.sql`.
3. Select the historical manifest in `src-tauri/src/db/history.rs` only when version 23 matches the known SHA-384 checksum, and continue verifying the full migration prefix and independently reconstructed `:memory:` schema in `checked_manifest`.
4. Add forward convergence migration 25:
   - `src-tauri/historical-migrations/0025_finalize_employer_branding.sql` adds `ALTER TABLE employer_branding ADD COLUMN asset_data TEXT;` and backfills any inline `logo_json.asset` into `asset_data` while stripping it from `logo_json`.
   - `src-tauri/migrations/0025_finalize_employer_branding.sql` runs `SELECT 1;` as a shared checkpoint for the seven-column history.

## Verification and existing installation

- `db::tests::migration_twenty_three_histories_preserve_saved_state_and_metadata` first failed with the exact reported `Migration history mismatch at version 23` error, and now passes across all 8 combinations of `(legacy_twelve, legacy_nineteen, legacy_twenty_three)` (`2 × 2 × 2`), preserving all prior checksums `<= 23`, saved jobs, notes, favorites, status history, settings, and cached branding assets.
- `db::tests::migration_twenty_three_unknown_or_drift_is_rejected_without_writes` verifies that unknown version-23 checksums and schema drift are refused without file mutation or backup creation.
- `SCRAPE_STARTUP_MODES=fresh,legacy23,current23,unknown23 npm run test:native-startup` passed in isolated WKWebView/Tauri instances:
  - `fresh`: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-cVD7ZO`
  - `legacy23`: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-4k8qxn`
  - `current23`: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-W4C9hL`
  - `unknown23`: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-Lsca5W`
- Upgraded the authorized existing installation to version 25 (`REPAIR_OK migration_version=25 integrity=ok`). Consistent pre-migration snapshot `scrape_data.db.pre-migration-955efef2-348f-4cd4-845d-5ff2c5765ba0.db` (at version 23) was created beside `scrape_data.db`. All 23 prior migration records, saved jobs, status events, and workflow sources matched identically between the version-23 backup and the upgraded version-25 database, and a repeated `repair_database` invocation succeeded idempotently without creating a duplicate backup.
