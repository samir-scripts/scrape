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

#[tokio::test]
async fn populated_v33_discovery_upgrade_rolls_back_retries_and_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("discovery-upgrade.db");
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(
        SqliteConnectOptions::new().filename(&path).create_if_missing(true).foreign_keys(true)
    ).await.unwrap();
    let mut history = sqlx::migrate!("./migrations");
    history.migrations = history.iter().filter(|m| m.version <= 33).cloned().collect::<Vec<_>>().into();
    history.run(&pool).await.unwrap();
    sqlx::raw_sql("
        INSERT INTO workflows(id,name,created_at) VALUES ('w','Protected','2026-01-01');
        INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES ('s','w','greenhouse','Protected','board','2026-01-01');
        INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,external_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited)
        VALUES ('j','s:1','s','w','1','Engineer','Employer','https://example.test/1','2026-01-01','2026-01-01','interviewing','2026-01-01','protected',1);
        INSERT INTO scrape_runs(id,trigger,status,started_at,source_id,workflow_id) VALUES ('r','single_source','success','2026-01-01','s','w');
    ").execute(&pool).await.unwrap();
    let mut failing = sqlx::migrate!("./migrations");
    failing.migrations = failing.iter().map(|m| {
        if m.version == 34 {
            sqlx::migrate::Migration::new(34, m.description.clone(), m.migration_type,
                m.sql.replace("DROP TABLE sources;", "DROP TABLE sources; SELECT * FROM injected_missing_34;").into(), false)
        } else { m.clone() }
    }).collect::<Vec<_>>().into();
    assert!(failing.run(&pool).await.is_err());
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations").fetch_one(&pool).await.unwrap(), 33);
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT source_id FROM jobs WHERE id='j'").fetch_one(&pool).await.unwrap(), "s");
    pool.close().await;
    let upgraded = init_db(&path).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT source_id FROM scrape_runs WHERE id='r'").fetch_one(&upgraded).await.unwrap(), "s");
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT notes FROM jobs WHERE id='j'").fetch_one(&upgraded).await.unwrap(), "protected");
    assert!(sqlx::query("PRAGMA foreign_key_check").fetch_all(&upgraded).await.unwrap().is_empty());
    sqlx::query("INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES ('web','w','web_discovery','Web','https://example.test/','2026-01-01')")
        .execute(&upgraded).await.unwrap();
    upgraded.close().await;
    let reopened = init_db(&path).await.unwrap();
    assert_eq!(sqlx::query_scalar::<_, String>("SELECT kind FROM sources WHERE id='web'").fetch_one(&reopened).await.unwrap(), "web_discovery");
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT favorited FROM jobs WHERE id='j'").fetch_one(&reopened).await.unwrap(), 1);
    assert!(sqlx::query("PRAGMA foreign_key_check").fetch_all(&reopened).await.unwrap().is_empty());
}

#[tokio::test]
async fn recognized_v34_histories_upgrade_without_rewriting_checksums_or_user_state() {
    for variant in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("recognized-v34.db");
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(
            SqliteConnectOptions::new().filename(&path).create_if_missing(true).foreign_keys(true)
        ).await.unwrap();
        let mut history = sqlx::migrate!("./migrations");
        history.migrations = history.iter().filter(|m| m.version <= 34).map(|m| {
            if m.version != 34 { return m.clone(); }
            let sql = match variant {
                0 => m.sql.to_string(),
                1 => m.sql.replace("WHERE kind NOT IN ('search','web_discovery');", "WHERE kind <> 'search';"),
                _ => m.sql.replace(
                    " source_id TEXT NOT NULL REFERENCES sources(id) ON DELETE CASCADE,",
                    " source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,\n source_key TEXT NOT NULL,"
                ).replace(" PRIMARY KEY (job_id, run_id, source_id)", " PRIMARY KEY (job_id, run_id, source_key)"),
            };
            sqlx::migrate::Migration::new(34, m.description.clone(), m.migration_type, sql.into(), false)
        }).collect::<Vec<_>>().into();
        history.run(&pool).await.unwrap();
        sqlx::raw_sql("
            INSERT INTO workflows(id,name,created_at) VALUES ('w','Protected','2026-01-01');
            INSERT INTO sources(id,workflow_id,kind,name,identifier,created_at) VALUES ('s','w','web_discovery','Protected','https://example.test/','2026-01-01');
            INSERT INTO jobs(id,dedupe_key,source_id,workflow_id,external_id,title,company,url,date_added,last_seen_at,status,status_changed_at,notes,favorited)
            VALUES ('j','s:1','s','w','1','Engineer','Employer','https://example.test/1','2026-01-01','2026-01-01','interviewing','2026-01-01','protected',1);
            INSERT INTO scrape_runs(id,trigger,status,started_at,source_id,workflow_id) VALUES ('r','single_source','success','2026-01-01','s','w');
        ").execute(&pool).await.unwrap();
        let insert = if variant == 2 {
            "INSERT INTO discovery_observations(job_id,run_id,source_id,source_key,observed_at,provenance) VALUES ('j','r','s','s','2026-01-01','{}')"
        } else {
            "INSERT INTO discovery_observations(job_id,run_id,source_id,observed_at,provenance) VALUES ('j','r','s','2026-01-01','{}')"
        };
        sqlx::query(insert).execute(&pool).await.unwrap();
        let applied: Vec<(i64, Vec<u8>, bool)> = sqlx::query_as("SELECT version,checksum,success FROM _sqlx_migrations ORDER BY version").fetch_all(&pool).await.unwrap();
        let mut failing = super::history::manifest(&applied);
        failing.migrations = failing.iter().map(|m| {
            if m.version == 35 {
                sqlx::migrate::Migration::new(35, m.description.clone(), m.migration_type,
                    m.sql.replace("DROP INDEX idx_sources_non_search_endpoint;", "DROP INDEX idx_sources_non_search_endpoint; SELECT * FROM injected_missing_35;").into(), false)
            } else { m.clone() }
        }).collect::<Vec<_>>().into();
        assert!(failing.run(&pool).await.is_err());
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations").fetch_one(&pool).await.unwrap(), 34);
        assert_eq!(sqlx::query_scalar::<_, String>("SELECT notes FROM jobs WHERE id='j'").fetch_one(&pool).await.unwrap(), "protected");
        let before: Vec<(i64, Vec<u8>)> = sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version").fetch_all(&pool).await.unwrap();
        pool.close().await;
        let upgraded = init_db(&path).await.unwrap();
        let after: Vec<(i64, Vec<u8>)> = sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations WHERE version<=34 ORDER BY version").fetch_all(&upgraded).await.unwrap();
        assert_eq!(before, after);
        assert_eq!(sqlx::query_scalar::<_, String>("SELECT notes FROM jobs WHERE id='j'").fetch_one(&upgraded).await.unwrap(), "protected");
        sqlx::query("DELETE FROM sources WHERE id='s'").execute(&upgraded).await.unwrap();
        let observation: (Option<String>, String, String) = sqlx::query_as("SELECT source_id,source_key,provenance FROM discovery_observations WHERE job_id='j'").fetch_one(&upgraded).await.unwrap();
        assert_eq!(observation, (None, "s".into(), "{}".into()));
        assert!(sqlx::query("PRAGMA foreign_key_check").fetch_all(&upgraded).await.unwrap().is_empty());
        upgraded.close().await;
        let reopened = init_db(&path).await.unwrap();
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations").fetch_one(&reopened).await.unwrap(), 35);
        reopened.close().await;
    }
}
