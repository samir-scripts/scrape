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
