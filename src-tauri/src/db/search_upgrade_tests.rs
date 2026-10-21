//! Populated v28 → v29 search-kind migration, backup and recovery contract.
use super::browser_upgrade_tests::snapshot;
use super::*;

async fn assert_preserved(pool: &SqlitePool, before: &[(String, String, Vec<String>)]) {
    for (table, projection, expected) in before {
        let condition = if table == "_sqlx_migrations" {
            " WHERE version <= 28"
        } else {
            ""
        };
        let actual: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT {projection} FROM \"{table}\"{condition} ORDER BY rowid"
        ))
        .fetch_all(pool)
        .await
        .unwrap();
        assert_eq!(&actual, expected, "changed historical table {table}");
    }
}

async fn assert_integrity(pool: &SqlitePool) {
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_all(pool)
            .await
            .unwrap(),
        vec!["ok".to_owned()]
    );
}

#[tokio::test]
async fn populated_v28_search_upgrade_preserves_rows_backup_failure_retry_and_restart() {
    for inject_failure in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("search-upgrade.db");
        let original = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let mut historical = sqlx::migrate!("./migrations");
        historical.migrations = historical
            .iter()
            .filter(|migration| migration.version <= 28)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&original).await.unwrap();
        sqlx::raw_sql(
            "INSERT INTO workflows(id,name,created_at,enabled,cadence,next_run_at) VALUES ('workflow','Protected hybrid workflow','2026-01-01',1,'daily','2030-01-01T09:00:00Z');
             INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config,filters,cadence,cadence_cron,next_run_at,script_trusted,script_sha256,order_index) VALUES
             ('board','workflow','greenhouse','Protected board','fixture','2026-01-01','{\"fixture\":true}','{}','daily',NULL,'2030-01-01T09:00:00Z',1,'protected-board-sha',0),
             ('workday','workflow','workday','Protected Workday','https://fixture.invalid/wday/cxs/tenant/board/jobs','2026-01-01','{\"tenant\":\"fixture\",\"site\":\"board\"}','{}','cron','0 9 * * *','2030-01-01T09:00:00Z',1,'protected-workday-sha',1);
             INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,external_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,company_branding_key) VALUES
             ('job-board','fixture:1','board','workflow','1','Engineer','Fixture','https://fixture.invalid/jobs/1','2026-01-01','2026-01-02','interviewing','2026-01-02','Keep board note',1,'brand'),
             ('job-workday','fixture:2','workday','workflow','2','Line Cook','Other employer','https://fixture.invalid/jobs/2','2026-01-01','2026-01-02','applied','2026-01-02','Keep Workday note',1,NULL);
             INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES ('job-board','interviewing','2026-01-02'),('job-workday','applied','2026-01-02');
             INSERT INTO scrape_runs(id,trigger,status,started_at,finished_at,jobs_found,jobs_new,source_id,workflow_id) VALUES
             ('run-board','single_source','success','2026-01-01','2026-01-01',1,1,'board','workflow'),
             ('run-workday','single_source','success','2026-01-02','2026-01-02',1,1,'workday','workflow');
             INSERT INTO scrape_logs(id,run_id,timestamp,level,source_id,message) VALUES ('log-board','run-board','2026-01-01','info','board','Protected board history'),('log-workday','run-workday','2026-01-02','info','workday','Protected Workday history');
             INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES ('blocked','Protected blocked posting','Fixture','2026-01-01');
             INSERT INTO employer_branding(key,source_id,company,logo_json,last_attempt_json,asset_reference,asset_data,updated_at) VALUES ('brand','board','Fixture','{}','{}','cached-asset','fixture-bytes','2026-01-01');
             INSERT INTO browser_sessions(id,label,domain,storage_relpath,status,created_at,updated_at) VALUES ('session','Protected login','fixture.invalid','sessions/protected','ready','2026-01-01','2026-01-02');",
        )
        .execute(&original)
        .await
        .unwrap();
        upgrade_workflow_settings_v4(&original).await.unwrap();
        upgrade_workflow_settings_v5(&original).await.unwrap();
        postings::backfill(&original).await.unwrap();
        compensation::backfill(&original).await.unwrap();
        assert_integrity(&original).await;
        let before = snapshot(&original).await;
        if inject_failure {
            let mut failing = sqlx::migrate!("./migrations");
            failing.migrations =
                failing
                    .iter()
                    .map(|migration| {
                        if migration.version == 29 {
                            assert!(migration.sql.contains("DROP TABLE sources;"));
                            sqlx::migrate::Migration::new(
                            29,
                            migration.description.clone(),
                            migration.migration_type,
                            migration.sql.replace(
                                "DROP TABLE sources;",
                                "DROP TABLE sources; SELECT * FROM injected_missing_table_29;",
                            ).into(),
                            false,
                        )
                        } else {
                            migration.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .into();
            assert!(failing.run(&original).await.is_err());
            assert_eq!(snapshot(&original).await, before);
            assert_integrity(&original).await;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM _sqlx_migrations WHERE version = 29"
                )
                .fetch_one(&original)
                .await
                .unwrap(),
                0
            );
        }
        let upgraded = init_db(&path).await.unwrap();
        assert_preserved(&upgraded, &before).await;
        assert_integrity(&upgraded).await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM _sqlx_migrations WHERE version = 29 AND success = 1"
            )
            .fetch_one(&upgraded)
            .await
            .unwrap(),
            1
        );
        let backups: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")
            })
            .map(|entry| entry.path())
            .collect();
        assert_eq!(backups.len(), 1);
        let backup = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&backups[0])
                    .read_only(true),
            )
            .await
            .unwrap();
        assert_eq!(snapshot(&backup).await, before);
        assert_integrity(&backup).await;
        backup.close().await;
        sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES ('search','workflow','search','Cross-Company Search','arbeitnow','2026-01-03','{\"max_pages\":5}')")
            .execute(&upgraded).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT kind FROM sources WHERE id='search'")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            "search"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
                .fetch_one(&upgraded)
                .await
                .unwrap(),
            1
        );
        let after = snapshot(&upgraded).await;
        original.close().await;
        upgraded.close().await;
        let restarted = init_db(&path).await.unwrap();
        assert_eq!(snapshot(&restarted).await, after);
        assert_integrity(&restarted).await;
        restarted.close().await;
        let restarted_backups: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration")
            })
            .map(|entry| entry.path())
            .collect();
        assert_eq!(restarted_backups, backups);
    }
}
