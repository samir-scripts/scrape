use crate::error::{AppError, Result};
use scrape_core::branding::{relationship_key, CompanyLogo, LogoStatus};
use sqlx::SqlitePool;

pub async fn record(
    pool: &SqlitePool,
    source_id: &str,
    company: &str,
    logo: &CompanyLogo,
) -> Result<()> {
    logo.validate().map_err(AppError::Validation)?;
    if company.trim().is_empty() {
        return Err(AppError::Validation(
            "Branding report lacks employer identity".into(),
        ));
    }
    let key = relationship_key(source_id, company);
    let mut metadata = logo.clone();
    metadata.cached_asset_reference = logo.asset_reference();
    metadata.asset = None;
    let json = serde_json::to_string(&metadata)?;
    let verified = logo.status == LogoStatus::Verified;
    sqlx::query("INSERT INTO employer_branding(key,source_id,company,logo_json,last_attempt_json,asset_reference,asset_data,updated_at) VALUES (?1,?2,?3,?4,?4,?5,?6,?7) ON CONFLICT(key) DO UPDATE SET logo_json=CASE WHEN ?8 OR employer_branding.asset_reference IS NULL THEN excluded.logo_json ELSE employer_branding.logo_json END, last_attempt_json=excluded.last_attempt_json, asset_reference=COALESCE(excluded.asset_reference,employer_branding.asset_reference), asset_data=COALESCE(excluded.asset_data,employer_branding.asset_data), updated_at=excluded.updated_at")
        .bind(key).bind(source_id).bind(company).bind(json).bind(logo.asset_reference()).bind(&logo.asset).bind(&logo.checked_at).bind(verified).execute(pool).await?;
    Ok(())
}
pub fn from_storage(logo: Option<String>, attempt: Option<String>) -> Result<Option<CompanyLogo>> {
    let mut logo = logo
        .map(|json| serde_json::from_str::<CompanyLogo>(&json))
        .transpose()?;
    if let (Some(cached), Some(attempt)) = (&mut logo, attempt) {
        let attempt: CompanyLogo = serde_json::from_str(&attempt)?;
        if cached.status == LogoStatus::Verified && attempt.status != LogoStatus::Verified {
            cached.diagnostic = Some(format!(
                "Using last verified logo. Latest lookup: {:?}. {}",
                attempt.status,
                attempt.diagnostic.as_deref().unwrap_or_default()
            ));
        }
    }
    if let Some(logo) = &mut logo {
        logo.asset = None;
    }
    Ok(logo)
}

pub async fn asset(pool: &SqlitePool, reference: &str) -> Result<String> {
    if reference.len() != 71
        || !reference.starts_with("sha256:")
        || !reference[7..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(AppError::Validation("Invalid logo cache reference".into()));
    }
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT logo_json,asset_data FROM employer_branding WHERE asset_reference=? LIMIT 1",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Cached employer logo not found".into()))?;
    let mut logo: CompanyLogo = serde_json::from_str(&row.get::<String, _>("logo_json"))?;
    logo.asset = row.get("asset_data");

    logo.validate().map_err(AppError::Validation)?;
    if logo.asset_reference().as_deref() != Some(reference) {
        return Err(AppError::Validation("Cached logo checksum mismatch".into()));
    }
    logo.asset
        .ok_or_else(|| AppError::NotFound("Cached logo has no image".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        jobs::{JobFilter, JobsRepo},
        sources::SourcesRepo,
    };
    use scrape_core::{
        branding::{LogoEvidence, LogoMethod},
        WorkerEvent,
    };
    use sqlx::{
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
        Row,
    };
    #[tokio::test]
    async fn populated_v22_upgrade_refresh_offline_failure_and_source_deletion_keep_one_verified_asset(
    ) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("branding.db");
        let old = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        let mut manifest = sqlx::migrate!("./migrations");
        manifest.migrations = manifest
            .iter()
            .filter(|m| m.version <= 22)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        manifest.run(&old).await.unwrap();
        sqlx::query("INSERT INTO sources(id,kind,name,identifier,enabled,cadence,created_at,next_run_at) VALUES ('source','greenhouse','Engineering jobs','fixture',1,'daily','2026-01-01','2030-01-01T09:00:00Z')").execute(&old).await.unwrap();
        sqlx::query("INSERT INTO app_settings VALUES ('migration.workflow_settings_v4','true','2026-01-01')").execute(&old).await.unwrap();
        sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited,company_icon_url) VALUES ('saved','source:42','source','42','Engineer','Fixture','full_time','https://fixture.test/jobs/42','2026-01-01','2026-01-01','applied','2026-01-01','Protected note',1,'https://fixture.test/old-logo.png')").execute(&old).await.unwrap();
        sqlx::query("INSERT INTO job_status_events(job_id,to_status,changed_at) VALUES ('saved','applied','2026-01-01')").execute(&old).await.unwrap();
        let checksums: Vec<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT version,checksum FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&old)
                .await
                .unwrap();
        old.close().await;
        let pool = crate::db::init_db(&path).await.unwrap();
        let state = JobsRepo::get(&pool, "saved").await.unwrap();
        assert_eq!(state.notes.as_deref(), Some("Protected note"));
        assert!(state.favorited);
        assert!(state.company_logo.is_none());
        let new_checksums: Vec<(i64, Vec<u8>)> = sqlx::query_as(
            "SELECT version,checksum FROM _sqlx_migrations WHERE version<=22 ORDER BY version",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(checksums, new_checksums);
        let backup = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .find(|e| e.file_name().to_string_lossy().contains("pre-migration"))
            .unwrap()
            .path();
        let snapshot = SqlitePoolOptions::new()
            .connect_with(SqliteConnectOptions::new().filename(backup).read_only(true))
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&snapshot)
                .await
                .unwrap(),
            22
        );
        snapshot.close().await;
        let evidence = LogoEvidence {
            url: "https://fixture.test/logo.svg".into(),
            page_url: "https://fixture.test/jobs".into(),
            method: LogoMethod::EmployerLabel,
        };
        let verified=CompanyLogo{evidence:Some(evidence.clone()),checked_at:"2026-10-16T00:00:00Z".into(),status:LogoStatus::Verified,cached_asset_reference:None,asset:Some(scrape_core::branding::cache_asset(br##"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><path fill="#f80" d="M0 0h80v20H0z"/></svg>"##).unwrap()),diagnostic:None};
        record(&pool, "source", "Fixture", &verified).await.unwrap();
        // Exercise the real worker-event mapper and upsert; refresh must attach the relationship.
        let event:WorkerEvent=serde_json::from_value(serde_json::json!({"type":"job","source_id":"source","external_id":"42","title":"Engineer","company":"Fixture","role_type":"full_time","url":"https://fixture.test/jobs/42","company_icon_url":evidence.url,"company_logo":evidence})).unwrap();
        let candidate = crate::services::ingestion::scraped_job_from_event(event).unwrap();
        assert_eq!(
            JobsRepo::upsert_scraped(&pool, std::slice::from_ref(&candidate))
                .await
                .unwrap()
                .updated,
            1
        );
        let restored = JobsRepo::get(&pool, "saved").await.unwrap();
        assert_eq!(
            restored
                .company_logo
                .as_ref()
                .unwrap()
                .cached_asset_reference,
            verified.asset_reference()
        );
        assert!(restored.company_logo.as_ref().unwrap().asset.is_none());
        assert_eq!(
            asset(&pool, &verified.asset_reference().unwrap())
                .await
                .unwrap(),
            verified.asset.as_ref().unwrap().as_str()
        );
        assert_eq!(restored.notes.as_deref(), Some("Protected note"));
        assert!(restored.favorited);
        assert_eq!(restored.status, scrape_core::JobStatus::Applied);
        let list = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(
            list.items[0]
                .company_logo
                .as_ref()
                .unwrap()
                .cached_asset_reference,
            verified.asset_reference()
        );
        for status in [
            LogoStatus::DownloadFailed,
            LogoStatus::DecodeFailed,
            LogoStatus::MissingMetadata,
        ] {
            let failure = CompanyLogo {
                evidence: if status == LogoStatus::MissingMetadata {
                    None
                } else {
                    Some(LogoEvidence {
                        url: "https://fixture.test/new-broken.png".into(),
                        ..evidence.clone()
                    })
                },
                checked_at: "2026-10-17T00:00:00Z".into(),
                status,
                asset: None,
                cached_asset_reference: None,
                diagnostic: Some("Authored temporary failure".into()),
            };
            record(&pool, "source", "Fixture", &failure).await.unwrap();
            JobsRepo::upsert_scraped(&pool, std::slice::from_ref(&candidate))
                .await
                .unwrap();
            let logo = JobsRepo::get(&pool, "saved")
                .await
                .unwrap()
                .company_logo
                .unwrap();
            assert_eq!(logo.cached_asset_reference, verified.asset_reference());
            assert_eq!(logo.evidence, verified.evidence);
            assert!(logo.diagnostic.unwrap().contains("last verified"));
        }
        let invalid = CompanyLogo {
            asset: Some("data:image/png;base64,broken".into()),
            ..verified.clone()
        };
        assert!(record(&pool, "source", "Fixture", &invalid).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM employer_branding")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        let row = sqlx::query("SELECT asset_reference,last_attempt_json FROM employer_branding")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            row.get::<String, _>("asset_reference"),
            verified.asset_reference().unwrap()
        );
        assert!(row
            .get::<String, _>("last_attempt_json")
            .contains("missing_metadata"));
        SourcesRepo::delete(&pool, "source").await.unwrap();
        let job = JobsRepo::get(&pool, "saved").await.unwrap();
        assert!(job.source_id.is_none());
        assert_eq!(
            job.company_logo.unwrap().cached_asset_reference,
            verified.asset_reference()
        );
        assert_eq!(job.notes.as_deref(), Some("Protected note"));
        pool.close().await;
        let reopened = crate::db::init_db(&path).await.unwrap();
        assert!(JobsRepo::get(&reopened, "saved")
            .await
            .unwrap()
            .company_logo
            .unwrap()
            .cached_asset_reference
            .is_some());
        assert_eq!(
            std::fs::read_dir(dir.path())
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().contains("pre-migration"))
                .count(),
            1
        );
    }
}

#[cfg(test)]
mod migration_failure_tests {
    use super::*;
    use sqlx::{
        migrate::Migration,
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    };
    #[tokio::test]
    async fn failed_branding_migration_rolls_back_columns_and_preserves_saved_rows_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("failed.db");
        let old = SqlitePoolOptions::new()
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(&path)
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let mut historical = sqlx::migrate!("./migrations");
        historical.migrations = historical
            .iter()
            .filter(|m| m.version <= 22)
            .cloned()
            .collect::<Vec<_>>()
            .into();
        historical.run(&old).await.unwrap();
        sqlx::query("INSERT INTO jobs(id,dedupe_key,title,company,url,date_added,last_seen_at,status_changed_at,notes,favorited) VALUES ('saved','saved','Engineer','Fixture','https://fixture.test/job','2026-01-01','2026-01-01','2026-01-01','Protected',1)").execute(&old).await.unwrap();
        let mut failing = sqlx::migrate!("./migrations");
        failing.migrations = failing
            .iter()
            .map(|m| {
                if m.version == 23 {
                    Migration::new(
                        23,
                        m.description.clone(),
                        m.migration_type,
                        format!("{}\nSELECT missing FROM missing_table;", m.sql).into(),
                        false,
                    )
                } else {
                    m.clone()
                }
            })
            .collect::<Vec<_>>()
            .into();
        assert!(failing.run(&old).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM sqlite_master WHERE name='employer_branding'"
            )
            .fetch_one(&old)
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM pragma_table_info('jobs') WHERE name='company_branding_key'"
            )
            .fetch_one(&old)
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT max(version) FROM _sqlx_migrations")
                .fetch_one(&old)
                .await
                .unwrap(),
            22
        );
        old.close().await;
        let upgraded = crate::db::init_db(&path).await.unwrap();
        let job = crate::db::jobs::JobsRepo::get(&upgraded, "saved")
            .await
            .unwrap();
        assert_eq!(job.notes.as_deref(), Some("Protected"));
        assert!(job.favorited);
        assert!(job.company_logo.is_none());
        assert!(asset(&upgraded, "sha256:invalid").await.is_err());
    }
}
