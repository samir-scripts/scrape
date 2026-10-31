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
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn worker() -> std::path::PathBuf {
    scrape_lib::services::process::packaged_worker_path().unwrap()
}
fn invoke(config: &WorkerConfig, preview: bool) -> Vec<WorkerEvent> {
    invoke_replay(config, preview, None)
}
fn invoke_replay(config: &WorkerConfig, preview: bool, origin: Option<&str>) -> Vec<WorkerEvent> {
    // This event-only helper exercises non-persisting sample mode; persistence
    // feedback is exercised separately with an interactive SQLite boundary.
    let mut sample_config = config.clone();
    sample_config.commit_feedback = false;
    let cwd = tempfile::tempdir().unwrap();
    let mut command = Command::new(worker());
    command.current_dir(cwd.path());
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
        .write_all(&serde_json::to_vec(&sample_config).unwrap())
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
async fn independent_web_discovery_persists_recent_us_jobs() {
    use scrape_lib::db::runs::{RunsRepo, TriggerType};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    };

    // Replay retains public HTTPS identities while routing requests to this local server.
    struct Server {
        origin: String,
        paths: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            self.thread.take().unwrap().join().unwrap();
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let paths = Arc::new(Mutex::new(Vec::new()));
    let captured = paths.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let thread = std::thread::spawn(move || {
        while !stopping.load(Ordering::SeqCst) {
            let (mut socket, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("{error}"),
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 8192];
            let Ok(n) = socket.read(&mut request) else {
                continue;
            };
            let request = String::from_utf8_lossy(&request[..n]);
            let Some(path) = request.split_whitespace().nth(1) else {
                continue;
            };
            captured.lock().unwrap().push(path.to_owned());
            let (status, content_type, body) = if path.ends_with("/robots.txt") {
                (
                    "200 OK",
                    "text/plain",
                    "User-agent: *\nAllow: /\n".to_owned(),
                )
            } else if path.ends_with("/sitemap.xml") {
                (
                    "200 OK",
                    "application/xml",
                    "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"></urlset>"
                        .to_owned(),
                )
            } else if path == "/directory.example/start" {
                (
                    "200 OK",
                    "text/html",
                    "<p>Directory starting page</p>".to_owned(),
                )
            } else if path == "/directory.example/companies" {
                (
                    "200 OK",
                    "text/html",
                    "<a href=\"https://employer.example/careers\">Employer careers</a>".to_owned(),
                )
            } else if path == "/employer.example/careers" {
                let mut body = String::new();
                for id in ["old", "undated", "foreign", "updated", "blocked"]
                    .into_iter()
                    .map(str::to_owned)
                    .chain((0..25).map(|n| format!("new-{n:02}")))
                {
                    body.push_str(&format!(
                        "<a href=\"/jobs/{id}\">Software Engineer {id}</a>"
                    ));
                }
                body.push_str("<a href=\"/jobs/new-00\">Duplicate job</a>");
                ("200 OK", "text/html", body)
            } else if let Some(id) = path.strip_prefix("/employer.example/jobs/") {
                let date = match id {
                    "old" => Some("2026-10-12T00:00:00Z"),
                    "undated" => None,
                    _ => Some("2026-10-15T00:00:00Z"),
                };
                let mut job = serde_json::json!({
                    "@context":"https://schema.org", "@type":"JobPosting",
                    "identifier":id, "title":format!("Software Engineer {id}"),
                    "hiringOrganization":{"name":"Fixture Employer"},
                    "jobLocation":{"@type":"Place","address":{"addressCountry":if id == "foreign" {"GB"} else {"US"},"addressRegion":"CA"}},
                    "description":"Build reliable distributed software systems."
                });
                if let Some(date) = date {
                    job["datePosted"] = date.into();
                }
                (
                    "200 OK",
                    "text/html",
                    format!("<script type=\"application/ld+json\">{job}</script>"),
                )
            } else if path == "/second.example/careers" {
                let body = (0..10)
                    .map(|n| format!("<a href=\"/jobs/second-{n:02}\">Software Engineer</a>"))
                    .collect::<String>();
                ("200 OK", "text/html", body)
            } else if let Some(id) = path.strip_prefix("/second.example/jobs/") {
                let job = serde_json::json!({
                    "@type":"JobPosting", "identifier":id, "title":format!("Software Engineer {id}"),
                    "hiringOrganization":{"name":"Second Fixture Employer"}, "datePosted":"2026-10-15T00:00:00Z",
                    "jobLocation":{"address":{"addressCountry":"US","addressRegion":"CA"}},
                    "description":"Build reliable distributed software systems."
                });
                (
                    "200 OK",
                    "text/html",
                    format!("<script type=\"application/ld+json\">{job}</script>"),
                )
            } else {
                ("404 Not Found", "text/plain", "not found".to_owned())
            };
            let _ = write!(socket, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        }
    });
    let server = Server {
        origin,
        paths,
        stop,
        thread: Some(thread),
    };

    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("discovery-quota.db"))
        .await
        .unwrap();
    let seed = "https://directory.example/companies";
    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::WebDiscovery,
            name: "Directory".into(),
            identifier: "https://directory.example/start".into(),
            config: Some(serde_json::json!({"max_domains":4,"max_depth":4,"max_pages":100})),
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
    let second_source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::WebDiscovery,
            name: "Second directory".into(),
            identifier: "https://second.example/careers".into(),
            config: Some(serde_json::json!({"max_domains":4,"max_depth":4,"max_pages":100})),
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
    for id in ["updated", "blocked"] {
        let event: WorkerEvent = serde_json::from_value(serde_json::json!({
            "type":"job","source_id":source.id,"external_id":id,
            "title":format!("Software Engineer {id}"),"company":"Fixture Employer",
            "url":format!("https://employer.example/jobs/{id}"),"location":"California, US","role_type":"unknown"
        })).unwrap();
        JobsRepo::upsert_scraped(&pool, &[scraped_job_from_event(event).unwrap()])
            .await
            .unwrap();
    }
    sqlx::query("UPDATE jobs SET notes='preserve', favorited=1 WHERE external_id='updated'")
        .execute(&pool)
        .await
        .unwrap();
    let blocked_id: String = sqlx::query_scalar("SELECT id FROM jobs WHERE external_id='blocked'")
        .fetch_one(&pool)
        .await
        .unwrap();
    postings::remove(&pool, &blocked_id, true).await.unwrap();
    let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
        .await
        .unwrap();
    let config = serde_json::json!({
        "version":PROTOCOL_VERSION, "commit_feedback":true, "max_jobs_per_run":20,
        "run_started_at":"2026-10-16T00:00:00Z",
        "sources":[{"id":source.id,"name":"Directory","kind":"web_discovery","identifier":"https://directory.example/start",
            "config":{"seed_urls":[seed],"max_domains":4,"max_depth":4,"max_pages":100,"seed_manifest":{"crawl_id":"CC-MAIN-2026-43","imported_at":"2026-10-16T00:00:00Z"}}},
            {"id":second_source.id,"name":"Second directory","kind":"web_discovery","identifier":"https://second.example/careers","config":{"max_domains":4,"max_depth":4,"max_pages":100}}],
        "filters":{"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,
            "posting_date":{"preset":"past_48h","timezone":"UTC","include_unknown":false}}},
        "limits":{"per_host_delay_ms":0,"request_timeout_s":3,"max_pages_per_source":100}
    });
    let mut child = Command::new(worker())
        .args(["--stdin", "--replay-origin", &server.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{config}").unwrap();
    let (sender, receiver) = mpsc::channel();
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let mut stderr = child.stderr.take().unwrap();
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut outcomes = Vec::new();
    let mut observed = Vec::new();
    let summary = loop {
        let line = match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(line) => line.unwrap(),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Discovery worker did not complete: {error}");
            }
        };
        let event: WorkerEvent = serde_json::from_str(&line).unwrap();
        match &event {
            WorkerEvent::Job {
                source_id,
                external_id,
                ..
            } => {
                observed.push(external_id.clone());
                let row = scraped_job_from_event(event.clone()).unwrap();
                let result = JobsRepo::upsert_scraped_for_run(&pool, &[row], Some(&run.id))
                    .await
                    .unwrap();
                let outcome = match (result.inserted, result.updated, result.blocked) {
                    (1, 0, 0) => "inserted",
                    (0, 1, 0) => "updated",
                    (0, 0, 1) => "blocked",
                    other => panic!("invalid outcome: {other:?}"),
                };
                outcomes.push(outcome);
                writeln!(stdin, "{}", serde_json::json!({"source_id":source_id,"external_id":external_id,"outcome":outcome})).unwrap();
            }
            WorkerEvent::Done { stats } => break stats.clone(),
            _ => {}
        }
    };
    drop(stdin);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("Discovery worker did not exit after Done");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let status = child.wait().unwrap();
    reader.join().unwrap();
    let stderr = stderr_reader.join().unwrap();
    assert!(status.success(), "{}", String::from_utf8_lossy(&stderr));
    assert_eq!(
        outcomes
            .iter()
            .filter(|&&outcome| outcome == "inserted")
            .count(),
        20,
        "observed={observed:?} outcomes={outcomes:?} summary={summary:#?} paths={:?}",
        server.paths.lock().unwrap()
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|&&outcome| outcome == "updated")
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|&&outcome| outcome == "blocked")
            .count(),
        1
    );
    assert_eq!(observed.len(), 22);
    assert!(observed.contains(&"updated".to_owned()) && observed.contains(&"blocked".to_owned()));
    for id in ["old", "undated", "foreign"] {
        assert!(
            server
                .paths
                .lock()
                .unwrap()
                .iter()
                .any(|path| path == &format!("/employer.example/jobs/{id}")),
            "{id} was not examined"
        );
        assert!(
            !observed.iter().any(|value| value == id),
            "{id} passed strict filters"
        );
    }
    assert_eq!(
        summary
            .sources
            .iter()
            .map(|scan| scan.newly_committed)
            .sum::<u64>(),
        20
    );
    assert!(summary
        .sources
        .iter()
        .any(|scan| scan.termination == scrape_core::protocol::CoverageTermination::JobLimit));
    let inserted: Vec<_> = observed
        .iter()
        .filter(|id| id.starts_with("new-") || id.starts_with("second-"))
        .collect();
    assert_eq!(inserted.len(), 20);
    assert_eq!(
        inserted
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        20
    );
    assert!(
        inserted.iter().any(|id| id.starts_with("new-"))
            && inserted.iter().any(|id| id.starts_with("second-"))
    );
    assert_eq!(
        server
            .paths
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.as_str() == "/employer.example/jobs/new-00")
            .count(),
        1
    );
    let preserved: (String, i64) =
        sqlx::query_as("SELECT notes, favorited FROM jobs WHERE external_id='updated'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(preserved, ("preserve".into(), 1));
    let job_count: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(job_count, 22); // 20 new, one updated, one retained blocked tombstone.
    let active_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE removed_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active_count, 21);
    let observations: Vec<(String, String, String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT j.external_id, o.run_id, json_extract(o.provenance, '$.seed_url'), json_extract(o.provenance, '$.final_url'), json_extract(o.provenance, '$.date_evidence.raw_value'), json_extract(o.provenance, '$.seed_manifest.crawl_id') FROM discovery_observations o JOIN jobs j ON j.id=o.job_id"
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(observations.len(), 21);
    for (id, run_id, observed_seed, final_url, date, crawl_id) in observations {
        assert_eq!(run_id, run.id);
        if id.starts_with("second-") {
            assert_eq!(observed_seed, "https://second.example/careers");
            assert_eq!(final_url, format!("https://second.example/jobs/{id}"));
            assert_eq!(crawl_id, None);
        } else {
            assert_eq!(observed_seed, seed);
            assert_eq!(final_url, format!("https://employer.example/jobs/{id}"));
            assert_eq!(crawl_id.as_deref(), Some("CC-MAIN-2026-43"));
        }
        assert_eq!(date, "2026-10-15T00:00:00Z");
        assert_ne!(id, "blocked");
    }
}

#[tokio::test]
async fn interactive_feedback_counts_only_committed_new_discoveries() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        for stream in listener.incoming().take(3) {
            let mut stream = stream.unwrap();
            let mut request = [0; 4096];
            let n = stream.read(&mut request).unwrap();
            let body = if String::from_utf8_lossy(&request[..n]).starts_with("GET /robots.txt ") {
                "User-agent: *\nAllow: /".to_string()
            } else {
                let jobs: Vec<_> = (0..5).map(|n| serde_json::json!({
                    "@type":"JobPosting","identifier":format!("{n}"),"title":format!("Engineer {n}"),
                    "url":format!("http://{address}/job/{n}"),"hiringOrganization":{"name":"Fixture"}
                })).collect();
                format!(
                    "<script type=\"application/ld+json\">{}</script>",
                    serde_json::to_string(&jobs).unwrap()
                )
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("quota.db"))
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
    let second_url = format!("http://{address}/careers-second");
    let second_source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Jsonld,
            name: "Second fixture".into(),
            identifier: second_url.clone(),
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
    let existing: WorkerEvent = serde_json::from_value(serde_json::json!({
        "type":"job","source_id":source.id,"external_id":"0","title":"Engineer 0","company":"Fixture",
        "url":format!("http://{address}/job/0"),"location":"","role_type":"unknown"
    })).unwrap();
    JobsRepo::upsert_scraped(&pool, &[scraped_job_from_event(existing).unwrap()])
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET notes='preserve', favorited=1 WHERE external_id='0'")
        .execute(&pool)
        .await
        .unwrap();
    let blocked: WorkerEvent = serde_json::from_value(serde_json::json!({
        "type":"job","source_id":source.id,"external_id":"1","title":"Engineer 1","company":"Fixture",
        "url":format!("http://{address}/job/1"),"location":"","role_type":"unknown"
    })).unwrap();
    JobsRepo::upsert_scraped(&pool, &[scraped_job_from_event(blocked).unwrap()])
        .await
        .unwrap();
    let blocked_id: String = sqlx::query_scalar("SELECT id FROM jobs WHERE external_id='1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    postings::remove(&pool, &blocked_id, true).await.unwrap();
    let config = WorkerConfig {
        commit_feedback: true,
        max_jobs_per_run: Some(2),
        sources: vec![
            WorkerSource {
                id: source.id.clone(),
                kind: SourceKind::Jsonld,
                name: "Fixture".into(),
                identifier: url,
                config: serde_json::json!({}),
            },
            WorkerSource {
                id: second_source.id.clone(),
                kind: SourceKind::Jsonld,
                name: "Second fixture".into(),
                identifier: second_url,
                config: serde_json::json!({}),
            },
        ],
        limits: WorkerLimits {
            per_host_delay_ms: 0,
            request_timeout_s: 3,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut child = Command::new(worker())
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", serde_json::to_string(&config).unwrap()).unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut observed = Vec::new();
    let mut outcomes = Vec::new();
    let mut summary = None;
    loop {
        let mut line = String::new();
        if output.read_line(&mut line).unwrap() == 0 {
            break;
        }
        let event: WorkerEvent = serde_json::from_str(&line).unwrap();
        match &event {
            WorkerEvent::Job {
                source_id,
                external_id,
                ..
            } => {
                observed.push(external_id.clone());
                let row = scraped_job_from_event(event.clone()).unwrap();
                let result = JobsRepo::upsert_scraped(&pool, &[row]).await.unwrap();
                let outcome = match (result.inserted, result.updated, result.blocked) {
                    (1, 0, 0) => "inserted",
                    (0, 1, 0) => "updated",
                    (0, 0, 1) => "blocked",
                    _ => panic!("invalid outcome"),
                };
                outcomes.push(outcome);
                writeln!(stdin, "{}", serde_json::json!({"source_id":source_id,"external_id":external_id,"outcome":outcome})).unwrap();
            }
            WorkerEvent::Done { stats } => {
                summary = Some(stats.clone());
                break;
            }
            _ => {}
        }
    }
    drop(stdin);
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    server.join().unwrap();
    assert_eq!(observed, ["0", "1", "2", "3"]);
    assert_eq!(outcomes, ["updated", "blocked", "inserted", "inserted"]);
    let scans = summary.unwrap().sources;
    assert_eq!(scans.iter().map(|scan| scan.accepted).sum::<u64>(), 4);
    assert_eq!(
        scans.iter().map(|scan| scan.newly_committed).sum::<u64>(),
        2
    );
    assert!(scans.iter().map(|scan| scan.duplicates).sum::<u64>() > 0);
    let preserved: (String, i64) =
        sqlx::query_as("SELECT notes, favorited FROM jobs WHERE external_id='0'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(preserved, ("preserve".into(), 1));
    assert_eq!(
        JobsRepo::list(&pool, JobFilter::default())
            .await
            .unwrap()
            .total,
        3
    );
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
async fn workday_cxs_survives_worker_and_sqlite() {
    let payload = include_str!("../../crates/scrape-worker/tests/fixtures/workday_jobs.json");
    let expected: serde_json::Value = serde_json::from_str(payload).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stopping = stop.clone();
    let server = std::thread::spawn(move || {
        while !stopping.load(std::sync::atomic::Ordering::SeqCst) {
            if let Ok((mut stream, _)) = listener.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                let req = String::from_utf8_lossy(&buffer[..n]);
                if req.contains("robots.txt") {
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nUser-agent: *\nAllow: /").unwrap();
                } else if req.starts_with("GET ") {
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html><head><title>TestCorp</title></head><body>Careers</body></html>").unwrap();
                } else if req.starts_with("POST ") {
                    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", payload.len(), payload).unwrap();
                }
            } else {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    });

    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("workday.db"))
        .await
        .unwrap();
    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Workday,
            name: "TestCorp".into(),
            identifier: "https://testcorp.wd5.myworkdayjobs.com/Careers".into(),
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
        max_jobs_per_run: Some(10),
        ..Default::default()
    };

    let events = invoke_replay(&config, false, Some(&format!("http://{address}")));
    if let Some(err) = events
        .iter()
        .find(|e| matches!(e, WorkerEvent::Error { .. }))
    {
        panic!("Unexpected worker error event: {err:?}");
    }
    let jobs: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();

    assert_eq!(jobs.len(), 3);
    let stats = JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();
    assert_eq!(stats.inserted, 3);

    let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(saved.total, 3);
    for job in saved.items {
        assert!(expected["jobPostings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["title"].as_str() == Some(&job.title)));
    }
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
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
        filters: scrape_lib::services::runner::resolve_worker_filters(&source.filters).unwrap(),
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
        let rows: Vec<_> = events
            .into_iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .map(|event| scraped_job_from_event(event).unwrap())
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
        filters: scrape_lib::services::runner::resolve_worker_filters(&source.filters).unwrap(),
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
        version: 2,
        external_id: 'script-1',
        title: 'Senior Backend Engineer',
        url: 'https://fixture.test/jobs/backend',
        location: 'Remote',
        salary: '$180,000',
        description: '<p>Backend microservices</p>',
        date_posted: '2026-10-15',
        company: 'Dynamic Systems',
        employer_logo_url: null
    }));
    console.log(JSON.stringify({
        type: 'continuation',
        version: 2,
        cursor: 'page=2'
    }));
    console.log(JSON.stringify({
        type: 'complete',
        version: 2,
        exhausted: false
    }));
} else {
    // Turn 2: 1 job that will be rejected by filter "Engineer" (Marketing Associate)
    console.log(JSON.stringify({
        type: 'candidate',
        version: 2,
        external_id: 'script-2',
        title: 'Junior Marketing Associate',
        url: 'https://fixture.test/jobs/marketing',
        location: 'New York, NY',
        salary: '$70,000',
        description: '<p>Growth marketing</p>',
        date_posted: '2026-10-16',
        company: 'Dynamic Systems',
        employer_logo_url: null
    }));
    console.log(JSON.stringify({
        type: 'complete',
        version: 2,
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
                        "interpreter": fixture_node_path(),
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

    let script_id = parent.config["__workflow_spec"]["sources"][1]["id"]
        .as_str()
        .unwrap();
    let script_child = SourcesRepo::get(&pool, script_id).await.unwrap();
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
    assert_eq!(
        jobs.iter()
            .find(|job| job.title == "Senior Backend Engineer")
            .unwrap()
            .salary_raw
            .as_deref(),
        Some("$180,000")
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

/// Manifest-selected canonical six-template acceptance. Known missing captures
/// stay partial and Meta stays unsupported; neither is a successful ingestion.
#[tokio::test]
async fn canonical_provider_workflow_matrix_reaches_discovered_with_truthful_coverage() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    if std::env::var_os("SCRAPE_VERIFY_NODE").is_none() {
        std::env::set_var("SCRAPE_VERIFY_NODE", fixture_node_path());
    }
    let report = scrape_lib::services::provider_verification::verify(root, false, false)
        .await
        .unwrap();
    assert_eq!(report["mode"], "recorded_workflows");
    let checks = report["checks"].as_array().unwrap();
    let failures: Vec<_> = checks.iter().filter(|c| c["status"] == "failed").collect();
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(checks
        .iter()
        .any(|c| c["companies"] == serde_json::json!(["Cloudflare"])));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "meta-unsupported" && c["status"] == "unsupported"));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "netflix-amazon-template" && c["status"] == "partial"));
    assert!(checks.iter().any(
        |c| c["id"] == "netflix-amazon-missing-pagination-metadata" && c["status"] == "partial"
    ));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "google-recorded-template" && c["status"] == "partial"));
    assert!(checks
        .iter()
        .any(|c| c["id"] == "apple-recorded-template" && c["status"] == "partial"));
    for company in [
        "Cloudflare",
        "Bitwarden",
        "Linear",
        "SEP",
        "Google",
        "Apple",
        "Netflix",
        "Amazon",
    ] {
        for control in ["template", "unrestricted", "strict"] {
            assert!(
                checks.iter().any(|c| c["control"] == control
                    && c["companies"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v == company)),
                "missing canonical {company} {control} control"
            );
        }
    }
}

#[test]
fn provider_acceptance_rejects_wrong_destination_missing_pagination_evidence_and_control_drift() {
    use scrape_lib::services::provider_verification::validate_observation;
    use serde_json::json;
    let expected = json!({
        "jobs":[{"external_id":"42","url":"https://jobs.example/42","location":"Cupertino","posting_identity":"[\"publisher\",\"https://jobs.example\",\"42\"]"}],
        "terminal":"exhausted","errors":[],
        "source_counts":{"Fixture":{"examined":1,"accepted":1}},
        "required_routes":["/jobs.example/search","/jobs.example/search?page=2"]
    });
    let valid = json!({
        "jobs":[{"external_id":"42","url":"https://jobs.example/42","location":"Cupertino","posting_identity":"[\"publisher\",\"https://jobs.example\",\"42\"]"}],
        "source_companies":{"fixture":"Fixture"},
        "source_stats":[{"source_id":"fixture","examined":1,"accepted":1,"termination":"exhausted","unresolved":0}],
        "errors":[],"inserted":1,"discovered":1,"saved_filters_match":true,
        "routes":{"requests":["/jobs.example/search","/jobs.example/search?page=2"],"unexpected_routes":[]}
    });
    assert!(validate_observation(&expected, &valid).is_ok());
    let mut wrong = valid.clone();
    wrong["jobs"][0]["url"] = "https://jobs.example/jobs/jobs/42".into();
    assert!(validate_observation(&expected, &wrong)
        .unwrap_err()
        .contains("destinations"));
    let mut wrong_identity = valid.clone();
    wrong_identity["jobs"][0]["posting_identity"] = serde_json::Value::Null;
    assert!(validate_observation(&expected, &wrong_identity)
        .unwrap_err()
        .contains("posting_identity evidence"));
    let mut missing_page = valid.clone();
    missing_page["routes"]["requests"] = json!(["/jobs.example/search"]);
    assert!(validate_observation(&expected, &missing_page)
        .unwrap_err()
        .contains("page request"));
    let mut missing_evidence = valid.clone();
    missing_evidence["jobs"][0]["location"] = serde_json::Value::Null;
    assert!(validate_observation(&expected, &missing_evidence)
        .unwrap_err()
        .contains("location evidence"));
    let mut unresolved = valid.clone();
    unresolved["source_stats"][0]["unresolved"] = 1.into();
    assert!(validate_observation(&expected, &unresolved)
        .unwrap_err()
        .contains("required field evidence"));
    let mut wrong_counter = valid.clone();
    wrong_counter["source_stats"][0]["accepted"] = 0.into();
    assert!(validate_observation(&expected, &wrong_counter)
        .unwrap_err()
        .contains("accepted mismatch"));
    let mut unexpected = valid.clone();
    unexpected["routes"]["unexpected_routes"] = json!(["GET /jobs.example/unknown"]);
    assert!(validate_observation(&expected, &unexpected)
        .unwrap_err()
        .contains("Unexpected fixture route"));
    let mut drift = valid.clone();
    drift["saved_filters_match"] = false.into();
    assert!(validate_observation(&expected, &drift)
        .unwrap_err()
        .contains("controls changed"));
    let mut missing_summary = valid.clone();
    missing_summary["source_stats"] = json!([]);
    assert!(validate_observation(&expected, &missing_summary)
        .unwrap_err()
        .contains("source scan evidence"));
    let mut premature_exhaustion = valid;
    premature_exhaustion["source_stats"][0]["termination"] = "job_limit".into();
    assert!(validate_observation(&expected, &premature_exhaustion)
        .unwrap_err()
        .contains("terminal coverage"));
}

#[tokio::test]
async fn canonical_provider_preflight_rejects_missing_owned_child_before_launch() {
    use scrape_lib::db::runs::{RunsRepo, TriggerType};
    use scrape_lib::services::{execution_diagnostics, provider_verification_profiles, workflow};
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("missing-child.db"))
        .await
        .unwrap();
    let parent = provider_verification_profiles::save(&pool, "bigtech-static-swe")
        .await
        .unwrap();
    let mut spec = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC").unwrap();
    spec.metadata.enabled = Some(true);
    for child in &mut spec.sources {
        child.enabled = true;
    }
    let parent = workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC")
        .await
        .unwrap();
    let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&parent.id))
        .await
        .unwrap();
    let config = execution_diagnostics::prepare(&parent, "UTC", &run.started_at).unwrap();
    assert_eq!(config.sources.len(), 2);
    sqlx::query("DELETE FROM sources WHERE id=?")
        .bind(&config.sources[1].id)
        .execute(&pool)
        .await
        .unwrap();
    let error = execution_diagnostics::preflight(&pool, &run.id, &parent.id, &config)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no owned record"), "{error}");
    assert!(RunsRepo::get_logs(&pool, &run.id)
        .await
        .unwrap()
        .iter()
        .any(|l| l.message.contains("No worker was launched")));
    pool.close().await;
}

/// Plan 12 section 1: the actual seed service and runner preparation, not a
/// hand-built replacement source. Cloudflare is an explicitly authored control;
/// Bitwarden and SEP are immutable recorded provider captures.
#[tokio::test]
async fn seeded_workflow_rules_explain_zero_results_through_discovered_query() {
    use scrape_lib::db::runs::{RunCounts, RunsRepo, TriggerType};
    use scrape_lib::services::{execution_diagnostics, provider_verification_profiles, workflow};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let bitwarden =
        std::fs::read_to_string(root.join("fixtures/providers/greenhouse/bitwarden.json")).unwrap();
    let linear =
        std::fs::read_to_string(root.join("fixtures/providers/ashby/linear.json")).unwrap();
    let sep = std::fs::read_to_string(root.join("fixtures/providers/lever/sep.json")).unwrap();
    let cloudflare = serde_json::json!({"meta":{"total":3},"jobs":[
        {"id":91001,"title":"Software Engineer","absolute_url":"https://fixture.invalid/cloudflare/91001","location":{"name":"Austin, TX"},"content":"Authored fixture. Hybrid work model for engineering."},
        {"id":91002,"title":"Software Engineering Intern","absolute_url":"https://fixture.invalid/cloudflare/91002","location":{"name":"Austin, TX"},"content":"Authored fixture. Hybrid work model for this internship."},
        {"id":91003,"title":"Sales Manager","absolute_url":"https://fixture.invalid/cloudflare/91003","location":{"name":"Austin, TX"},"content":"Authored fixture. Hybrid work model for sales."}
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
                "/api.ashbyhq.com/posting-api/job-board/linear?includeCompensation=true" => {
                    ("200 OK", linear.as_str())
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
        ("cloudflare", vec![(0, 3), (1, 2), (2, 1), (3, 2)]),
        ("bitwarden", vec![(0, 20), (1, 20), (2, 0), (3, 20)]),
        ("sep", vec![(0, 8), (1, 6), (2, 6), (3, 6)]),
        ("linear", vec![(0, 20), (1, 10), (3, 20)]),
    ] {
        for (variant, expected_jobs) in expected {
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
            let profile_id = match board {
                "cloudflare" => "cloudflare-systems-network",
                "bitwarden" => "bitwarden-remote-security",
                "sep" => "sep-midwest-consulting",
                "linear" => "linear-product-engineering",
                _ => unreachable!("fixture board has a verification profile"),
            };
            let mut source = provider_verification_profiles::save(&pool, profile_id)
                .await
                .unwrap();
            if variant == 2 {
                // Keep the exact restrictive matching profile separate from production seeds.
                let strict =
                    provider_verification_profiles::profile(&format!("{profile_id}-restrictive"))
                        .unwrap();
                let mut spec =
                    workflow::validate(source.config["__workflow_spec"].clone(), "UTC").unwrap();
                spec.filters = strict.filters.clone();
                source = workflow::persist(&pool, spec, Some(source.id.clone()), false, "UTC")
                    .await
                    .unwrap();
            }
            let run =
                RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
                    .await
                    .unwrap();
            let mut config =
                execution_diagnostics::prepare(&source, "UTC", &run.started_at).unwrap();
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
                assert_eq!((scan.examined, scan.rejected, scan.accepted), (8, 2, 6));
                assert!(scan.rejections.iter().any(|r| r.reason
                    == scrape_core::matching::RejectionReason::KeywordAny
                    && r.count == 2));
                assert!(!scan
                    .rejections
                    .iter()
                    .any(|r| r.reason == scrape_core::matching::RejectionReason::Geography));
            }
            let batch: Vec<_> = events
                .into_iter()
                .filter(|e| matches!(e, WorkerEvent::Job { .. }))
                .map(|e| scraped_job_from_event(e).unwrap())
                .collect();
            if board == "sep" && variant == 2 {
                let mut ids: Vec<_> = batch
                    .iter()
                    .map(|job| job.external_id.as_deref().unwrap())
                    .collect();
                ids.sort();
                assert_eq!(
                    ids,
                    [
                        "0b14158a-68e4-4e94-86f8-3f74c87cb3bd",
                        "2eb79283-b3fd-4316-9e93-e80dbebfba05",
                        "4efbdbce-a753-41b5-8ed7-0661cd193178",
                        "80cd8c15-bafc-49a9-b8f1-5d407e891ec0",
                        "98fc318a-8ce1-4730-a898-3ca372878bb2",
                        "f7ad9ffb-03dc-4fb2-9a92-e5f04a85ba08"
                    ],
                    "fixed SEP geography must return the independently expected postings"
                );
            }
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
    for (path, count) in [
        (
            "/boards-api.greenhouse.io/v1/boards/cloudflare/jobs?content=true",
            4,
        ),
        (
            "/boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true",
            4,
        ),
        ("/api.lever.co/v0/postings/sep?mode=json", 4),
        (
            "/api.ashbyhq.com/posting-api/job-board/linear?includeCompensation=true",
            3,
        ),
    ] {
        assert_eq!(
            requests.iter().filter(|p| p.as_str() == path).count(),
            count,
            "each explicit control/default variant must use the same listing once"
        );
    }
}

/// Authored selector control using the actual seeded Google/Apple configuration.
/// Unrestricted matching isolates ownership/persistence from the known strict rules.
#[tokio::test]
async fn seeded_multi_source_selector_jobs_persist_with_parent_attribution() {
    use scrape_lib::{
        db::runs::{RunCounts, RunsRepo, TriggerType},
        services::{execution_diagnostics, provider_verification_profiles},
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let mut pages = 0;
        let deadline = Instant::now() + Duration::from_secs(10);
        while pages < 2 && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap();
            let body = match path {
                "/www.google.com/about/careers/applications/jobs/results/" => {
                    pages += 1;
                    "<ul><li class='lLd3Je'><h3 class='QJPWVe'>Authored Google Engineer</h3><a href='https://www.google.com/about/careers/applications/jobs/results/authored-1'>Job</a><span class='r0wTof'>Austin</span></li></ul>"
                }
                "/jobs.apple.com/en-us/search" => {
                    pages += 1;
                    "<ul id='search-job-list'><li class='rc-accordion-item'><h3><a class='link-inline' href='https://jobs.apple.com/en-us/details/authored-2'>Authored Apple Engineer</a></h3></li></ul>"
                }
                _ if path.ends_with("/robots.txt") => "User-agent: *\nAllow: /",
                _ => panic!("unexpected fixture route {path}"),
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        assert_eq!(pages, 2);
    });
    let dir = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&dir.path().join("multi.db"))
        .await
        .unwrap();
    let parent = provider_verification_profiles::save(&pool, "bigtech-static-swe")
        .await
        .unwrap();
    let mut spec =
        scrape_lib::services::workflow::validate(parent.config["__workflow_spec"].clone(), "UTC")
            .unwrap();
    spec.metadata.enabled = Some(true);
    for entry in &mut spec.sources {
        entry.enabled = true;
    }
    let parent =
        scrape_lib::services::workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC")
            .await
            .unwrap();
    let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&parent.id))
        .await
        .unwrap();
    let mut config = execution_diagnostics::prepare(&parent, "UTC", &run.started_at).unwrap();
    config.filters = Default::default();
    config.limits.per_host_delay_ms = 0;
    execution_diagnostics::preflight(&pool, &run.id, &parent.id, &config)
        .await
        .unwrap();
    let events = invoke_replay(&config, false, Some(&origin));
    server.join().unwrap();
    let batch: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(batch.len(), 2);
    assert_ne!(batch[0].source_id, batch[1].source_id);
    execution_diagnostics::persist_batch(&pool, &run.id, &batch, &mut RunCounts::default())
        .await
        .unwrap();
    let saved = JobsRepo::list(
        &pool,
        JobFilter {
            source_id: Some(parent.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(saved.total, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE workflow_id=?")
            .bind(&parent.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    assert!(saved
        .items
        .iter()
        .any(|j| j.title == "Authored Google Engineer"));
    assert!(saved
        .items
        .iter()
        .any(|j| j.title == "Authored Apple Engineer"));
    pool.close().await;
}

/// Labelled authored fixtures: exact compatibility rules through import/save,
/// preview and run worker processes, ingestion and the Discovered parent query.
#[tokio::test]
async fn compatibility_rules_survive_saved_workflow_preview_run_and_discovered() {
    use scrape_lib::services::{execution_diagnostics, workflow};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    let cases = [
        (
            json!({"locations":["Remote","Austin"],"matching":{}}),
            vec!["1", "2", "3", "5", "6"],
        ),
        (
            json!({"include_keywords":["Software"],"matching":{"keywords":{"any":["Engineer"],"fields":["title"]}}}),
            vec!["2", "5", "6"],
        ),
        (
            json!({"query":"title LIKE '%Engineer%' OR title LIKE '%Developer%' OR title LIKE '%Intern%'","matching":{}}),
            vec!["2", "3", "5", "6"],
        ),
        (
            json!({"query":"title LIKE '%Engineer%' OR title LIKE '%Intern%'","matching":{}}),
            vec!["2", "3", "5", "6"],
        ),
        (
            json!({"include_keywords":["Austin"],"matching":{"keywords":{"any":["Engineer","Developer"],"all":["Software"],"exclude":["Senior"],"fields":["title"]}}}),
            vec!["2"],
        ),
        (
            json!({"exclude_keywords":["Remote"],"matching":{"keywords":{"any":["Systems"],"fields":["description"]}}}),
            vec!["1", "2", "4", "6"],
        ),
        (
            json!({"role_types":["full_time"],"matching":{"role_types":["internship"]}}),
            vec![],
        ),
        (
            json!({"query":"location LIKE '%Austin%'","policy":{"title":"%Software%"},"matching":{"keywords":{"exclude":["Senior"],"fields":["title"]}}}),
            vec!["1", "2"],
        ),
    ];
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let pages = Arc::new(AtomicUsize::new(0));
    let server_stop = stop.clone();
    let served = pages.clone();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        while !server_stop.load(Ordering::SeqCst) {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap();
            let body = match path {
                "/fixture.test/robots.txt" => "User-agent: *\nAllow: /".to_string(),
                "/fixture.test/compatibility" => {
                    served.fetch_add(1, Ordering::SeqCst);
                    [("1","Software Designer","Austin"),("2","Software Engineer","Austin"),("3","Engineer","Remote"),("4","Designer","London"),("5","Software Engineer","Remote"),("6","Senior Software Engineer","Austin")].into_iter().map(|(id,title,location)|format!("<article data-id='{id}'><h2>{title}</h2><a href='/jobs/{id}'>Apply</a><span>{location}</span><p>Systems work</p></article>")).collect::<String>()
                }
                _ => panic!("unexpected compatibility fixture route {path}"),
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("compatibility.db"))
        .await
        .unwrap();
    for (index, (filters, expected)) in cases.iter().enumerate() {
        let document = json!({"version":6,"metadata":{"name":format!("Authored compatibility {index}")},"sources":[{"id":"fixture","label":"Fixture","employer_hint":"Fixture","extractor":{"kind":"selector","url":"https://fixture.test/compatibility","source_config":{"item":"article","id":".","id_attribute":"data-id","title":"h2","link":"a","location":"span","description":"p","description_attribute":"text"}}}],"schedule":{"cadence":"manual"},"filters":filters});
        let spec = workflow::validate_legacy_import(
            document,
            "UTC",
            "0 0 9 * * *",
            &WorkerFilters {
                include_keywords: vec!["must-not-inherit".into()],
                ..Default::default()
            },
            false,
        )
        .unwrap();
        let saved = workflow::persist(&pool, spec, None, true, "UTC")
            .await
            .unwrap();
        let reopened = SourcesRepo::get(&pool, &saved.id).await.unwrap();
        let mut config =
            execution_diagnostics::prepare(&reopened, "UTC", "2026-10-18T00:00:00Z").unwrap();
        config.limits.per_host_delay_ms = 0;
        let original: WorkerFilters = serde_json::from_value(filters.clone()).unwrap();
        assert_eq!(config.filters, original);
        for preview in [true, false] {
            let events = invoke_replay(&config, preview, Some(&origin));
            let WorkerEvent::Done { stats } = events.last().unwrap() else {
                panic!("missing completion")
            };
            let scan = &stats.sources[0];
            scan.validate().unwrap();
            assert_eq!(
                (scan.examined, scan.rejected, scan.unresolved, scan.accepted),
                (6, 6 - expected.len() as u64, 0, expected.len() as u64),
                "case {index}"
            );
            assert_eq!(
                scan.termination,
                scrape_core::protocol::CoverageTermination::Exhausted
            );
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, WorkerEvent::Error { .. })),
                "case {index}: {events:?}"
            );
            let batch: Vec<_> = events
                .into_iter()
                .filter(|event| matches!(event, WorkerEvent::Job { .. }))
                .map(|event| scraped_job_from_event(event).unwrap())
                .collect();
            let mut ids: Vec<_> = batch
                .iter()
                .map(|job| job.external_id.as_deref().unwrap())
                .collect();
            ids.sort();
            assert_eq!(&ids, expected, "case {index}, preview {preview}");
            if !preview {
                let stats = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
                assert_eq!(stats.inserted, expected.len() as i64);
            }
            let discovered = JobsRepo::list(
                &pool,
                JobFilter {
                    source_id: Some(saved.id.clone()),
                    statuses: Some(vec![
                        scrape_core::JobStatus::Scraped,
                        scrape_core::JobStatus::Viewed,
                    ]),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(
                discovered.total,
                if preview { 0 } else { expected.len() as i64 }
            );
        }
    }
    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert_eq!(pages.load(Ordering::SeqCst), 16);
    pool.close().await;
}

#[tokio::test]
async fn recorded_sep_country_evidence_survives_filtered_preview_run_and_repository() {
    use scrape_lib::services::{execution_diagnostics, workflow};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let listings = Arc::new(AtomicUsize::new(0));
    let observed = listings.clone();
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while !stopped.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap();
            let body = match path {
                "/api.lever.co/v0/postings/sep?mode=json" => {
                    observed.fetch_add(1, Ordering::SeqCst);
                    include_str!("../../fixtures/providers/lever/sep.json")
                }
                "/api.lever.co/robots.txt" | "/jobs.lever.co/robots.txt" => {
                    "User-agent: *\nAllow: /"
                }
                "/jobs.lever.co/sep/7b96b0c6-8429-4861-bf69-a00956e42a8d" => {
                    "<main>Authored branding-free posting page for recorded SEP replay</main>"
                }
                _ => panic!("unexpected recorded SEP route {path}"),
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&dir.path().join("sep-country.db"))
        .await
        .unwrap();
    let spec=workflow::validate(json!({"version":6,"metadata":{"name":"Recorded SEP evidence control","company":"SEP"},"sources":[{"id":"sep","label":"SEP","extractor":{"kind":"lever","url":"sep"}}],"schedule":{"cadence":"manual"},"filters":{"matching":{"keywords":{"any":["Engineer"],"fields":["title"]},"role_types":["full_time"],"countries":["US"],"regions":["US-IN"],"cities":["Westfield"]}}}),"UTC").unwrap();
    let rules = spec.filters.as_ref().unwrap().matching.clone().unwrap();
    let parent = workflow::persist(&pool, spec, None, false, "UTC")
        .await
        .unwrap();
    let mut config =
        execution_diagnostics::prepare(&parent, "UTC", "2026-10-18T00:00:00Z").unwrap();
    config.limits.per_host_delay_ms = 0;
    let expected = [
        "0b14158a-68e4-4e94-86f8-3f74c87cb3bd",
        "2eb79283-b3fd-4316-9e93-e80dbebfba05",
        "80cd8c15-bafc-49a9-b8f1-5d407e891ec0",
        "98fc318a-8ce1-4730-a898-3ca372878bb2",
        "f7ad9ffb-03dc-4fb2-9a92-e5f04a85ba08",
    ];
    for preview in [true, false] {
        let events = invoke_replay(&config, preview, Some(&origin));
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!()
        };
        assert_eq!(
            (
                stats.sources[0].examined,
                stats.sources[0].accepted,
                stats.sources[0].rejected
            ),
            (8, 5, 3)
        );
        assert_eq!(stats.sources[0].unresolved, 0);
        let batch: Vec<_> = events
            .into_iter()
            .filter(|e| matches!(e, WorkerEvent::Job { .. }))
            .map(|e| scraped_job_from_event(e).unwrap())
            .collect();
        let mut ids: Vec<_> = batch
            .iter()
            .map(|j| j.external_id.as_deref().unwrap())
            .collect();
        ids.sort();
        assert_eq!(ids, expected);
        if !preview {
            JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        }
        let saved = JobsRepo::list(
            &pool,
            JobFilter {
                source_id: Some(parent.id.clone()),
                matching: Some(rules.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(saved.total, if preview { 0 } else { 5 });
        for job in saved.items {
            assert_eq!(job.role_type, scrape_core::RoleType::FullTime);
            assert_eq!(job.locations[0].raw, "Westfield, IN");
            assert_eq!(job.locations[0].country_code.as_deref(), Some("US"));
            assert_eq!(job.locations[0].region_code.as_deref(), Some("US-IN"));
        }
    }
    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert_eq!(listings.load(Ordering::SeqCst), 2);
    pool.close().await;
}

#[tokio::test]
async fn recorded_netflix_workplace_only_evidence_survives_script_trust_and_persistence() {
    use scrape_lib::services::{execution_diagnostics, script_trust, workflow};
    use serde_json::json;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&dir.path().join("netflix-evidence.db"))
        .await
        .unwrap();
    let spec=workflow::validate(json!({"version":6,"metadata":{"name":"Recorded Netflix workplace control"},"sources":[{"id":"netflix","label":"Netflix","extractor":{"kind":"script","url":"https://explore.jobs.netflix.net/api/apply/v2/jobs","script":{"runtime":"js","interpreter":fixture_node_path(),"path":root.join("crates/scrape-worker/examples/extractors/netflix_extractor.js").to_str().unwrap(),"args":[root.join("fixtures/providers/script/netflix.json").to_str().unwrap()]}}}],"schedule":{"cadence":"manual"},"filters":{"matching":{"workplaces":["remote"]}}}),"UTC").unwrap();
    let parent = workflow::persist(&pool, spec, None, false, "UTC")
        .await
        .unwrap();
    assert!(!parent.enabled);
    assert!(parent.needs_review);
    script_trust::trust(
        &pool,
        &parent.id,
        parent.script_sha256.as_deref().unwrap(),
        true,
    )
    .await
    .unwrap();
    let mut enabled: workflow::WorkflowSpec =
        serde_json::from_value(parent.config["__workflow_spec"].clone()).unwrap();
    enabled.metadata.enabled = Some(true);
    enabled.sources[0].enabled = true;
    let parent = workflow::persist(&pool, enabled, Some(parent.id.clone()), false, "UTC")
        .await
        .unwrap();
    let config = execution_diagnostics::prepare(&parent, "UTC", "2026-10-18T00:00:00Z").unwrap();
    script_trust::validate_execution(&pool, &config.sources)
        .await
        .unwrap();
    let events = invoke(&config, false);
    let WorkerEvent::Done { stats } = events.last().unwrap() else {
        panic!()
    };
    assert_eq!(
        (
            stats.sources[0].examined,
            stats.sources[0].accepted,
            stats.sources[0].rejected,
            stats.sources[0].unresolved
        ),
        (10, 2, 8, 0)
    );
    let batch: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    let mut ids: Vec<_> = batch
        .iter()
        .map(|j| j.external_id.as_deref().unwrap())
        .collect();
    ids.sort();
    assert_eq!(ids, ["790318858893", "790318883356"]);
    assert!(batch
        .iter()
        .all(|j| j.role_type == scrape_core::RoleType::Unknown
            && j.description_text.is_none()
            && j.workplace_type == scrape_core::WorkplaceType::Remote));
    JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
    let saved = JobsRepo::list(
        &pool,
        JobFilter {
            matching: Some(serde_json::from_value(json!({"workplaces":["remote"]})).unwrap()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(saved.total, 2);
    pool.close().await;
}

fn fixture_node_path() -> String {
    let output = std::process::Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .expect("Node.js fixture runtime");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Recorded bounded pages, NOT live/full-site coverage. The actual seed is set
/// up explicitly, saved/reviewed/enabled, then run outside the repository cwd.
#[tokio::test]
async fn seeded_bundled_scripts_run_after_explicit_setup_from_non_repository_directory() {
    use scrape_lib::services::{
        execution_diagnostics, provider_verification_profiles, script_trust, workflow,
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("script-setup.db"))
        .await
        .unwrap();
    let parent = provider_verification_profiles::save(&pool, "streaming-cloud-script")
        .await
        .unwrap();
    assert!(!parent.enabled && parent.needs_review);
    let mut spec = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC").unwrap();
    for entry in &mut spec.sources {
        let script = entry.extractor.script.as_mut().unwrap();
        assert!(script.path.is_none());
        script.interpreter = Some(fixture_node_path());
    }
    // Persisted source IDs are UUIDs; pick fixture names by employer instead.
    for entry in &mut spec.sources {
        entry.extractor.script.as_mut().unwrap().args = Some(vec![root
            .join(format!(
                "fixtures/providers/script/{}.json",
                entry.employer_hint.as_ref().unwrap().to_lowercase()
            ))
            .to_string_lossy()
            .to_string()]);
    }
    let parent = workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC")
        .await
        .unwrap();
    let mut spec = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC").unwrap();
    for entry in &spec.sources {
        let row = SourcesRepo::get(&pool, &entry.id).await.unwrap();
        assert!(row.needs_review && !row.script_trusted && !row.enabled);
        script_trust::trust(
            &pool,
            &entry.id,
            row.script_sha256.as_deref().unwrap(),
            true,
        )
        .await
        .unwrap();
        assert!(!SourcesRepo::get(&pool, &entry.id).await.unwrap().enabled);
    }
    spec.metadata.enabled = Some(true);
    for entry in &mut spec.sources {
        entry.enabled = true;
    }
    let parent = workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC")
        .await
        .unwrap();
    let mut config =
        execution_diagnostics::prepare(&parent, "UTC", "2026-10-18T00:00:00Z").unwrap();
    config.limits.per_host_delay_ms = 0;
    script_trust::validate_execution(&pool, &config.sources)
        .await
        .unwrap();
    let events = invoke(&config, false);
    let WorkerEvent::Done { stats } = events.last().unwrap() else {
        panic!()
    };
    assert_eq!(stats.sources.len(), 2);
    assert!(
        stats.sources.iter().all(|s| s.termination
            == scrape_core::protocol::CoverageTermination::PageLimit
            && s.outcome == Some(scrape_core::protocol::ScanOutcome::Partial)),
        "one captured page with a larger provider total is partial, not exhausted: {:?}",
        stats.sources
    );
    assert!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                WorkerEvent::Warning {
                    eligibility_incomplete: true,
                    termination: Some(scrape_core::protocol::CoverageTermination::PageLimit),
                    ..
                }
            ))
            .count()
            >= 2
    );
    let batch: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .cloned()
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    let mut ids: Vec<_> = batch
        .iter()
        .map(|job| job.external_id.as_deref().unwrap())
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            "10421270",
            "10436388",
            "10489389",
            "10507775",
            "10524143",
            "10582071",
            "10582075",
            "790318858893"
        ]
    );
    assert_eq!(
        batch
            .iter()
            .filter(|job| job.role_type == scrape_core::RoleType::Unknown)
            .count(),
        7
    );
    assert_eq!(
        batch
            .iter()
            .filter(|job| job.role_type == scrape_core::RoleType::Internship)
            .count(),
        1
    );
    let saved = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
    assert_eq!(saved.inserted, 8);
    assert_eq!(
        JobsRepo::list(
            &pool,
            JobFilter {
                source_id: Some(parent.id.clone()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        8
    );
    // Explicit known + unknown choices survive save/reopen and worker matching.
    let mut spec = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC").unwrap();
    let rules = spec.filters.as_mut().unwrap().matching.as_mut().unwrap();
    rules.role_types = vec![
        scrape_core::RoleType::Internship,
        scrape_core::RoleType::Unknown,
    ];
    rules.workplaces = vec![
        scrape_core::WorkplaceType::Remote,
        scrape_core::WorkplaceType::Unknown,
    ];
    let parent = workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC")
        .await
        .unwrap();
    let config = execution_diagnostics::prepare(&parent, "UTC", "2026-10-18T00:00:00Z").unwrap();
    script_trust::validate_execution(&pool, &config.sources)
        .await
        .unwrap();
    let explicit: Vec<_> = invoke(&config, false)
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    let mut explicit_ids: Vec<_> = explicit
        .iter()
        .map(|job| job.external_id.as_deref().unwrap())
        .collect();
    explicit_ids.sort();
    assert_eq!(explicit_ids, ids);
    let refreshed = JobsRepo::upsert_scraped(&pool, &explicit).await.unwrap();
    assert_eq!((refreshed.inserted, refreshed.updated), (0, 8));
    pool.close().await;
}

/// Full-pipeline test for Section 6:
/// - Google document base resolution (no duplicated jobs/results/ in destination URL)
/// - Apple field location extraction (div.job-title-location span:not(.a11y))
/// - Multi-page pagination across rejected Page 1 postings to matching Page 2 postings
/// - Persistence to temporary SQLite under parent workflow ownership
#[tokio::test]
async fn seeded_google_apple_pagination_and_document_base_destination_persist_to_sqlite() {
    use scrape_lib::{
        db::runs::{RunCounts, RunsRepo, TriggerType},
        services::{execution_diagnostics, provider_verification_profiles, workflow},
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let mut pages = 0;
        let deadline = Instant::now() + Duration::from_secs(10);
        while pages < 4 && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap();
            let body = match path {
                "/www.google.com/about/careers/applications/jobs/results/" => {
                    pages += 1;
                    concat!(
                        "<html><head><base href=\"https://www.google.com/about/careers/applications/\"></head>",
                        "<body><ul><li class='lLd3Je'><h3 class='QJPWVe'>Google Sales Lead</h3>",
                        "<a href='jobs/results/google-sales'>Job</a><span class='r0wTof'>Austin</span></li></ul>",
                        "<a href=\"https://www.google.com/about/careers/applications/jobs/results/?page=2\" aria-label=\"Go to next page\">Next</a>",
                        "</body></html>"
                    )
                }
                "/www.google.com/about/careers/applications/jobs/results/?page=2" => {
                    pages += 1;
                    concat!(
                        "<html><head><base href=\"https://www.google.com/about/careers/applications/\"></head>",
                        "<body><ul><li class='lLd3Je'><h3 class='QJPWVe'>Google Distributed Systems Rust Engineer</h3>",
                        "<a href='jobs/results/131138433534829254-google-rust-engineer'>Job</a><span class='r0wTof'>Austin</span></li></ul></body></html>"
                    )
                }
                "/jobs.apple.com/en-us/search" => {
                    pages += 1;
                    concat!(
                        "<html><head><link rel=\"next\" href=\"https://jobs.apple.com/en-us/search?page=2\"></head>",
                        "<body><ul id='search-job-list'><li class='rc-accordion-item'><h3><a class='link-inline' href='/en-us/details/apple-retail'>Apple Store Lead</a></h3>",
                        "<div class='job-title-location'><span class='a11y'>Location</span><span>Cupertino</span></div></li></ul></body></html>"
                    )
                }
                "/jobs.apple.com/en-us/search?page=2" => {
                    pages += 1;
                    concat!(
                        "<html><head></head>",
                        "<body><ul id='search-job-list'><li class='rc-accordion-item'><h3><a class='link-inline' href='/en-us/details/200689992/coreos-firmware-engineer'>CoreOS Firmware Engineer</a></h3>",
                        "<div class='job-title-location'><span class='a11y'>Location</span><span>Cupertino</span></div></li></ul></body></html>"
                    )
                }
                _ if path.ends_with("/robots.txt") => "User-agent: *\nAllow: /",
                _ => panic!("unexpected fixture route {path}"),
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        assert_eq!(pages, 4, "Expected all 4 pages to be visited");
    });

    let dir = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&dir.path().join("pagination.db"))
        .await
        .unwrap();
    let parent = provider_verification_profiles::save(&pool, "bigtech-static-swe")
        .await
        .unwrap();
    let mut spec = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC").unwrap();
    spec.metadata.enabled = Some(true);
    for entry in &mut spec.sources {
        entry.enabled = true;
    }
    // Filter by "Engineer": early Page 1 items are rejected, later Page 2 items match
    let matching = spec.filters.as_mut().unwrap().matching.as_mut().unwrap();
    let kw = matching.keywords.as_mut().unwrap();
    kw.any = vec!["Engineer".into()];
    kw.fields = vec![scrape_core::FilterField::Title];
    matching.role_types = vec![];
    matching.workplaces = vec![];
    matching.countries = vec![];
    matching.regions = vec![];
    matching.cities = vec![];

    let parent = workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC")
        .await
        .unwrap();
    let run = RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&parent.id))
        .await
        .unwrap();
    let mut config = execution_diagnostics::prepare(&parent, "UTC", &run.started_at).unwrap();
    config.limits.per_host_delay_ms = 0;
    execution_diagnostics::preflight(&pool, &run.id, &parent.id, &config)
        .await
        .unwrap();

    let events = invoke_replay(&config, false, Some(&origin));
    server.join().unwrap();

    let batch: Vec<_> = events
        .into_iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();

    assert_eq!(
        batch.len(),
        2,
        "Expected exactly 2 accepted jobs from later pages"
    );
    assert_ne!(batch[0].source_id, batch[1].source_id);

    let google_job = batch
        .iter()
        .find(|j| j.title.contains("Google"))
        .expect("Expected Google job");
    assert_eq!(
        google_job.url,
        "https://www.google.com/about/careers/applications/jobs/results/131138433534829254-google-rust-engineer"
    );
    assert!(
        !google_job.url.contains("jobs/results/jobs/results/"),
        "Must not duplicate jobs/results/ in destination"
    );

    let apple_job = batch
        .iter()
        .find(|j| j.title.contains("CoreOS"))
        .expect("Expected Apple job");
    assert_eq!(
        apple_job.url,
        "https://jobs.apple.com/en-us/details/200689992/coreos-firmware-engineer"
    );
    assert_eq!(apple_job.location.as_deref(), Some("Cupertino"));

    execution_diagnostics::persist_batch(&pool, &run.id, &batch, &mut RunCounts::default())
        .await
        .unwrap();

    let saved = JobsRepo::list(
        &pool,
        JobFilter {
            source_id: Some(parent.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(saved.total, 2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM jobs WHERE workflow_id=?")
            .bind(&parent.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );

    pool.close().await;
}

/// Authored employer evidence, not preclassified worker events: the same HTTP
/// documents must produce the independently specified worker and SQL identities.
#[tokio::test]
async fn required_experience_and_exact_employer_artwork_survive_worker_sql_and_reopen() {
    use scrape_core::{
        branding::{relationship_key, LogoStatus},
        matching::{ExperienceMatchMode, PostingRules},
        JobStatus, RoleType, TriState,
    };
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    const ACME_ART: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="256" height="128"><path fill="#c20" d="M0 0h256v128H0z"/></svg>"##;
    const LABS_ART: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="128" height="256"><path fill="#06c" d="M0 0h128v256H0z"/></svg>"##;
    let fixtures = [
        (
            "iii-forward",
            "SOFTWARE ENGINEER III",
            Some("5 years of professional experience required; 1 year of Kubernetes preferred"),
            json!("FULL_TIME"),
            "Acme",
        ),
        (
            "iii-reverse",
            "SOFTWARE ENGINEER III",
            Some("1 year of Kubernetes preferred; 5 years of professional experience required"),
            json!(["FullTime", "FULL_TIME"]),
            "Acme",
        ),
        (
            "range",
            "Software Engineer",
            Some("1–5 years of professional experience required"),
            json!("FULL_TIME"),
            "Acme",
        ),
        (
            "three",
            "Software Engineer",
            Some("3+ years of professional experience required"),
            json!("CONTRACT"),
            "Acme Labs",
        ),
        (
            "graduate",
            "Software Engineer",
            Some("New graduates welcome. 0–1 years of professional experience required"),
            json!("INTERN"),
            "Acme Labs",
        ),
        (
            "maximum",
            "Software Engineer",
            Some("At most 5 years of professional experience"),
            json!(null),
            "Acme Labs",
        ),
        (
            "unknown",
            "Internal Tools Engineer",
            Some("We build reliable systems."),
            json!("Internal"),
            "Acme Labs",
        ),
        (
            "title-only",
            "SOFTWARE ENGINEER III",
            None,
            json!(null),
            "Acme",
        ),
        (
            "conflict",
            "Software Engineer",
            None,
            json!(["FULL_TIME", "CONTRACT"]),
            "Acme Labs",
        ),
    ];
    let postings: Vec<_> = fixtures
        .iter()
        .map(|(id, title, description, employment_type, company)| {
            let logo = if *company == "Acme" {
                json!({"@type":"ImageObject","url":"/logos/acme.svg"})
            } else {
                json!("/logos/acme-labs.svg")
            };
            let mut value = json!({
                "@type":"JobPosting",
                "identifier":id,
                "title":title,
                "url":format!("/jobs/{id}"),
                "hiringOrganization":{"name":company,"logo":logo},
            });
            if let Some(description) = description {
                value["description"] = json!(description);
            }
            if !employment_type.is_null() {
                value["employmentType"] = employment_type.clone();
            }
            value
        })
        .collect();
    let html = format!(
        "<html><script type=\"application/ld+json\">{}</script></html>",
        serde_json::to_string(&postings).unwrap()
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_server = Arc::clone(&stopped);
    let requests = Arc::new(Mutex::new(
        std::collections::BTreeMap::<String, usize>::new(),
    ));
    let server_requests = Arc::clone(&requests);
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !stop_server.load(Ordering::SeqCst) && Instant::now() < deadline {
            if let Ok((mut stream, _)) = listener.accept() {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = [0; 8192];
                let n = stream.read(&mut bytes).unwrap();
                let request = String::from_utf8_lossy(&bytes[..n]);
                let path = request.split_whitespace().nth(1).unwrap().to_owned();
                *server_requests
                    .lock()
                    .unwrap()
                    .entry(path.clone())
                    .or_default() += 1;
                let (content_type, body) = match path.as_str() {
                    "/shared-ats.example.test/robots.txt" => {
                        ("text/plain", "User-agent: *\nAllow: /")
                    }
                    "/shared-ats.example.test/board" => ("text/html", html.as_str()),
                    "/shared-ats.example.test/logos/acme.svg" => ("image/svg+xml", ACME_ART),
                    "/shared-ats.example.test/logos/acme-labs.svg" => ("image/svg+xml", LABS_ART),
                    _ => panic!("Unexpected employer fixture request: {path}"),
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ).unwrap();
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("experience-artwork.db");
    let pool = scrape_lib::db::init_db(&database).await.unwrap();
    // This saved legacy rule deliberately omits the mode. Reopening it must
    // preserve overlap rather than reinterpret the same bounds as a minimum.
    let legacy_rule = json!({"experience_min":3,"experience_max":5});
    let source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Jsonld,
            name: "Shared ATS, independent employers".into(),
            identifier: "https://shared-ats.example.test/board".into(),
            config: Some(json!({})),
            enabled: Some(true),
            cadence: Some(scrape_core::SourceCadence::Manual),
            cadence_cron: None,
            retention_days: Some(30),
            retention_policy: Some("untracked_only".into()),
            filters: Some(json!({"matching":legacy_rule})),
        },
    )
    .await
    .unwrap();
    let mut config = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![WorkerSource {
            id: source.id.clone(),
            kind: source.kind,
            name: source.name.clone(),
            identifier: source.identifier.clone(),
            config: source.config.clone(),
        }],
        limits: WorkerLimits {
            request_timeout_s: 3,
            per_host_delay_ms: 0,
            concurrency: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let job_ids = |events: &[WorkerEvent]| {
        let mut ids: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorkerEvent::Job { external_id, .. } => Some(external_id.clone()),
                _ => None,
            })
            .collect();
        ids.sort();
        ids
    };
    let all_ids = vec![
        "conflict",
        "graduate",
        "iii-forward",
        "iii-reverse",
        "maximum",
        "range",
        "three",
        "title-only",
        "unknown",
    ];
    let assert_clean = |events: &[WorkerEvent]| {
        let errors: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Error { .. }))
            .collect();
        assert!(errors.is_empty(), "Fixture extraction errors: {errors:?}");
    };
    // Invoke the actual packaged dry-run worker and retain ephemeral artwork,
    // but never feed preview results to either persistence API.
    let preview = invoke_replay(&config, true, Some(&origin));
    assert_clean(&preview);
    assert_eq!(job_ids(&preview), all_ids);
    let preview_logos: Vec<_> = preview
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Branding {
                source_id,
                company,
                logo,
            } => Some((source_id, company, logo)),
            _ => None,
        })
        .collect();
    assert_eq!(preview_logos.len(), 2);
    for (source_id, company, logo) in preview_logos {
        assert_eq!(source_id, &source.id);
        assert!(matches!(company.as_str(), "Acme" | "Acme Labs"));
        assert_eq!(logo.status, LogoStatus::Verified);
        logo.validate().unwrap();
        assert!(logo
            .asset
            .as_deref()
            .unwrap()
            .starts_with("data:image/webp;base64,"));
    }
    for table in ["jobs", "employer_branding"] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap(),
            0,
            "dry-run must not populate {table}"
        );
    }

    let events = invoke_replay(&config, false, Some(&origin));
    assert_clean(&events);
    assert_eq!(job_ids(&events), all_ids);
    let rows: Vec<_> = events
        .iter()
        .filter(|event| matches!(event, WorkerEvent::Job { .. }))
        .cloned()
        .map(|event| scraped_job_from_event(event).unwrap())
        .collect();
    assert_eq!(
        JobsRepo::upsert_scraped(&pool, &rows)
            .await
            .unwrap()
            .inserted,
        9
    );
    // Exercise the production Branding recorder *after* job insertion so the
    // readback also covers attachment of the exact employer relationship.
    for event in &events {
        if let WorkerEvent::Branding {
            source_id,
            company,
            logo,
        } = event
        {
            scrape_lib::db::branding::record(&pool, source_id, company, logo)
                .await
                .unwrap();
        }
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM employer_branding")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    let saved = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    let expected_acme = scrape_core::branding::cache_asset(ACME_ART.as_bytes()).unwrap();
    let expected_labs = scrape_core::branding::cache_asset(LABS_ART.as_bytes()).unwrap();
    assert_ne!(expected_acme, expected_labs);
    let mut protected_id = None;
    let mut references = std::collections::BTreeMap::new();
    for job in &saved.items {
        let id = job.external_id.as_deref().unwrap();
        let (min, max, role) = match id {
            "iii-forward" | "iii-reverse" => (Some(5), None, RoleType::FullTime),
            "range" => (Some(1), Some(5), RoleType::FullTime),
            "three" => (Some(3), None, RoleType::Contract),
            "graduate" => (Some(0), Some(1), RoleType::Internship),
            "maximum" => (None, Some(5), RoleType::Unknown),
            "title-only" | "unknown" | "conflict" => (None, None, RoleType::Unknown),
            _ => panic!("Unexpected saved identity: {id}"),
        };
        assert_eq!((job.experience_min, job.experience_max), (min, max), "{id}");
        assert_eq!(job.role_type, role, "{id}");
        if id.starts_with("iii-") || id == "title-only" {
            assert_eq!(job.is_new_grad, TriState::No, "{id}");
            assert!(job
                .experience_evidence
                .as_deref()
                .is_some_and(
                    |evidence| evidence.contains("5 years") || evidence.contains("ENGINEER III")
                ));
        }
        if id == "graduate" {
            assert_eq!(job.is_new_grad, TriState::Yes);
        }
        let logo = job
            .company_logo
            .as_ref()
            .expect("Exact employer artwork attached");
        assert_eq!(logo.status, LogoStatus::Verified);
        assert!(
            logo.asset.is_none(),
            "Read models expose references, not repeated artwork"
        );
        let evidence = logo.evidence.as_ref().unwrap();
        assert_eq!(evidence.page_url, "https://shared-ats.example.test/board");
        let (url, expected) = match job.company.as_str() {
            "Acme" => (
                "https://shared-ats.example.test/logos/acme.svg",
                &expected_acme,
            ),
            "Acme Labs" => (
                "https://shared-ats.example.test/logos/acme-labs.svg",
                &expected_labs,
            ),
            other => panic!("Unexpected employer: {other}"),
        };
        assert_eq!(evidence.url, url);
        let reference = logo.cached_asset_reference.as_ref().unwrap();
        assert_eq!(
            &scrape_lib::db::branding::asset(&pool, reference)
                .await
                .unwrap(),
            expected
        );
        references.insert(job.company.clone(), reference.clone());
        let key: String = sqlx::query_scalar("SELECT company_branding_key FROM jobs WHERE id=?")
            .bind(&job.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(key, relationship_key(&source.id, &job.company));
        if id == "iii-forward" {
            protected_id = Some(job.id.clone());
        }
    }
    assert_eq!(references.len(), 2);
    assert_ne!(references["Acme"], references["Acme Labs"]);
    let missing_artwork =
        scrape_lib::db::branding::asset(&pool, &format!("sha256:{}", "0".repeat(64)))
            .await
            .unwrap_err();
    assert_eq!(
        missing_artwork.to_string(),
        "Item not found: Cached employer logo not found",
        "Unrecorded artwork must not fall back to a different employer"
    );

    // Expectations are authored independently of both the worker evaluator and
    // SQL compiler: agreement between two implementations alone is insufficient.
    let cases: Vec<(&str, serde_json::Value, Vec<&str>)> = vec![
        (
            "minimum 0–2",
            json!({"experience_min":0,"experience_max":2,"experience_match_mode":"required_minimum"}),
            vec!["graduate", "range"],
        ),
        (
            "minimum 3–5",
            json!({"experience_min":3,"experience_max":5,"experience_match_mode":"required_minimum"}),
            vec!["iii-forward", "iii-reverse", "three"],
        ),
        (
            "minimum 3–5, unknown included",
            json!({"experience_min":3,"experience_max":5,"experience_match_mode":"required_minimum","experience_include_unknown":true}),
            vec![
                "conflict",
                "iii-forward",
                "iii-reverse",
                "maximum",
                "three",
                "title-only",
                "unknown",
            ],
        ),
        (
            "explicit saved overlap",
            json!({"experience_min":3,"experience_max":5,"experience_match_mode":"advertised_range_overlap"}),
            vec!["iii-forward", "iii-reverse", "maximum", "range", "three"],
        ),
        (
            "legacy omitted mode",
            legacy_rule,
            vec!["iii-forward", "iii-reverse", "maximum", "range", "three"],
        ),
        (
            "new grad independent",
            json!({"new_grad":"only"}),
            vec!["graduate"],
        ),
        (
            "minimum AND new grad",
            json!({"experience_min":3,"experience_max":5,"experience_match_mode":"required_minimum","new_grad":"only"}),
            vec![],
        ),
        (
            "explicit employment",
            json!({"role_types":["full_time"]}),
            vec!["iii-forward", "iii-reverse", "range"],
        ),
    ];
    for (name, value, expected) in cases {
        let rules: PostingRules = serde_json::from_value(value).unwrap();
        config.filters.matching = Some(rules.clone());
        let filtered = invoke_replay(&config, false, Some(&origin));
        assert_clean(&filtered);
        assert_eq!(job_ids(&filtered), expected, "worker: {name}");
        let mut persisted: Vec<_> = JobsRepo::list(
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
        persisted.sort();
        assert_eq!(persisted, expected, "SQL: {name}");
    }
    let protected_id = protected_id.unwrap();
    JobsRepo::update_notes(&pool, &protected_id, Some("Protected experience/logo note"))
        .await
        .unwrap();
    JobsRepo::set_favorite(&pool, &protected_id, true)
        .await
        .unwrap();
    JobsRepo::update_status(&pool, &protected_id, JobStatus::Applied)
        .await
        .unwrap();
    let history = JobsRepo::get_status_events(&pool, &protected_id)
        .await
        .unwrap();
    assert!(!history.is_empty());
    config.filters = WorkerFilters::default();
    let refresh = invoke_replay(&config, false, Some(&origin));
    assert_clean(&refresh);
    let refreshed: Vec<_> = refresh
        .iter()
        .filter(|event| matches!(event, WorkerEvent::Job { .. }))
        .cloned()
        .map(|event| scraped_job_from_event(event).unwrap())
        .collect();
    let counts = JobsRepo::upsert_scraped(&pool, &refreshed).await.unwrap();
    assert_eq!((counts.inserted, counts.updated), (0, 9));
    for event in refresh {
        if let WorkerEvent::Branding {
            source_id,
            company,
            logo,
        } = event
        {
            scrape_lib::db::branding::record(&pool, &source_id, &company, &logo)
                .await
                .unwrap();
        }
    }
    stopped.store(true, Ordering::SeqCst);
    server.join().unwrap();
    {
        let requested = requests.lock().unwrap();
        assert!(requested["/shared-ats.example.test/logos/acme.svg"] > 0);
        assert!(requested["/shared-ats.example.test/logos/acme-labs.svg"] > 0);
    }
    pool.close().await;

    let reopened = scrape_lib::db::init_db(&database).await.unwrap();
    let job = JobsRepo::get(&reopened, &protected_id).await.unwrap();
    assert_eq!(job.external_id.as_deref(), Some("iii-forward"));
    assert_eq!(job.notes.as_deref(), Some("Protected experience/logo note"));
    assert!(job.favorited);
    assert_eq!(job.status, JobStatus::Applied);
    assert_eq!(job.experience_min, Some(5));
    let reopened_history = JobsRepo::get_status_events(&reopened, &protected_id)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(reopened_history).unwrap(),
        serde_json::to_value(history).unwrap()
    );
    let logo = job.company_logo.unwrap();
    assert_eq!(
        logo.cached_asset_reference.as_ref(),
        Some(&references["Acme"])
    );
    assert_eq!(
        scrape_lib::db::branding::asset(&reopened, logo.cached_asset_reference.as_deref().unwrap())
            .await
            .unwrap(),
        expected_acme
    );
    let reopened_source = SourcesRepo::get(&reopened, &source.id).await.unwrap();
    let saved_overlap: PostingRules =
        serde_json::from_value(reopened_source.filters["matching"].clone()).unwrap();
    assert_eq!(
        saved_overlap.experience_match_mode,
        ExperienceMatchMode::AdvertisedRangeOverlap
    );
    let mut overlap_ids: Vec<_> = JobsRepo::list(
        &reopened,
        JobFilter {
            matching: Some(saved_overlap),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .items
    .into_iter()
    .map(|job| job.external_id.unwrap())
    .collect();
    overlap_ids.sort();
    assert_eq!(
        overlap_ids,
        vec!["iii-forward", "iii-reverse", "maximum", "range", "three"]
    );
    reopened.close().await;
}

/// Authored publication evidence crosses the real canonical workflow, packaged
/// NDJSON producer, production mapper and durable Discovered repository boundary.
#[tokio::test]
async fn role_first_search_survives_canonical_save_worker_discovered_refresh_and_reopen() {
    use scrape_core::{matching::PostingRules, JobStatus};
    use scrape_lib::services::{execution_diagnostics, workflow};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    const STARTED: &str = "2026-10-18T12:00:00Z";
    const PUBLISHED: &str = "2026-10-18T06:00:00Z";
    const DESCRIPTION: &str =
        "<p>Build reliable systems. 3 years of professional experience required.</p>";
    // The Muse publication_date is original posting evidence. In particular,
    // an aggregator import timestamp must not stand in for this missing field.
    let fixtures = [
        (
            1,
            "Software Engineer",
            "Google",
            "Austin, TX",
            Some(PUBLISHED),
            DESCRIPTION,
        ),
        (
            2,
            "Software Engineer II",
            "Amazon",
            "Austin, TX",
            Some(PUBLISHED),
            DESCRIPTION,
        ),
        (
            3,
            "Software Engineer",
            "Meta",
            "Austin, TX",
            Some(PUBLISHED),
            DESCRIPTION,
        ),
        (
            4,
            "Product Manager",
            "Google",
            "Austin, TX",
            Some(PUBLISHED),
            DESCRIPTION,
        ),
        (
            5,
            "Software Engineer",
            "Google",
            "Austin, TX",
            Some("2026-10-17T23:00:00Z"),
            DESCRIPTION,
        ),
        (
            6,
            "Software Engineer",
            "Amazon",
            "Austin, TX",
            Some(PUBLISHED),
            "<p>8 years of professional experience required.</p>",
        ),
        (
            7,
            "Software Engineer",
            "Google",
            "London, United Kingdom",
            Some(PUBLISHED),
            DESCRIPTION,
        ),
        (
            8,
            "Software Engineer",
            "Amazon",
            "Austin, TX",
            None,
            DESCRIPTION,
        ),
    ];
    let results: Vec<_> = fixtures
        .iter()
        .map(|(id, title, company, location, published, description)| {
            json!({
                "id":id, "name":title, "contents":description,
                "publication_date":published,
                "locations":[{"name":location}],
                "refs":{"landing_page":format!("https://role-search.fixture.test/postings/{id}")},
                "company":{"id":if *company == "Google" { 10 } else { 20 }, "name":company}
            })
        })
        .collect();
    let feed = json!({"page":0,"page_count":1,"total":8,"results":results}).to_string();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_server = Arc::clone(&stopped);
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_requests = Arc::clone(&requests);
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(60);
        while !stop_server.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap().to_string();
            assert_eq!(
                path.split('?').next().unwrap(),
                "/role-search.fixture.test/feed",
                "No career-board, detail, branding or second-page request is needed"
            );
            server_requests.lock().unwrap().push(path);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{feed}",
                feed.len()
            ).unwrap();
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("role-first.db");
    let pool = scrape_lib::db::init_db(&database).await.unwrap();
    let authored = json!({
        "version":workflow::WORKFLOW_SPEC_VERSION,
        "metadata":{"name":"Software Engineer at Google & Amazon"},
        "sources":[{"id":"broad-search","label":"Cross-Company Search","enabled":true,
            "extractor":{"kind":"search","url":"","source_config":{
                "endpoint":"https://role-search.fixture.test/feed","max_pages":5
            }}}],
        "max_jobs_per_run":20,
        "schedule":{"cadence":"manual"},
        "filters":{"matching":{
            "target_roles":["Software Engineer"],"companies":["Google","Amazon"],
            "posting_date":{"preset":"past_12h","timezone":"UTC","include_unknown":false},
            "countries":["US"],"cities":["Austin"],
            "experience_min":3,"experience_max":5,
            "experience_match_mode":"required_minimum","experience_include_unknown":false
        }}
    });
    assert!(authored
        .pointer("/filters/matching/posting_date/last_hours")
        .is_none());
    let spec = workflow::validate(authored, "UTC").unwrap();
    let original_filters = serde_json::to_value(&spec.filters).unwrap();
    let saved = workflow::persist(&pool, spec, None, true, "UTC")
        .await
        .unwrap();
    let source = SourcesRepo::get(&pool, &saved.id).await.unwrap();
    assert_eq!(source.kind, SourceKind::Search);
    assert_eq!(source.identifier, "themuse");
    assert_eq!(
        source.config["__workflow_spec"]["filters"], original_filters,
        "Saving must not rewrite preset-only canonical rules"
    );
    let mut config = execution_diagnostics::prepare(&source, "UTC", STARTED).unwrap();
    config.limits.per_host_delay_ms = 0;
    config.limits.request_timeout_s = 3;
    assert_eq!(config.version, PROTOCOL_VERSION);
    assert_eq!(config.sources.len(), 1);
    assert_eq!(config.sources[0].id, source.id);
    assert_eq!(config.sources[0].identifier, "themuse");
    assert_eq!(
        config
            .filters
            .matching
            .as_ref()
            .unwrap()
            .posting_date
            .as_ref()
            .unwrap()
            .preset
            .as_deref(),
        Some("past_12h")
    );
    let ids = |events: &[WorkerEvent]| {
        let mut ids: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorkerEvent::Job { external_id, .. } => Some(external_id.clone()),
                _ => None,
            })
            .collect();
        ids.sort();
        ids
    };
    for preview in [true, false] {
        let events = invoke_replay(&config, preview, Some(&origin));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WorkerEvent::Error { .. })),
            "{events:?}"
        );
        assert_eq!(ids(&events), vec!["1", "2"]);
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!("Missing terminal worker accounting")
        };
        assert_eq!(stats.sources.len(), 1);
        let scan = &stats.sources[0];
        scan.validate().unwrap();
        assert_eq!(
            (scan.examined, scan.accepted, scan.rejected, scan.unresolved),
            (8, 2, 5, 1)
        );
        assert_eq!(scan.outcome, Some(scrape_core::protocol::ScanOutcome::Partial),
            "A missing publication date must remain unresolved, not become a complete empty/rejected scan");
        let batch: Vec<_> = events
            .into_iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .map(|event| scraped_job_from_event(event).unwrap())
            .collect();
        for row in &batch {
            assert_eq!(row.source_id.as_deref(), Some(source.id.as_str()));
            assert_eq!(row.date_posted.as_deref(), Some(PUBLISHED));
            assert_eq!(row.experience_min, Some(3));
            assert_eq!(row.description_source.as_deref(), Some(DESCRIPTION));
            assert!(row
                .description_text
                .as_deref()
                .unwrap()
                .contains("Build reliable systems."));
        }
        if !preview {
            let counts = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
            assert_eq!((counts.inserted, counts.updated), (2, 0));
        }
        let discovered = JobsRepo::list(
            &pool,
            JobFilter {
                source_id: Some(source.id.clone()),
                statuses: Some(vec![JobStatus::Scraped, JobStatus::Viewed]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(discovered.total, if preview { 0 } else { 2 });
    }
    let discovered = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    let mut owners = std::collections::BTreeMap::new();
    for job in &discovered.items {
        let external_id = job.external_id.as_deref().unwrap();
        let (title, company) = match external_id {
            "1" => ("Software Engineer", "Google"),
            "2" => ("Software Engineer II", "Amazon"),
            other => panic!("Unexpected persisted identity {other}"),
        };
        let url = format!("https://role-search.fixture.test/postings/{external_id}");
        assert_eq!(job.title, title);
        assert_eq!(job.company, company);
        assert_eq!(job.url, url);
        assert_eq!(job.source_id.as_deref(), Some(source.id.as_str()));
        assert_eq!(job.date_posted.as_deref(), Some(PUBLISHED));
        assert_eq!(job.experience_min, Some(3));
        assert!(job
            .experience_evidence
            .as_deref()
            .unwrap()
            .contains("3 years"));
        assert_eq!(
            job.posting_identity.as_deref(),
            Some(serde_json::to_string(&("url", &url)).unwrap().as_str())
        );
        let raw: String = sqlx::query_scalar("SELECT description_source FROM jobs WHERE id=?")
            .bind(&job.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(raw, DESCRIPTION);
        owners.insert(external_id.to_string(), job.id.clone());
    }
    // This is the production SQL compiler used by Discovered, not an in-memory
    // company comparison or a worker-filtered mock response.
    for (company, expected) in [
        ("gOoGlE", vec!["1"]),
        ("Amazon", vec!["2"]),
        ("Meta", vec![]),
    ] {
        let rules: PostingRules = serde_json::from_value(json!({"companies":[company]})).unwrap();
        let rows = JobsRepo::list(
            &pool,
            JobFilter {
                source_id: Some(source.id.clone()),
                matching: Some(rules),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let actual: Vec<_> = rows
            .items
            .iter()
            .map(|job| job.external_id.as_deref().unwrap())
            .collect();
        assert_eq!(actual, expected);
    }
    let protected = &owners["1"];
    JobsRepo::update_notes(
        &pool,
        protected,
        Some("Do not overwrite my application notes"),
    )
    .await
    .unwrap();
    JobsRepo::set_favorite(&pool, protected, true)
        .await
        .unwrap();
    JobsRepo::update_status(&pool, protected, JobStatus::Applied)
        .await
        .unwrap();
    let history = JobsRepo::get_status_events(&pool, protected).await.unwrap();
    let refresh = invoke_replay(&config, false, Some(&origin));
    assert_eq!(ids(&refresh), vec!["1", "2"]);
    assert!(!refresh
        .iter()
        .any(|event| matches!(event, WorkerEvent::Error { .. })));
    let batch: Vec<_> = refresh
        .into_iter()
        .filter(|event| matches!(event, WorkerEvent::Job { .. }))
        .map(|event| scraped_job_from_event(event).unwrap())
        .collect();
    let counts = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
    assert_eq!((counts.inserted, counts.updated), (0, 2));

    // A pre-role-first company-board specification retains its exact matching
    // semantics through canonical validation and save alongside broad search.
    let board_filters = json!({"include_keywords":["Systems"],"locations":["Remote","Austin"],
        "matching":{"experience_min":3,"experience_max":5,"experience_match_mode":"advertised_range_overlap"}});
    let board = workflow::validate(json!({
        "version":7,"metadata":{"name":"Existing company board"},
        "sources":[{"id":"existing","label":"Existing board","employer_hint":"Existing employer",
            "extractor":{"kind":"lever","url":"https://jobs.lever.co/existing"}}],
        "schedule":{"cadence":"manual"},"filters":board_filters
    }), "UTC").unwrap();
    let before = serde_json::to_value(&board.filters).unwrap();
    let board = workflow::persist(&pool, board, None, true, "UTC")
        .await
        .unwrap();
    let board = SourcesRepo::get(&pool, &board.id).await.unwrap();
    assert_eq!(board.config["__workflow_spec"]["filters"], before);
    let prepared_board = execution_diagnostics::prepare(&board, "UTC", STARTED).unwrap();
    let expected_board: WorkerFilters = serde_json::from_value(board_filters).unwrap();
    assert_eq!(prepared_board.filters, expected_board);
    assert_eq!(
        JobsRepo::list(
            &pool,
            JobFilter {
                source_id: Some(board.id.clone()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0,
        "The attached company-board workflow must not own search rows"
    );

    stopped.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert_eq!(
        requests.lock().unwrap().len(),
        3,
        "Exactly one provider page per preview, run and refresh"
    );
    pool.close().await;
    let reopened = scrape_lib::db::init_db(&database).await.unwrap();
    let job = JobsRepo::get(&reopened, protected).await.unwrap();
    assert_eq!(
        job.notes.as_deref(),
        Some("Do not overwrite my application notes")
    );
    assert!(job.favorited);
    assert_eq!(job.status, JobStatus::Applied);
    assert_eq!(job.source_id.as_deref(), Some(source.id.as_str()));
    assert_eq!(job.external_id.as_deref(), Some("1"));
    assert_eq!(
        serde_json::to_value(
            JobsRepo::get_status_events(&reopened, protected)
                .await
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(history).unwrap()
    );
    let rows = JobsRepo::list(
        &reopened,
        JobFilter {
            source_id: Some(source.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.total, 2);
    for job in rows.items {
        assert_eq!(&job.id, &owners[job.external_id.as_deref().unwrap()]);
    }
    let reopened_source = SourcesRepo::get(&reopened, &source.id).await.unwrap();
    assert_eq!(
        reopened_source.config["__workflow_spec"]["filters"],
        original_filters
    );
    reopened.close().await;
}

/// Screenshot-equivalent criteria, with authored (not live-provider) responses:
/// default Muse endpoint, all employers, software/data/IT literal roles, a
/// twelve-hour window including unknown dates, and no experience restriction.
#[tokio::test]
async fn screenshot_search_filters_survive_preview_save_run_and_discovered() {
    use scrape_core::{
        matching::RejectionReason,
        protocol::{CoverageTermination, ScanOutcome},
        JobStatus, SourceCadence,
    };
    use scrape_lib::{
        db::runs::{RunCounts, RunStatus, RunsRepo, TriggerType},
        services::{execution_diagnostics, workflow},
    };
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    const STARTED: &str = "2026-10-22T13:00:00Z";
    const RECENT: &str = "2026-10-22T12:00:00Z";
    const BOUNDARY: &str = "2026-10-22T01:00:00Z";
    let roles = [
        "Software Engineer",
        "Data Scientist",
        "Data Engineer",
        "IT Support",
    ];
    let fixtures = [
        (
            1,
            "Software Engineer",
            "Local Software Co",
            Some(RECENT),
            "<p>8 years of professional experience required.</p>",
        ),
        (
            2,
            "Data Scientist",
            "Research Employer",
            Some(BOUNDARY),
            "<p>Build statistical models.</p>",
        ),
        (
            3,
            "IT Support Specialist",
            "Neighborhood Employer",
            None,
            "<p>Support our users.</p>",
        ),
        (
            4,
            "Software Engineer",
            "Older Employer",
            Some("2026-10-22T00:59:59Z"),
            "<p>Build systems.</p>",
        ),
        (
            5,
            "Product Manager",
            "Other Employer",
            Some(RECENT),
            "<p>Manage products.</p>",
        ),
        (
            6,
            "Senior Data Engineer",
            "Another Employer",
            Some(RECENT),
            "<p>12 years of professional experience required.</p>",
        ),
        (
            7,
            "Software Engineer II",
            "Unknown Date Employer",
            None,
            "<p>Build services.</p>",
        ),
    ];
    let results: Vec<_> = fixtures
        .iter()
        .map(|(id, title, company, published, description)| {
            json!({
                "id":id,"name":title,"company":{"name":company},
                "publication_date":published,"contents":description,
                "locations":[{"name":"Remote"}],
                "refs":{"landing_page":format!("https://screenshot.fixture.test/jobs/{id}")}
            })
        })
        .collect();
    // The last two matches are deliberately on the next provider page.
    let pages = [
        json!({"page":0,"page_count":2,"results":&results[..5]}).to_string(),
        json!({"page":1,"page_count":2,"results":&results[5..]}).to_string(),
    ];
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_server = Arc::clone(&stopped);
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_requests = Arc::clone(&requests);
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(45);
        while !stop_server.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = [0; 8192];
            let n = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            let path = request.split_whitespace().nth(1).unwrap().to_string();
            let body = match path.as_str() {
                "/www.themuse.com/api/public/jobs?page=0" => &pages[0],
                "/www.themuse.com/api/public/jobs?page=1" => &pages[1],
                _ => panic!("Unexpected default-provider request: {path}"),
            };
            server_requests.lock().unwrap().push(path);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ).unwrap();
        }
    });
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("screenshot-search.db");
    let pool = scrape_lib::db::init_db(&database).await.unwrap();
    let spec = workflow::validate(json!({
        "version":workflow::WORKFLOW_SPEC_VERSION,
        "metadata":{"name":"Software, data and IT — all employers","enabled":true},
        "sources":[{"id":"broad-search","label":"Cross-Company Search","enabled":true,
            "extractor":{"kind":"search","url":""}}],
        "max_jobs_per_run":20,
        "schedule":{"cadence":"daily","time":"09:00","timezone":"America/New_York"},
        "filters":{"matching":{
            "target_roles":roles,"companies":[],
            "posting_date":{"preset":"past_12h","timezone":"America/New_York","include_unknown":true}
        }}
    }), "America/New_York").unwrap();
    let original_filters = serde_json::to_value(&spec.filters).unwrap();
    let saved = workflow::persist(&pool, spec, None, false, "America/New_York")
        .await
        .unwrap();
    let source = SourcesRepo::get(&pool, &saved.id).await.unwrap();
    assert_eq!(source.kind, SourceKind::Search);
    assert_eq!(source.identifier, "themuse");
    assert_eq!(source.cadence, SourceCadence::Daily);
    assert_eq!(source.config["__schedule"]["time"], "09:00");
    assert_eq!(source.config["__schedule"]["timezone"], "America/New_York");
    assert_eq!(
        source.config["__workflow_spec"]["filters"],
        original_filters
    );
    assert!(
        requests.lock().unwrap().is_empty(),
        "Save must not launch a provider"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scrape_runs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0,
        "Save must not open a run"
    );
    // Deliberately pass a different device timezone: stored workflow criteria and
    // calendar timezone must remain authoritative.
    let mut config = execution_diagnostics::prepare(&source, "UTC", STARTED).unwrap();
    config.limits.per_host_delay_ms = 0;
    config.limits.request_timeout_s = 3;
    assert_eq!(config.version, PROTOCOL_VERSION);
    assert_eq!(config.run_started_at.as_deref(), Some(STARTED));
    assert_eq!(config.max_jobs_per_run, Some(20));
    assert_eq!(config.sources.len(), 1);
    assert_eq!(config.sources[0].id, source.id);
    assert_eq!(config.sources[0].identifier, "themuse");
    assert!(config.sources[0].config.get("endpoint").is_none());
    assert_eq!(config.sources[0].config["target_roles"], json!(roles));
    assert_eq!(config.sources[0].config["companies"], json!([]));
    assert_eq!(config.sources[0].config["max_days_old"], 1);
    let rules = config.filters.matching.as_ref().unwrap();
    assert_eq!(rules.target_roles, roles);
    assert!(rules.companies.is_empty());
    assert_eq!(rules.experience_min, None);
    assert_eq!(rules.experience_max, None);
    let date = rules.posting_date.as_ref().unwrap();
    assert_eq!(date.preset.as_deref(), Some("past_12h"));
    assert!(date.include_unknown);
    assert_eq!(date.timezone, "America/New_York");
    assert_eq!(
        serde_json::to_value(rules).unwrap(),
        source.filters["matching"],
        "Canonical execution must preserve every structured criterion"
    );
    assert_eq!(config.filters.locations, Vec::<String>::new());

    for preview in [true, false] {
        let run = if preview {
            None
        } else {
            let run =
                RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&source.id))
                    .await
                    .unwrap();
            execution_diagnostics::preflight(&pool, &run.id, &source.id, &config)
                .await
                .unwrap();
            Some(run)
        };
        let events = invoke_replay(&config, preview, Some(&origin));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WorkerEvent::Error { .. })),
            "{events:?}"
        );
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!("Missing worker completion")
        };
        assert_eq!(
            (stats.sources_ok, stats.sources_failed, stats.jobs_emitted),
            (1, 0, 5)
        );
        assert_eq!(stats.sources.len(), 1);
        let scan = &stats.sources[0];
        scan.validate().unwrap();
        assert_eq!(scan.source_id, source.id);
        assert_eq!(scan.limit, if preview { 10 } else { 20 });
        assert_eq!(
            (scan.examined, scan.accepted, scan.rejected, scan.unresolved),
            (7, 5, 2, 0)
        );
        assert_eq!((scan.invalid, scan.duplicates, scan.blocked), (0, 0, 0));
        assert!(!scan.limit_reached);
        assert_eq!(scan.termination, CoverageTermination::Exhausted);
        assert_eq!(scan.outcome, Some(ScanOutcome::Success));
        assert_eq!(scan.rejections.len(), 2);
        for reason in [RejectionReason::RoleType, RejectionReason::PostingDate] {
            assert_eq!(
                scan.rejections
                    .iter()
                    .find(|entry| entry.reason == reason)
                    .unwrap()
                    .count,
                1
            );
        }
        let batch: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .cloned()
            .map(|event| scraped_job_from_event(event).unwrap())
            .collect();
        let mut ids: Vec<_> = batch
            .iter()
            .map(|job| job.external_id.as_deref().unwrap())
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["1", "2", "3", "6", "7"]);
        for row in &batch {
            assert_eq!(row.source_id.as_deref(), Some(source.id.as_str()));
            let expected = fixtures
                .iter()
                .find(|(id, ..)| id.to_string() == row.external_id.as_deref().unwrap())
                .unwrap();
            assert_eq!(row.title, expected.1);
            assert_eq!(row.company, expected.2);
            assert_eq!(row.date_posted.as_deref(), expected.3);
            assert_eq!(row.description_source.as_deref(), Some(expected.4));
        }
        if let Some(run) = run {
            let mut counts = RunCounts {
                fetched: scan.examined as i64,
                rejected: scan.rejected as i64,
                sources: Some(stats.sources.clone()),
                ..Default::default()
            };
            let persisted =
                execution_diagnostics::persist_batch(&pool, &run.id, &batch, &mut counts)
                    .await
                    .unwrap();
            assert_eq!(
                (persisted.inserted, persisted.updated, persisted.blocked),
                (5, 0, 0)
            );
            let finished = RunsRepo::finish(&pool, &run.id, RunStatus::Success, 5, 5, None)
                .await
                .unwrap();
            assert_eq!(finished.status, RunStatus::Success);
            assert_eq!((finished.jobs_found, finished.jobs_new), (5, 5));
            assert_eq!(finished.counts.as_ref(), Some(&counts));
            assert_eq!(
                finished.effective_config.as_ref().unwrap().filters,
                config.filters
            );
        } else {
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scrape_runs")
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                0,
                "Preview must not create durable run accounting"
            );
            let unchanged = SourcesRepo::get(&pool, &source.id).await.unwrap();
            assert_eq!(unchanged.next_run_at, source.next_run_at);
            assert_eq!(unchanged.config, source.config);
        }
        let discovered = JobsRepo::list(
            &pool,
            JobFilter {
                source_id: Some(source.id.clone()),
                statuses: Some(vec![JobStatus::Scraped, JobStatus::Viewed]),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(discovered.total, if preview { 0 } else { 5 });
    }
    stopped.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert_eq!(
        *requests.lock().unwrap(),
        vec![
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.themuse.com/api/public/jobs?page=1",
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.themuse.com/api/public/jobs?page=1",
        ]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM scrape_runs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    pool.close().await;
    let reopened = scrape_lib::db::init_db(&database).await.unwrap();
    let source = SourcesRepo::get(&reopened, &saved.id).await.unwrap();
    assert_eq!(
        source.config["__workflow_spec"]["filters"],
        original_filters
    );
    assert_eq!(source.config["__schedule"]["time"], "09:00");
    assert_eq!(source.config["__schedule"]["timezone"], "America/New_York");
    let discovered = JobsRepo::list(
        &reopened,
        JobFilter {
            source_id: Some(saved.id.clone()),
            statuses: Some(vec![JobStatus::Scraped, JobStatus::Viewed]),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(discovered.total, 5);
    let mut ids: Vec<_> = discovered
        .items
        .iter()
        .map(|job| job.external_id.as_deref().unwrap())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["1", "2", "3", "6", "7"]);
    for job in &discovered.items {
        let expected = fixtures
            .iter()
            .find(|(id, ..)| id.to_string() == job.external_id.as_deref().unwrap())
            .unwrap();
        assert_eq!(job.title, expected.1);
        assert_eq!(job.company, expected.2);
        assert_eq!(job.date_posted.as_deref(), expected.3);
        assert_eq!(job.source_id.as_deref(), Some(saved.id.as_str()));
        assert_eq!(
            job.url,
            format!("https://screenshot.fixture.test/jobs/{}", expected.0)
        );
        assert_eq!(
            job.posting_identity.as_deref(),
            Some(serde_json::to_string(&("url", &job.url)).unwrap().as_str())
        );
    }
    reopened.close().await;
}

#[tokio::test]
async fn modified_software_engineering_workflow_scrapes_public_html_and_ats_with_sleep_and_job_limit(
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let request_times = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(String, Instant)>::new()));
    let times = request_times.clone();
    let _server = std::thread::spawn(move || {
        let mut count = 0;
        while count < 20 {
            let (mut stream, _) = match listener.accept() {
                Ok(s) => s,
                Err(_) => break,
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut buf = [0; 4096];
            let n = match stream.read(&mut buf) {
                Ok(n) if n > 0 => n,
                _ => break,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req.split_whitespace().nth(1).unwrap_or("");
            times
                .lock()
                .unwrap()
                .push((path.to_string(), Instant::now()));
            count += 1;
            let (status, body) = match path {
                p if p.ends_with("/robots.txt") => {
                    ("200 OK", "User-agent: *\nAllow: /\n".to_string())
                }
                p if p.ends_with("/html-careers") => (
                    "200 OK",
                    r#"<!DOCTYPE html>
<html>
<head><title>Custom Tech Careers</title></head>
<body>
  <h1>Engineering Openings</h1>
  <div class="career-cards">
    <div class="job-card">
      <a href="/careers/senior-backend-engineer">Senior Backend Engineer</a>
      <span class="location">Remote</span>
    </div>
    <div class="job-card">
      <a href="/careers/platform-engineer">Platform Engineer</a>
      <span class="location">New York, NY</span>
    </div>
  </div>
</body>
</html>"#
                        .to_string(),
                ),
                p if p.ends_with("/careers/senior-backend-engineer") => (
                    "200 OK",
                    r#"<!DOCTYPE html>
<html>
<head><title>Senior Backend Engineer - Custom Tech</title></head>
<body>
  <h1>Senior Backend Engineer</h1>
  <p class="salary">$180,000 - $220,000 per year</p>
  <div class="description">Build distributed database ingestion engines.</div>
</body>
</html>"#
                        .to_string(),
                ),
                p if p.ends_with("/careers/platform-engineer") => (
                    "200 OK",
                    r#"<!DOCTYPE html>
<html>
<head><title>Platform Engineer - Custom Tech</title></head>
<body>
  <h1>Platform Engineer</h1>
  <p class="salary">$160,000 - $190,000 per year</p>
  <div class="description">Scale Kubernetes clusters and deployment workflows.</div>
</body>
</html>"#
                        .to_string(),
                ),
                p if p.contains("/cloudflare/jobs") => (
                    "200 OK",
                    serde_json::json!({
                        "jobs": [
                            {
                                "id": 101,
                                "title": "Software Engineer",
                                "absolute_url": format!("http://{address}/jobs/101"),
                                "location": {"name": "San Francisco, CA"},
                                "content": "<p>Build high-performance edge compute platforms.</p>"
                            }
                        ]
                    })
                    .to_string(),
                ),
                _ => ("404 Not Found", "Not Found".to_string()),
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let directory = tempfile::tempdir().unwrap();
    let pool = scrape_lib::db::init_db(&directory.path().join("test_se.db"))
        .await
        .unwrap();

    let html_source = SourcesRepo::create(
        &pool,
        CreateSourceInput {
            kind: SourceKind::Selector,
            name: "Custom Tech Careers".into(),
            identifier: format!("http://{address}/html-careers"),
            config: Some(serde_json::json!({
                "crawl": {
                    "max_pages": 5,
                    "max_depth": 2,
                    "extraction": "public_html"
                },
                "per_host_delay_ms": 20,
                "jitter_ms": 15,
                "ignore_robots": true,
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

    let gh_source = SourcesRepo::create(
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

    // 1. Run with max_jobs_per_run: Some(2) — verifies job limit cap is enforced by coordinator
    let config_capped = WorkerConfig {
        version: PROTOCOL_VERSION,
        sources: vec![
            WorkerSource {
                id: html_source.id.clone(),
                kind: html_source.kind,
                name: html_source.name.clone(),
                identifier: html_source.identifier.clone(),
                config: html_source.config.clone(),
            },
            WorkerSource {
                id: gh_source.id.clone(),
                kind: gh_source.kind,
                name: gh_source.name.clone(),
                identifier: gh_source.identifier.clone(),
                config: gh_source.config.clone(),
            },
        ],
        filters: WorkerFilters {
            include_keywords: vec!["Engineer".into()],
            ..Default::default()
        },
        limits: WorkerLimits {
            request_timeout_s: 5,
            per_host_delay_ms: 20,
            jitter_ms: Some(15),
            ignore_robots: Some(true),
            ..Default::default()
        },
        max_jobs_per_run: Some(2),
        ..Default::default()
    };

    let events = invoke_replay(&config_capped, false, Some(&format!("http://{address}")));
    let mut jobs = Vec::new();
    for event in events {
        if let WorkerEvent::Job {
            title,
            company,
            url,
            ..
        } = &event
        {
            assert!(!title.is_empty(), "Job title must not be empty");
            assert!(!company.is_empty(), "Company must not be empty");
            assert!(url.starts_with("http"), "URL must be valid HTTP");
            let input = scraped_job_from_event(event).unwrap();
            jobs.push(input);
        }
    }

    JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();

    // Exactly 2 jobs are scraped and saved, matching max_jobs_per_run cap!
    assert_eq!(
        jobs.len(),
        2,
        "Expected exactly 2 jobs due to max_jobs_per_run cap"
    );

    let discovered = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(
        discovered.total, 2,
        "SQLite must hold exactly 2 jobs from the capped run"
    );

    // 2. Verify built-in sleep pacing: requests to the public HTML host are paced
    let recorded = request_times.lock().unwrap().clone();
    assert!(
        recorded.len() >= 2,
        "Server must have received multiple requests"
    );
    let html_requests: Vec<_> = recorded
        .into_iter()
        .filter(|(p, _)| p.contains("html-careers") || p.contains("careers"))
        .collect();
    if html_requests.len() >= 2 {
        let total_span = html_requests
            .last()
            .unwrap()
            .1
            .duration_since(html_requests[0].1);
        assert!(
            total_span >= Duration::from_millis(15),
            "Requests must be paced with built-in sleep"
        );
    }

    pool.close().await;
}

#[tokio::test]
async fn imported_software_engineering_crawler_yaml_caps_persisted_discoveries_at_forty() {
    use scrape_lib::services::{execution_diagnostics, workflow};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    const STARTED: &str = "2026-10-26T16:00:00Z";
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let parse_script = r#"import fs from "node:fs"; import { parse } from "yaml"; console.log(JSON.stringify(parse(fs.readFileSync("test-workflows/software-engineering-web-scraper-test.yaml", "utf8"))));"#;
    let parsed_yaml = Command::new("node")
        .current_dir(repo)
        .args(["--input-type=module", "-e", parse_script])
        .output()
        .unwrap();
    assert!(
        parsed_yaml.status.success(),
        "workflow YAML parser failed: {}",
        String::from_utf8_lossy(&parsed_yaml.stderr)
    );
    let yaml_value: serde_json::Value = serde_json::from_slice(&parsed_yaml.stdout).unwrap();
    let spec = workflow::validate(yaml_value, "America/New_York").unwrap();
    assert_eq!(spec.sources.len(), 2);
    assert_eq!(spec.max_jobs_per_run, Some(40));
    assert_eq!(
        spec.filters
            .as_ref()
            .unwrap()
            .matching
            .as_ref()
            .unwrap()
            .target_roles
            .len(),
        30
    );

    let mut details = std::collections::HashMap::new();
    let mut hubs = std::collections::HashMap::new();
    for (slug, prefix) in [("palantir", "palantir"), ("sep", "sep")] {
        let mut links = Vec::new();
        for n in 0..23 {
            let id = format!("{prefix}-{n:02}");
            if n == 0 {
                links.push(format!(
                    "<article><a href='/{slug}/jobs/{id}'>Software Engineer</a></article>"
                ));
                details.insert(
                    format!("/jobs.lever.co/{slug}/jobs/{id}"),
                    format!(
                        r#"<!DOCTYPE html><html><head><script type="application/ld+json">{{"@context":"https://schema.org","@type":"JobPosting","title":"Software Engineer","hiringOrganization":{{"name":"{prefix}"}},"datePosted":"2026-10-26T10:00:00Z","jobLocation":{{"address":{{"addressCountry":"US","addressLocality":"Austin"}}}},"description":"Build dependable distributed software systems."}}</script></head><body><h1>Software Engineer</h1></body></html>"#
                    ),
                );
            } else if n == 1 {
                links.push(format!(
                    r#"<div class="posting"><a class="posting-title" href="/{slug}/jobs/{id}"><h5>Software Engineer</h5></a></div>"#
                ));
                details.insert(
                    format!("/jobs.lever.co/{slug}/jobs/{id}"),
                    "<html><h1>Software Engineer</h1><main><div class='job-description'><p>Build dependable software systems and distributed platform services.</p></div><div class='location'>Austin, TX</div><time datetime='2026-10-26T10:00:00Z' itemprop='datePosted'>Posted today</time></main></html>".to_string(),
                );
            } else {
                links.push(format!(
                    "<article><a href='/{slug}/jobs/{id}'>Software Engineer</a></article>"
                ));
                details.insert(
                    format!("/jobs.lever.co/{slug}/jobs/{id}"),
                    "<html><h1>Software Engineer</h1><main><div class='job-description'><p>Build dependable software systems and distributed platform services.</p></div><div class='location'>Austin, TX</div><time datetime='2026-10-26T10:00:00Z' itemprop='datePosted'>Posted today</time></main></html>".to_string(),
                );
            }
        }
        if slug == "palantir" {
            for (id, title, location, date) in [
                (
                    "old",
                    "Software Engineer",
                    "Austin, TX",
                    Some("2026-10-25T15:59:59.999Z"),
                ),
                ("undated", "Software Engineer", "Austin, TX", None),
                (
                    "canada",
                    "Software Engineer",
                    "Toronto, ON, Canada",
                    Some("2026-10-26T10:00:00Z"),
                ),
                (
                    "unknown",
                    "Software Engineer",
                    "Remote",
                    Some("2026-10-26T10:00:00Z"),
                ),
                (
                    "unrelated",
                    "Accounting Manager",
                    "Austin, TX",
                    Some("2026-10-26T10:00:00Z"),
                ),
            ] {
                links.push(format!(
                    "<article><a href='/palantir/jobs/{id}'>{title}</a></article>"
                ));
                let date = date
                    .map(|value| {
                        format!(
                            "<time datetime='{value}' itemprop='datePosted'>Posted today</time>"
                        )
                    })
                    .unwrap_or_default();
                details.insert(
                    format!("/jobs.lever.co/palantir/jobs/{id}"),
                    format!(
                        "<html><h1>{title}</h1><main><div class='job-description'><p>Build dependable software systems and distributed platform services.</p></div><div class='location'>{location}</div>{date}</main></html>"
                    ),
                );
            }
        }
        hubs.insert(
            format!("/jobs.lever.co/{slug}/openings"),
            format!("<html><body>{}</body></html>", links.join("")),
        );
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_server = Arc::clone(&stopped);
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_requests = Arc::clone(&requests);
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(90);
        while !stop_server.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buffer = [0; 8192];
            let count = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]);
            let path = request.split_whitespace().nth(1).unwrap().to_string();
            server_requests.lock().unwrap().push(path.clone());
            let route = path.split('?').next().unwrap().trim_end_matches('/');
            let body = match route {
                "/jobs.lever.co/palantir" => {
                    "<a href='/palantir/openings'>Openings</a>".to_string()
                }
                "/jobs.lever.co/sep" => "<a href='/sep/openings'>Openings</a>".to_string(),
                "/jobs.lever.co/palantir/openings" | "/jobs.lever.co/sep/openings" => hubs
                    .get(route)
                    .unwrap_or_else(|| panic!("unexpected crawl hub {route}"))
                    .clone(),
                other => details
                    .get(other)
                    .unwrap_or_else(|| panic!("unexpected replay route {other}"))
                    .clone(),
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });

    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("crawler-workflow.db");
    let pool = scrape_lib::db::init_db(&database).await.unwrap();
    let saved = workflow::persist(&pool, spec, None, true, "America/New_York")
        .await
        .unwrap();
    let saved = SourcesRepo::get(&pool, &saved.id).await.unwrap();
    let mut config = execution_diagnostics::prepare(&saved, "America/New_York", STARTED).unwrap();
    assert_eq!(config.max_jobs_per_run, Some(40));
    assert_eq!(config.sources.len(), 2);
    let mut preview = config.clone();
    execution_diagnostics::cap_preview_crawl_pages(&mut preview.sources);
    for source in &preview.sources {
        assert_eq!(source.config["crawl"]["max_pages"], json!(6));
    }
    let unchanged = SourcesRepo::get(&pool, &saved.id).await.unwrap();
    let full_run = execution_diagnostics::prepare(&unchanged, "America/New_York", STARTED).unwrap();
    for source in &full_run.sources {
        assert_eq!(source.config["crawl"]["max_pages"], json!(50));
    }
    assert_eq!(full_run.limits.max_source_duration_s, 180);
    assert_eq!(full_run.limits.max_workflow_duration_s, 600);
    for source in &mut config.sources {
        assert_eq!(source.kind, SourceKind::Selector);
        source.config["per_host_delay_ms"] = json!(0);
        source.config["jitter_ms"] = json!(0);
    }
    config.limits.per_host_delay_ms = 0;
    config.limits.jitter_ms = Some(0);
    config.limits.request_timeout_s = 5;
    for source in &mut preview.sources {
        source.config["per_host_delay_ms"] = json!(0);
        source.config["jitter_ms"] = json!(0);
    }
    preview.limits.per_host_delay_ms = 0;
    preview.limits.jitter_ms = Some(0);
    preview.limits.request_timeout_s = 5;
    preview.limits.max_source_duration_s = 20;
    preview.limits.max_workflow_duration_s = 45;
    preview.max_jobs_per_run = Some(10);
    let preview_events = invoke_replay(&preview, true, Some(&origin));
    assert!(
        !preview_events
            .iter()
            .any(|event| matches!(event, WorkerEvent::Error { .. })),
        "preview worker errors: {preview_events:?}"
    );
    let WorkerEvent::Done { stats } = preview_events.last().unwrap() else {
        panic!("missing Preview completion")
    };
    assert_eq!(stats.sources.len(), 2);
    assert!(stats.jobs_emitted <= 10);
    assert!(preview
        .sources
        .iter()
        .all(|source| { source.config["crawl"]["max_pages"] == json!(6) }));
    assert!(config
        .sources
        .iter()
        .all(|source| { source.config["crawl"]["max_pages"] == json!(50) }));

    let mut expected_ids = None;
    for pass in 0..2 {
        let events = invoke_replay(&config, false, Some(&origin));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, WorkerEvent::Error { .. })),
            "worker errors: {events:?}"
        );
        let WorkerEvent::Done { stats } = events.last().unwrap() else {
            panic!("missing worker completion")
        };
        assert_eq!(stats.jobs_emitted, 40);
        let batch: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .cloned()
            .map(|event| scraped_job_from_event(event).unwrap())
            .collect();
        assert_eq!(batch.len(), 40);
        let ids: std::collections::BTreeSet<_> = batch
            .iter()
            .map(|job| job.external_id.clone().unwrap())
            .collect();
        assert_eq!(ids.len(), 40);
        if let Some(first) = &expected_ids {
            assert_eq!(&ids, first, "rerun changed which 40 jobs were accepted");
        } else {
            expected_ids = Some(ids.clone());
        }
        for job in &batch {
            assert_eq!(job.date_posted.as_deref(), Some("2026-10-26T10:00:00Z"));
            assert!(
                job.locations
                    .iter()
                    .any(|location| { location.country_code.as_deref() == Some("US") }),
                "job lacks US evidence: {job:?}"
            );
            assert!(!job.title.contains("Accounting Manager"));
        }
        let counts = JobsRepo::upsert_scraped(&pool, &batch).await.unwrap();
        assert_eq!(
            (counts.inserted, counts.updated),
            if pass == 0 { (40, 0) } else { (0, 40) }
        );
        let discovered = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
        assert_eq!(discovered.total, 40);
        assert!(discovered.items.iter().all(|job| {
            job.date_posted.as_deref() == Some("2026-10-26T10:00:00Z")
                && job
                    .locations
                    .iter()
                    .any(|location| location.country_code.as_deref() == Some("US"))
        }));
        for negative in ["old", "undated", "canada", "unknown", "unrelated"] {
            assert!(
                !discovered.items.iter().any(|job| {
                    job.external_id
                        .as_deref()
                        .is_some_and(|id| id.ends_with(negative))
                }),
                "negative candidate {negative} persisted"
            );
        }
    }
    stopped.store(true, Ordering::SeqCst);
    server.join().unwrap();
    {
        let observed = requests.lock().unwrap();
        assert!(observed
            .iter()
            .any(|path| path == "/jobs.lever.co/palantir"));
        assert!(observed.iter().any(|path| path == "/jobs.lever.co/sep"));
        assert!(observed.iter().any(|path| path.contains("/jobs/palantir-")));
        assert!(observed.iter().any(|path| path.contains("/jobs/sep-")));
        assert!(observed
            .iter()
            .all(|path| !path.contains("api") && !path.ends_with("/robots.txt")));
    }

    pool.close().await;
    let reopened = scrape_lib::db::init_db(&database).await.unwrap();
    let reopened_source = SourcesRepo::get(&reopened, &saved.id).await.unwrap();
    assert_eq!(
        reopened_source.config["__workflow_spec"]["sources"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        reopened_source.config["__workflow_spec"]["max_jobs_per_run"],
        json!(40)
    );
    assert_eq!(
        JobsRepo::list(&reopened, JobFilter::default())
            .await
            .unwrap()
            .total,
        40
    );
    reopened.close().await;
}

#[test]
fn public_html_preview_cap_preserves_smaller_and_unrelated_sources() {
    use scrape_lib::services::execution_diagnostics::cap_preview_crawl_pages;
    use serde_json::json;
    let make = |kind, config| WorkerSource {
        id: "preview-fixture".into(),
        name: "Fictional Preview".into(),
        kind,
        identifier: "https://example.com/jobs".into(),
        config,
    };
    let mut sources = vec![
        make(
            SourceKind::Selector,
            json!({"crawl":{"extraction":"public_html","max_pages":50}}),
        ),
        make(
            SourceKind::Selector,
            json!({"crawl":{"extraction":"public_html","max_pages":2}}),
        ),
        make(
            SourceKind::Selector,
            json!({"crawl":{"extraction":"public_html"}}),
        ),
        make(
            SourceKind::Selector,
            json!({"crawl":{"extraction":"jsonld","max_pages":50}}),
        ),
        make(
            SourceKind::Jsonld,
            json!({"crawl":{"extraction":"public_html","max_pages":50}}),
        ),
        make(SourceKind::Selector, json!({"container":".job"})),
        make(
            SourceKind::Search,
            json!({"follow_detail": true, "max_pages": 50}),
        ),
        make(
            SourceKind::Search,
            json!({"follow_detail": true, "max_pages": 3}),
        ),
        make(
            SourceKind::Search,
            json!({"follow_detail": false, "max_pages": 50}),
        ),
    ];
    let original = sources.clone();
    cap_preview_crawl_pages(&mut sources);
    assert_eq!(sources[0].config["crawl"]["max_pages"], json!(6));
    assert_eq!(sources[1].config["crawl"]["max_pages"], json!(2));
    assert_eq!(sources[2].config["crawl"]["max_pages"], json!(6));
    assert_eq!(sources[6].config["max_pages"], json!(6));
    assert_eq!(sources[7].config["max_pages"], json!(3));
    assert_eq!(sources[8].config["max_pages"], json!(50));
    for index in [3, 4, 5, 8] {
        assert_eq!(sources[index].config, original[index].config);
    }
    assert_eq!(original[0].config["crawl"]["max_pages"], json!(50));
}

#[test]
fn preview_scan_summary_distinguishes_coverage_limits_and_unresolved_evidence() {
    use scrape_core::protocol::{CoverageTermination, ScanOutcome, SourceScanStats};
    use scrape_lib::services::execution_diagnostics::preview_scan_summary;
    let time_limited = SourceScanStats {
        termination: CoverageTermination::TimeLimit,
        outcome: Some(ScanOutcome::Partial),
        ..Default::default()
    };
    let summary = preview_scan_summary(&time_limited);
    assert!(summary.sampled);
    assert_eq!(summary.error, None);

    let mut unresolved = SourceScanStats {
        termination: CoverageTermination::PageLimit,
        outcome: Some(ScanOutcome::Partial),
        unresolved: 3,
        ..Default::default()
    };
    let summary = preview_scan_summary(&unresolved);
    assert!(summary.sampled);
    assert_eq!(
        summary.error.as_deref(),
        Some("Source processing incomplete: 3 candidates need eligibility evidence. Inspect scan details and retry.")
    );

    unresolved.outcome = Some(ScanOutcome::Failed);
    assert_eq!(
        preview_scan_summary(&unresolved).error.as_deref(),
        Some("Source could not complete usable processing. Inspect diagnostics and retry.")
    );
}

#[tokio::test]
async fn imported_software_engineering_discovery_yaml_persists_discoveries_to_sqlite() {
    use scrape_lib::services::{execution_diagnostics, workflow};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    const STARTED: &str = "2026-10-26T16:00:00Z";
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let parse_script = r#"import fs from "node:fs"; import { parse } from "yaml"; console.log(JSON.stringify(parse(fs.readFileSync("test-workflows/software-engineering-discovery-test.yaml", "utf8"))));"#;
    let parsed_yaml = Command::new("node")
        .current_dir(repo)
        .args(["--input-type=module", "-e", parse_script])
        .output()
        .unwrap();
    assert!(
        parsed_yaml.status.success(),
        "discovery workflow YAML parser failed: {}",
        String::from_utf8_lossy(&parsed_yaml.stderr)
    );
    let yaml_value: serde_json::Value = serde_json::from_slice(&parsed_yaml.stdout).unwrap();
    let spec = workflow::validate(yaml_value, "America/New_York").unwrap();
    assert_eq!(spec.version, 11);
    assert_eq!(spec.sources.len(), 4);
    assert_eq!(
        spec.sources
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        [
            "heuristic-public-html",
            "muse-computer-it",
            "muse-data-analytics",
            "muse-data-science"
        ]
    );
    assert_eq!(spec.sources[0].extractor.kind, SourceKind::Search);
    assert_eq!(
        spec.sources[0]
            .extractor
            .source_config
            .as_ref()
            .and_then(|c| c.get("follow_detail"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        spec.sources[0]
            .extractor
            .source_config
            .as_ref()
            .and_then(|c| c.get("max_pages"))
            .and_then(|v| v.as_u64()),
        Some(50)
    );
    assert!(spec
        .filters
        .as_ref()
        .unwrap()
        .matching
        .as_ref()
        .unwrap()
        .companies
        .is_empty());

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_server = Arc::clone(&stopped);
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let server_requests = Arc::clone(&requests);

    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(60);
        while !stop_server.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(2));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buffer = [0; 8192];
            let count = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..count]);
            let path = request.split_whitespace().nth(1).unwrap().to_string();
            server_requests.lock().unwrap().push(path.clone());
            let (status, body) = if path.ends_with("/robots.txt") {
                ("200 OK", "User-agent: *\nAllow: /\n".to_string())
            } else if path.starts_with("/www.themuse.com/api/public/jobs") {
                ("200 OK", json!({
                    "page": 0, "page_count": 1,
                    "results": [
                        {
                            "id": 901, "name": "Software Engineer", "company": {"name": "Discovered Corp"},
                            "refs": {"landing_page": "https://www.themuse.com/jobs/discovered/901"},
                            "contents": "<p>Engineering role</p>",
                            "publication_date": "2026-10-26T10:00:00Z",
                            "locations": [{"name": "Austin, TX"}]
                        }
                    ]
                }).to_string())
            } else if path == "/www.themuse.com/jobs/discovered/901" {
                ("200 OK", r#"<!DOCTYPE html><html><head><script type="application/ld+json">{"@context":"https://schema.org","@type":"JobPosting","title":"Software Engineer","hiringOrganization":{"name":"Discovered Corp"},"datePosted":"2026-10-26T10:00:00Z","jobLocation":{"address":{"addressCountry":"US","addressLocality":"Austin"}},"description":"Build distributed software systems"}</script></head><body><h1>Software Engineer</h1></body></html>"#.to_string())
            } else {
                ("404 Not Found", "not found".to_string())
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });

    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("discovery-workflow.db");
    let pool = scrape_lib::db::init_db(&database).await.unwrap();
    let saved = workflow::persist(&pool, spec, None, true, "America/New_York")
        .await
        .unwrap();
    let saved = SourcesRepo::get(&pool, &saved.id).await.unwrap();
    let mut config = execution_diagnostics::prepare(&saved, "America/New_York", STARTED).unwrap();
    assert_eq!(config.max_jobs_per_run, Some(40));

    let mut preview = config.clone();
    execution_diagnostics::cap_preview_crawl_pages(&mut preview.sources);
    assert_eq!(preview.sources[0].config["max_pages"], json!(6));

    let full_run = execution_diagnostics::prepare(&saved, "America/New_York", STARTED).unwrap();
    assert_eq!(full_run.sources[0].config["max_pages"], json!(50));

    config.limits.per_host_delay_ms = 0;
    config.limits.request_timeout_s = 5;
    let events = invoke_replay(&config, false, Some(&origin));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, WorkerEvent::Error { .. })),
        "errors: {events:?}"
    );
    let jobs: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .cloned()
        .map(|e| scraped_job_from_event(e).unwrap())
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].company, "Discovered Corp");
    assert_eq!(jobs[0].date_posted.as_deref(), Some("2026-10-26T10:00:00Z"));

    JobsRepo::upsert_scraped(&pool, &jobs).await.unwrap();
    let discovered = JobsRepo::list(&pool, JobFilter::default()).await.unwrap();
    assert_eq!(discovered.total, 1);
    assert_eq!(discovered.items[0].company, "Discovered Corp");
    let paths = requests.lock().unwrap();
    for category in [
        "Computer%20and%20IT",
        "Data%20and%20Analytics",
        "Data%20Science",
    ] {
        assert!(
            paths
                .iter()
                .any(|path| path.contains(&format!("category={category}"))),
            "{paths:?}"
        );
    }
    drop(paths);

    stopped.store(true, Ordering::SeqCst);
    server.join().unwrap();
}

#[test]
fn version_ten_compatibility_and_previous_worker_protocol_enforcement() {
    use scrape_lib::services::workflow;
    use serde_json::json;

    // v9 converts to v11 without changing filters or opting into detail crawling.
    let v9 = json!({
        "version": 9,
        "metadata": {"name": "V9 Compatibility"},
        "sources": [{
            "id": "s1", "label": "Search", "enabled": true,
            "extractor": {"kind": "search", "url": "themuse"}
        }],
        "filters": {"role_types": []}
    });
    for version in [9, 10] {
        let mut previous = v9.clone();
        previous["version"] = version.into();
        let spec = workflow::validate(previous, "UTC").unwrap();
        assert_eq!(spec.version, 11);
        assert!(spec.sources[0].extractor.source_config.is_none());
    }

    // The worker rejects the immediately previous protocol with an explicit error.
    let mut child = Command::new(worker())
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let old_protocol_config = json!({
        "version": 14,
        "sources": [],
        "limits": { "per_host_delay_ms": 0, "request_timeout_s": 5 }
    });
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&old_protocol_config).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported protocol version: expected 15, got 14"),
        "unexpected stderr: {stderr}"
    );
}
