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
        },
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
        while captured.lock().unwrap().len() < 12 && Instant::now() < deadline {
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
        assert_eq!(captured.lock().unwrap().len(), 12);
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
