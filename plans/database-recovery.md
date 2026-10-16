# Database recovery

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
