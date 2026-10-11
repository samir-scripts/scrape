use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::time::Duration;

pub mod jobs;
pub mod metrics;
pub mod notifications;
pub mod runs;
pub mod settings;
pub mod sources;

pub async fn init_db(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let migrator = sqlx::migrate!("./migrations");
    match migrator.run(pool).await {
        Ok(_) => Ok(()),
        Err(sqlx::migrate::MigrateError::VersionMismatch(v)) => {
            tracing::warn!(
                "Migration checksum mismatch detected for version {}. Repairing _sqlx_migrations...",
                v
            );
            if let Some(target_mig) = migrator.iter().find(|m| m.version == v) {
                let _ = sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
                    .bind(&*target_mig.checksum)
                    .bind(v)
                    .execute(pool)
                    .await;
                // Re-run migration execution
                migrator.run(pool).await?;
                Ok(())
            } else {
                Err(sqlx::migrate::MigrateError::VersionMismatch(v).into())
            }
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
pub async fn init_memory_db() -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    #[tokio::test]
    async fn test_all_migrations_apply_cleanly_up_to_version_7() {
        let pool = init_memory_db()
            .await
            .expect("Failed to initialize in-memory DB");

        let count_row =
            sqlx::query("SELECT COUNT(*) as cnt, MAX(version) as max_v FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await
                .expect("Failed to query _sqlx_migrations");

        let cnt: i64 = count_row.get("cnt");
        let max_v: i64 = count_row.get("max_v");

        assert_eq!(cnt, 7, "Expected exactly 7 migrations to be applied");
        assert_eq!(
            max_v, 7,
            "Expected migration version 7 to be highest version"
        );

        // Verify that app_notifications allows 'error'
        let insert_res = sqlx::query(
            "INSERT INTO app_notifications (id, kind, title, message, read, created_at) VALUES ('test-err-id', 'error', 'Error Title', 'Error Message', 0, '2026-10-11T00:00:00Z')"
        )
        .execute(&pool)
        .await;

        assert!(
            insert_res.is_ok(),
            "Inserting 'error' notification kind must succeed on migrated DB"
        );
    }

    #[tokio::test]
    async fn test_migration_recovery_on_version_mismatch() {
        let pool = init_memory_db()
            .await
            .expect("Failed to initialize in-memory DB");

        // Corrupt the checksum for migration version 6 to simulate a version mismatch
        let corrupt_checksum = vec![0xDE, 0xAD, 0xBE, 0xEF];
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 6")
            .bind(corrupt_checksum)
            .execute(&pool)
            .await
            .expect("Failed to corrupt checksum");

        // Verifying standard migrator throws VersionMismatch(6)
        let raw_migrator = sqlx::migrate!("./migrations");
        let err = raw_migrator
            .run(&pool)
            .await
            .expect_err("Raw migrator should have failed with VersionMismatch");
        match err {
            sqlx::migrate::MigrateError::VersionMismatch(v) => {
                assert_eq!(v, 6, "Expected version mismatch on version 6");
            }
            other => panic!("Expected VersionMismatch, got: {:?}", other),
        }

        // Now run our resilient run_migrations healing function
        let heal_res = run_migrations(&pool).await;
        assert!(
            heal_res.is_ok(),
            "run_migrations must heal the version mismatch and succeed: {:?}",
            heal_res
        );

        // Verify that subsequent migration execution succeeds directly
        let second_run = raw_migrator.run(&pool).await;
        assert!(
            second_run.is_ok(),
            "Subsequent migration run after healing must succeed"
        );
    }
}
