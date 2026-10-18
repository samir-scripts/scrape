//! Populated v26 → v27 migration, backup, rollback and restart contract.
use super::browser_upgrade_tests::snapshot;
use super::*;
async fn unchanged(pool: &SqlitePool, before: &[(String, String, Vec<String>)]) {
    for (table, projection, rows) in before {
        let filter = if table == "_sqlx_migrations" {
            " WHERE version<=26"
        } else {
            ""
        };
        let actual: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT {projection} FROM \"{table}\"{filter} ORDER BY rowid"
        ))
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(&actual, rows, "changed protected table {table}");
    }
}
#[tokio::test]
async fn populated_v26_endpoint_upgrade_keeps_all_rows_links_backup_and_failure_recovery() {
    for inject in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ownership.db");
        let original = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let mut history = sqlx::migrate!("./migrations");
        history.migrations = history
            .iter()
            .filter(|m| m.version <= 26)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        history.run(&original).await.unwrap();
        sqlx::raw_sql("INSERT INTO workflows(id,name,created_at,enabled,cadence,next_run_at) VALUES ('parent','Protected', '2026-01-01',1,'daily','2030-01-01T09:00:00Z');
        INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,cadence,next_run_at,script_trusted,script_sha256) VALUES ('parent','parent','greenhouse','Protected','same-board','2026-01-01','daily','2030-01-01T09:00:00Z',1,'preserved-fingerprint');
        INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,external_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited) VALUES ('job','fixture:1','parent','parent','1','Authored protected job','Authored','https://fixture.invalid/job/1','2026-01-01','2026-01-01','interviewing','2026-01-01','Protected note',1);
        INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES ('job','interviewing','2026-01-01');
        INSERT INTO scrape_runs(id,trigger,status,started_at,source_id,workflow_id) VALUES ('run','single_source','success','2026-01-01','parent','parent');
        INSERT INTO scrape_logs(id,run_id,timestamp,level,source_id,message) VALUES ('log','run','2026-01-01','info','parent','Protected history');
        INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES ('blocked','Authored block','Authored','2026-01-01');
        INSERT INTO browser_sessions(id,label,domain,storage_relpath,status,created_at,updated_at) VALUES ('session','Protected','fixture.invalid','sessions/protected','ready','2026-01-01','2026-01-01');").execute(&original).await.unwrap();
        upgrade_workflow_settings_v4(&original).await.unwrap();
        upgrade_workflow_settings_v5(&original).await.unwrap();
        postings::backfill(&original).await.unwrap();
        compensation::backfill(&original).await.unwrap();
        let before = snapshot(&original).await;
        if inject {
            let mut failing = sqlx::migrate!("./migrations");
            failing.migrations = failing
                .iter()
                .map(|m| {
                    if m.version == 27 {
                        sqlx::migrate::Migration::new(
                            27,
                            m.description.clone(),
                            m.migration_type,
                            m.sql
                                .replace(
                                    "DROP TABLE sources;",
                                    "DROP TABLE sources; SELECT * FROM injected_missing_table_27;",
                                )
                                .into(),
                            false,
                        )
                    } else {
                        m.clone()
                    }
                })
                .collect::<Vec<_>>()
                .into();
            assert!(failing.run(&original).await.is_err());
            unchanged(&original, &before).await;
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                    .fetch_one(&original)
                    .await
                    .unwrap(),
                26
            );
        }
        let upgraded = init_db(&path).await.unwrap();
        unchanged(&upgraded, &before).await;
        assert_eq!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_all(&upgraded)
                .await
                .unwrap()
                .len(),
            0
        );
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")
            })
            .collect();
        assert_eq!(backups.len(), 1);
        let backup = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(backups[0].path())
                    .read_only(true),
            )
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&backup)
                .await
                .unwrap(),
            26
        );
        unchanged(&backup, &before).await;
        backup.close().await;
        upgraded.close().await;
        original.close().await;
        let reopened = init_db(&path).await.unwrap();
        unchanged(&reopened, &before).await;
        let other = sources::SourcesRepo::create(
            &reopened,
            sources::CreateSourceInput {
                kind: scrape_core::SourceKind::Greenhouse,
                name: "Other workflow".into(),
                identifier: "same-board".into(),
                config: None,
                enabled: Some(false),
                cadence: Some(scrape_core::SourceCadence::Manual),
                cadence_cron: None,
                retention_days: None,
                retention_policy: None,
                filters: None,
            },
        )
        .await
        .unwrap();
        assert_ne!(other.id, "parent");
        assert!(sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES ('duplicate','parent','greenhouse','Duplicate','same-board','2026-01-01')").execute(&reopened).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
                .fetch_one(&reopened)
                .await
                .unwrap(),
            1
        );
        reopened.close().await;
        assert_eq!(
            std::fs::read_dir(dir.path())
                .unwrap()
                .flatten()
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration"))
                .count(),
            1
        );
    }
}
