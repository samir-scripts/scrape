# Incident 25: native startup rejects the previously applied migration 12

Status: researched; repair proposed, not implemented. Reported 2026-10-13 after
completion claim `85a78b8`. Research performed on `implementation/trustworthy-scraping`.

## Reported error

```text
thread 'main' (29726800) panicked at src-tauri/src/lib.rs:57:22:
Failed to initialize database and core services: Validation("Migration history mismatch at version 12. No checksums were changed. Close the application, preserve the database and its WAL files, and restore a compatible pre-migration backup or use a reviewed forward repair. See plans/database-recovery.md.")
thread 'main' (29726800) panicked at .../core/src/panicking.rs:225:5:
panic in a function that cannot unwind
19: tao::platform_impl::platform::app_delegate::did_finish_launching
    .../tao-0.37.1/src/platform_impl/macos/app_delegate.rs:124:1
thread caused non-unwinding panic. aborting.
```

The address-heavy remainder of the supplied backtrace is omitted; the originating
application call and non-unwinding macOS delegate frame are retained above.

## Confirmed evidence

Read-only investigation used SQLite `mode=ro&immutable=1`, inspected migration
metadata/schema only, and did not run application tests against the live database.
Its WAL was zero bytes, so the immutable main-file view contained the committed
history. The main database SHA-256 remained identical before/after inspection.
No database, WAL, checksums or backups were modified.

The application database is `~/Library/Application Support/dev.scrape.app/scrape_data.db`.
It contains successful migrations 1–12; 1–11 match current SQL exactly. Version 12
was installed at `2026-10-13 14:55:37` (SQLite recorded timestamp).

| Migration 12 | SHA-384 |
|---|---|
| Stored/applied historical bytes | `A78E9F098DB1BA2F7B74AA5555B60274579E02ACC3B668BC2726E966FD1E07C0DB484ECD3B6A02D9AC493351BA7904B3` |
| Current file / commit `24288dd` | `9A3D64557AE7AE2C22EBB400F226F35D1D60C0874E62197C50E018AF2DC785C90622B35F067CD27F0F57D0CFB0C7FDB0` |

[Historical SQL evidence](fixtures/0012_posting_removal.applied.sql) was reconstructed
by removing the later `source_name_snapshot` addition and `posting_legacy_blocks`
table/index from the current file. Its hash matches the stored checksum exactly.
Applying migrations 1–11 plus those historical bytes in an independent in-memory
SQLite database yields an exact match for all 31 non-internal schema objects in
the application database. The historical database lacks:

- `jobs.source_name_snapshot`;
- `posting_legacy_blocks` and `idx_legacy_block_identity`.

Git records migration 12 first at `24288dd`, committed at 15:03:54 UTC, after the
recorded application timestamp. The precise process that applied the intermediate
file is not established, but the byte and schema matches establish which SQL ran.
This is an unsupported historical variant introduced during our implementation,
not evidence of an arbitrary damaged database.

Two pre-migration snapshots exist: one at version 10 and one at version 11. Neither
is the current version-12 state; replacing the active database with an older
snapshot could discard subsequent saved state. Restoration is not the default
repair recommendation.

## Failure chain and verification gap

1. `db::has_pending_migrations` correctly rejects the unrecognized version-12
   checksum during read-only preflight, before backups/writable pragmas/upgrades.
2. `lib.rs` uses `.expect()` inside native `.setup()`, converting the recoverable
   migration refusal into a panic during the Objective-C launch callback.
3. A panic through that macOS callback cannot unwind, so the process aborts.
4. `scripts/test-app-startup.mjs` creates Vite and calls `transformIndexHtml`.
   It does not start Tauri, open a database, or test native application readiness.
5. The native harness uses an isolated fresh database; populated upgrade tests use
   version 10 and current committed migration bytes. None tested these exact
   intermediate version-12 bytes. The reported passing suites were real, but the
   claim that the full upgrade work was complete was too broad.

Tauri 2.12.2's locally installed `app.rs` also panics if `.setup()` returns `Err`
during `RuntimeRunEvent::Ready`. Merely replacing `.expect()` with `?` is insufficient.
The repair must explicitly prevent the framework's setup-error panic path.

## Proposed repair sequence

1. **Capture regression fixtures first.** Use the exact historical SQL above to
   build a synthetic populated version-12 fixture with notes, all tracked states,
   favorites, status history, workflow settings, removals and suppression. Add the
   current version-12 history as a separate fixture. Reproduce the refusal and
   native abort using only temporary app directories.
2. **Support the two known histories without relabeling them.** Keep the current
   committed migration 12 immutable. Embed the exact historical variant outside
   the default migration directory. Select a historical manifest only when its
   known checksum, complete applied prefix, and independently rebuilt schema all
   match. Introduce forward migration 18 to finalize the missing posting-removal
   schema and backfill source snapshots/legacy block mappings. Use explicit,
   immutable migration-18 variants for the two histories if their different input
   schemas require different SQL; both must converge to the same logical schema.
   Preserve every previously stored migration checksum. Unknown states still fail.
   Do not simply restore/rewrite migration 12, which would break databases already
   using its committed checksum.
3. **Back up before repair.** Once the exact known history is recognized, create a
   consistent SQLite backup, then run the chosen forward migrations transactionally.
   Validate final schema, annotations and suppression. Exercise interrupted
   migration/backup failure and restart idempotency before touching live data.
4. **Handle startup failures without unwinding.** Move fallible initialization out
   of the native launch callback, or explicitly handle failure within setup,
   record an actionable recovery error, request controlled nonzero shutdown, and
   return success from the callback. Do not start scheduler/tray or expose normal
   commands with uninitialized state. Handle the outer `.run()` error as well.
   Verify the exact mechanism against Tauri's actual process lifecycle. Include a
   visible recovery message for GUI launches without a terminal.
5. **Verify actual development startup.** Extend isolated native verification to
   invoke the real `npm run tauri dev` path (sidecar build, Vite, native setup and
   readiness IPC), both fresh and historical databases. For unknown histories,
   require a useful error and controlled exit, no `SIGABRT`/panic, and byte-identical
   database metadata/data. Preserve Vite-only checking under an honest name.
   Add `cargo:rerun-if-changed=migrations` (and the embedded historical directory)
   so adding SQL files rebuilds the embedded manifest.
6. **Validate and document before applying to live data.** Run targeted migration
   and actual native-startup checks, then the required suites. Update recovery and
   completion documentation. Only then perform the backed-up live upgrade and
   confirm normal native readiness; never use the live database as a test fixture.

Acceptance: both known histories start successfully with saved state and previous
checksums preserved; unknown histories exit safely untouched; actual development
startup proves native readiness rather than just HTML transformation.

## References

- [SQLx 0.8.6 migration embedding and rebuild tracking](https://docs.rs/sqlx/0.8.6/sqlx/macro.migrate.html).
- [Tauri setup hook contract](https://docs.rs/tauri/latest/tauri/struct.Builder.html#method.setup);
  the installed 2.12.2 `app.rs` provides the definitive setup-error panic evidence.
- Local SQLx 0.8.6 `migration.rs` hashes SQL with SHA-384; `migrator.rs` rejects a
  differing applied checksum.
- [Database recovery](../database-recovery.md).
