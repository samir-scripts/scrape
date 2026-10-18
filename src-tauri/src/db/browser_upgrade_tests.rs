//! Populated, isolated migration-26 recovery contract. Never opens app storage.
use super::*;
use sqlx::Row;

pub(super) async fn snapshot(pool: &SqlitePool) -> Vec<(String, String, Vec<String>)> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut result = Vec::new();
    for table in tables {
        let columns: Vec<String> = sqlx::query(&format!("PRAGMA table_info(\"{table}\")"))
            .fetch_all(pool)
            .await
            .unwrap()
            .iter()
            .map(|row| format!("quote(\"{}\")", row.get::<String, _>("name")))
            .collect();
        let projection = columns.join(" || '|' || ");
        let rows = sqlx::query_scalar(&format!(
            "SELECT {projection} FROM \"{table}\" ORDER BY rowid"
        ))
        .fetch_all(pool)
        .await
        .unwrap();
        result.push((table, projection, rows));
    }
    result
}

async fn assert_preserved(pool: &SqlitePool, before: &[(String, String, Vec<String>)]) {
    for (table, projection, expected) in before {
        let condition = if table == "_sqlx_migrations" {
            " WHERE version <= 25"
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

#[tokio::test]
async fn populated_v25_upgrade_preserves_all_rows_backup_failure_retry_and_restart() {
    assert_eq!(
        sqlx::migrate!("./migrations")
            .iter()
            .last()
            .unwrap()
            .version,
        27
    );
    for inject_failure in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("browser-upgrade.db");
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
            .filter(|m| m.version <= 25)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&original).await.unwrap();
        sqlx::query("INSERT INTO workflows(id,name,enabled,cadence,next_run_at,created_at) VALUES ('source','Fixture workflow',1,'daily','2030-01-01T09:00:00Z','2026-01-01')").execute(&original).await.unwrap();
        sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at,cadence,next_run_at,config,filters) VALUES ('source','greenhouse','Fixture','fixture','2026-01-01','daily','2030-01-01T09:00:00Z','{}','{}')").execute(&original).await.unwrap();
        sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,company_branding_key) VALUES ('job','fixture:42','source','42','Engineer','Fixture','https://fixture.test/jobs/42','2026-01-01','2026-01-01','interviewing','2026-01-01','Keep this note',1,'brand')").execute(&original).await.unwrap();
        sqlx::query(
            "UPDATE sources SET workflow_id='source'; UPDATE jobs SET workflow_id='source';",
        )
        .execute(&original)
        .await
        .unwrap();
        sqlx::query("INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES ('job','interviewing','2026-01-01')").execute(&original).await.unwrap();
        sqlx::query("INSERT INTO blocked_postings(identity,title,company,blocked_at) VALUES ('blocked','Blocked','Fixture','2026-01-01')").execute(&original).await.unwrap();
        sqlx::query("INSERT INTO employer_branding(key,source_id,company,logo_json,last_attempt_json,asset_reference,asset_data,updated_at) VALUES ('brand','source','Fixture','{}','{}','cached-asset','fixture-bytes','2026-01-01')").execute(&original).await.unwrap();
        sqlx::query("INSERT INTO scrape_runs(id,trigger,status,started_at,finished_at,jobs_found,jobs_new,source_id,workflow_id) VALUES ('run','single_source','success','2026-01-01','2026-01-01',1,1,'source','source')").execute(&original).await.unwrap();
        sqlx::query("INSERT INTO scrape_logs(id,run_id,timestamp,level,message) VALUES ('log','run','2026-01-01','info','Preserved log')").execute(&original).await.unwrap();
        // Finish the pre-existing Rust backfills before the historical snapshot.
        upgrade_workflow_settings_v4(&original).await.unwrap();
        upgrade_workflow_settings_v5(&original).await.unwrap();
        postings::backfill(&original).await.unwrap();
        compensation::backfill(&original).await.unwrap();
        let before = snapshot(&original).await;
        if inject_failure {
            let mut failing = sqlx::migrate!("./migrations");
            failing.migrations = failing
                .iter()
                .map(|m| {
                    if m.version == 26 {
                        sqlx::migrate::Migration::new(
                            26,
                            m.description.clone(),
                            m.migration_type,
                            format!("{}\nSELECT * FROM injected_missing_table;", m.sql).into(),
                            false,
                        )
                    } else {
                        m.clone()
                    }
                })
                .collect::<Vec<_>>()
                .into();
            assert!(failing.run(&original).await.is_err());
            assert_preserved(&original, &before).await;
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM sqlite_master WHERE name='browser_sessions'"
                )
                .fetch_one(&original)
                .await
                .unwrap(),
                0
            );
            assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM pragma_table_info('sources') WHERE name IN ('script_trusted','script_sha256')").fetch_one(&original).await.unwrap(), 0);
        }
        let upgraded = init_db(&path).await.unwrap();
        assert_preserved(&upgraded, &before).await;
        let trust: (i64, Option<String>) =
            sqlx::query_as("SELECT script_trusted,script_sha256 FROM sources WHERE id='source'")
                .fetch_one(&upgraded)
                .await
                .unwrap();
        assert_eq!(trust, (0, None));
        sqlx::query("INSERT INTO browser_sessions(id,label,domain,storage_relpath,created_at,updated_at) VALUES ('session','Fixture login','fixture.test','sessions/fixture.json','2026-01-01','2026-01-01')").execute(&upgraded).await.unwrap();
        assert!(sqlx::query("UPDATE browser_sessions SET status='unknown'")
            .execute(&upgraded)
            .await
            .is_err());
        assert!(sqlx::query("UPDATE sources SET kind='browser'")
            .execute(&upgraded)
            .await
            .is_err());
        let backups: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("pre-migration"))
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
        assert_preserved(&backup, &before).await;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&backup)
                .await
                .unwrap(),
            25
        );
        backup.close().await;
        original.close().await;
        upgraded.close().await;
        let restarted = init_db(&path).await.unwrap();
        assert_preserved(&restarted, &before).await;
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM browser_sessions WHERE id='session'"
            )
            .fetch_one(&restarted)
            .await
            .unwrap(),
            "ready"
        );
        assert_eq!(
            std::fs::read_dir(directory.path())
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().contains("pre-migration"))
                .count(),
            1
        );
        restarted.close().await;
    }
}
