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
    let mut command = Command::new(worker());
    command.arg("--stdin");
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
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Sidecar fixture timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
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
                      {"@type":"JobPosting","identifier":"42","title":"Native Hourly Engineer","url":"/apply?gh_jid=42&utm_source=test","hiringOrganization":{"name":"Fixture Co"},"datePosted":"2026-10-12T09:00:00Z","baseSalary":{"currency":"USD","value":{"minValue":45.25,"maxValue":55.75,"unitText":"HOUR"}}},
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
