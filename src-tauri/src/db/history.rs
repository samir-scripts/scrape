//! Immutable manifests for verified migration histories. Never relabel metadata.
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
    let original = manifest
        .iter()
        .find(|migration| migration.version == 19)
        .unwrap();
    let early_description = Migration::new(
        19,
        original.description.clone(),
        original.migration_type,
        include_str!("../../historical-migrations/0019_structured_job_descriptions.sql").into(),
        false,
    );
    if applied.iter().any(|(version, checksum, _)| {
        *version == 19 && checksum.as_slice() == early_description.checksum.as_ref()
    }) {
        manifest.migrations = manifest
            .iter()
            .map(|migration| match migration.version {
                19 => early_description.clone(),
                20 => Migration::new(
                    20,
                    migration.description.clone(),
                    migration.migration_type,
                    include_str!(
                        "../../historical-migrations/0020_finalize_structured_descriptions.sql"
                    )
                    .into(),
                    false,
                ),
                _ => migration.clone(),
            })
            .collect::<Vec<_>>()
            .into();
    }
    let original = manifest
        .iter()
        .find(|migration| migration.version == 23)
        .unwrap();
    let early_branding = Migration::new(
        23,
        original.description.clone(),
        original.migration_type,
        include_str!("../../historical-migrations/0023_employer_branding.sql").into(),
        false,
    );
    if applied.iter().any(|(version, checksum, _)| {
        *version == 23 && checksum.as_slice() == early_branding.checksum.as_ref()
    }) {
        manifest.migrations = manifest
            .iter()
            .map(|migration| match migration.version {
                23 => early_branding.clone(),
                25 => Migration::new(
                    25,
                    migration.description.clone(),
                    migration.migration_type,
                    include_str!("../../historical-migrations/0025_finalize_employer_branding.sql")
                        .into(),
                    false,
                ),
                _ => migration.clone(),
            })
            .collect::<Vec<_>>()
            .into();
    }
    let role_variants = [
        include_str!("../../historical-migrations/0030_role_titles.sql"),
        include_str!("../../historical-migrations/0030_role_titles_role_first.sql"),
    ];
    let originals = manifest
        .iter()
        .find(|migration| migration.version == 30)
        .unwrap();
    let role_migrations = role_variants.map(|sql| {
        Migration::new(
            30,
            originals.description.clone(),
            originals.migration_type,
            sql.into(),
            false,
        )
    });
    if let Some((_, checksum, _)) = applied.iter().find(|entry| entry.0 == 30) {
        if let Some(variant) = role_migrations
            .iter()
            .find(|migration| migration.checksum.as_ref() == checksum.as_slice())
        {
            manifest.migrations = manifest
                .iter()
                .map(|migration| {
                    if migration.version == 30 {
                        variant.clone()
                    } else {
                        migration.clone()
                    }
                })
                .collect::<Vec<_>>()
                .into();
        }
    }
    // The caller validates the entire history and independently reconstructed
    // schema against this manifest before permitting writes.
    manifest
}
