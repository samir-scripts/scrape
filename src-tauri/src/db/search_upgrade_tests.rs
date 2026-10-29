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

#[tokio::test]
async fn populated_v32_search_endpoint_upgrade_preserves_data_and_recovers() {
    for inject_failure in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("search-endpoints.db");
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
            .filter(|migration| migration.version <= 32)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&original).await.unwrap();
        sqlx::raw_sql(
            "INSERT INTO workflows(id,name,created_at,enabled,cadence,next_run_at) VALUES
             ('workflow','Protected workflow','2026-01-01',1,'daily','2030-01-01'),
             ('other','Other workflow','2026-01-01',1,'manual',NULL);
             INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config,filters,cadence,next_run_at,script_trusted,script_sha256,order_index) VALUES
             ('search','workflow','search','Protected search','themuse','2026-01-01','{\"max_pages\":50}','{}','daily','2030-01-01',1,'protected-search-sha',0),
             ('board','workflow','greenhouse','Protected board','fixture','2026-01-01','{\"url\":\"fixture\"}','{}','daily','2030-01-01',1,'protected-board-sha',1),
             ('malformed','workflow','search','Legacy invalid search','arbeitnow','2026-01-01','not-json','{}','manual',NULL,0,NULL,2),
             ('other-search','other','search','Other search','themuse','2026-01-01','{}','{}','manual',NULL,0,NULL,0);
             INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,external_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,company_branding_key) VALUES
             ('job-search','search:1','search','workflow','1','Engineer','Fixture','https://fixture.invalid/jobs/1','2026-01-01','2026-01-02','interviewing','2026-01-02','Keep search note',1,'brand'),
             ('job-board','board:2','board','workflow','2','Engineer','Other','https://fixture.invalid/jobs/2','2026-01-01','2026-01-02','applied','2026-01-02','Keep board note',1,NULL);
             INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES ('job-search','interviewing','2026-01-02'),('job-board','applied','2026-01-02');
             INSERT INTO scrape_runs(id,trigger,status,started_at,finished_at,jobs_found,jobs_new,source_id,workflow_id) VALUES
             ('run-search','single_source','success','2026-01-01','2026-01-01',1,1,'search','workflow'),
             ('run-board','single_source','success','2026-01-01','2026-01-01',1,1,'board','workflow');
             INSERT INTO scrape_logs(id,run_id,timestamp,level,source_id,message) VALUES ('log','run-search','2026-01-01','info','search','Preserved history');
             INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES ('blocked','Protected blocked posting','Fixture','2026-01-01');
             INSERT INTO employer_branding(key,source_id,company,logo_json,last_attempt_json,asset_reference,asset_data,updated_at) VALUES ('brand','search','Fixture','{}','{}','asset','fixture-bytes','2026-01-01');
             INSERT INTO browser_sessions(id,label,domain,storage_relpath,status,created_at,updated_at) VALUES ('session','Protected login','fixture.invalid','sessions/protected','ready','2026-01-01','2026-01-02');
             INSERT INTO role_titles(id,soc_code,title,priority_tier) VALUES (1,'00-0000','Fixture role',1);
             INSERT INTO role_title_terms(term,role_title_id) VALUES ('fixture',1);",
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
                        if migration.version == 33 {
                            assert!(migration.sql.contains("DROP TABLE sources;"));
                            sqlx::migrate::Migration::new(
                            33,
                            migration.description.clone(),
                            migration.migration_type,
                            migration.sql.replace(
                                "DROP TABLE sources;",
                                "DROP TABLE sources; SELECT * FROM injected_missing_table_33;",
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
                    "SELECT count(*) FROM _sqlx_migrations WHERE version=33"
                )
                .fetch_one(&original)
                .await
                .unwrap(),
                0
            );
            assert!(sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES ('before-retry','workflow','search','Distinct feed','themuse','2026-01-03','{\"endpoint\":\"https://fixture.invalid/feed\"}')")
                .execute(&original).await.is_err(), "old uniqueness must remain after rollback");
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT count(*) FROM sqlite_master WHERE name IN ('sources_next_33','idx_sources_search_endpoint','idx_sources_non_search_endpoint')")
                    .fetch_one(&original).await.unwrap(),
                0
            );
        }

        let upgraded = init_db(&path).await.unwrap();
        for (table, projection, expected) in &before {
            let condition = if table == "_sqlx_migrations" {
                " WHERE version <= 32"
            } else {
                ""
            };
            let actual: Vec<String> = sqlx::query_scalar(&format!(
                "SELECT {projection} FROM \"{table}\"{condition} ORDER BY rowid"
            ))
            .fetch_all(&upgraded)
            .await
            .unwrap();
            assert_eq!(&actual, expected, "changed historical table {table}");
        }
        assert_integrity(&upgraded).await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM _sqlx_migrations WHERE version=33 AND success=1"
            )
            .fetch_one(&upgraded)
            .await
            .unwrap(),
            1
        );
        let backups: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("pre-migration")
            })
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

        // The unfiltered default and two explicit category feeds coexist.
        for (id, endpoint) in [
            (
                "computer",
                "https://www.themuse.com/api/public/jobs?page=0&category=Computer%20and%20IT",
            ),
            (
                "data",
                "https://www.themuse.com/api/public/jobs?page=0&category=Data%20Science",
            ),
        ] {
            sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES (?,'workflow','search',?,'themuse','2026-01-03',?)")
                .bind(id).bind(id).bind(serde_json::json!({"endpoint":endpoint}).to_string())
                .execute(&upgraded).await.unwrap();
        }
        for endpoint in [
            "https://www.themuse.com/api/public/jobs?page=0",
            "https://www.themuse.com/api/public/jobs?page=0&category=Computer%20and%20IT",
        ] {
            assert!(sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES ('duplicate','workflow','search','Duplicate','themuse','2026-01-03',?)")
                .bind(serde_json::json!({"endpoint":endpoint}).to_string())
                .execute(&upgraded).await.is_err());
        }
        // The same endpoint with different non-identity options is still a duplicate.
        assert!(sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES ('duplicate-options','workflow','search','Duplicate','themuse','2026-01-03',?)")
            .bind(serde_json::json!({"endpoint":"https://www.themuse.com/api/public/jobs?page=0&category=Computer%20and%20IT","max_pages":1}).to_string())
            .execute(&upgraded).await.is_err());
        for (provider, endpoint) in [
            ("arbeitnow", scrape_core::search_source::ARBEITNOW_ENDPOINT),
            ("adzuna", scrape_core::search_source::ADZUNA_ENDPOINT),
            ("jobicy", scrape_core::search_source::JOBICY_ENDPOINT),
        ] {
            if provider != "arbeitnow" {
                sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES (?,'workflow','search',?,?,'2026-01-03')")
                    .bind(provider).bind(provider).bind(provider).execute(&upgraded).await.unwrap();
            }
            assert!(sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES ('duplicate-provider','workflow','search','Duplicate',?,'2026-01-03',?)")
                .bind(provider).bind(serde_json::json!({"endpoint":endpoint}).to_string())
                .execute(&upgraded).await.is_err());
        }
        assert!(
            sqlx::query("UPDATE sources SET config='{}' WHERE id='computer'")
                .execute(&upgraded)
                .await
                .is_err()
        );
        assert!(sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES ('duplicate-board','workflow','greenhouse','Duplicate board','fixture','2026-01-03')")
            .execute(&upgraded).await.is_err());
        sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at,config) VALUES ('other-category','other','search','Other category','themuse','2026-01-03','{\"endpoint\":\"https://www.themuse.com/api/public/jobs?page=0&category=Computer%20and%20IT\"}')")
            .execute(&upgraded).await.unwrap();
        sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES ('other-board','other','greenhouse','Other board','fixture','2026-01-03')")
            .execute(&upgraded).await.unwrap();
        assert_integrity(&upgraded).await;
        let after = snapshot(&upgraded).await;
        original.close().await;
        upgraded.close().await;
        let restarted = init_db(&path).await.unwrap();
        assert_eq!(snapshot(&restarted).await, after);
        assert_integrity(&restarted).await;
        assert_eq!(
            std::fs::read_dir(directory.path())
                .unwrap()
                .flatten()
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("pre-migration"))
                .count(),
            1
        );
        restarted.close().await;
    }
}
