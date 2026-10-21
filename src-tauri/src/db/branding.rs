use crate::error::{AppError, Result};
use scrape_core::branding::{relationship_key, CompanyLogo, LogoStatus};
use sqlx::{SqliteConnection, SqlitePool};

pub async fn record(
    pool: &SqlitePool,
    source_id: &str,
    company: &str,
    logo: &CompanyLogo,
) -> Result<()> {
    logo.validate().map_err(AppError::Validation)?;
    if source_id.trim().is_empty() || company.trim().is_empty() {
        return Err(AppError::Validation(
            "Branding report lacks source/employer identity".into(),
        ));
    }
    let key = relationship_key(source_id, company);
    let mut metadata = logo.clone();
    metadata.cached_asset_reference = logo.asset_reference();
    metadata.asset = None;
    let json = serde_json::to_string(&metadata)?;
    let verified = logo.status == LogoStatus::Verified;
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO employer_branding(key,source_id,company,logo_json,last_attempt_json,asset_reference,asset_data,updated_at) VALUES (?1,?2,?3,?4,?4,?5,?6,?7) ON CONFLICT(key) DO UPDATE SET logo_json=CASE WHEN ?8 OR employer_branding.asset_reference IS NULL THEN excluded.logo_json ELSE employer_branding.logo_json END, last_attempt_json=excluded.last_attempt_json, asset_reference=COALESCE(excluded.asset_reference,employer_branding.asset_reference), asset_data=COALESCE(excluded.asset_data,employer_branding.asset_data), updated_at=excluded.updated_at")
        .bind(&key).bind(source_id).bind(company).bind(json).bind(logo.asset_reference()).bind(&logo.asset).bind(&logo.checked_at).bind(verified).execute(&mut *tx).await?;
    // Use the same Rust identity owner as ingestion, including its normalization,
    // rather than reconstructing relationship keys with SQLite string functions.
    let companies: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT company FROM jobs WHERE source_id=? AND company_branding_key IS NULL",
    )
    .bind(source_id)
    .fetch_all(&mut *tx)
    .await?;
    for job_company in companies {
        if relationship_key(source_id, &job_company) == key {
            attach_jobs(&mut tx, source_id, &job_company, &key).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Find only exact source/employer relationships already represented in the cache.
/// Source-deleted jobs and jobs with an existing relationship are deliberately excluded.
pub(crate) async fn pending_job_links(pool: &SqlitePool) -> Result<Vec<(String, String, String)>> {
    let stored: Vec<(String, String, String)> =
        sqlx::query_as("SELECT source_id,company,key FROM employer_branding")
            .fetch_all(pool)
            .await?;
    let known: std::collections::HashSet<String> = stored
        .into_iter()
        .filter_map(|(source_id, company, key)| {
            (relationship_key(&source_id, &company) == key).then_some(key)
        })
        .collect();
    if known.is_empty() {
        return Ok(Vec::new());
    }
    let unlinked: Vec<(String, String)> = sqlx::query_as(
        "SELECT DISTINCT source_id,company FROM jobs WHERE company_branding_key IS NULL AND source_id IS NOT NULL ORDER BY source_id,company",
    )
    .fetch_all(pool)
    .await?;
    Ok(unlinked
        .into_iter()
        .filter_map(|(source_id, company)| {
            let key = relationship_key(&source_id, &company);
            known.contains(&key).then_some((source_id, company, key))
        })
        .collect())
}

pub(crate) async fn attach_jobs(
    connection: &mut SqliteConnection,
    source_id: &str,
    company: &str,
    key: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE jobs SET company_branding_key=? WHERE source_id=? AND company=? AND company_branding_key IS NULL",
    )
    .bind(key)
    .bind(source_id)
    .bind(company)
    .execute(connection)
    .await?;
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
    .await?;

    let (logo_json, asset_data) = match row {
        Some(r) => (r.get::<String, _>("logo_json"), r.get("asset_data")),
        None => {
            let curated = [
                "cloudflare",
                "google",
                "apple",
                "meta",
                "amazon",
                "netflix",
                "bitwarden",
                "linear",
                "sep",
                "nvidia",
            ]
            .into_iter()
            .find_map(|name| {
                let logo = resolve_curated_logo(name)?;
                (logo.asset_reference().as_deref() == Some(reference)).then_some(logo)
            })
            .ok_or_else(|| AppError::NotFound("Cached employer logo not found".into()))?;
            let mut metadata = curated.clone();
            metadata.cached_asset_reference = curated.asset_reference();
            metadata.asset = None;
            (serde_json::to_string(&metadata)?, curated.asset)
        }
    };

    let mut logo: CompanyLogo = serde_json::from_str(&logo_json)?;
    logo.asset = asset_data;

    logo.validate().map_err(AppError::Validation)?;
    if logo.asset_reference().as_deref() != Some(reference) {
        return Err(AppError::Validation("Cached logo checksum mismatch".into()));
    }
    logo.asset
        .ok_or_else(|| AppError::NotFound("Cached logo has no image".into()))
}

pub fn resolve_curated_logo(company: &str) -> Option<CompanyLogo> {
    let normalized = company.trim().to_lowercase();
    let (bytes, url, page_url): (&[u8], &str, &str) = match normalized.as_str() {
        "cloudflare" => (
            include_bytes!("../../assets/logos/cloudflare.jpg"),
            "https://recruiting.cdn.greenhouse.io/external_greenhouse_job_boards/logos/000/007/685/original/Image_20221024_121529_116.png?1666631611",
            "https://boards.greenhouse.io/cloudflare",
        ),
        "google" => (
            include_bytes!("../../assets/logos/google.svg"),
            "https://www.gstatic.com/hiring/CportalUi/google_logo_dark.png",
            "https://www.google.com/about/careers/applications/jobs/results/",
        ),
        "apple" => (
            include_bytes!("../../assets/logos/apple.svg"),
            "https://jobs.apple.com/public/images/apple-logo.svg",
            "https://jobs.apple.com/en-us/search",
        ),
        "meta" => (
            include_bytes!("../../assets/logos/meta.svg"),
            "https://static.xx.fbcdn.net/rsrc.php/y3/r/iUyd7C5uqWN.webp",
            "https://www.metacareers.com",
        ),
        "amazon" => (
            include_bytes!("../../assets/logos/amazon.svg"),
            "https://m.media-amazon.com/images/G/01/wfm/Home/amazon_logo_RGB._CB485934546_.png",
            "https://amazon.jobs",
        ),
        "netflix" => (
            include_bytes!("../../assets/logos/netflix.svg"),
            "https://static.vscdn.net/images/careers/demo/netflix/1746474854::netflix_jobs_logo",
            "https://jobs.netflix.com",
        ),
        "bitwarden" => (
            include_bytes!("../../assets/logos/bitwarden.svg"),
            "https://bitwarden.com/images/icon.png",
            "https://bitwarden.com/careers",
        ),
        "linear" => (
            include_bytes!("../../assets/logos/linear.svg"),
            "https://linear.app/static/apple-touch-icon.png",
            "https://linear.app/careers",
        ),
        "sep" => (
            include_bytes!("../../assets/logos/sep.svg"),
            "https://sep.com/wp-content/uploads/2021/04/sep-logo.svg",
            "https://sep.com/careers",
        ),
        "nvidia" => (
            include_bytes!("../../assets/logos/nvidia.svg"),
            "https://www.nvidia.com/content/dam/en-zz/Solutions/about-nvidia/logo-and-brand/01-nvidia-logo-horiz-500x200-2c50-p@2x.png",
            "https://www.nvidia.com/en-us/about-nvidia/careers",
        ),
        _ => return None,
    };
    let asset = scrape_core::branding::cache_asset(bytes).ok()?;
    let logo = CompanyLogo {
        evidence: Some(scrape_core::branding::LogoEvidence {
            url: url.into(),
            page_url: page_url.into(),
            method: scrape_core::branding::LogoMethod::Provider,
        }),
        checked_at: "2026-10-16T18:23:08.416410+00:00".into(),
        status: LogoStatus::Verified,
        asset: Some(asset),
        cached_asset_reference: None,
        diagnostic: None,
    };
    logo.validate().ok()?;
    Some(logo)
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
        assert_eq!(
            JobsRepo::get(&pool, "saved")
                .await
                .unwrap()
                .company_logo
                .unwrap()
                .cached_asset_reference,
            verified.asset_reference()
        );
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

    fn authored_logo(color: &str, employer: &str) -> CompanyLogo {
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="80" height="20"><path fill="{color}" d="M0 0h80v20H0z"/></svg>"#
        );
        CompanyLogo {
            evidence: Some(LogoEvidence {
                url: format!("https://employers.test/{employer}/logo.svg"),
                page_url: format!("https://shared.ats.test/{employer}/jobs"),
                method: LogoMethod::EmployerLabel,
            }),
            checked_at: "2026-10-20T00:00:00Z".into(),
            status: LogoStatus::Verified,
            asset: Some(scrape_core::branding::cache_asset(svg.as_bytes()).unwrap()),
            cached_asset_reference: None,
            diagnostic: None,
        }
    }

    #[tokio::test]
    async fn record_attaches_only_exact_unlinked_jobs_and_rolls_back_report_with_link_failure() {
        let pool = crate::db::init_memory_db().await.unwrap();
        sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at) VALUES ('one','lever','One','one','2026-01-01'),('two','lever','Two','two','2026-01-01')")
            .execute(&pool).await.unwrap();
        for (id, source, company, key) in [
            ("acme", Some("one"), "Acme", None),
            ("acme-lower", Some("one"), " acme ", None),
            ("labs", Some("one"), "Acme Labs", None),
            ("other-source", Some("two"), "Acme", None),
            ("source-deleted", None, "Acme", None),
            ("pinned", Some("one"), "Acme", Some("preserved-key")),
        ] {
            sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,title,company,url,date_added,last_seen_at,status_changed_at,company_branding_key) VALUES (?1,?1,?2,'Engineer',?3,'https://shared.ats.test/jobs/'||?1,'2026-01-01','2026-01-01','2026-01-01',?4)")
                .bind(id).bind(source).bind(company).bind(key).execute(&pool).await.unwrap();
        }
        let acme = authored_logo("#f80", "acme");
        assert!(record(&pool, "", "Acme", &acme).await.is_err());
        sqlx::query("CREATE TRIGGER reject_branding_link BEFORE UPDATE OF company_branding_key ON jobs WHEN NEW.id='acme-lower' BEGIN SELECT RAISE(ABORT, 'authored branding link failure'); END")
            .execute(&pool).await.unwrap();
        assert!(record(&pool, "one", "Acme", &acme).await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM employer_branding")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM jobs WHERE company_branding_key IS NOT NULL"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        sqlx::query("DROP TRIGGER reject_branding_link")
            .execute(&pool)
            .await
            .unwrap();
        record(&pool, "one", "Acme", &acme).await.unwrap();
        for id in ["acme", "acme-lower"] {
            assert_eq!(
                JobsRepo::get(&pool, id)
                    .await
                    .unwrap()
                    .company_logo
                    .unwrap()
                    .cached_asset_reference,
                acme.asset_reference()
            );
        }
        for id in ["labs", "other-source", "source-deleted"] {
            assert!(JobsRepo::get(&pool, id)
                .await
                .unwrap()
                .company_logo
                .is_none());
        }
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT company_branding_key FROM jobs WHERE id='pinned'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "preserved-key"
        );
        let labs = authored_logo("#08f", "acme-labs");
        let other_source = authored_logo("#0a5", "other-acme");
        record(&pool, "one", "Acme Labs", &labs).await.unwrap();
        record(&pool, "two", "Acme", &other_source).await.unwrap();
        assert_eq!(
            JobsRepo::get(&pool, "labs")
                .await
                .unwrap()
                .company_logo
                .unwrap()
                .cached_asset_reference,
            labs.asset_reference()
        );
        assert_eq!(
            JobsRepo::get(&pool, "other-source")
                .await
                .unwrap()
                .company_logo
                .unwrap()
                .cached_asset_reference,
            other_source.asset_reference()
        );
        assert_ne!(acme.asset_reference(), labs.asset_reference());
        assert_ne!(acme.asset_reference(), other_source.asset_reference());
    }

    #[tokio::test]
    async fn startup_links_existing_exact_branding_without_stale_experience_or_rescraping() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing-logos.db");
        let pool = crate::db::init_db(&path).await.unwrap();
        sqlx::query("INSERT INTO sources(id,kind,name,identifier,created_at) VALUES ('one','lever','One','one','2026-01-01'),('two','lever','Two','two','2026-01-01')")
            .execute(&pool).await.unwrap();
        let acme = authored_logo("#f80", "acme");
        let labs = authored_logo("#08f", "acme-labs");
        let other_source = authored_logo("#0a5", "other-acme");
        record(&pool, "one", "Acme", &acme).await.unwrap();
        record(&pool, "one", "Acme Labs", &labs).await.unwrap();
        record(&pool, "two", "Acme", &other_source).await.unwrap();
        for (id, source, company, key) in [
            ("acme", Some("one"), "Acme", None),
            ("labs", Some("one"), "Acme Labs", None),
            ("other-source", Some("two"), "Acme", None),
            ("unmatched", Some("two"), "Acme Labs", None),
            ("source-deleted", None, "Acme", None),
            ("pinned", Some("one"), "Acme", Some("preserved-key")),
        ] {
            sqlx::query("INSERT INTO jobs(id,dedupe_key,source_id,title,company,url,date_added,last_seen_at,status_changed_at,notes,favorited,status,company_branding_key,enrichment_version) VALUES (?1,?1,?2,'Engineer',?3,'https://shared.ats.test/jobs/'||?1,'2026-01-01','2026-01-01','2026-01-01','Protected note',1,'applied',?4,?5)")
                .bind(id).bind(source).bind(company).bind(key)
                .bind(scrape_core::enrich::ENRICHMENT_VERSION).execute(&pool).await.unwrap();
        }
        assert!(JobsRepo::get(&pool, "acme")
            .await
            .unwrap()
            .company_logo
            .is_none());
        assert_eq!(JobsRepo::reclassify_jobs(&pool).await.unwrap(), 0);
        for (id, logo) in [
            ("acme", &acme),
            ("labs", &labs),
            ("other-source", &other_source),
        ] {
            let job = JobsRepo::get(&pool, id).await.unwrap();
            assert_eq!(
                job.company_logo.unwrap().cached_asset_reference,
                logo.asset_reference()
            );
            assert_eq!(job.notes.as_deref(), Some("Protected note"));
            assert!(job.favorited);
            assert_eq!(job.status, scrape_core::JobStatus::Applied);
        }
        for id in ["unmatched", "source-deleted"] {
            assert!(JobsRepo::get(&pool, id)
                .await
                .unwrap()
                .company_logo
                .is_none());
        }
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT company_branding_key FROM jobs WHERE id='pinned'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "preserved-key"
        );
        let backups: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("pre-description-repair")
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
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM jobs WHERE company_branding_key IS NULL"
            )
            .fetch_one(&backup)
            .await
            .unwrap(),
            5
        );
        backup.close().await;
        pool.close().await;
        let reopened = crate::db::init_db(&path).await.unwrap();
        assert_eq!(JobsRepo::reclassify_jobs(&reopened).await.unwrap(), 0);
        assert_eq!(
            std::fs::read_dir(directory.path())
                .unwrap()
                .filter(|entry| {
                    entry
                        .as_ref()
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .contains("pre-description-repair")
                })
                .count(),
            1
        );
        assert_eq!(
            JobsRepo::get(&reopened, "labs")
                .await
                .unwrap()
                .company_logo
                .unwrap()
                .cached_asset_reference,
            labs.asset_reference()
        );
    }
    #[tokio::test]
    async fn all_curated_logos_resolve_validate_and_serve_via_asset_lookup() {
        let pool = crate::db::init_memory_db().await.unwrap();
        let companies = [
            "cloudflare",
            "google",
            "apple",
            "meta",
            "amazon",
            "netflix",
            "bitwarden",
            "linear",
            "sep",
            "nvidia",
        ];
        for name in companies {
            let logo = resolve_curated_logo(name)
                .unwrap_or_else(|| panic!("Missing curated logo for {name}"));
            assert_eq!(logo.status, LogoStatus::Verified);
            assert!(logo.evidence.is_some());
            let evidence = logo.evidence.as_ref().unwrap();
            assert!(scrape_core::branding::http_url(&evidence.url));
            assert!(scrape_core::branding::http_url(&evidence.page_url));
            assert!(logo.asset.is_some());
            assert!(logo
                .asset
                .as_ref()
                .unwrap()
                .starts_with("data:image/webp;base64,"));
            assert!(logo.validate().is_ok());

            let reference = logo.asset_reference().unwrap();
            let asset_data = asset(&pool, &reference).await.unwrap();
            assert_eq!(asset_data, logo.asset.unwrap());
        }
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
