//! Immutable manifests for two verified migration-12 histories. Never relabel metadata.
use sqlx::migrate::{Migration, Migrator};

pub(super) type Applied = (i64, Vec<u8>, bool);

pub(super) fn manifest(applied: &[Applied]) -> Migrator {
    let mut manifest = sqlx::migrate!("./migrations");
    let original = manifest
        .iter()
        .find(|migration| migration.version == 12)
        .unwrap();
    let legacy = Migration::new(
        12,
        original.description.clone(),
        original.migration_type,
        include_str!("../../historical-migrations/0012_posting_removal.sql").into(),
        false,
    );
    if applied.iter().any(|(version, checksum, _)| {
        *version == 12 && checksum.as_slice() == legacy.checksum.as_ref()
    }) {
        manifest.migrations = manifest
            .iter()
            .map(|migration| match migration.version {
                12 => legacy.clone(),
                18 => Migration::new(
                    18,
                    migration.description.clone(),
                    migration.migration_type,
                    include_str!("../../historical-migrations/0018_finalize_posting_removal.sql")
                        .into(),
                    false,
                ),
                _ => migration.clone(),
            })
            .collect::<Vec<_>>()
            .into();
    }
    // The caller validates the entire history and independently reconstructed
    // schema against this manifest before permitting writes.
    manifest
}
