//! Deterministic native boundary: local HTTP → packaged sidecar → NDJSON →
//! production ingestion mapper → SQLite. Build the sidecar before running.
use scrape_core::{
    SourceKind, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource,
    PROTOCOL_VERSION,
};
use scrape_lib::db::{
    jobs::{JobFilter, JobsRepo},
    postings,
    sources::{CreateSourceInput, SourcesRepo},
};
use scrape_lib::services::ingestion::scraped_job_from_event;
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn worker() -> std::path::PathBuf {
    let triple = match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("x86_64", "macos") => "x86_64-apple-darwin",
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu",
        _ => panic!("Set up a packaged native sidecar for this test platform"),
    };
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("binaries/scrape-worker-{triple}"));
    assert!(
        file.exists(),
        "Build the sidecar with npm run build:sidecar before Rust integration tests"
    );
    file
}
fn invoke(config: &WorkerConfig, preview: bool) -> Vec<WorkerEvent> {
    invoke_replay(config, preview, None)
}
fn invoke_replay(config: &WorkerConfig, preview: bool, origin: Option<&str>) -> Vec<WorkerEvent> {
    let mut command = Command::new(worker());
    command.arg("--stdin");
    if let Some(origin) = origin {
        command.args(["--replay-origin", origin]);
    }
    if preview {
        command.arg("--dry-run");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(config).unwrap())
        .unwrap();
    // Drain both pipes while the worker is running. Structured source evidence
    // can exceed the OS pipe buffer, and waiting before reading would deadlock.
    let mut stdout = child.stdout.take().unwrap();
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let mut stderr = child.stderr.take().unwrap();
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            let stderr = stderr_reader.join().unwrap();
            let _ = stdout_reader.join();
            panic!(
                "Sidecar fixture timed out: {}",
                String::from_utf8_lossy(&stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let status = child.wait().unwrap();
    let stdout = stdout_reader.join().unwrap();
    let stderr = stderr_reader.join().unwrap();
    assert!(status.success(), "{}", String::from_utf8_lossy(&stderr));
    let events: Vec<WorkerEvent> = String::from_utf8(stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        matches!(events.last(), Some(WorkerEvent::Done { .. })),
        "Terminal completion is required"
    );
    events
}

#[tokio::test]
async fn preview_single_global_identity_and_native_pay_survive_persistence_and_blocking() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut pages = 0;
        while pages < 3 && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                let body = if request.starts_with("GET /robots.txt ") {
                    "User-agent: *\nAllow: /"
                } else {
                    pages += 1;
                    r#"<script type="application/ld+json">[
                      {"@type":"JobPosting","identifier":"42","title":"Native Hourly Engineer","url":"/apply?gh_jid=42&utm_source=test","hiringOrganization":{"name":"Fixture Co"},"datePosted":"2026-10-12T09:00:00Z","description":"&lt;h2&gt;About the role&lt;/h2&gt;&lt;p&gt;A &amp; B&lt;/p&gt;","baseSalary":{"currency":"USD","value":{"minValue":45.25,"maxValue":55.75,"unitText":"HOUR"}}},

                      {"@type":"JobPosting","identifier":"43","title":"Annual Engineer","url":"/apply?gh_jid=43","baseSalary":{"currency":"USD","value":{"minValue":100000,"unitText":"YEAR"}}}
                    ]</script>"#
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert_eq!(pages, 3);
    });
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("jobs.db"))
        .await
        .unwrap();
    let url = format!("http://{address}/careers");
    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Jsonld,
            name: "Fixture".into(),
            identifier: url.clone(),
            config: Some(serde_json::json!({})),
            enabled: Some(true),
            cadence: Some(scrape_core::SourceCadence::Manual),
            cadence_cron: None,
            retention_days: Some(30),
            retention_policy: Some("untracked_only".into()),
            filters: None,
        },
    )
    .await
    .unwrap();
    let filters: WorkerFilters = serde_json::from_value(serde_json::json!({"compensation":{"min_minor":5000,"max_minor":6000,"currency":"USD","interval":"hour","include_unknown":false}})).unwrap();
    let mut config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: url,
            config: serde_json::json!({}),
        }],
        filters: filters.clone(),
        filters_by_source: Default::default(),
        run_started_at: Some("2026-10-13T13:00:00Z".into()),
        limits: WorkerLimits {
            request_timeout_s: 3,
            per_host_delay_ms: 0,
            concurrency: 1,
        },
    };
    let mut identities = vec![];
    for mode in 0..3 {
        if mode == 2 {
            config.filters = WorkerFilters::default();
            config
                .filters_by_source
                .insert(source.id.clone(), filters.clone());
        }
        let events = invoke(&config, mode == 0);
        let batch: Vec<_> = events
            .into_iter()
            .filter(|e| matches!(e, WorkerEvent::Job { .. }))
            .map(|e| scraped_job_from_event(e).unwrap())
            .collect();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].title, "Native Hourly Engineer");
        assert_eq!(
            batch[0].description_text.as_deref(),
            Some("About the role\n\nA & B")
        );
        assert!(batch[0]
            .description_source
            .as_deref()
            .unwrap()
            .contains("&lt;h2&gt;"));
        assert_eq!(
            batch[0].description_format,
            Some(scrape_core::enrich::DescriptionFormat::Html)
        );
        assert!(batch[0]
            .description
            .as_ref()
            .unwrap()
            .blocks
            .iter()
            .any(|block| matches!(block, scrape_core::enrich::DescriptionBlock::Heading { .. })));
        assert_eq!(batch[0].compensation[0].min_minor, Some(4525));
        assert_eq!(
            batch[0].url,
            format!("http://{address}/apply?gh_jid=42&utm_source=test")
        );
        let identity = scrape_core::PostingIdentity::for_posting(
            Some(source.kind),
            Some(&source.identifier),
            batch[0].external_id.as_deref(),
            &batch[0].url,
        )
        .unwrap()
        .key;
        identities.push(identity);
        if mode == 0 {
            assert_eq!(
                JobsRepo::list(&pool, JobFilter::default())
                    .await
                    .unwrap()
                    .total,
                0
            );
        } else if mode == 1 {
            assert_eq!(
                JobsRepo::upsert_scraped(&pool, &batch)
                    .await
                    .unwrap()
                    .inserted,
                1
            );
            let job = JobsRepo::list(&pool, JobFilter::default())
                .await
                .unwrap()
                .items
                .remove(0);
            assert_eq!(job.compensation.as_ref(), Some(&batch[0].compensation));
            assert_eq!(job.url, batch[0].url);
            assert_eq!(
                job.description_text.as_deref(),
                Some("About the role\n\nA & B")
            );
            let evidence: String =
                sqlx::query_scalar("SELECT description_source FROM jobs WHERE id = ?")
                    .bind(&job.id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert!(evidence.contains("&lt;h2&gt;"));
            let format: String =
                sqlx::query_scalar("SELECT description_format FROM jobs WHERE id = ?")
                    .bind(&job.id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(format, "html");
            assert!(job
                .description
                .as_ref()
                .unwrap()
                .blocks
                .iter()
                .any(|block| matches!(
                    block,
                    scrape_core::enrich::DescriptionBlock::Heading { .. }
                )));
            postings::remove(&pool, &job.id, true).await.unwrap();
        } else {
            assert_eq!(
                JobsRepo::upsert_scraped(&pool, &batch)
                    .await
                    .unwrap()
                    .inserted,
                0
            );
            assert_eq!(
                JobsRepo::list(&pool, JobFilter::default())
                    .await
                    .unwrap()
                    .total,
                0
            );
        }
    }
    assert!(identities.windows(2).all(|p| p[0] == p[1]));
    server.join().unwrap();
    pool.close().await;
}

#[tokio::test]
async fn recorded_lever_response_survives_actual_worker_and_repository() {
    let payload = include_str!("../../fixtures/providers/lever/sep.json");
    let expected: serde_json::Value = serde_json::from_str(payload).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).unwrap();
        assert!(String::from_utf8_lossy(&buffer[..n])
            .starts_with("GET /api.lever.co/v0/postings/sep?mode=json "));
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",payload.len(),payload).unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("recorded.db"))
        .await
        .unwrap();
    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Lever,
            name: "SEP".into(),
            identifier: "sep".into(),
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
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: source.identifier.clone(),
            config: serde_json::json!({}),
        }],
        filters: WorkerFilters::default(),
        filters_by_source: Default::default(),
        run_started_at: Some("2026-10-14T03:27:51Z".into()),
        limits: WorkerLimits {
            request_timeout_s: 3,
            per_host_delay_ms: 0,
            concurrency: 1,
        },
    };
    let events = invoke_replay(&config, false, Some(&format!("http://{address}")));
    assert!(!events
        .iter()
        .any(|event| matches!(event, WorkerEvent::Error { .. })));
    let jobs: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(jobs.len(), expected.as_array().unwrap().len());
    JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();
    let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(saved.total, jobs.len() as i64);
    let recent_id = saved.items[0].id.clone();
    sqlx::query("UPDATE jobs SET status_changed_at = '2030-01-01T03:00:00Z'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET status_changed_at = '2030-01-01 12:00:00' WHERE id = ?")
        .bind(&recent_id)
        .execute(&pool)
        .await
        .unwrap();
    let ordered = JobsRepo::list(
        &pool,
        JobFilter {
            sort: Some(scrape_lib::db::jobs::JobSort::StatusChangedAtDesc),
            limit: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(ordered.items[0].id, recent_id);
    for job in saved.items {
        let provider = expected
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"].as_str() == job.external_id.as_deref())
            .unwrap();
        assert_eq!(job.title, provider["text"].as_str().unwrap());
        assert_eq!(job.url, provider["hostedUrl"].as_str().unwrap());
        assert_eq!(
            job.posting_identity.as_deref(),
            Some(
                scrape_core::PostingIdentity::for_posting(
                    Some(SourceKind::Lever),
                    Some("sep"),
                    job.external_id.as_deref(),
                    &job.url
                )
                .unwrap()
                .key
                .as_str()
            )
        );
    }
    server.join().unwrap();
    pool.close().await;
}
