# Database recovery

## Browser/session and script trust upgrade (migration 26)

Migration 26 adds `browser_sessions` metadata (label, domain, relative storage
reference, status and verification times), plus `sources.script_trusted` (default
false) and `sources.script_sha256` (default unknown). It does not rebuild tables,
change the five source kinds, rewrite workflows, or place session credentials in
SQLite or exported YAML. Session material management follows in Phase 6C.

The normal read-only history check and consistent pre-upgrade snapshot apply.
Populated version-25 fixtures compare every historical table/column and migration
record before/after upgrade, including branding bytes, notes, favorites, tracking
history, suppression, runs, schedules and workflow ownership. Injected migration
failure rolls back both new schema objects and columns; retry and restart preserve
the original checksums. Restore the compatible pre-migration backup to roll back;
reverting code alone does not undo a database upgrade. No live database is used by
these tests.

Applied SQL and migration checksums are immutable. Startup refuses unknown, failed,
modified, or incomplete migration histories. Never UPDATE `_sqlx_migrations` to
make an error disappear. The old checksum-rewriting behavior is removed.

Before pending upgrades, startup uses SQLite `VACUUM INTO` to create a consistent
snapshot beside the database, named `<database>.pre-migration-<uuid>.db`. This
includes committed WAL data. A backup failure stops startup before migration.
Fresh empty databases need no snapshot. Preserve backups until the upgraded
application and user state have been verified. Backups contain personal data;
keep them with the same access protections as the original database.

If startup reports a mismatch:

1. Close all application processes. Preserve the database and any `-wal`/`-shm`
   files together before attempting recovery. Do not copy an open database file
   alone as a backup.
2. Identify the application version that produced the database. Retain the
   failing database for inspection. Restore a compatible pre-migration snapshot
   to a separate location and verify it with that version before replacing data.
3. With the application stopped, move the failing database and its sidecars
   together out of the active location. Restore the standalone snapshot as the
   active database; do not reuse the old WAL with it.
4. Check notes, favorites, tracked status, history, workflows, and blocked jobs.
   A Git revert alone does not reverse a schema migration.

No automatic reconciliation is currently offered for databases whose checksums
were previously rewritten. Their actual schema needs review and a tested forward
repair. Startup compares the actual schema against an independently built historical
migration prefix, so rewritten checksums cannot hide a structural mismatch.
Unknown states are rejected without automatic reconciliation.

Migration 0006 already matches the restored historical file in commit 9998f5a;
its follow-up 0007 remains separate. This implementation does not rewrite either.
Migration 0011 converts legacy retention policy `all` to `untracked_only`.
Both accepted runtime policy names now preserve applied/interviewing/offer/rejected
jobs, favorites, and nonblank notes. Retention does not touch tombstones.

Migration 0012 adds persistent posting identities, soft removal, block metadata,
and a mapping for every historical suppression key. Backfill uses the shared Rust
identity implementation. Unresolved keys remain blocked as “Legacy posting” with
unknown destination/details; they are never dropped. Unblocking clears their
historical keys as well as the new identity block so startup cannot recreate them.
Soft removal retains notes, favorites, status events and source-name snapshots.
Automatic retention excludes removed records. Restore clears suppression and
restores saved copies atomically; ordinary removal permits later rediscovery.

Migration 0013 stores native compensation ranges separately from historical
annual analytics. Backfill first uses explicit unit evidence where available;
otherwise it reverses documented backend annualization only when both recorded
interval and extraction source are known, and marks the range derived. Values
without sufficient provenance remain recoverable in the legacy fields and do
not match strict native-unit filters. Native hourly amounts retain cents.

Migration 0017 repairs only recognized Greenhouse records last seen before the
0011 upgrade. The older adapter stored provider edit time as posting time; that
value is retained in `provider_updated_at` and the false posting date is cleared.
Refreshed publication evidence remains intact. Populated version-10 upgrade tests
verify the backup, four tracked statuses, notes, favorites, history, workflow
filters, custom settings, native-hourly recovery and historical suppression.

Changing a workflow board does not transfer saved user state to an unrelated
posting with the same provider ID. Repository ingestion selects the shared
posting identity and retains separate saved copies when the board changes.

## Verified migration-12 repair (2026-10-13)

Two immutable migration manifests are supported: the committed migration 12 and
the exact earlier variant documented in [incident 25](errors/25-native-startup-migration-12.md).
Selection requires the known checksum, full successful history prefix and exact
independently reconstructed schema. Neither history is relabeled. Forward migration
18 fills the historical schema's missing source snapshot column and legacy-block
table/index; both manifests retain their own immutable 18 SQL and converge to the
same supported fields. All prior migration records remain unchanged.

Startup creates a consistent pre-upgrade snapshot before applying the forward
repair. Failure rolls back that migration; restart can safely resume. Backfill
retains source snapshots and legacy suppression mappings. Unknown checksums or
schema variants still refuse migration. Native initialization failures now display
recovery information or exit cleanly instead of unwinding through macOS callbacks.

The authorized existing installation was upgraded to version 18 with integrity
`ok` and independently compared protected-state fingerprints. Notes, favorites,
tracking/removal state, history, workflows, settings, prior blocks, tombstones and
all previous checksums were unchanged. The pre-repair snapshot is
`scrape_data.db.pre-migration-0b579102-05e0-4581-8e4a-79c430503c59.db` beside the database.
A second open succeeded without another migration/backup.

For an explicitly authorized manual upgrade after preserving recovery options:

```bash
cargo run -p scrape --example repair_database -- /absolute/path/to/existing/database.db
```

This helper runs migrations/backfills and integrity checks without starting
schedulers or changing application defaults. It refuses nonexistent paths.
Required regression tests use synthetic temporary databases, never this live file.

## Verified migration-19 repair (2026-10-14)

[Incident 26](errors/26-native-startup-migration-19.md) identifies an exact earlier
migration 19 that added only `description_source` and `description_json`. The
committed version also adds `description_format`. Both SQL histories remain
immutable. A known checksum plus the complete successful history and exact
reconstructed schema selects the appropriate manifest. Migration 20 adds the
missing format column for the earlier history and is a no-op for the three-column
history. Unknown variants remain rejected; no old checksum is rewritten.

The affected installation was backed up consistently and upgraded to migration 20.
All original data across 13 tables and migration records through 19 were unchanged;
integrity was `ok`. Retain backup
`scrape_data.db.pre-migration-2f3f55eb-9e4f-4cd9-b591-9c704b3c9db1.db` beside the database.
The same repair helper documented above is safe to run again: recognized migrated
histories have no pending schema upgrade. Rebuild/relaunch the updated desktop
application so its embedded migration manifest includes this repair.

## Settings bulk cleanup recovery

Settings → Data management offers explicit hard deletion, separate from reversible
single-posting removal. Every bulk action first creates a consistent SQLite
snapshot in `backups/pre-cleanup-<uuid>.db` beside the active database. The success
message gives its absolute path. A backup failure aborts deletion. Backups can
contain private notes and remain after full reset; reset does not delete recovery
files or change migration history.

To restore a cleanup backup:

1. Quit Scrape completely, including its tray/menu-bar process. Do not restore
   while a worker or another SQLite connection is open.
2. Preserve the current database **and its `-wal`/`-shm` files** elsewhere if you
   want to retain changes made since cleanup.
3. Replace `scrape_data.db` with the selected complete `pre-cleanup-*.db` snapshot.
   Move aside any old `scrape_data.db-wal` and `scrape_data.db-shm` together; never
   combine a restored main database with WAL files from a different database.
4. Start the compatible app version. Review automatic scheduling before resuming
   work: the snapshot contains the settings from before cleanup.

Restoration replaces current data; it is not a record-level Undo or merge. Jobs-only
cleanup preserves workflows, notifications, run logs, settings (except the chosen
pause), and all suppression entries. Favorite/tracked preservation keeps **all saved
copies of each qualifying posting identity**, including removed copies and their
notes/history/locations. Full reset clears app database tables and frontend
preferences, disables launch at login, clears the diagnostic log, and leaves
scraping paused. OS/log-cleanup failures are shown as warnings after database
success. Schema, migration metadata and recovery backups remain. A reset marker
prevents the sample workflow from being silently recreated on restart.


## Execution outcome upgrade (migration 22)

Migration 22 adds `scrape_runs.outcome` and `sources.last_outcome`. It does not
rewrite existing CHECK constraints, statuses, records or migration checksums.
Historical outcomes remain NULL; reads fall back to their original status.
New warnings and cancellation use the explicit outcome alongside a compatible
coarse stored status. Scheduling, source/job identities and protected user state
remain unchanged.

The normal recognized-history pre-migration snapshot applies. Populated version-21
upgrade tests cover notes, favorites, tracking/history, suppression, run/log records,
schedules, unchanged applied checksums, restart, injected transactional migration
failure and successful retry. Restore the consistent pre-migration snapshot with a
compatible app to roll back; reverting application code alone does not roll back
schema or changes made since the snapshot. Never edit stored checksums.


## Employer branding upgrade (migration 23)

Migration 23 adds a branding registry and nullable job relationship references.
Existing image URLs, job/source IDs, annotations, tracking/history and scheduling
remain intact; historical evidence is not fabricated. Verified bytes are stored
once per source/employer relationship, separately from metadata and last-attempt
diagnostics. A failed lookup never overwrites a verified asset. Source deletion
keeps artwork referenced by saved jobs. Full reset deletes the registry as well.

The recognized-history snapshot applies before upgrade. Populated version-22
upgrade/restart and injected transaction-failure tests verify preserved state,
unchanged old checksums, rollback of both new schema objects, and safe retry.
Restore a consistent compatible backup to roll back; Git revert alone is not a
database rollback. Never edit applied SQL/checksums.


## Multi-source workflow ownership upgrade (migration 24)

Migration 24 introduces multi-source workflow ownership. It creates the parent
`workflows` table and adds `workflow_id` foreign keys to `sources`, `jobs`, and
`scrape_runs`. Backfill converts existing `sources` records into parent workflow
records while preserving original source/job IDs, user state (notes, favorites,
application stages, status history), suppression, run logs, and schedules. Invalid
originals stay preserved with `needs_review = 1`. Deletion of a single child source
stops ingestion from that endpoint while preserving all saved jobs, notes, history,
and retention ownership at the parent workflow level.

The normal pre-migration snapshot is created before migration. Populated version-23
upgrade tests verify preserved user state, parent/child foreign key backfill,
retention pruning by workflow, unchanged previous checksums, rollback after an
injected transaction failure, and idempotent restart. To roll back, restore the
consistent pre-migration snapshot with a compatible application build; Git revert
alone is not a database rollback. Never edit applied SQL or checksums.


## Verified migration-23 repair (2026-10-16)

[Incident 27](errors/27-native-startup-migration-23.md) identifies an earlier
six-column migration 23 (`employer_branding` without `asset_data`) applied before the
committed seven-column migration 23 gained `asset_data TEXT`. Both SQL histories
remain immutable. A known checksum plus the complete successful history and exact
independently reconstructed schema selects the appropriate manifest. Forward
migration 25 (`0025_finalize_employer_branding.sql`) adds `asset_data TEXT` (and
backfills any inline `logo_json.asset` into `asset_data`) for the six-column
history and is a no-op checkpoint (`SELECT 1;`) for the seven-column history.
Unknown variants and schema drift remain rejected; no old checksum is rewritten.

The affected installation was backed up consistently and upgraded to migration 25
with integrity `ok`. All original records and migration checksums through version 23
were unchanged. Retain backup
`scrape_data.db.pre-migration-955efef2-348f-4cd4-845d-5ff2c5765ba0.db` beside the
database.

## Workflow-owned source endpoints (migration 27)

Migration `0027_workflow_source_endpoints.sql` replaces the global endpoint
uniqueness constraint with `UNIQUE(workflow_id, kind, identifier)`. Existing source
IDs, provenance, settings, trust fingerprints and job/run relationships are
preserved. A consistent pre-migration backup is created by normal startup before
applying pending migrations. No earlier SQL or checksum is changed.

The replacement uses a new table, copies all rows, drops the old table, and
renames the new table. SQLite's existing `ON DELETE SET NULL` actions are kept
enabled; transaction-local temporary maps restore the two affected source-link
columns before commit. See SQLite's [table-replacement procedure](https://www.sqlite.org/lang_altertable.html#otheralter)
and [DROP TABLE foreign-key behavior](https://www.sqlite.org/foreignkeys.html#fk_schemacommands).
The populated v26 regression compares all historical rows and checks foreign keys,
backup contents, injected failure immediately after the old table is dropped,
rollback/retry, and idempotent restart.

An older executable alone cannot undo this migration. Restore the consistent
pre-migration database with a compatible build, or use a tested forward repair.

### Explicit ownership data repair

Migration 27 changes the schema; it does not guess how to rewrite existing
malformed workflow configurations. Normal workflow Save now materializes all
sources atomically. For an already saved multi-source workflow with missing source
rows, an explicit helper preserves its existing source ID, name, filters, schedule,
job state and history while creating missing owned children and canonicalizing the
embedded source IDs:

```sh
# Quit Scrape, including the tray process, before invoking against an existing DB.
cargo run -p scrape --example repair_workflow_ownership -- /absolute/path/to/database.db WORKFLOW_ID
```

The helper refuses nonexistent databases, uses normal checked migrations/backups,
creates a separate consistent `pre-ownership-repair-<uuid>.db` snapshot before data
repair, and aborts on backup failure or conflicting source ownership. A consistent
workflow is a no-op with no additional ownership backup. Script sources remain
untrusted and disabled; no extraction or scheduling starts. This helper has been
tested only on isolated fixtures during this revision; the user's live database
has not been repaired or opened.
## Workday ATS source kind (migration 28)

Migration `0028_workday_source_kind.sql` updates the `CHECK` constraint on `sources.kind`
to permit `'workday'` alongside `'greenhouse'`, `'lever'`, `'ashby'`, `'jsonld'`, and `'selector'`.
Existing source IDs, workflows, provenance, settings, trust fingerprints, and job/run relationships
are strictly preserved. A pre-migration backup is automatically taken by normal startup
before applying pending migrations.

The table replacement follows SQLite's standard table-recreation procedure: referencing links
in `jobs.source_id` and `scrape_runs.source_id` are saved in temporary mapping tables,
`sources_next_28` is populated with `SELECT * FROM sources`, `sources` is dropped,
the replacement is renamed to `sources`, the foreign-key links are restored, and indexes
(`idx_sources_cadence`, `idx_sources_workflow_id`) are recreated before commit.

## Enrichment version 4 and employer branding attachment repair

Startup reclassification repairs saved jobs with stale enrichment versions (`enrichment_version < 4`) in deterministic 200-row transactions with yields before automatic scheduling starts. A single `VACUUM INTO` backup named `<database>.pre-description-repair-<uuid>.db` is taken once per invocation if stale rows or unlinked branding relationships exist. If backup creation fails, startup halts before any data mutation. If a batch fails, its transaction rolls back and startup halts; previously committed batches remain intact and restart safely resumes.

Version-3 rows update only `experience_min`, `experience_max`, `experience_raw`, `is_new_grad`, and `experience_evidence` from the corrected classifier; title-only records (null descriptions) receive title-only classification without inventing numeric years. Pre-version-3 rows with descriptions run the historical description repair before classification. Known employment types, notes, favorites, tracked status, compensation, and schedules are strictly preserved.

Exact employer branding keys (`relationship_key(source_id, company)`) are linked to unlinked saved jobs (`company_branding_key IS NULL`) where an exact verified relationship already exists in `employer_branding`, without requiring a rescrape. Unlinked jobs from deleted sources or without exact employer records remain untouched; no fuzzy or cross-source employer merging is performed.
