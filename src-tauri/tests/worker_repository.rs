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
                // Accepted sockets may inherit nonblocking mode on macOS.
                stream.set_nonblocking(false).unwrap();
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
                      {"@type":"JobPosting","identifier":"42","title":"Native Hourly Engineer","url":"/apply?gh_jid=42&utm_source=test","jobLocationType":"TELECOMMUTE","hiringOrganization":{"name":"Fixture Co"},"datePosted":"2026-10-12T09:00:00Z","description":"&lt;h2&gt;Available Locations&lt;/h2&gt;&lt;ul&gt;&lt;li&gt;US&lt;/li&gt;&lt;/ul&gt;&lt;h2&gt;About the role&lt;/h2&gt;&lt;p&gt;A &amp; B&lt;/p&gt;","baseSalary":{"currency":"USD","value":{"minValue":45.25,"maxValue":55.75,"unitText":"HOUR"}}},

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
        blocked_postings: vec![],
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
            enrichment: None,
            request_timeout_s: 3,
            per_host_delay_ms: 0,
            concurrency: 1,
            ..Default::default()
        },
        ..Default::default()
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
            Some("Available Locations\n\n• US\n\nAbout the role\n\nA & B")
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
        assert!(batch[0].location_data_complete);
        assert!(batch[0].locations.iter().any(|location| {
            location.country_code.as_deref() == Some("US")
                && location.raw == "Available Locations: US"
        }));
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
            let remote_us = JobsRepo::list(
                &pool,
                JobFilter {
                    countries: Some(vec!["US".into()]),
                    remote: Some("remote".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(remote_us.total, 1);
            assert_eq!(remote_us.items[0].url, batch[0].url);
            assert_eq!(job.compensation.as_ref(), Some(&batch[0].compensation));
            assert_eq!(job.url, batch[0].url);
            assert_eq!(
                job.description_text.as_deref(),
                Some("Available Locations\n\n• US\n\nAbout the role\n\nA & B")
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
        blocked_postings: vec![],
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
            enrichment: None,
            request_timeout_s: 3,
            per_host_delay_ms: 0,
            concurrency: 1,
            ..Default::default()
        },
        ..Default::default()
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

#[tokio::test]
async fn remote_us_eligibility_survives_greenhouse_worker_persistence_and_restart() {
    let captured: serde_json::Value = serde_json::from_str(include_str!(
        "../../fixtures/providers/regressions/cloudflare-8024849.json"
    ))
    .unwrap();
    assert_eq!(
        captured["fixture_kind"],
        "captured_provider_field_projection"
    );
    assert_eq!(captured["provider_job"]["id"], 8024849);
    let control: serde_json::Value = serde_json::from_str(include_str!(
        "../../fixtures/providers/regressions/remote-us-availability-authored.json"
    ))
    .unwrap();
    assert_eq!(
        control["fixture_kind"],
        "authored_regression_not_raw_provider_capture"
    );
    // Pair the captured Cloudflare posting with the authored negative control
    // that ensures unrelated company prose does not grant US eligibility.
    let jobs = vec![captured["provider_job"].clone(), control["jobs"][1].clone()];

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut requests = 0;
        while requests < 5 && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                // Accepted sockets may inherit nonblocking mode on macOS.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]);
                let path = request.split_whitespace().nth(1).unwrap_or_default();
                let body = if path.ends_with("/jobs?content=true") {
                    serde_json::json!({"jobs": jobs}).to_string()
                } else if path.ends_with("/robots.txt") {
                    "User-agent: *\nAllow: /".into()
                } else if path.starts_with("/boards.greenhouse.io/cloudflare/jobs/") {
                    "<html><body>Authored empty branding response</body></html>".into()
                } else {
                    assert!(
                        path.starts_with("/boards-api.greenhouse.io/"),
                        "Unexpected fixture request: {path}"
                    );
                    let id = path
                        .rsplit('/')
                        .next()
                        .unwrap_or_default()
                        .split('?')
                        .next()
                        .unwrap_or_default()
                        .parse::<i64>()
                        .unwrap();
                    jobs.iter()
                        .find(|job| job["id"].as_i64() == Some(id))
                        .unwrap()
                        .to_string()
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
                requests += 1;
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert_eq!(
            requests, 5,
            "one board, two details, robots and branding requests"
        );
    });

    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("remote-us.db");
    let pool = scrape_lib::db::init_db(&database_path).await.unwrap();
    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Greenhouse,
            name: "Cloudflare".into(),
            identifier: "cloudflare".into(),
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
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        blocked_postings: vec![],
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: source.identifier.clone(),
            config: serde_json::json!({}),
        }],
        filters: WorkerFilters::default(),
        filters_by_source: Default::default(),
        run_started_at: Some("2026-10-14T12:00:00Z".into()),
        limits: WorkerLimits {
            enrichment: None,
            request_timeout_s: 3,
            per_host_delay_ms: 0,
            concurrency: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let origin = format!("http://{address}");
    let events = invoke_replay(&config, false, Some(&origin));
    server.join().unwrap();
    let jobs: Vec<_> = events
        .into_iter()
        .filter(|event| matches!(event, WorkerEvent::Job { .. }))
        .map(|event| scraped_job_from_event(event).unwrap())
        .collect();
    assert_eq!(jobs.len(), 2);
    assert!(jobs
        .iter()
        .all(|job| job.workplace_type == scrape_core::WorkplaceType::Remote));
    let eligible = jobs
        .iter()
        .find(|job| job.external_id.as_deref() == Some("8024849"))
        .unwrap();
    assert!(eligible.location_data_complete);
    assert!(eligible.locations.iter().any(|location| {
        location.country_code.as_deref() == Some("US") && location.raw == "Available Locations: US"
    }));
    let boilerplate_only = jobs
        .iter()
        .find(|job| job.external_id.as_deref() == Some("4202"))
        .unwrap();
    assert!(!boilerplate_only.location_data_complete);
    assert!(boilerplate_only
        .locations
        .iter()
        .all(|location| location.country_code.is_none()));

    JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();
    let remote_us = JobFilter {
        countries: Some(vec!["US".into()]),
        remote: Some("remote".into()),
        ..Default::default()
    };
    let matched = JobsRepo::list(&pool, remote_us.clone()).await.unwrap();
    assert_eq!(matched.total, 1);
    assert_eq!(matched.items[0].external_id.as_deref(), Some("8024849"));

    pool.close().await;
    let reopened = scrape_lib::db::init_db(&database_path).await.unwrap();
    let after_restart = JobsRepo::list(&reopened, remote_us).await.unwrap();
    assert_eq!(after_restart.total, 1);
    assert_eq!(after_restart.items[0].url, eligible.url);
    reopened.close().await;
}

#[tokio::test]
async fn bounded_worker_refresh_preview_and_lower_override_preserve_saved_state() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut pages = 0;
        while pages < 4 && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut buf = [0; 4096];
                let n = stream.read(&mut buf).unwrap();
                let request = String::from_utf8_lossy(&buf[..n]);
                let body = if request.starts_with("GET /robots.txt ") {
                    "User-agent: *\nAllow: /".into()
                } else {
                    assert!(
                        request.starts_with("GET /careers "),
                        "Unnecessary detail request after reaching budget: {request}"
                    );
                    pages += 1;
                    let jobs: Vec<_> = (0..120).map(|id| serde_json::json!({"@type":"JobPosting","identifier":id.to_string(),"title":format!("Engineer {id}"),"url":format!("/jobs/{id}")})).collect();
                    format!("<script type=\"application/ld+json\">{}</script><a class=detail href=/unnecessary>More</a>", serde_json::json!(jobs))
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(pages, 4);
    });
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("bounded.db");
    let pool = scrape_lib::db::init_db(&db).await.unwrap();
    let spec = scrape_lib::services::workflow::validate(serde_json::json!({"version":3,"metadata":{"name":"Bounded fixture"},"extractor":{"kind":"jsonld","url":format!("http://{address}/careers"),"source_config":{"follow_links":"a.detail"}},"schedule":{"cadence":"manual"}}),"UTC").unwrap();
    let source = SourcesRepo::create(
        &pool,
        scrape_lib::services::workflow::source_input(&spec, "UTC").unwrap(),
    )
    .await
    .unwrap();
    let mut config = WorkerConfig {
        version: PROTOCOL_VERSION,
        blocked_postings: vec![],
        run_started_at: None,
        filters: WorkerFilters::default(),
        filters_by_source: Default::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        sources: vec![WorkerSource {
            id: source.id,
            kind: source.kind,
            name: source.name,
            identifier: source.identifier,
            config: source.config,
        }],
        ..Default::default()
    };
    let batch = |events: Vec<WorkerEvent>| {
        events
            .into_iter()
            .filter(|e| matches!(e, WorkerEvent::Job { .. }))
            .map(|e| scraped_job_from_event(e).unwrap())
            .collect::<Vec<_>>()
    };
    let first = batch(invoke(&config, false));
    assert_eq!(first.len(), 20);
    assert_eq!(
        JobsRepo::upsert_scraped(&pool, &first)
            .await
            .unwrap()
            .inserted,
        20
    );
    let saved = JobsRepo::list(&pool, JobFilter::default())
        .await
        .unwrap()
        .items
        .remove(0);
    JobsRepo::update_notes(&pool, &saved.id, Some("Keep my notes"))
        .await
        .unwrap();
    JobsRepo::set_favorite(&pool, &saved.id, true)
        .await
        .unwrap();
    JobsRepo::update_status(&pool, &saved.id, scrape_core::JobStatus::Applied)
        .await
        .unwrap();
    let preview = batch(invoke(&config, true));
    assert_eq!(preview.len(), 10);
    assert_eq!(
        JobsRepo::list(&pool, JobFilter::default())
            .await
            .unwrap()
            .total,
        20
    );
    let refreshed = batch(invoke(&config, false));
    let result = JobsRepo::upsert_scraped(&pool, &refreshed).await.unwrap();
    assert_eq!((result.inserted, result.updated), (0, 20));
    config.sources[0].config["__workflow_spec"]["max_jobs_per_run"] = 7.into();
    assert_eq!(batch(invoke(&config, false)).len(), 7);
    server.join().unwrap();
    pool.close().await;
    let reopened = scrape_lib::db::init_db(&db).await.unwrap();
    let preserved = JobsRepo::get(&reopened, &saved.id).await.unwrap();
    assert_eq!(preserved.notes.as_deref(), Some("Keep my notes"));
    assert!(preserved.favorited);
    assert_eq!(preserved.status, scrape_core::JobStatus::Applied);
    assert!(!JobsRepo::get_status_events(&reopened, &saved.id)
        .await
        .unwrap()
        .is_empty());
    reopened.close().await;
}

#[tokio::test]
async fn keyword_geography_decisions_and_diagnostics_match_worker_and_sql() {
    use scrape_core::matching::PostingRules;
    let cases: Vec<serde_json::Value> = vec![
        serde_json::json!({"keywords":{"any":["Engineer","Developer"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"all":["Engineer","Developer"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"any":["Engineer","Developer"],"all":["Rust"],"exclude":["Senior"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"exclude":["Engineer"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"all":["Engin"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"any":["Machine Learning"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"any":["Engineer"],"all":["Software"],"exclude":["Manager"],"fields":["title"]}}),
        serde_json::json!({"keywords":{"any":["Remote"],"fields":["location"]}}),
        serde_json::json!({"countries":["US"]}),
        serde_json::json!({"countries":["US"],"workplaces":["remote"]}),
        serde_json::json!({"countries":["US"],"regions":["US-CA"],"workplaces":["remote"]}),
        serde_json::json!({"countries":["CA"],"regions":["CA-ON"],"workplaces":["hybrid"]}),
        serde_json::json!({"countries":["US"],"regions":["ON"]}),
        serde_json::json!({"countries":["US"],"regions":["US-TX"],"workplaces":["onsite"]}),
        serde_json::json!({"countries":["US"],"location_include_unknown":true}),
        serde_json::json!({"countries":["US"],"regions":["US-CA"],"location_include_unknown":true}),
        serde_json::json!({"workplaces":["hybrid","onsite"]}),
        serde_json::json!({"countries":["US","CA"],"regions":["CA-ON"],"location_include_unknown":true}),
    ];
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let requests = cases.len() + 1;
    let server = std::thread::spawn(move || {
        let postings: Vec<_> = [
            ("remote-us","Remote - US","Remote"),
            ("remote-ca-state","Remote - US-CA","Remote"),
            ("hybrid-on","Toronto, Ontario, Canada","Hybrid"),
            ("onsite-tx","Austin, TX, United States","In-Office"),
            ("onsite-gb","London, England, UK","On-site"),
            ("unknown","Remote","Remote"),
            ("us-unknown-state","United States","Remote"),
            ("remote-canada","Remote (Canada)","Remote"),
            ("unknown-plus-canada","Remote; Canada","Remote"),
            ("onsite-ny","New York, US","In-Person"),
            ("multi","Remote - US-WA; Toronto, Ontario, Canada","Remote"),
            ("unknown-plus-us","Remote; US-CA","Remote"),
        ].into_iter().map(|(id,location,workplace)| serde_json::json!({"id":id,"title":match id { "hybrid-on" => "Machine Learning Engineer", "multi" => "Software Developer", "onsite-gb" => "Engineering Manager", _ => "Software Engineer" },"jobUrl":format!("https://fixture.test/{id}"),"location":location,"workplaceType":workplace})).collect();
        let body = serde_json::json!({"jobs":postings}).to_string();
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut count = 0;
        while count < requests * 3 && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = [0; 4096];
                let n = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                let path = request.split_whitespace().nth(1).unwrap();
                let response = match path {
                    "/api.ashbyhq.com/posting-api/job-board/geography?includeCompensation=true" => {
                        body.as_str()
                    }
                    "/fixture.test/robots.txt" => "User-agent: *\nAllow: /",
                    "/fixture.test/remote-us" => {
                        "<html><body>Authored empty branding response</body></html>"
                    }
                    _ => panic!("Unexpected fixture request: {path}"),
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.len(),
                    response
                )
                .unwrap();
                count += 1;
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(count, requests * 3);
    });
    let dir = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&dir.path().join("geography.db"))
        .await
        .unwrap();
    let spec=scrape_lib::services::workflow::validate(serde_json::json!({"version":3,"metadata":{"name":"Geography fixture"},"extractor":{"kind":"ashby","url":"geography"},"schedule":{"cadence":"manual"}}),"UTC").unwrap();
    let source = SourcesRepo::create(
        &pool,
        scrape_lib::services::workflow::source_input(&spec, "UTC").unwrap(),
    )
    .await
    .unwrap();
    let mut config = WorkerConfig {
        version: PROTOCOL_VERSION,
        run_started_at: None,
        blocked_postings: vec![],
        filters: WorkerFilters::default(),
        filters_by_source: Default::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        sources: vec![WorkerSource {
            id: source.id,
            kind: source.kind,
            name: source.name,
            identifier: source.identifier,
            config: source.config,
        }],
        ..Default::default()
    };
    let origin = format!("http://{address}");
    let unfiltered = invoke_replay(&config, false, Some(&origin));
    let all_candidates = unfiltered.clone();
    let rows: Vec<_> = unfiltered
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(rows.len(), 12);
    JobsRepo::upsert_scraped(&pool, &rows).await.unwrap();
    // Historical partial rows can retain a qualified subdivision without country.
    let changed =
        sqlx::query("UPDATE job_locations SET country_code=NULL WHERE region_code='US-CA'")
            .execute(&pool)
            .await
            .unwrap();
    assert!(changed.rows_affected() > 0);
    for value in cases {
        let rules: PostingRules = serde_json::from_value(value.clone()).unwrap();
        config.filters.matching = Some(rules.clone());
        let events = invoke_replay(&config, false, Some(&origin));
        let evaluator = scrape_core::matching::MatchingEvaluator::new(&config.filters).unwrap();
        let started = chrono::Utc::now();
        let mut expected_reasons = std::collections::BTreeMap::new();
        for event in all_candidates
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
        {
            let reasons = evaluator.rejection_reasons(event, started);
            assert_eq!(
                evaluator.matches_at(event, started),
                rules.matches_event_at(event, started)
            );
            if let Some(reason) = reasons.first() {
                *expected_reasons.entry(*reason).or_insert(0u64) += 1;
            }
        }
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!("Missing completion");
        };
        assert_eq!(
            stats.sources[0].rejected,
            expected_reasons.values().sum::<u64>()
        );
        assert_eq!(
            stats.sources[0]
                .rejections
                .iter()
                .map(|r| (r.reason, r.count))
                .collect::<std::collections::BTreeMap<_, _>>(),
            expected_reasons
        );
        let mut actual: Vec<_> = events
            .into_iter()
            .filter_map(|e| {
                if let WorkerEvent::Job { external_id, .. } = e {
                    Some(external_id)
                } else {
                    None
                }
            })
            .collect();
        let mut expected: Vec<_> = JobsRepo::list(
            &pool,
            JobFilter {
                matching: Some(rules),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|job| job.external_id.unwrap())
        .collect();
        actual.sort();
        expected.sort();
        if value
            == serde_json::json!({"countries":["US","CA"],"regions":["CA-ON"],"location_include_unknown":true})
        {
            assert!(!actual.contains(&"us-unknown-state".to_string()));
        }
        assert_eq!(actual, expected, "{value}");
        if value == serde_json::json!({"countries":["US"],"regions":["ON"]}) {
            assert!(
                actual.is_empty(),
                "country and region must match the same entry"
            );
        }
        if value == serde_json::json!({"countries":["US"],"location_include_unknown":true}) {
            assert!(actual.contains(&"unknown".to_string()));
            assert!(
                !actual.contains(&"unknown-plus-canada".to_string()),
                "a bare Remote row cannot override known Canadian geography"
            );
        }
    }
    server.join().unwrap();
    pool.close().await;
}

#[tokio::test]
async fn late_greenhouse_matches_refresh_with_provenance_and_protected_state() {
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while captured.lock().unwrap().len() < 14 && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap();
                captured.lock().unwrap().push(path.to_string());
                let body = if path.ends_with("?content=true") {
                    json!({"jobs":(0..200).map(|id|json!({"id":id,"title":if id<140 {"Sales"} else {"Engineer"},"absolute_url":format!("https://fixture.test/jobs/{id}"),"company_name":"Fixture","content":"<p>Listing evidence</p>"})).collect::<Vec<_>>(),"meta":{"total":200}}).to_string()
                } else if path.contains("?pay_transparency=true") {
                    let id: u64 = path
                        .rsplit('/')
                        .next()
                        .unwrap()
                        .split('?')
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap();
                    assert!((140..143).contains(&id));
                    json!({"id":id,"title":"Changed detail title","absolute_url":"https://changed.test/job","first_published":"2026-10-15T00:00:00Z"}).to_string()
                } else if path == "/fixture.test/employer.svg" {
                    r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><path fill="#f80" d="M0 0h32v32H0z"/></svg>"##.into()
                } else if path == "/fixture.test/robots.txt" {
                    "User-agent: *\nAllow: /".into()
                } else {
                    assert_eq!(path, "/fixture.test/jobs/140");
                    "<img alt='Fixture Logo' src='/employer.svg'>".into()
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(captured.lock().unwrap().len(), 14);
    });
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("late.db"))
        .await
        .unwrap();
    let spec=scrape_lib::services::workflow::validate(json!({"version":4,"metadata":{"name":"Late candidate fixture","company":"Fixture"},"extractor":{"kind":"greenhouse","url":"board"},"max_jobs_per_run":3,"schedule":{"cadence":"manual","timezone":"UTC"},"filters":{"matching":{"keywords":{"any":["Engineer"],"fields":["title"]},"posting_date":{"last_hours":72,"timezone":"UTC"}}}}),"UTC").unwrap();
    let source = SourcesRepo::create(
        &pool,
        scrape_lib::services::workflow::source_input(&spec, "UTC").unwrap(),
    )
    .await
    .unwrap();
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: "Fixture".into(),
            identifier: source.identifier.clone(),
            config: source.config.clone(),
        }],
        filters: scrape_lib::services::runner::resolve_worker_filters(
            &source.filters,
            WorkerFilters::default(),
        )
        .unwrap(),
        filters_by_source: Default::default(),
        run_started_at: Some("2026-10-16T00:00:00Z".into()),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        blocked_postings: vec![],
        ..Default::default()
    };
    let mut protected = None;
    for pass in 0..2 {
        let events = invoke_replay(&config, false, Some(&format!("http://{address}")));
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!("Missing completion")
        };
        let scan = &stats.sources[0];
        scan.validate().unwrap();
        assert_eq!((scan.examined, scan.rejected, scan.accepted), (143, 140, 3));
        assert_eq!(
            scan.outcome,
            Some(scrape_core::protocol::ScanOutcome::Success)
        );
        assert_eq!(scan.provider_total, Some(200));
        let mut budget =
            scrape_lib::services::ingestion::RunPostingBudget::new(&config.sources).unwrap();
        let rows: Vec<_> = events
            .into_iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .map(|event| {
                budget.take(event.source_id().unwrap()).unwrap();
                scraped_job_from_event(event).unwrap()
            })
            .collect();
        let stats = JobsRepo::upsert_scraped(&pool, &rows).await.unwrap();
        assert_eq!(
            if pass == 0 {
                stats.inserted
            } else {
                stats.updated
            },
            3
        );
        let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(saved.total, 3);
        for job in &saved.items {
            assert_eq!(job.title, "Engineer");
            assert!(job.url.starts_with("https://fixture.test/jobs/"));
            assert_eq!(job.date_posted.as_deref(), Some("2026-10-15T00:00:00Z"));
            assert_eq!(job.description_text.as_deref(), Some("Listing evidence"));
        }
        if pass == 0 {
            let id = saved.items[0].id.clone();
            sqlx::query("UPDATE jobs SET notes='Protected late-match note',favorited=1,status='applied' WHERE id=?").bind(&id).execute(&pool).await.unwrap();
            protected = Some(id);
        } else {
            let state: (String, bool, String) =
                sqlx::query_as("SELECT notes,favorited,status FROM jobs WHERE id=?")
                    .bind(protected.as_ref().unwrap())
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(
                state,
                ("Protected late-match note".into(), true, "applied".into())
            );
        }
    }
    server.join().unwrap();
    assert_eq!(
        requests
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.contains("?pay_transparency=true"))
            .count(),
        6
    );
    pool.close().await;
}

#[tokio::test]
async fn mixed_xpath_css_fields_survive_worker_preview_refresh_and_persistence() {
    verify_static_worker_persistence(false, false).await;
}
#[tokio::test]
async fn hydration_fields_survive_worker_preview_refresh_and_persistence() {
    verify_static_worker_persistence(true, false).await;
}
#[tokio::test]
async fn paginated_static_details_survive_preview_refresh_and_persistence() {
    verify_static_worker_persistence(false, true).await;
}
async fn verify_static_worker_persistence(hydration: bool, paginated: bool) {
    use serde_json::json;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut pages = 0;
        let mut robots = 0;
        let mut requests = 0;
        let expected_requests = if paginated { 18 } else { 6 };
        while requests < expected_requests && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap();
                let body = match path {
                    "/fixture.test/robots.txt" => {
                        robots += 1;
                        "User-agent: *\nAllow: /".to_string()
                    }
                    "/fixture.test/careers" => {
                        pages += 1;
                        if paginated {
                            "<article class='job-card' data-job-id='req-sales'><h2>Role: Sales</h2><a data-url='/jobs/sales'>Apply</a></article><a rel='next' href='/second'>Next</a>".into()
                        } else if hydration {
                            let jobs: Vec<_> = [("sales", "Sales"), ("42", "Rust Engineer"), ("43", "Platform Engineer"), ("44", "Later Engineer")].into_iter().map(|(id,title)| json!({"id":format!("req-{id}"),"title":format!("Role: {title}"),"url":format!("/jobs/{id}?ref=original"),"pay":"Pay: $45.25 - $55.75 per hour","published":"2026-10-15","body":"Original description"})).collect();
                            format!(
                                "<script id='__NEXT_DATA__' type='application/json'>{}</script>",
                                json!({"props":{"jobs":jobs}})
                            )
                        } else {
                            [("sales", "Sales"), ("42", "Rust Engineer"), ("43", "Platform Engineer"), ("44", "Later Engineer")].into_iter().map(|(id,title)| format!("<article class='job-card' data-job-id='req-{id}'><h2>Role: {title}</h2><a data-url='/jobs/{id}?ref=original'>Apply</a><span class='pay'>Pay: $45.25 - $55.75 per hour</span><time datetime='2026-10-15'></time><p>Original description</p></article>")).collect::<String>()
                        }
                    }
                    "/fixture.test/second" if paginated => "<main>Empty middle page</main><a rel='next' href='/third'>Next</a>".into(),
                    "/fixture.test/third" if paginated => format!("{}<a rel='next' href='/unvisited'>Next</a>", ["42","43","44"].into_iter().map(|id|format!("<article class='job-card' data-job-id='req-{id}'><h2>Role: {} Engineer</h2><a data-url='/jobs/{id}?ref=original'>Apply</a><a rel='detail' href='/evidence/{id}'>Details</a></article>",if id=="42" {"Rust"}else{"Platform"})).collect::<String>()),
                    "/fixture.test/evidence/42" | "/fixture.test/evidence/43" if paginated => "<h1>Changed title must be ignored</h1><main><p class=description>Original description</p><span class=pay>Pay: $45.25 - $55.75 per hour</span><time datetime='2026-10-15'></time></main>".into(),
                    _ => panic!("Unexpected extraction request: {path}"),
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
                requests += 1;
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!((pages, robots, requests), (3, 3, expected_requests));
    });
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("locators.db"))
        .await
        .unwrap();
    let mut extraction = if hydration {
        json!({
            "hydration_pointer":"/props/jobs",
            "hydration_fields":{"title":"/title","link":"/url","id":"/id","salary":"/pay","date":"/published","description":"/body"},
            "title_regex":"Role: (?P<value>.+)","id_regex":"req-(?P<value>.+)","salary_regex":"Pay: (?P<value>.+)","description_attribute":"text"
        })
    } else {
        json!({
            "item":"//article[contains(@class,'job-card')]", "title":"xpath:.//h2/text()", "title_regex":"Role: (?P<value>.+)",
            "link":"a@data-url", "id":".", "id_attribute":"data-job-id", "id_regex":"req-(?P<value>.+)",
            "date":"time", "date_attribute":"datetime", "description":"p", "description_attribute":"text",
            "salary":".pay", "salary_regex":"Pay: (?P<value>.+)"
        })
    };
    if paginated {
        extraction["next_page"] = "xpath:(//a[@rel='next'])[1]/@href".into();
        extraction["detail"] = "xpath:.//a[@rel='detail']/@href".into();
        extraction["detail_container"] = "main".into();
        extraction["detail_fields"] = json!({"description":".description","description_attribute":"text","salary":".pay","salary_regex":"Pay: (?P<value>.+)","date":"time"});
        extraction["wait_for"] = "body".into();
    }
    let spec = scrape_lib::services::workflow::validate(json!({
        "version":5,"metadata":{"name":"Mixed locator fixture"},"max_jobs_per_run":2,
        "sources":[{"id":"fixture", "label":"Fixture", "employer_hint":"Fixture", "extractor":{
            "kind":"selector", "url":"https://fixture.test/careers", "source_config":extraction
        }}],
        "schedule":{"cadence":"manual"}, "filters":{"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}
    }), "UTC").unwrap();
    let mut spec = spec;
    if paginated {
        let mut value = serde_json::to_value(&spec).unwrap();
        value["filters"]["matching"]["posting_date"] = json!({"last_hours":72,"timezone":"UTC"});
        value["filters"]["compensation"] = json!({"min_minor":5000,"max_minor":6000,"currency":"USD","interval":"hour","include_unknown":false});
        spec = scrape_lib::services::workflow::validate(value, "UTC").unwrap();
    }
    let source = SourcesRepo::create(
        &pool,
        scrape_lib::services::workflow::source_input(&spec, "UTC").unwrap(),
    )
    .await
    .unwrap();
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        max_jobs_per_run: Some(2),
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: source.identifier.clone(),
            config: source.config.clone(),
        }],
        filters: scrape_lib::services::runner::resolve_worker_filters(
            &source.filters,
            WorkerFilters::default(),
        )
        .unwrap(),
        run_started_at: Some("2026-10-16T00:00:00Z".into()),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut protected_id = None;
    for pass in 0..3 {
        let events = invoke_replay(&config, pass == 0, Some(&format!("http://{address}")));
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!("missing completion")
        };
        let scan = &stats.sources[0];
        scan.validate().unwrap();
        assert_eq!((scan.examined, scan.rejected, scan.accepted), (3, 1, 2));
        assert_eq!(
            scan.termination,
            scrape_core::protocol::CoverageTermination::JobLimit
        );
        let batch: Vec<_> = events
            .into_iter()
            .filter(|e| matches!(e, WorkerEvent::Job { .. }))
            .map(|e| scraped_job_from_event(e).unwrap())
            .collect();
        assert_eq!(
            batch
                .iter()
                .map(|j| j.external_id.as_deref())
                .collect::<Vec<_>>(),
            [Some("42"), Some("43")]
        );
        assert_eq!(batch[0].title, "Rust Engineer");
        assert_eq!(batch[0].url, "https://fixture.test/jobs/42?ref=original");
        assert_eq!(batch[0].date_posted.as_deref(), Some("2026-10-15"));
        assert_eq!(
            batch[0].description_text.as_deref(),
            Some("Original description")
        );
        if pass == 0 {
            assert_eq!(
                JobsRepo::list(&pool, JobFilter::default())
                    .await
                    .unwrap()
                    .total,
                0
            );
            continue;
        }
        let result = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(
            (result.inserted, result.updated),
            if pass == 1 { (2, 0) } else { (0, 2) }
        );
        let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(saved.total, 2);
        let pay_json: String =
            sqlx::query_scalar("SELECT compensation_json FROM jobs WHERE external_id='42'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let pay: serde_json::Value = serde_json::from_str(&pay_json).unwrap();
        assert_eq!(pay[0]["min_minor"], 4525);
        assert_eq!(pay[0]["max_minor"], 5575);
        assert_eq!(pay[0]["interval"], "hour");
        if pass == 1 {
            let id = saved.items[0].id.clone();
            sqlx::query(
                "UPDATE jobs SET notes='Keep locator note',favorited=1,status='applied' WHERE id=?",
            )
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();
            protected_id = Some(id);
        } else {
            let state: (String, bool, String) =
                sqlx::query_as("SELECT notes,favorited,status FROM jobs WHERE id=?")
                    .bind(protected_id.as_ref().unwrap())
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(state, ("Keep locator note".into(), true, "applied".into()));
        }
    }
    server.join().unwrap();
    pool.close().await;
}

#[tokio::test]
async fn browser_cdp_extraction_and_session_persistence_survive_repository_ingestion() {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha1::{Digest, Sha1};

    let ws_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let ws_addr = ws_listener.local_addr().unwrap();
    let ws_server = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = ws_listener.accept() {
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut buf = Vec::new();
            let mut byte = [0u8; 1];
            while !buf.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                buf.push(byte[0]);
            }
            let req = String::from_utf8_lossy(&buf);
            let key = req
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        Some(value.trim().to_string())
                    } else {
                        None
                    }
                })
                .unwrap();
            let mut sha1 = Sha1::new();
            sha1.update(key.as_bytes());
            sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
            let accept = BASE64.encode(sha1.finalize());
            let resp = format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
            );
            stream.write_all(resp.as_bytes()).unwrap();

            loop {
                let mut header = [0u8; 2];
                if stream.read_exact(&mut header).is_err() {
                    break;
                }
                if (header[0] & 0x0f) == 0x8 {
                    break;
                }
                let masked = (header[1] & 0x80) != 0;
                let mut len = (header[1] & 0x7f) as usize;
                if len == 126 {
                    let mut ext = [0u8; 2];
                    stream.read_exact(&mut ext).unwrap();
                    len = u16::from_be_bytes(ext) as usize;
                }
                let mut mask = [0u8; 4];
                if masked {
                    stream.read_exact(&mut mask).unwrap();
                }
                let mut payload = vec![0u8; len];
                stream.read_exact(&mut payload).unwrap();
                if masked {
                    for (i, b) in payload.iter_mut().enumerate() {
                        *b ^= mask[i % 4];
                    }
                }
                let msg: serde_json::Value = serde_json::from_slice(&payload).unwrap();
                let id = msg.get("id").and_then(|v| v.as_u64()).unwrap_or(1);
                let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
                let session_id = msg
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);
                let result = match method {
                    "Target.createBrowserContext" => {
                        serde_json::json!({ "browserContextId": "ctx-repo" })
                    }
                    "Target.createTarget" => serde_json::json!({ "targetId": "target-repo" }),
                    "Target.attachToTarget" => serde_json::json!({ "sessionId": "sess-repo" }),
                    "Network.setCookies" => serde_json::json!({ "success": true }),
                    "Runtime.evaluate" => {
                        let snapshot = serde_json::json!({
                            "readyState": "complete",
                            "href": "https://portal.fixture.test/careers",
                            "title": "Careers",
                            "hasPassword": false,
                            "html": r#"<html><body>
                                <article class="job" data-job-id="cdp-900">
                                  <h2>Staff Distributed Systems Engineer</h2>
                                  <a class="apply" href="https://portal.fixture.test/jobs/cdp-900">Apply</a>
                                  <span class="loc">Seattle, WA</span>
                                  <span class="pay">$190,000 - $220,000 a year</span>
                                  <p class="desc">Build resilient Rust consensus services in US locations.</p>
                                </article>
                            </body></html>"#
                        });
                        serde_json::json!({
                            "result": {
                                "type": "string",
                                "value": snapshot.to_string()
                            }
                        })
                    }
                    _ => serde_json::json!({}),
                };
                let mut reply = serde_json::json!({ "id": id, "result": result });
                if let Some(sid) = session_id {
                    reply["sessionId"] = serde_json::Value::String(sid);
                }
                let out_bytes = serde_json::to_vec(&reply).unwrap();
                let mut frame = vec![0x81];
                if out_bytes.len() <= 125 {
                    frame.push(out_bytes.len() as u8);
                } else {
                    frame.push(126);
                    frame.extend_from_slice(&(out_bytes.len() as u16).to_be_bytes());
                }
                frame.extend_from_slice(&out_bytes);
                let _ = stream.write_all(&frame);
            }
        }
    });

    let dir = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&dir.path().join("jobs.db"))
        .await
        .unwrap();
    let session_dir = dir.path().join("browser_sessions");
    std::fs::create_dir_all(&session_dir).unwrap();
    let session = scrape_core::browser::StoredBrowserSession {
        id: "workday_repo".into(),
        domain: "portal.fixture.test".into(),
        saved_at: "2026-10-17T00:00:00Z".into(),
        cookies: vec![scrape_core::browser::BrowserCookie {
            name: "sid".into(),
            value: "token-repo".into(),
            domain: ".portal.fixture.test".into(),
            path: "/".into(),
            secure: true,
            http_only: true,
            same_site: None,
            expires: None,
        }],
    };
    std::fs::write(
        session_dir.join("workday_repo.json"),
        serde_json::to_vec(&session).unwrap(),
    )
    .unwrap();

    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Selector,
            name: "Portal Browser Fixture".into(),
            identifier: "https://portal.fixture.test/careers".into(),
            config: Some(serde_json::json!({
                "url": "https://portal.fixture.test/careers",
                "mode": "browser",
                "session_id": "workday_repo",
                "wait_for": "article.job",
                "item": "article.job",
                "id": ".",
                "id_attribute": "data-job-id",
                "title": "h2",
                "link": "a.apply",
                "location": ".loc",
                "salary": ".pay",
                "description": ".desc"
            })),
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

    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: source.identifier.clone(),
            config: source.config.clone(),
        }],
        filters: WorkerFilters::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            navigation_timeout_s: 5,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut command = Command::new(worker());
    command
        .arg("--stdin")
        .env(
            "SCRAPE_CDP_WS_URL",
            format!("ws://{ws_addr}/devtools/browser/repo"),
        )
        .env("SCRAPE_SESSION_DIR", &session_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let jobs: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].external_id.as_deref(), Some("cdp-900"));
    assert_eq!(jobs[0].title, "Staff Distributed Systems Engineer");

    let stats = JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();
    assert_eq!(stats.inserted, 1);
    let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(saved.total, 1);
    assert_eq!(saved.items[0].title, "Staff Distributed Systems Engineer");

    let _ = ws_server.join();
    pool.close().await;
}

#[tokio::test]
async fn script_extraction_trust_gate_and_sqlite_persistence() {
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("jobs.db"))
        .await
        .unwrap();

    // 1. Setup loopback HTTP server for Source 1 (Static selector)
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let http_server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut served_page = false;
        while !served_page && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut buffer = [0; 4096];
                if let Ok(n) = stream.read(&mut buffer) {
                    let request = String::from_utf8_lossy(&buffer[..n]);
                    let body = if request.starts_with("GET /robots.txt ") {
                        "User-agent: *\nAllow: /\n"
                    } else {
                        served_page = true;
                        r#"<!DOCTYPE html><html><body>
                            <article data-id="static-1">
                                <h2>Platform Engineer</h2>
                                <a href="/jobs/platform">Apply</a>
                                <span class="location">Remote</span>
                                <span class="salary">$150,000</span>
                                <div class="desc"><p>Platform infra role</p></div>
                            </article>
                        </body></html>"#
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    });

    let script_code = r#"
const fs = require('fs');
const input = JSON.parse(fs.readFileSync(0, 'utf-8'));

if (!input.continuation) {
    // Turn 1: 1 matching job + continuation
    console.log(JSON.stringify({
        type: 'candidate',
        version: 1,
        external_id: 'script-1',
        title: 'Senior Backend Engineer',
        url: 'https://fixture.test/jobs/backend',
        location: 'Remote',
        salary_text: '$180,000',
        description: '<p>Backend microservices</p>',
        date_posted: '2026-10-15',
        company: 'Dynamic Systems',
        employer_logo_url: null
    }));
    console.log(JSON.stringify({
        type: 'continuation',
        version: 1,
        cursor: 'page=2'
    }));
    console.log(JSON.stringify({
        type: 'complete',
        version: 1,
        exhausted: false
    }));
} else {
    // Turn 2: 1 job that will be rejected by filter "Engineer" (Marketing Associate)
    console.log(JSON.stringify({
        type: 'candidate',
        version: 1,
        external_id: 'script-2',
        title: 'Junior Marketing Associate',
        url: 'https://fixture.test/jobs/marketing',
        location: 'New York, NY',
        salary_text: '$70,000',
        description: '<p>Growth marketing</p>',
        date_posted: '2026-10-16',
        company: 'Dynamic Systems',
        employer_logo_url: null
    }));
    console.log(JSON.stringify({
        type: 'complete',
        version: 1,
        exhausted: true
    }));
}
"#;

    // 2. Validate and import multi-source workflow spec
    let spec_json = serde_json::json!({
        "version": 6,
        "metadata": {
            "name": "Fair Budgeting Multi-Source Spec",
            "company": "Acme Group"
        },
        "sources": [
            {
                "id": "src-static",
                "label": "Static Jobs",
                "enabled": true,
                "extractor": {
                    "kind": "selector",
                    "url": format!("http://{addr}/jobs"),
                    "job_container": "article",
                    "title_selector": "h2",
                    "url_selector": "a"
                }
            },
            {
                "id": "src-script",
                "label": "Script Jobs",
                "enabled": true,
                "extractor": {
                    "kind": "script",
                    "url": "https://fixture.test/jobs",
                    "script": {
                        "runtime": "js",
                        "code": script_code
                    }
                }
            }
        ],
        "filters": {
            "include_keywords": ["Engineer"]
        }
    });

    let spec = scrape_lib::services::workflow::validate(spec_json, "UTC").unwrap();
    let quarantined = scrape_lib::services::script_trust::quarantine_import(spec.clone()).unwrap();
    let parent = scrape_lib::services::workflow::persist(&pool, quarantined, None, true, "UTC")
        .await
        .unwrap();

    // Verify imported workflow starts disabled and needs review
    assert!(parent.needs_review);
    assert!(!parent.enabled);

    let script_child = SourcesRepo::get(&pool, "src-script").await.unwrap();
    assert!(!script_child.script_trusted);
    assert!(script_child.needs_review);
    assert!(!script_child.enabled);

    let static_child = SourcesRepo::get(&pool, &parent.id).await.unwrap();

    let inputs = scrape_lib::services::workflow::workflow_source_inputs(&spec, "UTC").unwrap();
    let worker_static_src = WorkerSource {
        id: static_child.id.clone(),
        kind: inputs[0].kind,
        name: static_child.name.clone(),
        identifier: inputs[0].identifier.clone(),
        config: inputs[0].config.clone().unwrap(),
    };
    let worker_script_src = WorkerSource {
        id: script_child.id.clone(),
        kind: inputs[1].kind,
        name: script_child.name.clone(),
        identifier: inputs[1].identifier.clone(),
        config: inputs[1].config.clone().unwrap(),
    };

    // 3. Attempting to execute before trusting fails explicitly
    assert!(scrape_lib::services::script_trust::validate_execution(
        &pool,
        &[worker_static_src.clone(), worker_script_src.clone()]
    )
    .await
    .unwrap_err()
    .to_string()
    .contains("untrusted"));

    // 4. Explicitly trust the script
    let hash = script_child.script_sha256.as_deref().unwrap();
    let trusted_child =
        scrape_lib::services::script_trust::trust(&pool, &script_child.id, hash, true)
            .await
            .unwrap();
    assert!(trusted_child.script_trusted);
    assert!(!trusted_child.needs_review);

    // After trusting, validation passes
    scrape_lib::services::script_trust::validate_execution(
        &pool,
        &[worker_static_src.clone(), worker_script_src.clone()],
    )
    .await
    .unwrap();

    // 5. Run multi-source fair budgeting execution through packaged sidecar
    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![worker_static_src.clone(), worker_script_src.clone()],
        filters: WorkerFilters {
            include_keywords: vec!["Engineer".into()],
            ..Default::default()
        },
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            navigation_timeout_s: 5,
            ..Default::default()
        },
        ..Default::default()
    };

    let events = invoke(&config, false);

    // Collect matched jobs
    let jobs: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();

    // Exactly 2 jobs matched "Engineer" (1 from static source, 1 from script source)
    assert_eq!(jobs.len(), 2, "Expected 2 jobs, got: {jobs:?}");
    let titles: Vec<_> = jobs.iter().map(|j| j.title.as_str()).collect();
    assert!(
        titles.contains(&"Platform Engineer"),
        "Missing Platform Engineer in {titles:?}"
    );
    assert!(
        titles.contains(&"Senior Backend Engineer"),
        "Missing Senior Backend Engineer in {titles:?}"
    );
    // Verify Marketing Associate was rejected by filter and not emitted as Job
    assert!(!titles.contains(&"Junior Marketing Associate"));

    // Persist to SQLite
    let stats = JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();
    assert_eq!(stats.inserted, 2);

    let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(saved.total, 2);

    // 6. Editing script code invalidates trust
    let mut edited_config = script_child.config.clone();
    edited_config["script"]["code"] = "console.log('tampered')".into();
    let edited = SourcesRepo::update(
        &pool,
        scrape_lib::db::sources::UpdateSourceInput {
            id: script_child.id.clone(),
            identifier: None,
            name: None,
            config: Some(edited_config),
            enabled: Some(true),
            cadence: None,
            cadence_cron: None,
            retention_days: None,
            retention_policy: None,
            filters: None,
        },
    )
    .await
    .unwrap();
    assert!(!edited.script_trusted);
    assert!(edited.needs_review);
    assert!(!edited.enabled);

    assert!(scrape_lib::services::script_trust::require_trusted(
        &pool,
        &script_child.id,
        &edited.config
    )
    .await
    .is_err());

    let _ = http_server.join();
    pool.close().await;
}

#[tokio::test]
async fn recorded_provider_company_verification_matrix_persists_to_sqlite() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let bitwarden_payload =
        std::fs::read_to_string(root.join("fixtures/providers/greenhouse/bitwarden.json")).unwrap();
    let google_payload =
        std::fs::read_to_string(root.join("fixtures/providers/selector/google.html")).unwrap();
    let apple_payload =
        std::fs::read_to_string(root.join("fixtures/providers/selector/apple.html")).unwrap();
    let meta_payload =
        std::fs::read_to_string(root.join("fixtures/providers/meta/meta.html")).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();

    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while running_clone.load(std::sync::atomic::Ordering::Relaxed) && Instant::now() < deadline
        {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = match stream.read(&mut buffer) {
                    Ok(n) if n > 0 => n,
                    _ => continue,
                };
                let req = String::from_utf8_lossy(&buffer[..n]);
                let path = req.split_whitespace().nth(1).unwrap_or("");
                let (ctype, body) = if path.contains("bitwarden") {
                    ("application/json", bitwarden_payload.as_str())
                } else if path.contains("careers/applications") || path.contains("google") {
                    ("text/html; charset=utf-8", google_payload.as_str())
                } else if path.contains("apple") {
                    ("text/html; charset=utf-8", apple_payload.as_str())
                } else if path.contains("metacareers") || path.contains("meta") {
                    ("text/html; charset=utf-8", meta_payload.as_str())
                } else {
                    ("text/plain", "User-agent: *\nAllow: /")
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes());
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    });

    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("providers_matrix.db"))
        .await
        .unwrap();

    // 1. Bitwarden (Greenhouse ATS)
    let bitwarden_src = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Greenhouse,
            name: "Bitwarden".into(),
            identifier: "bitwarden".into(),
            config: None,
            enabled: Some(true),
            cadence: Some(scrape_core::SourceCadence::Manual),
            cadence_cron: None,
            retention_days: None,
            retention_policy: None,
            filters: None,
        },
    )
    .await
    .unwrap();

    let bw_config = WorkerConfig {
        version: PROTOCOL_VERSION,
        max_jobs_per_run: Some(100),
        sources: vec![WorkerSource {
            id: bitwarden_src.id.clone(),
            kind: bitwarden_src.kind,
            name: bitwarden_src.name.clone(),
            identifier: bitwarden_src.identifier.clone(),
            config: serde_json::json!({
                "__workflow_spec": { "max_jobs_per_run": 50 }
            }),
        }],
        filters: WorkerFilters::default(),
        limits: WorkerLimits {
            enrichment: Some(scrape_core::protocol::EnrichmentLimits {
                max_requests: 0,
                timeout_ms: 5000,
            }),
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let bw_events = invoke_replay(&bw_config, false, Some(&format!("http://{address}")));
    let bw_jobs: Vec<_> = bw_events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(bw_jobs.len(), 39);
    let bw_stats = JobsRepo::upsert_scraped(&pool, &bw_jobs).await.unwrap();
    assert_eq!(bw_stats.inserted, 39);

    // Verify canonical identity format on saved jobs
    let bw_saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(bw_saved.total, 39);
    for job in &bw_saved.items {
        let id_str = job.posting_identity.as_ref().unwrap();
        assert!(id_str.contains("\"greenhouse\"") && id_str.contains("\"bitwarden\""));
    }

    // 2. Google Careers (Static Selector)
    let google_src = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Selector,
            name: "Google".into(),
            identifier: "https://www.google.com/about/careers/applications/jobs/results/".into(),
            config: Some(serde_json::json!({
                "mode": "static",
                "item": "li.lLd3Je",
                "title": "h3.QJPWVe",
                "link": "a[href*='jobs/results/']",
                "location": "span.r0wTof",
            })),
            enabled: Some(true),
            cadence: Some(scrape_core::SourceCadence::Manual),
            cadence_cron: None,
            retention_days: None,
            retention_policy: None,
            filters: None,
        },
    )
    .await
    .unwrap();

    let google_config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: google_src.id.clone(),
            kind: google_src.kind,
            name: google_src.name.clone(),
            identifier: google_src.identifier.clone(),
            config: google_src.config.clone(),
        }],
        filters: WorkerFilters::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let g_events = invoke_replay(&google_config, false, Some(&format!("http://{address}")));
    let g_jobs: Vec<_> = g_events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(g_jobs.len(), 20);
    let g_stats = JobsRepo::upsert_scraped(&pool, &g_jobs).await.unwrap();
    assert_eq!(g_stats.inserted, 20);

    // 3. Apple Jobs (Static Selector)
    let apple_src = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Selector,
            name: "Apple".into(),
            identifier: "https://jobs.apple.com/en-us/search".into(),
            config: Some(serde_json::json!({
                "mode": "static",
                "item": "ul#search-job-list > li.rc-accordion-item",
                "title": "h3 a.link-inline",
                "link": "a.link-inline",
                "location": "span.table--advanced-search__location-sub",
                "date": "span.job-posted-date",
            })),
            enabled: Some(true),
            cadence: Some(scrape_core::SourceCadence::Manual),
            cadence_cron: None,
            retention_days: None,
            retention_policy: None,
            filters: None,
        },
    )
    .await
    .unwrap();

    let apple_config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: apple_src.id.clone(),
            kind: apple_src.kind,
            name: apple_src.name.clone(),
            identifier: apple_src.identifier.clone(),
            config: apple_src.config.clone(),
        }],
        filters: WorkerFilters::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let a_events = invoke_replay(&apple_config, false, Some(&format!("http://{address}")));
    let a_jobs: Vec<_> = a_events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(a_jobs.len(), 20);
    let a_stats = JobsRepo::upsert_scraped(&pool, &a_jobs).await.unwrap();
    assert_eq!(a_stats.inserted, 20);

    // 4. Netflix (Versioned Script Extractor)
    let netflix_extractor_path = root
        .join("crates/scrape-worker/examples/extractors/netflix_extractor.js")
        .to_string_lossy()
        .to_string();
    let netflix_fixture_path = root
        .join("fixtures/providers/script/netflix.json")
        .to_string_lossy()
        .to_string();

    let netflix_src = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Selector,
            name: "Netflix".into(),
            identifier: "https://explore.jobs.netflix.net/api/apply/v2/jobs".into(),
            config: Some(serde_json::json!({
                "mode": "script",
                "script": {
                    "runtime": "js",
                    "interpreter": "node",
                    "path": netflix_extractor_path,
                    "args": [netflix_fixture_path]
                }
            })),
            enabled: Some(true),
            cadence: Some(scrape_core::SourceCadence::Manual),
            cadence_cron: None,
            retention_days: None,
            retention_policy: None,
            filters: None,
        },
    )
    .await
    .unwrap();

    let nf_hash = netflix_src.script_sha256.as_deref().unwrap();
    scrape_lib::services::script_trust::trust(&pool, &netflix_src.id, nf_hash, true)
        .await
        .unwrap();

    let nf_config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: netflix_src.id.clone(),
            kind: netflix_src.kind,
            name: netflix_src.name.clone(),
            identifier: netflix_src.identifier.clone(),
            config: netflix_src.config.clone(),
        }],
        filters: WorkerFilters::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let nf_events = invoke(&nf_config, false);
    let nf_jobs: Vec<_> = nf_events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(nf_jobs.len(), 10);
    for job in &nf_jobs {
        assert_eq!(job.company, "Netflix");
    }
    let nf_stats = JobsRepo::upsert_scraped(&pool, &nf_jobs).await.unwrap();
    assert_eq!(nf_stats.inserted, 10);

    // 5. Meta (Truthful acceptance gap / empty DOM shell)
    let meta_config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: "meta-test".into(),
            kind: SourceKind::Selector,
            name: "Meta".into(),
            identifier: "https://www.metacareers.com/jobs".into(),
            config: serde_json::json!({
                "mode": "static",
                "item": ".job-item",
                "title": ".job-title",
                "link": "a",
                "__workflow_spec": { "max_jobs_per_run": 20 }
            }),
        }],
        filters: WorkerFilters::default(),
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    let meta_events = invoke_replay(&meta_config, false, Some(&format!("http://{address}")));
    let meta_jobs: Vec<_> = meta_events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .collect();
    assert_eq!(meta_jobs.len(), 0, "Meta returns 0 jobs from static DOM");

    // Total jobs in SQLite: 39 (Bitwarden) + 20 (Google) + 20 (Apple) + 10 (Netflix) = 89
    let total_saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(total_saved.total, 89);

    running.store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = server.join();
    pool.close().await;
}

/// Plan 12 section 1: the actual seed service and runner preparation, not a
/// hand-built replacement source. Cloudflare is an explicitly authored control;
/// Bitwarden and SEP are immutable recorded provider captures.
#[tokio::test]
async fn seeded_workflow_rules_explain_zero_results_through_discovered_query() {
    use scrape_lib::db::runs::{RunCounts, RunsRepo, TriggerType};
    use scrape_lib::services::{execution_diagnostics, sample_workflows};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let bitwarden =
        std::fs::read_to_string(root.join("fixtures/providers/greenhouse/bitwarden.json")).unwrap();
    let sep = std::fs::read_to_string(root.join("fixtures/providers/lever/sep.json")).unwrap();
    let cloudflare = serde_json::json!({"meta":{"total":3},"jobs":[
        {"id":91001,"title":"Software Engineer","absolute_url":"https://fixture.invalid/cloudflare/91001","location":{"name":"Austin, TX"},"content":"Authored fixture. Hybrid engineering."},
        {"id":91002,"title":"Software Engineering Intern","absolute_url":"https://fixture.invalid/cloudflare/91002","location":{"name":"Austin, TX"},"content":"Authored fixture. Hybrid internship."},
        {"id":91003,"title":"Sales Manager","absolute_url":"https://fixture.invalid/cloudflare/91003","location":{"name":"Austin, TX"},"content":"Authored fixture. Hybrid sales."}
    ]}).to_string();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let running = Arc::new(AtomicBool::new(true));
    let requests = Arc::new(Mutex::new(Vec::new()));
    let active = running.clone();
    let observed = requests.clone();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(30);
        while active.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap_or(0);
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap_or("");
            observed.lock().unwrap().push(path.to_string());
            let (status, body) = match path {
                "/boards-api.greenhouse.io/v1/boards/cloudflare/jobs?content=true" => {
                    ("200 OK", cloudflare.as_str())
                }
                "/boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true" => {
                    ("200 OK", bitwarden.as_str())
                }
                "/api.lever.co/v0/postings/sep?mode=json" => ("200 OK", sep.as_str()),
                _ if path.ends_with("/robots.txt") => ("200 OK", "User-agent: *\nAllow: /"),
                // No uncaptured detail/branding request may masquerade as a listing.
                _ => ("404 Not Found", "Uncaptured optional fixture resource"),
            };
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    for (board, expected) in [
        ("cloudflare", [3, 2, 1]),
        ("bitwarden", [20, 20, 0]),
        ("sep", [8, 6, 0]),
    ] {
        for (variant, expected_jobs) in expected.into_iter().enumerate() {
            let directory = tempfile::tempdir().unwrap();
            let pool = scrape_lib::db::init_db(&directory.path().join("seeded.db"))
                .await
                .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                1
            );
            let seeded = sample_workflows::seed_all(&pool).await.unwrap();
            let source = seeded.iter().find(|s| s.identifier == board).unwrap();
            let run =
                RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
                    .await
                    .unwrap();
            let mut config =
                execution_diagnostics::prepare(source, "UTC", &run.started_at).unwrap();
            config.filters=match variant {
                0 => WorkerFilters::default(),
                1 => serde_json::from_value(serde_json::json!({"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}})).unwrap(),
                _ => config.filters,
            };
            config.limits.per_host_delay_ms = 0;
            config.limits.enrichment = Some(scrape_core::protocol::EnrichmentLimits {
                max_requests: 0,
                timeout_ms: 5000,
            });
            execution_diagnostics::preflight(&pool, &run.id, &source.id, &config)
                .await
                .unwrap();
            let events = invoke_replay(&config, false, Some(&origin));
            let WorkerEvent::Done { stats } = events.last().unwrap() else {
                panic!("missing terminal stats")
            };
            assert_eq!(
                stats.jobs_emitted,
                expected_jobs,
                "{board}, variant {variant}; stats={stats:?}; errors={:?}; requests={:?}",
                events
                    .iter()
                    .filter(|e| matches!(e, WorkerEvent::Error { .. }))
                    .collect::<Vec<_>>(),
                requests.lock().unwrap()
            );
            let scan = stats.sources[0].clone();
            scan.validate().unwrap();
            assert_eq!(scan.accepted, expected_jobs);
            if board == "bitwarden" && variant == 2 {
                assert_eq!((scan.examined, scan.rejected, scan.accepted), (39, 39, 0));
                assert!(scan.rejections.iter().any(|r| r.reason
                    == scrape_core::matching::RejectionReason::RoleType
                    && r.count == 23));
            }
            if board == "sep" && variant == 2 {
                assert_eq!((scan.examined, scan.rejected, scan.accepted), (8, 8, 0));
                assert!(scan.rejections.iter().any(|r| r.reason
                    == scrape_core::matching::RejectionReason::Geography
                    && r.count == 6));
            }
            let batch: Vec<_> = events
                .into_iter()
                .filter(|e| matches!(e, WorkerEvent::Job { .. }))
                .map(|e| scraped_job_from_event(e).unwrap())
                .collect();
            let mut counts = RunCounts {
                fetched: scan.examined as i64,
                rejected: scan.rejected as i64,
                sources: Some(vec![scan]),
                ..Default::default()
            };
            execution_diagnostics::persist_batch(&pool, &run.id, &batch, &mut counts)
                .await
                .unwrap();
            let filter = JobFilter {
                statuses: Some(vec![
                    scrape_core::JobStatus::Scraped,
                    scrape_core::JobStatus::Viewed,
                ]),
                ..Default::default()
            };
            assert_eq!(
                JobsRepo::list(&pool, filter.clone()).await.unwrap().total,
                expected_jobs as i64
            );
            let saved = RunsRepo::get(&pool, &run.id).await.unwrap();
            assert_eq!(saved.counts.unwrap().inserted, expected_jobs as i64);
            assert_eq!(saved.effective_config.unwrap().filters, config.filters);
            assert!(RunsRepo::get_logs(&pool, &run.id).await.unwrap()[0]
                .message
                .contains("execution_preparation"));
            if expected_jobs > 0 {
                let first = JobsRepo::list(&pool, filter.clone())
                    .await
                    .unwrap()
                    .items
                    .remove(0);
                JobsRepo::update_status(&pool, &first.id, scrape_core::JobStatus::Applied)
                    .await
                    .unwrap();
                execution_diagnostics::persist_batch(&pool, &run.id, &batch, &mut counts)
                    .await
                    .unwrap();
                assert_eq!(
                    JobsRepo::list(&pool, filter).await.unwrap().total,
                    expected_jobs as i64 - 1,
                    "refresh must preserve tracked status"
                );
                assert_eq!(counts.updated, expected_jobs as i64);
            }
            pool.close().await;
        }
    }
    running.store(false, Ordering::SeqCst);
    server.join().unwrap();
    let requests = requests.lock().unwrap();
    for path in [
        "/boards-api.greenhouse.io/v1/boards/cloudflare/jobs?content=true",
        "/boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true",
        "/api.lever.co/v0/postings/sep?mode=json",
    ] {
        assert_eq!(
            requests.iter().filter(|p| p.as_str() == path).count(),
            3,
            "each variant must use the same listing once"
        );
    }
}
