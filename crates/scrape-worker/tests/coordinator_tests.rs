use scrape_core::{WorkerEvent, PROTOCOL_VERSION};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

struct Server {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new(handler: impl Fn(&str) -> String + Send + 'static) -> Self {
        Self::new_response(move |path| (200, Vec::new(), handler(path)))
    }

    fn new_response(
        handler: impl Fn(&str) -> (u16, Vec<(String, String)>, String) + Send + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stopping = stop.clone();
        let seen = requests.clone();
        let thread = thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let mut buf = [0; 4096];
                        let n = stream.read(&mut buf).unwrap();
                        let request = String::from_utf8_lossy(&buf[..n]);
                        let path = request.split_whitespace().nth(1).unwrap().to_string();
                        seen.lock().unwrap().push(path.clone());
                        let (status, headers, body) = if path.ends_with("/robots.txt") {
                            (200, Vec::new(), "User-agent: *\nAllow: /".into())
                        } else {
                            handler(&path)
                        };
                        let mut extra_headers = String::new();
                        for (name, value) in headers {
                            extra_headers.push_str(&format!("{name}: {value}\r\n"));
                        }
                        let _ = write!(
                            stream,
                            "HTTP/1.1 {status} Response\r\n{extra_headers}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(), body
                        );
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            origin,
            requests,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn generate_lever_postings(count: usize, prefix: &str) -> String {
    let items: Vec<Value> = (0..count)
        .map(|i| {
            json!({
                "id": format!("{prefix}-{i}"),
                "text": format!("Software Engineer {prefix} {i}"),
                "hostedUrl": format!("https://jobs.lever.co/{prefix}/{prefix}-{i}"),
                "createdAt": 1700000000000i64,
                "categories": {
                    "location": "San Francisco, CA",
                    "commitment": "Full-time"
                }
            })
        })
        .collect();
    serde_json::to_string(&items).unwrap()
}

fn run_worker(
    server: &Server,
    sources: Vec<Value>,
    max_jobs_per_run: u32,
    starting_source_index: usize,
    preview: bool,
) -> Vec<WorkerEvent> {
    run_worker_config(
        server,
        sources,
        json!({}),
        max_jobs_per_run,
        starting_source_index,
        preview,
        worker_limits(),
    )
}

fn worker_limits() -> Value {
    json!({"per_host_delay_ms": 0, "request_timeout_s": 3})
}

fn run_worker_config(
    server: &Server,
    sources: Vec<Value>,
    filters: Value,
    max_jobs_per_run: u32,
    starting_source_index: usize,
    preview: bool,
    limits: Value,
) -> Vec<WorkerEvent> {
    let config = json!({
        "version": PROTOCOL_VERSION,
        "sources": sources,
        "filters": filters,
        "blocked_postings": [],
        "run_started_at": "2026-10-16T00:00:00Z",
        "limits": limits,
        "max_jobs_per_run": max_jobs_per_run,
        "starting_source_index": starting_source_index,
    });
    let output = worker_output(server, &config, preview);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<WorkerEvent>(line).unwrap())
        .collect()
}

fn worker_output(server: &Server, config: &Value, preview: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scrape-worker"));
    command.args(["--stdin", "--replay-origin", &server.origin]);
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
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();

    child.wait_with_output().unwrap()
}

#[test]
fn feedback_quota_counts_committed_insertions_instead_of_refreshes() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-a") {
            generate_lever_postings(6, "source-a")
        } else {
            "[]".into()
        }
    });
    let config = json!({
        "version": PROTOCOL_VERSION,
        "commit_feedback": true,
        "sources": [{"id":"source-a","kind":"lever","name":"A","identifier":"source-a","config":{}}],
        "limits": worker_limits(),
        "max_jobs_per_run": 2
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &server.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{config}").unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut jobs = 0;
    let mut done = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            break;
        }
        match serde_json::from_str::<WorkerEvent>(&line).unwrap() {
            WorkerEvent::Job {
                source_id,
                external_id,
                ..
            } => {
                jobs += 1;
                let outcome = if jobs <= 2 { "updated" } else { "inserted" };
                writeln!(
                    input,
                    "{}",
                    json!({"source_id":source_id,"external_id":external_id,"outcome":outcome})
                )
                .unwrap();
            }
            WorkerEvent::Done { stats } => {
                done = Some(stats);
                break;
            }
            _ => {}
        }
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(jobs, 4);
    let stats = done.unwrap();
    assert_eq!(stats.sources[0].accepted, 4);
    assert_eq!(stats.sources[0].newly_committed, 2);
    assert!(stats.sources[0].limit_reached);
    stats.sources[0].validate().unwrap();
}

#[test]
fn feedback_rejects_wrong_malformed_duplicate_and_eof_acknowledgements() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-a") {
            generate_lever_postings(3, "source-a")
        } else {
            "[]".into()
        }
    });
    for mode in ["wrong", "malformed", "duplicate", "eof"] {
        let config = json!({"version":PROTOCOL_VERSION,"commit_feedback":true,
            "sources":[{"id":"source-a","kind":"lever","name":"A","identifier":"source-a","config":{}}],
            "limits":worker_limits(),"max_jobs_per_run":2});
        let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
            .args(["--stdin", "--replay-origin", &server.origin])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        writeln!(input, "{config}").unwrap();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        loop {
            let mut line = String::new();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if let WorkerEvent::Job {
                source_id,
                external_id,
                ..
            } = serde_json::from_str(&line).unwrap()
            {
                match mode {
                    "wrong" => writeln!(
                        input,
                        "{}",
                        json!({"source_id":source_id,"external_id":"wrong","outcome":"inserted"})
                    )
                    .unwrap(),
                    "malformed" => writeln!(input, "{{bad").unwrap(),
                    "duplicate" => {
                        let ack = json!({"source_id":source_id,"external_id":external_id,"outcome":"updated"});
                        writeln!(input, "{ack}\n{ack}").unwrap();
                    }
                    _ => {}
                }
                break;
            }
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success(), "{mode} unexpectedly succeeded");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Commit acknowledgement failed"),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn missing_acknowledgement_fails_at_workflow_deadline_without_closing_stdin() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-a") {
            generate_lever_postings(1, "source-a")
        } else {
            "[]".into()
        }
    });
    let config = json!({"version":PROTOCOL_VERSION,"commit_feedback":true,
        "sources":[{"id":"source-a","kind":"lever","name":"A","identifier":"source-a","config":{}}],
        "limits":{"per_host_delay_ms":0,"request_timeout_s":3,"max_workflow_duration_s":30},"max_jobs_per_run":2});
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &server.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{config}").unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if matches!(
            serde_json::from_str::<WorkerEvent>(&line).unwrap(),
            WorkerEvent::Job { .. }
        ) {
            break;
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(35);
    while child.try_wait().unwrap().is_none() {
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            panic!("Worker did not terminate after acknowledgement deadline");
        }
        thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    drop(input);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Commit acknowledgement timed out"));
}

#[test]
fn feedback_preserves_fairness_and_reports_exhaustion_below_target() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-a") {
            generate_lever_postings(1, "source-a")
        } else if path.contains("/v0/postings/source-b") {
            generate_lever_postings(1, "source-b")
        } else {
            "[]".into()
        }
    });
    let config = json!({"version":PROTOCOL_VERSION,"commit_feedback":true,
        "sources":[{"id":"source-a","kind":"lever","name":"A","identifier":"source-a","config":{}},
                   {"id":"source-b","kind":"lever","name":"B","identifier":"source-b","config":{}}],
        "limits":worker_limits(),"max_jobs_per_run":3});
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &server.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{config}").unwrap();
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut order = Vec::new();
    let stats = loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        match serde_json::from_str::<WorkerEvent>(&line).unwrap() {
            WorkerEvent::Job {
                source_id,
                external_id,
                ..
            } => {
                let outcome = if source_id == "source-a" {
                    "updated"
                } else {
                    "inserted"
                };
                order.push(source_id.clone());
                writeln!(
                    input,
                    "{}",
                    json!({"source_id":source_id,"external_id":external_id,"outcome":outcome})
                )
                .unwrap();
            }
            WorkerEvent::Done { stats } => break stats,
            _ => {}
        }
    };
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(order, ["source-a", "source-b"]);
    assert_eq!(
        stats.sources.iter().map(|s| s.newly_committed).sum::<u64>(),
        1
    );
    for scan in &stats.sources {
        scan.validate().unwrap();
        assert!(!scan.limit_reached);
        assert_eq!(
            scan.termination,
            scrape_core::protocol::CoverageTermination::Exhausted
        );
    }
}

#[test]
fn feedback_target_stops_before_fetching_next_detail() {
    let server = Server::new(|path| {
        match path {
        "/fixture.test/careers" => (0..4).map(|id| format!("<article data-id='{id}'><a class=title href='/jobs/{id}'>Engineer {id}</a></article>")).collect(),
        "/fixture.test/jobs/0" | "/fixture.test/jobs/1" | "/fixture.test/jobs/2" => "<time datetime='2026-10-15'></time>".into(),
        _ => panic!("Unexpected request: {path}"),
    }
    });
    let config = json!({"version":PROTOCOL_VERSION,"commit_feedback":true,"run_started_at":"2026-10-16T00:00:00Z",
        "sources":[{"id":"static","kind":"selector","name":"Fixture","identifier":"https://fixture.test/careers",
            "config":{"item":"article","title":".title","link":"a","id":".@data-id","follow_detail":true,"detail_fields":{"date":"time"}}}],
        "filters":{"matching":{"posting_date":{"last_hours":72,"timezone":"UTC"}}},
        "limits":worker_limits(),"max_jobs_per_run":2});
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &server.origin])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{config}").unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut jobs = 0;
    loop {
        let mut line = String::new();
        assert_ne!(output.read_line(&mut line).unwrap(), 0);
        match serde_json::from_str::<WorkerEvent>(&line).unwrap() {
            WorkerEvent::Job {
                source_id,
                external_id,
                ..
            } => {
                jobs += 1;
                let outcome = if jobs == 1 { "updated" } else { "inserted" };
                writeln!(
                    input,
                    "{}",
                    json!({"source_id":source_id,"external_id":external_id,"outcome":outcome})
                )
                .unwrap();
            }
            WorkerEvent::Done { .. } => break,
            _ => {}
        }
    }
    drop(input);
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(jobs, 3);
    let requests = server.requests.lock().unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|path| path.contains("/jobs/"))
            .count(),
        3
    );
    assert!(!requests.iter().any(|path| path.contains("/jobs/3")));
}

#[test]
fn three_healthy_sources_yield_7_7_6_at_cap_20() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-a") {
            generate_lever_postings(30, "source-a")
        } else if path.contains("/v0/postings/source-b") {
            generate_lever_postings(30, "source-b")
        } else if path.contains("/v0/postings/source-c") {
            generate_lever_postings(30, "source-c")
        } else {
            "[]".into()
        }
    });

    let sources = vec![
        json!({
            "id": "source-a",
            "kind": "lever",
            "name": "Company A",
            "identifier": "source-a",
            "config": {}
        }),
        json!({
            "id": "source-b",
            "kind": "lever",
            "name": "Company B",
            "identifier": "source-b",
            "config": {}
        }),
        json!({
            "id": "source-c",
            "kind": "lever",
            "name": "Company C",
            "identifier": "source-c",
            "config": {}
        }),
    ];

    let events = run_worker(&server, sources, 20, 0, false);

    let mut count_a = 0;
    let mut count_b = 0;
    let mut count_c = 0;
    let mut total_jobs = 0;

    for event in &events {
        if let WorkerEvent::Job { source_id, .. } = event {
            total_jobs += 1;
            match source_id.as_str() {
                "source-a" => count_a += 1,
                "source-b" => count_b += 1,
                "source-c" => count_c += 1,
                other => panic!("Unexpected source_id: {other}"),
            }
        }
    }

    assert_eq!(
        total_jobs, 20,
        "Total accepted jobs must equal the workflow cap of 20"
    );
    assert_eq!(count_a, 7, "Source A must yield 7 jobs");
    assert_eq!(count_b, 7, "Source B must yield 7 jobs");
    assert_eq!(count_c, 6, "Source C must yield 6 jobs");

    // Also verify stats in Done event
    let done = events
        .iter()
        .find_map(|e| match e {
            WorkerEvent::Done { stats } => Some(stats),
            _ => None,
        })
        .expect("Worker must emit Done event");

    assert_eq!(done.jobs_emitted, 20);
    let stat_a = done
        .sources
        .iter()
        .find(|s| s.source_id == "source-a")
        .unwrap();
    let stat_b = done
        .sources
        .iter()
        .find(|s| s.source_id == "source-b")
        .unwrap();
    let stat_c = done
        .sources
        .iter()
        .find(|s| s.source_id == "source-c")
        .unwrap();

    assert_eq!(stat_a.accepted, 7);
    assert_eq!(stat_b.accepted, 7);
    assert_eq!(stat_c.accepted, 6);
}

#[test]
fn starting_source_rotation_rotates_fair_budget_split() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-a") {
            generate_lever_postings(30, "source-a")
        } else if path.contains("/v0/postings/source-b") {
            generate_lever_postings(30, "source-b")
        } else if path.contains("/v0/postings/source-c") {
            generate_lever_postings(30, "source-c")
        } else {
            "[]".into()
        }
    });

    let sources = vec![
        json!({
            "id": "source-a",
            "kind": "lever",
            "name": "Company A",
            "identifier": "source-a",
            "config": {}
        }),
        json!({
            "id": "source-b",
            "kind": "lever",
            "name": "Company B",
            "identifier": "source-b",
            "config": {}
        }),
        json!({
            "id": "source-c",
            "kind": "lever",
            "name": "Company C",
            "identifier": "source-c",
            "config": {}
        }),
    ];

    // Start at source 1 (Source B): Round-robin order B, C, A
    // Cap 20: B gets 7, C gets 7, A gets 6!
    let events = run_worker(&server, sources.clone(), 20, 1, false);

    let mut count_a = 0;
    let mut count_b = 0;
    let mut count_c = 0;
    for event in &events {
        if let WorkerEvent::Job { source_id, .. } = event {
            match source_id.as_str() {
                "source-a" => count_a += 1,
                "source-b" => count_b += 1,
                "source-c" => count_c += 1,
                _ => {}
            }
        }
    }
    assert_eq!(
        count_a, 6,
        "Source A must yield 6 when rotated to last position"
    );
    assert_eq!(count_b, 7, "Source B must yield 7 when starting");
    assert_eq!(count_c, 7, "Source C must yield 7 when second");

    // Start at source 2 (Source C): Order C, A, B -> C: 7, A: 7, B: 6!
    let events = run_worker(&server, sources, 20, 2, false);
    let mut count_a = 0;
    let mut count_b = 0;
    let mut count_c = 0;
    for event in &events {
        if let WorkerEvent::Job { source_id, .. } = event {
            match source_id.as_str() {
                "source-a" => count_a += 1,
                "source-b" => count_b += 1,
                "source-c" => count_c += 1,
                _ => {}
            }
        }
    }
    assert_eq!(count_a, 7, "Source A must yield 7 when second");
    assert_eq!(count_b, 6, "Source B must yield 6 when third");
    assert_eq!(count_c, 7, "Source C must yield 7 when starting");
}

#[test]
fn slow_or_failed_source_consumes_no_slot_and_redistributes_capacity() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/healthy-a") {
            generate_lever_postings(30, "healthy-a")
        } else if path.contains("/v0/postings/healthy-b") {
            generate_lever_postings(30, "healthy-b")
        } else {
            // failed-source returns invalid JSON or 0 postings
            "invalid-json".into()
        }
    });

    let sources = vec![
        json!({
            "id": "failed-source",
            "kind": "lever",
            "name": "Failing Co",
            "identifier": "failed-source",
            "config": {}
        }),
        json!({
            "id": "healthy-a",
            "kind": "lever",
            "name": "Company A",
            "identifier": "healthy-a",
            "config": {}
        }),
        json!({
            "id": "healthy-b",
            "kind": "lever",
            "name": "Company B",
            "identifier": "healthy-b",
            "config": {}
        }),
    ];

    let events = run_worker(&server, sources, 20, 0, false);

    let mut count_failed = 0;
    let mut count_a = 0;
    let mut count_b = 0;
    for event in &events {
        if let WorkerEvent::Job { source_id, .. } = event {
            match source_id.as_str() {
                "failed-source" => count_failed += 1,
                "healthy-a" => count_a += 1,
                "healthy-b" => count_b += 1,
                _ => {}
            }
        }
    }

    assert_eq!(count_failed, 0, "Failed source must consume no slots");
    assert_eq!(
        count_a, 10,
        "Healthy A must receive redistributed capacity (10)"
    );
    assert_eq!(
        count_b, 10,
        "Healthy B must receive redistributed capacity (10)"
    );
}

#[test]
fn workflow_wide_canonical_deduplication_does_not_count_duplicate_towards_round_slot() {
    let server = Server::new(|path| {
        if path.contains("/v0/postings/source-dup-a") {
            // Both sources have an identical posting with the same destination URL
            let postings = vec![
                json!({
                    "id": "shared-1",
                    "text": "Shared Role",
                    "hostedUrl": "https://jobs.example.com/roles/shared-1",
                    "createdAt": 1700000000000i64,
                    "categories": { "location": "Remote", "commitment": "Full-time" }
                }),
                json!({
                    "id": "unique-a",
                    "text": "Unique Role A",
                    "hostedUrl": "https://jobs.example.com/roles/unique-a",
                    "createdAt": 1700000000000i64,
                    "categories": { "location": "Remote", "commitment": "Full-time" }
                }),
            ];
            serde_json::to_string(&postings).unwrap()
        } else if path.contains("/v0/postings/source-dup-b") {
            let postings = vec![
                json!({
                    "id": "shared-1-dup",
                    "text": "Shared Role",
                    "hostedUrl": "https://jobs.example.com/roles/shared-1", // same canonical posting identity!
                    "createdAt": 1700000000000i64,
                    "categories": { "location": "Remote", "commitment": "Full-time" }
                }),
                json!({
                    "id": "unique-b",
                    "text": "Unique Role B",
                    "hostedUrl": "https://jobs.example.com/roles/unique-b",
                    "createdAt": 1700000000000i64,
                    "categories": { "location": "Remote", "commitment": "Full-time" }
                }),
            ];
            serde_json::to_string(&postings).unwrap()
        } else {
            "[]".into()
        }
    });

    let sources = vec![
        json!({
            "id": "source-dup-a",
            "kind": "lever",
            "name": "Source A",
            "identifier": "source-dup-a",
            "config": {}
        }),
        json!({
            "id": "source-dup-b",
            "kind": "lever",
            "name": "Source B",
            "identifier": "source-dup-b",
            "config": {}
        }),
    ];

    let events = run_worker(&server, sources, 10, 0, false);

    let mut accepted_urls = Vec::new();
    for event in &events {
        if let WorkerEvent::Job { url, .. } = event {
            accepted_urls.push(url.clone());
        }
    }

    // shared-1 should appear only ONCE across the entire workflow
    assert_eq!(
        accepted_urls
            .iter()
            .filter(|u| u.ends_with("/roles/shared-1"))
            .count(),
        1,
        "Shared role must be deduplicated workflow-wide"
    );
    // And source B should have progressed to unique-b
    assert!(
        accepted_urls.iter().any(|u| u.ends_with("/roles/unique-b")),
        "Source B must yield unique-b after skipping the duplicate"
    );
}

#[test]
fn richer_search_observation_can_qualify_after_unresolved_identity() {
    let server = Server::new(|path| {
        match path {
        "/www.themuse.com/api/public/jobs?page=0" => json!({
            "page": 0, "page_count": 3,
            "results": [muse_job(701, "Software Engineer", "Fixture", Value::Null)]
        }).to_string(),
        "/www.themuse.com/api/public/jobs?page=1" |
        "/www.themuse.com/api/public/jobs?page=2" => json!({
            "page": if path.ends_with("=1") { 1 } else { 2 }, "page_count": 3,
            "results": [muse_job(701, "Software Engineer", "Fixture", json!("2026-10-15T12:00:00Z"))]
        }).to_string(),
        _ => panic!("unexpected request {path}"),
    }
    });
    let events = run_worker_config(
        &server,
        vec![search_source("themuse", json!({}))],
        json!({"matching": {"posting_date": {
            "preset": "past_24h", "timezone": "UTC", "include_unknown": false
        }}}),
        10,
        0,
        false,
        worker_limits(),
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .count(),
        1
    );
    let scan = &scans(&events)[0];
    assert_eq!(scan.examined, 3);
    assert_eq!(scan.unresolved, 1);
    assert_eq!(scan.accepted, 1);
    assert_eq!(scan.duplicates, 1);
    assert_eq!(scan.matched, 1);
}

#[test]
fn healthy_paginated_and_single_page_sources_keep_round_robin_7_7_6_distribution() {
    verify_paginated_fairness(false);
}
#[test]
fn optional_detail_warnings_do_not_cost_a_paginated_source_its_round() {
    verify_paginated_fairness(true);
}
fn verify_paginated_fairness(optional_warnings: bool) {
    let server = Server::new(|path| {
        if path.contains("/jobs/") {
            return "<p>Detail container unavailable</p>".into();
        }
        let (path, page) = path.split_once("?page=").unwrap();
        let board = path.rsplit('/').next().unwrap();
        let page: usize = page.parse().unwrap();
        assert!((1..=7).contains(&page));
        if board != "a" {
            return (1..=20).map(|id| format!("<div class=job-item><a class=job-title href='/jobs/{board}-{id}'>Engineer {board} {id}</a></div>")).collect();
        }
        format!("<div class=job-item><a class=job-title href='/jobs/{board}-{page}'>Engineer {board} {page}</a></div><a class=next href='/{board}?page={}'>Next</a>", page+1)
    });
    let mut sources: Vec<Value> = ["a","b","c"].into_iter().map(|id| json!({"id":id,"name":"Fixture","kind":"selector","identifier":format!("https://fixture.test/{id}?page=1"),"config":{"next_page":"a.next"}})).collect();
    if optional_warnings {
        sources[0]["config"]["follow_detail"] = true.into();
        sources[0]["config"]["detail_container"] = "main".into();
        sources[0]["config"]["detail_fields"] = json!({"date":"time"});
    }
    let events = run_worker(&server, sources, 20, 0, false);
    let counts: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|id| {
            events
                .iter()
                .filter(|event| matches!(event,WorkerEvent::Job {source_id,..} if source_id == id))
                .count()
        })
        .collect();
    assert_eq!(counts, [7, 7, 6]);
}

fn search_source(provider: &str, config: Value) -> Value {
    json!({
        "id": "search", "name": "Cross-company feed", "kind": "search",
        "identifier": provider, "config": config,
    })
}

fn arbeitnow_job(id: &str, title: &str, company: &str) -> Value {
    json!({
        "slug": id, "title": title, "company_name": company,
        "url": format!("https://employer.test/jobs/{id}"),
        "location": "Berlin, Germany", "remote": true,
        "description": "<p>Requires 2 years of experience.</p><p>Visa sponsorship available.</p>",
        "created_at": 1792101600,
    })
}

fn muse_job(id: u64, title: &str, company: &str, publication: Value) -> Value {
    json!({
        "id": id, "name": title, "company": {"name": company},
        "refs": {"landing_page": format!("https://www.themuse.com/jobs/employer/{id}")},
        "contents": "<p>Requires 2 years of experience.</p>",
        "publication_date": publication,
        "locations": [{"name": "Austin, TX"}, {"name": "Berlin, Germany"}],
    })
}

fn scans(events: &[WorkerEvent]) -> &[scrape_core::protocol::SourceScanStats] {
    events
        .iter()
        .find_map(|event| match event {
            WorkerEvent::Done { stats } => Some(stats.sources.as_slice()),
            _ => None,
        })
        .expect("worker summary")
}

fn feed_requests(server: &Server) -> Vec<String> {
    server
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|path| !path.ends_with("/robots.txt"))
        .cloned()
        .collect()
}

#[test]
fn search_hospitality_roles_without_company_restrictions_resolve_today_in_local_timezone() {
    let server = Server::new(|path| {
        assert_eq!(path, "/www.themuse.com/api/public/jobs?page=0");
        json!({"page":0,"page_count":1,"results":[
            muse_job(1, "Line Cook", "Burger King", json!("2026-10-15T14:00:00Z")),
            muse_job(2, "Store Associate", "Target", json!("2026-10-15T18:00:00Z")),
            muse_job(3, "Software Engineer", "Google", json!("2026-10-15T18:00:00Z")),
            muse_job(4, "Line Cook", "Wendy's", json!("2026-10-15T03:59:00Z")),
        ]})
        .to_string()
    });
    let events = run_worker_config(
        &server,
        vec![search_source("", json!({}))],
        json!({"matching":{"target_roles":["Line Cook","Store Associate"],"companies":[],
            "posting_date":{"preset":"today","timezone":"America/New_York","include_unknown":false}}}),
        20,
        0,
        false,
        worker_limits(),
    );
    let jobs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                external_id,
                company,
                ..
            } => Some((external_id.as_str(), company.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(jobs, [("1", "Burger King"), ("2", "Target")]);
    let scan = &scans(&events)[0];
    assert_eq!((scan.accepted, scan.rejected, scan.unresolved), (2, 2, 0));
    assert_eq!(
        scan.outcome,
        Some(scrape_core::protocol::ScanOutcome::Success)
    );
    assert_eq!(
        feed_requests(&server),
        ["/www.themuse.com/api/public/jobs?page=0"]
    );
}

#[test]
fn search_arbeitnow_admits_later_page_employers_without_inventing_publication_dates() {
    let server = Server::new(|path| match path {
        "/www.arbeitnow.com/api/job-board-api" => json!({
            "data": [
                arbeitnow_job("wrong-role", "Product Manager", "Google"),
                arbeitnow_job("wrong-company", "Software Engineer", "Meta"),
            ],
            "links": {"next": "https://www.arbeitnow.com/api/job-board-api?page=2"}
        })
        .to_string(),
        "/www.arbeitnow.com/api/job-board-api?page=2" => json!({
            "data": [
                arbeitnow_job("google", "Software Engineer", "Google"),
                arbeitnow_job("amazon", "Software Engineer", "Amazon"),
            ],
            "links": {"next": null}
        })
        .to_string(),
        _ => panic!("unexpected request {path}"),
    });
    let events = run_worker_config(
        &server,
        vec![search_source(
            "arbeitnow",
            json!({
                "query": "Software Engineer Google Amazon",
                "target_roles": ["Software Engineer"], "companies": ["Google", "Amazon"],
                "location": "Germany", "max_days_old": 7,
            }),
        )],
        json!({"matching": {"target_roles": ["Software Engineer"], "companies": ["Google", "Amazon"]}}),
        10,
        0,
        false,
        worker_limits(),
    );
    let companies: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                company,
                date_posted,
                description_text,
                description_source,
                description_format,
                sponsorship,
                experience,
                locations,
                ..
            } => {
                assert!(
                    date_posted.is_none(),
                    "import/order timestamp is not publication evidence"
                );
                assert!(description_text.as_ref().unwrap().contains("2 years"));
                assert!(!description_text.as_ref().unwrap().contains("<p>"));
                assert!(description_source.as_ref().unwrap().contains("<p>"));
                assert_eq!(
                    *description_format,
                    Some(scrape_core::enrich::DescriptionFormat::Html)
                );
                assert!(sponsorship.is_some());
                assert!(experience.is_some());
                assert!(!locations.is_empty());
                Some(company.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(companies, ["Google", "Amazon"]);
    assert_eq!(
        feed_requests(&server),
        [
            "/www.arbeitnow.com/api/job-board-api",
            "/www.arbeitnow.com/api/job-board-api?page=2",
        ]
    );
}

#[test]
fn default_public_search_feeds_are_both_examined_on_an_explicit_run() {
    let server = Server::new(|path| {
        match path {
        "/www.themuse.com/api/public/jobs?page=0" => json!({
            "page": 0, "page_count": 1,
            "results": [muse_job(1, "Software Engineer", "Muse Fixture", json!("2026-10-15T18:00:00Z"))]
        })
        .to_string(),
        "/www.arbeitnow.com/api/job-board-api" => json!({
            "data": [arbeitnow_job("arbeitnow-1", "Software Engineer", "Arbeitnow Fixture")],
            "links": {"next": null}
        })
        .to_string(),
        _ => panic!("unexpected request {path}"),
    }
    });
    let mut muse = search_source("themuse", json!({}));
    muse["id"] = json!("muse");
    let mut arbeitnow = search_source("arbeitnow", json!({}));
    arbeitnow["id"] = json!("arbeitnow");
    let events = run_worker_config(
        &server,
        vec![muse, arbeitnow],
        json!({"matching": {"target_roles": ["Software Engineer"], "companies": []}}),
        20,
        0,
        false,
        worker_limits(),
    );
    let sources = scans(&events);
    assert_eq!(sources.len(), 2);
    assert_eq!(
        sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<Vec<_>>(),
        ["muse", "arbeitnow"]
    );
    assert!(sources
        .iter()
        .all(|source| source.examined == 1 && source.accepted == 1));
    assert!(sources
        .iter()
        .all(|source| source.outcome == Some(scrape_core::protocol::ScanOutcome::Success)));
    assert_eq!(
        feed_requests(&server),
        [
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.arbeitnow.com/api/job-board-api",
        ]
    );
}

#[test]
fn search_default_muse_evaluates_later_page_roles_companies_and_publication_window() {
    let server = Server::new(|path| match path {
        "/www.themuse.com/api/public/jobs?page=0" => json!({
            "page": 0, "page_count": 2,
            "results": [
                muse_job(1, "Software Engineer", "Google", json!("2026-10-14T00:00:00Z")),
                muse_job(2, "Product Manager", "Google", json!("2026-10-15T20:00:00Z")),
                muse_job(3, "Software Engineer", "Meta", json!("2026-10-15T20:00:00Z")),
            ]
        })
        .to_string(),
        "/www.themuse.com/api/public/jobs?page=1" => json!({
            "page": 1, "page_count": 2,
            "results": [
                muse_job(4, "Software Engineer", "Google", json!("2026-10-15T20:00:00Z")),
                muse_job(5, "Software Engineer", "Amazon", json!("2026-10-15T18:00:00Z")),
                muse_job(6, "Software Engineer", "Google", Value::Null),
            ]
        })
        .to_string(),
        _ => panic!("unexpected request {path}"),
    });
    let events = run_worker_config(
        &server,
        vec![search_source("", json!({}))],
        json!({"matching": {
            "target_roles": ["Software Engineer"], "companies": ["Google", "Amazon"],
            "posting_date": {"preset": "past_12h", "timezone": "UTC", "include_unknown": false}
        }}),
        10,
        0,
        false,
        worker_limits(),
    );
    let jobs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                external_id,
                company,
                date_posted,
                locations,
                ..
            } => {
                assert!(date_posted.is_some());
                assert!(locations
                    .iter()
                    .any(|location| location.raw.contains("Austin")));
                assert!(locations
                    .iter()
                    .any(|location| location.raw.contains("Berlin")));
                Some((external_id.as_str(), company.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(jobs, [("4", "Google"), ("5", "Amazon")]);
    assert_eq!(scans(&events)[0].accepted, 2);
    assert_eq!(scans(&events)[0].unresolved, 1);
    assert_eq!(
        feed_requests(&server),
        [
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.themuse.com/api/public/jobs?page=1",
        ]
    );
}

#[test]
fn search_first_page_workflow_and_source_caps_stop_before_next_request() {
    for source_cap in [false, true] {
        let server = Server::new(|path| {
            assert_eq!(path, "/www.arbeitnow.com/api/job-board-api");
            json!({
                "data": [arbeitnow_job("one", "Line Cook", "Chipotle"),
                    arbeitnow_job("two", "Store Associate", "Target")],
                "links": {"next": "?page=2"}
            })
            .to_string()
        });
        let config = if source_cap {
            json!({"__workflow_spec": {"max_jobs_per_run": 1}})
        } else {
            json!({})
        };
        let events = run_worker(
            &server,
            vec![search_source("arbeitnow", config)],
            if source_cap { 10 } else { 1 },
            0,
            false,
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, WorkerEvent::Job { .. }))
                .count(),
            1
        );
        assert_eq!(
            feed_requests(&server),
            ["/www.arbeitnow.com/api/job-board-api"]
        );
        assert_eq!(
            scans(&events)[0].termination,
            scrape_core::protocol::CoverageTermination::JobLimit
        );
    }
}

#[test]
fn search_malformed_envelopes_fail_instead_of_claiming_empty_success() {
    for (provider, payload) in [
        ("arbeitnow", "{}"),
        ("arbeitnow", "[]"),
        ("arbeitnow", r#"{"data":null}"#),
        ("arbeitnow", r#"{"data":[],"links":{"next":42}}"#),
        ("themuse", "{}"),
        ("themuse", r#"{"page":0,"page_count":1,"results":null}"#),
        ("themuse", r#"{"page":1,"page_count":2,"results":[]}"#),
    ] {
        let server = Server::new(move |_| payload.into());
        let events = run_worker(
            &server,
            vec![search_source(provider, json!({}))],
            10,
            0,
            false,
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                WorkerEvent::Error {
                    invalid_posting: Some(false),
                    ..
                }
            )),
            "{provider}: {payload}"
        );
        assert_eq!(
            scans(&events)[0].outcome,
            Some(scrape_core::protocol::ScanOutcome::Failed)
        );
        assert_eq!(feed_requests(&server).len(), 1);
    }
}

#[test]
fn search_invalid_individual_postings_are_isolated_and_never_fabricate_employers() {
    let server = Server::new(|_| {
        let valid = arbeitnow_job("valid", "Line Cook", "Chipotle");
        let mut missing_company = valid.clone();
        missing_company
            .as_object_mut()
            .unwrap()
            .remove("company_name");
        let mut missing_title = valid.clone();
        missing_title.as_object_mut().unwrap().remove("title");
        let mut missing_url = valid.clone();
        missing_url.as_object_mut().unwrap().remove("url");
        let mut empty_company = valid.clone();
        empty_company["company_name"] = json!(" ");
        let mut empty_title = valid.clone();
        empty_title["title"] = json!(" ");
        let mut invalid_url = valid.clone();
        invalid_url["url"] = json!("https://user:secret@jobs.test/role");
        let mut no_slug = arbeitnow_job("fallback", "Cashier", "Target");
        no_slug.as_object_mut().unwrap().remove("slug");
        no_slug["url"] = json!("https://employer.test/jobs/fallback?utm_source=feed#apply");
        json!({"data": [missing_company, missing_title, missing_url, empty_company,
            empty_title, invalid_url, 7, valid, no_slug]})
        .to_string()
    });
    let events = run_worker(
        &server,
        vec![search_source("arbeitnow", json!({}))],
        10,
        0,
        false,
    );
    assert_eq!(scans(&events)[0].invalid, 7);
    let jobs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                external_id,
                company,
                ..
            } => Some((external_id.as_str(), company.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        jobs,
        [
            ("valid", "Chipotle"),
            ("https://employer.test/jobs/fallback", "Target"),
        ]
    );
}

#[test]
fn search_slug_fallback_uses_canonical_destination_not_a_hash() {
    let server = Server::new(|_| {
        let mut job = arbeitnow_job("fallback", "Cashier", "Target");
        job.as_object_mut().unwrap().remove("slug");
        job["url"] = json!("https://employer.test/jobs/fallback?utm_source=feed");
        json!({"data": [job]}).to_string()
    });
    let events = run_worker(
        &server,
        vec![search_source("arbeitnow", json!({}))],
        10,
        0,
        false,
    );
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job {
        external_id, company, .. } if external_id == "https://employer.test/jobs/fallback" && company == "Target")));
}

#[test]
fn search_page_limit_reports_partial_coverage_and_honors_global_page_bound() {
    for global_bound in [false, true] {
        let server = Server::new(|path| {
            assert_eq!(path, "/www.arbeitnow.com/api/job-board-api");
            json!({"data": [arbeitnow_job("reject", "Product Manager", "Google")],
                "links": {"next": "?page=2"}})
            .to_string()
        });
        let mut limits = worker_limits();
        if global_bound {
            limits["max_pages_per_source"] = json!(1);
        }
        let events = run_worker_config(
            &server,
            vec![search_source(
                "arbeitnow",
                json!({"max_pages": if global_bound { 5 } else { 1 }}),
            )],
            json!({"matching": {"target_roles": ["Software Engineer"]}}),
            10,
            0,
            false,
            limits,
        );
        let scan = &scans(&events)[0];
        assert_eq!(
            scan.termination,
            scrape_core::protocol::CoverageTermination::PageLimit
        );
        assert_eq!(
            scan.outcome,
            Some(scrape_core::protocol::ScanOutcome::Partial)
        );
        assert!(events.iter().any(|event| matches!(
            event,
            WorkerEvent::Warning {
                eligibility_incomplete: true,
                termination: Some(scrape_core::protocol::CoverageTermination::PageLimit),
                ..
            }
        )));
        assert_eq!(
            feed_requests(&server),
            ["/www.arbeitnow.com/api/job-board-api"]
        );
    }
}

#[test]
fn search_unsafe_or_cyclic_continuations_are_never_requested() {
    for next in [
        "https://evil.test/api/job-board-api?page=2".to_string(),
        "https://user:secret@www.arbeitnow.com/api/job-board-api?page=2".to_string(),
        "?page=1".to_string(),
        "?page=2#fragment".to_string(),
        "/other?page=2".to_string(),
        "?page=2&query=imaginary".to_string(),
        format!("?page=2&padding={}", "x".repeat(8192)),
    ] {
        let server = Server::new(move |_| {
            json!({
                "data": [arbeitnow_job("one", "Line Cook", "Chipotle")],
                "links": {"next": next}
            })
            .to_string()
        });
        let events = run_worker(
            &server,
            vec![search_source("arbeitnow", json!({}))],
            10,
            0,
            false,
        );
        assert!(events.iter().any(|event| matches!(
            event,
            WorkerEvent::Error {
                invalid_posting: Some(false),
                ..
            }
        )));
        assert_eq!(
            feed_requests(&server),
            ["/www.arbeitnow.com/api/job-board-api"]
        );
    }
}

#[test]
fn search_same_schema_endpoint_uses_configured_origin_without_extra_query_filters() {
    let server = Server::new(|path| match path {
        "/feed.fixture.test/listings?tenant=demo" => json!({
            "data": [arbeitnow_job("one", "Line Cook", "Chipotle")],
            "links": {"next": "?tenant=demo&page=2"}
        })
        .to_string(),
        "/feed.fixture.test/listings?tenant=demo&page=2" => json!({
            "data": [arbeitnow_job("two", "Cashier", "Target")], "links": {"next": null}
        })
        .to_string(),
        _ => panic!("unexpected request {path}"),
    });
    let events = run_worker(
        &server,
        vec![search_source(
            "arbeitnow",
            json!({
                "endpoint": "https://feed.fixture.test/listings?tenant=demo",
                "query": "Line Cook Cashier", "companies": [], "location": "", "max_days_old": 7
            }),
        )],
        10,
        0,
        false,
    );
    assert_eq!(scans(&events)[0].accepted, 2);
    assert_eq!(
        feed_requests(&server),
        [
            "/feed.fixture.test/listings?tenant=demo",
            "/feed.fixture.test/listings?tenant=demo&page=2",
        ]
    );
}

#[test]
fn search_and_company_board_sources_get_fair_candidate_turns_at_shared_cap() {
    let server = Server::new(|path| {
        if path == "/www.arbeitnow.com/api/job-board-api" {
            return json!({"data": (0..30).map(|index|
                arbeitnow_job(&format!("feed-{index}"), "Software Engineer",
                    if index % 2 == 0 { "Apple" } else { "Google" })).collect::<Vec<_>>(),
                "links": {"next": "?page=2"}
            })
            .to_string();
        }
        if path.contains("/v0/postings/company-board") {
            return generate_lever_postings(30, "company-board");
        }
        if path == "/board.fixture.test/jobs" {
            return (0..30).map(|index| format!(
                "<div class=job-item><a class=job-title href='/jobs/{index}'>Software Engineer {index}</a></div>"
            )).collect();
        }
        if path == "/jobs.lever.co/company-board/company-board-0" {
            return "<html><title>Company Board</title></html>".into();
        }
        panic!("unexpected request {path}");
    });
    let events = run_worker(
        &server,
        vec![
            search_source("arbeitnow", json!({})),
            json!({"id":"lever","name":"Company Board","kind":"lever","identifier":"company-board","config":{}}),
            json!({"id":"selector","name":"Direct Board","kind":"selector","identifier":"https://board.fixture.test/jobs","config":{}}),
        ],
        20,
        0,
        false,
    );
    let counts: Vec<_> = ["search", "lever", "selector"]
        .iter()
        .map(|id| {
            events
                .iter()
                .filter(|event| {
                    matches!(event, WorkerEvent::Job {
            source_id, .. } if source_id.as_str() == *id)
                })
                .count()
        })
        .collect();
    assert_eq!(counts.iter().sum::<usize>(), 20);
    assert!(
        counts.iter().max().unwrap() - counts.iter().min().unwrap() <= 1,
        "{counts:?}"
    );
    assert_eq!(
        feed_requests(&server)
            .iter()
            .filter(|path| path.contains("arbeitnow"))
            .count(),
        1
    );
}

#[test]
fn search_muse_invalid_items_do_not_hide_later_matches_or_fetch_past_cap() {
    let server = Server::new(|path| {
        assert_eq!(path, "/www.themuse.com/api/public/jobs?page=0");
        let mut missing_employer = muse_job(1, "Cashier", "Walmart", Value::Null);
        missing_employer.as_object_mut().unwrap().remove("company");
        json!({"page": 0, "page_count": 2, "results": [
            muse_job(2, "Cashier", "Walmart", json!("not-a-date")),
            missing_employer,
            muse_job(3, "Cashier", "Walmart", json!("2026-10-15T20:00:00Z")),
            muse_job(4, "Software Engineer", "Google", json!("2026-10-15T20:00:00Z")),
        ]})
        .to_string()
    });
    let events = run_worker(
        &server,
        vec![search_source("default", json!({}))],
        1,
        0,
        false,
    );
    assert_eq!(scans(&events)[0].invalid, 2);
    assert_eq!(scans(&events)[0].accepted, 1);
    assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job {
        external_id, company, .. } if external_id == "3" && company == "Walmart")));
    assert_eq!(
        feed_requests(&server),
        ["/www.themuse.com/api/public/jobs?page=0"]
    );
}

#[test]
fn search_unknown_provider_and_malformed_config_fail_before_requests() {
    for source in [
        search_source("imaginary-provider", json!({})),
        search_source(
            "themuse",
            json!({"endpoint": "https://user:secret@feed.test/jobs"}),
        ),
        search_source("arbeitnow", json!({"max_pages": 0})),
        search_source("arbeitnow", json!({"max_pages": 1.5})),
        search_source("themuse", json!({"companies": [" "]})),
        search_source("themuse", json!({"endpoint": "file:///etc/passwd"})),
        search_source(
            "themuse",
            json!({"endpoint": "https://feed.test/jobs#fragment"}),
        ),
    ] {
        let server = Server::new(|path| panic!("invalid config requested {path}"));
        let output = worker_output(
            &server,
            &json!({
                "version":PROTOCOL_VERSION, "sources":[source], "filters":{},
                "limits":worker_limits(), "max_jobs_per_run":10,
            }),
            false,
        );
        if output.status.success() {
            let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert!(events.iter().any(|event| matches!(
                event,
                WorkerEvent::Error {
                    invalid_posting: Some(false),
                    ..
                }
            )));
            assert_eq!(
                scans(&events)[0].outcome,
                Some(scrape_core::protocol::ScanOutcome::Failed)
            );
        } else {
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.contains("max_pages"), "{stderr}");
        }
        assert!(feed_requests(&server).is_empty());
    }
}

fn adzuna_config() -> Value {
    json!({"__provider_credentials":{"app_id":"fictional-app-id","app_key":"fictional-app-key"}})
}

fn adzuna_job(id: &str) -> Value {
    json!({"id":id,"title":"Crew Member","company":{"display_name":"Chipotle"},
        "redirect_url":format!("https://www.adzuna.com/land/ad/{id}"),
        "description":"Quick-service restaurant crew", "created":"2026-10-15T20:00:00Z",
        "location":{"display_name":"Cherry Hill, New Jersey","area":["US","New Jersey","Camden County","Cherry Hill"]},
        "salary_min":50000,"salary_max":80000,"salary_is_predicted":"0"})
}

fn jobicy_job(id: u64) -> Value {
    json!({"id":id,"jobTitle":"Software Engineer","companyName":"Fixture",
        "url":format!("https://jobicy.com/jobs/{id}"),"jobDescription":"<p>Build software</p>",
        "pubDate":"2026-10-15 20:00:00","jobGeo":"Anywhere","jobType":["full-time"],
        "salaryMin":50000,"salaryMax":80000,"salaryCurrency":"USD","salaryPeriod":"year"})
}

#[test]
fn adzuna_uses_one_based_safe_requests_and_original_dates_without_invented_pay() {
    let server = Server::new(|path| {
        let url = url::Url::parse(&format!("http://fixture{path}")).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query.get("app_id").unwrap(), "fictional-app-id");
        assert_eq!(query.get("app_key").unwrap(), "fictional-app-key");
        assert_eq!(query.get("results_per_page").unwrap(), "50");
        assert_eq!(query.get("sort_by").unwrap(), "date");
        if url.path().ends_with("/1") {
            let mut predicted = adzuna_job("predicted");
            predicted["salary_is_predicted"] = json!(true);
            predicted["description"] = json!("Salary: USD 70000 per year");
            json!({"count":51,"results":[adzuna_job("first"),predicted,{"title":"broken"}]})
                .to_string()
        } else {
            assert!(url.path().ends_with("/2"));
            json!({"count":51,"results":[adzuna_job("last")]}).to_string()
        }
    });
    let events = run_worker(
        &server,
        vec![search_source("adzuna", adzuna_config())],
        20,
        0,
        false,
    );
    let ids: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                external_id,
                date_posted,
                salary,
                locations,
                ..
            } => {
                assert_eq!(date_posted.as_deref(), Some("2026-10-15T20:00:00Z"));
                assert!(salary
                    .as_ref()
                    .is_none_or(|s| s.ranges.iter().all(|r| r.interval.is_none())));
                assert!(locations
                    .iter()
                    .any(|loc| loc.city.as_deref() == Some("Cherry Hill")));
                Some(external_id.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(ids, ["first", "predicted", "last"]);
    assert_eq!(scans(&events)[0].invalid, 1);
    assert_eq!(feed_requests(&server).len(), 2);
    let encoded = serde_json::to_string(&events).unwrap();
    assert!(!encoded.contains("fictional-app"));
}

#[test]
fn jobicy_cursor_is_encoded_once_and_public_pages_reused_across_workers() {
    let cache = tempfile::tempdir().unwrap();
    let server = Server::new(|path| {
        let url = url::Url::parse(&format!("http://fixture{path}")).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query.get("count").unwrap(), "100");
        if let Some(cursor) = query.get("cursor") {
            assert_eq!(cursor, "opaque+/= token");
            json!({"jobs":[jobicy_job(2)],"hasMore":false,"nextCursor":null}).to_string()
        } else {
            json!({"jobs":[jobicy_job(1),{"jobTitle":"invalid"}],"hasMore":true,"nextCursor":"opaque+/= token"}).to_string()
        }
    });
    for _ in 0..2 {
        let events = run_worker(
            &server,
            vec![search_source(
                "jobicy",
                json!({
                    "__provider_cache_dir":cache.path().to_str().unwrap()
                }),
            )],
            20,
            0,
            false,
        );
        let ids: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorkerEvent::Job {
                    external_id,
                    workplace_type,
                    salary,
                    locations,
                    ..
                } => {
                    assert_eq!(*workplace_type, scrape_core::WorkplaceType::Remote);
                    assert!(locations.iter().all(|loc| loc.country_code.is_none()));
                    assert_eq!(salary.as_ref().unwrap().ranges[0].max_minor, Some(8000000));
                    Some(external_id.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(ids, ["1", "2"]);
        assert_eq!(scans(&events)[0].invalid, 1);
    }
    assert_eq!(feed_requests(&server).len(), 2);
}

#[test]
fn adzuna_redirect_status_and_decode_errors_never_expose_credentials_or_bodies() {
    for status in [302, 401, 200] {
        let server = Server::new_response(move |_| {
            (
                status,
                vec![(
                    "Location".into(),
                    "https://leak.test/?app_key=fictional-app-key".into(),
                )],
                "fictional-app-id fictional-app-key provider-private-response".into(),
            )
        });
        let output = worker_output(
            &server,
            &json!({
                "version":PROTOCOL_VERSION,"sources":[search_source("adzuna", adzuna_config())],
                "filters":{},"limits":worker_limits(),"max_jobs_per_run":20
            }),
            false,
        );
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        for secret in [
            "fictional-app-id",
            "fictional-app-key",
            "provider-private-response",
            "https://leak.test",
        ] {
            assert!(!stdout.contains(secret), "{stdout}");
            assert!(!stderr.contains(secret), "{stderr}");
        }
        assert!(stdout.contains("Adzuna"));
        assert_eq!(
            feed_requests(&server).len(),
            1,
            "redirects must not be followed"
        );
    }
}

#[test]
fn provider_setup_and_custom_adzuna_endpoints_fail_without_network() {
    for source in [
        search_source("adzuna", json!({})),
        search_source(
            "adzuna",
            json!({"endpoint":"https://leak.test/jobs",
            "__provider_credentials":{"app_id":"fictional-app-id","app_key":"fictional-app-key"}}),
        ),
        search_source("jobicy", json!({})),
        search_source("jobicy", json!({"__provider_cache_dir":"relative/path"})),
    ] {
        let server = Server::new(|path| panic!("unexpected request {path}"));
        let events = run_worker(&server, vec![source], 20, 0, false);
        assert!(events.iter().any(|event| matches!(
            event,
            WorkerEvent::Error {
                invalid_posting: Some(false),
                ..
            }
        )));
        assert!(feed_requests(&server).is_empty());
        assert!(!serde_json::to_string(&events)
            .unwrap()
            .contains("fictional-app"));
    }
}

#[test]
fn jobicy_invalid_pagination_retains_valid_candidates_as_partial_coverage() {
    for metadata in [
        json!({"hasMore":true,"nextCursor":null}),
        json!({"hasMore":false,"nextCursor":"contradictory"}),
        json!({"hasMore":"true","nextCursor":"cursor"}),
        json!({}),
    ] {
        let cache = tempfile::tempdir().unwrap();
        let server = Server::new(move |_| {
            let mut value = metadata.clone();
            value["jobs"] = json!([jobicy_job(1)]);
            value.to_string()
        });
        let events = run_worker(
            &server,
            vec![search_source(
                "jobicy",
                json!({
                    "__provider_cache_dir":cache.path().to_str().unwrap()
                }),
            )],
            20,
            0,
            false,
        );
        assert_eq!(scans(&events)[0].accepted, 1);
        assert_eq!(
            scans(&events)[0].outcome,
            Some(scrape_core::protocol::ScanOutcome::Partial)
        );
        assert!(events.iter().any(|event| matches!(event, WorkerEvent::Job {
            external_id, .. } if external_id == "1")));
        assert_eq!(feed_requests(&server).len(), 1);
    }
}

#[test]
fn jobicy_repeated_cursor_and_page_failure_preserve_prior_jobs() {
    for fail_second in [false, true] {
        let cache = tempfile::tempdir().unwrap();
        let server = Server::new_response(move |path| {
            let second = path.contains("cursor=");
            if second && fail_second {
                return (400, Vec::new(), "unavailable".into());
            }
            (
                200,
                Vec::new(),
                json!({"jobs":[jobicy_job(if second {2} else {1})],
                "hasMore":true,"nextCursor":"repeat"})
                .to_string(),
            )
        });
        let events = run_worker(
            &server,
            vec![search_source(
                "jobicy",
                json!({
                    "__provider_cache_dir":cache.path().to_str().unwrap()
                }),
            )],
            20,
            0,
            false,
        );
        assert_eq!(scans(&events)[0].accepted, if fail_second { 1 } else { 2 });
        assert_eq!(
            scans(&events)[0].outcome,
            Some(scrape_core::protocol::ScanOutcome::Partial)
        );
        assert_eq!(feed_requests(&server).len(), 2);
    }
}

#[test]
fn new_search_provider_empty_collections_and_malformed_collections_are_distinct() {
    for provider in ["adzuna", "jobicy"] {
        for valid in [false, true] {
            let cache = tempfile::tempdir().unwrap();
            let server = Server::new(move |_| {
                if !valid {
                    return r#"{"error":"not a feed"}"#.into();
                }
                if provider == "adzuna" {
                    json!({"count":0,"results":[]}).to_string()
                } else {
                    json!({"jobs":[],"hasMore":false,"nextCursor":null}).to_string()
                }
            });
            let config = if provider == "adzuna" {
                adzuna_config()
            } else {
                json!({"__provider_cache_dir":cache.path().to_str().unwrap()})
            };
            let events = run_worker(&server, vec![search_source(provider, config)], 20, 0, false);
            assert_eq!(scans(&events)[0].accepted, 0);
            assert_eq!(
                scans(&events)[0].outcome,
                Some(if valid {
                    scrape_core::protocol::ScanOutcome::Success
                } else {
                    scrape_core::protocol::ScanOutcome::Failed
                })
            );
            assert_eq!(feed_requests(&server).len(), 1);
        }
    }
}

#[test]
fn new_search_providers_stop_requests_at_job_and_page_caps() {
    for provider in ["adzuna", "jobicy"] {
        for acceptance_cap in [false, true] {
            let cache = tempfile::tempdir().unwrap();
            let server = Server::new(move |_| {
                if provider == "adzuna" {
                    json!({"count":1000,"results":[adzuna_job("first"),adzuna_job("second")]})
                        .to_string()
                } else {
                    json!({"jobs":[jobicy_job(1),jobicy_job(2)],"hasMore":true,"nextCursor":"next"})
                        .to_string()
                }
            });
            let mut config = if provider == "adzuna" {
                adzuna_config()
            } else {
                json!({"__provider_cache_dir":cache.path().to_str().unwrap()})
            };
            config["max_pages"] = json!(1);
            let events = run_worker(
                &server,
                vec![search_source(provider, config)],
                if acceptance_cap { 1 } else { 20 },
                0,
                false,
            );
            assert_eq!(
                scans(&events)[0].accepted,
                if acceptance_cap { 1 } else { 2 }
            );
            assert_eq!(feed_requests(&server).len(), 1);
            if !acceptance_cap {
                assert_eq!(
                    scans(&events)[0].outcome,
                    Some(scrape_core::protocol::ScanOutcome::Partial)
                );
            }
        }
    }
}

#[test]
fn adzuna_discovery_hints_are_or_terms_and_native_pay_requires_snippet_evidence() {
    let server = Server::new(|path| {
        let url = url::Url::parse(&format!("http://fixture{path}")).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(
            query.get("what_or").unwrap(),
            "restaurant crew cashier cook fast food"
        );
        assert_eq!(query.get("where").unwrap(), "Cherry Hill, New Jersey");
        assert_eq!(query.get("max_days_old").unwrap(), "30");
        assert!(!query.contains_key("what"));
        assert!(!query.contains_key("salary_min"));
        let bare = adzuna_job("bare");
        let mut advertised = adzuna_job("advertised");
        advertised["description"] = json!("Remote work. Salary: USD 50000 - 80000 per year.");
        let mut predicted = advertised.clone();
        predicted["id"] = json!("predicted");
        predicted["redirect_url"] = json!("https://www.adzuna.com/land/ad/predicted");
        predicted["salary_is_predicted"] = json!(true);
        json!({"count":3,"results":[bare,advertised,predicted]}).to_string()
    });
    let mut config = adzuna_config();
    config["what_or"] = json!("restaurant crew cashier cook fast food");
    config["where"] = json!("Cherry Hill, New Jersey");
    config["max_days_old"] = json!(30);
    let events = run_worker(&server, vec![search_source("adzuna", config)], 20, 0, false);
    let pay: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job {
                external_id,
                salary,
                ..
            } => Some((
                external_id.as_str(),
                salary.as_ref().and_then(|salary| salary.ranges[0].interval),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        pay,
        [
            ("bare", None),
            ("advertised", Some(scrape_core::SalaryInterval::Year)),
            ("predicted", None)
        ]
    );
}

#[test]
fn jobicy_optional_pay_stays_native_and_canonical_remote_salary_rules_are_authoritative() {
    let cache = tempfile::tempdir().unwrap();
    let server = Server::new(|_| {
        let mut exact = jobicy_job(2);
        exact["salaryMin"] = json!(60000.01);
        exact["salaryMax"] = Value::Null;
        let mut lower = jobicy_job(3);
        lower["salaryMin"] = json!(50000);
        lower["salaryMax"] = Value::Null;
        let mut hourly = jobicy_job(4);
        hourly["salaryPeriod"] = json!("hour");
        let mut eur = jobicy_job(5);
        eur["salaryCurrency"] = json!("EUR");
        let mut malformed = jobicy_job(6);
        malformed["salaryMin"] = json!("not a number");
        let mut unknown_interval = jobicy_job(7);
        unknown_interval["salaryPeriod"] = Value::Null;
        json!({"jobs":[jobicy_job(1),exact,lower,hourly,eur,malformed,unknown_interval],
            "hasMore":false,"nextCursor":null})
        .to_string()
    });
    let events = run_worker_config(
        &server,
        vec![search_source(
            "jobicy",
            json!({"__provider_cache_dir":cache.path().to_str().unwrap()}),
        )],
        json!({"matching":{"workplaces":["remote"],"compensation":{
            "min_minor":6000001,"currency":"USD","interval":"year",
            "include_unknown":false,"match_mode":"advertised_amount"
        }}}),
        20,
        0,
        false,
        worker_limits(),
    );
    let ids: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Job { external_id, .. } => Some(external_id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, ["1", "2"]);
    assert_eq!(scans(&events)[0].examined, 7);
    assert_eq!(scans(&events)[0].accepted, 2);
}

#[test]
fn adzuna_malformed_count_retains_valid_candidates_and_stops_pagination() {
    for count in [Value::Null, json!(-1), json!("unknown"), json!(0)] {
        let server = Server::new(move |_| {
            json!({"count":count,"results":[adzuna_job("valid")]}).to_string()
        });
        let events = run_worker(
            &server,
            vec![search_source("adzuna", adzuna_config())],
            20,
            0,
            false,
        );
        assert_eq!(scans(&events)[0].accepted, 1);
        assert_eq!(
            scans(&events)[0].outcome,
            Some(scrape_core::protocol::ScanOutcome::Partial)
        );
        assert_eq!(feed_requests(&server).len(), 1);
    }
}

#[test]
fn jobicy_cached_json_is_schema_validated_again_before_reuse() {
    let cache = tempfile::tempdir().unwrap();
    let server = Server::new(|_| {
        json!({"jobs":[jobicy_job(1)],
        "hasMore":false,"nextCursor":null})
        .to_string()
    });
    let source = search_source(
        "jobicy",
        json!({"__provider_cache_dir":cache.path().to_str().unwrap()}),
    );
    assert_eq!(
        scans(&run_worker(&server, vec![source.clone()], 20, 0, false))[0].accepted,
        1
    );
    let files: Vec<_> = std::fs::read_dir(cache.path().join("jobicy-pages-v1"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    let mut entry: Value = serde_json::from_slice(&std::fs::read(&files[0]).unwrap()).unwrap();
    entry["response"] = json!({"jobs":null,"hasMore":false,"nextCursor":null});
    std::fs::write(&files[0], serde_json::to_vec(&entry).unwrap()).unwrap();
    assert_eq!(
        scans(&run_worker(&server, vec![source], 20, 0, false))[0].accepted,
        1
    );
    assert_eq!(feed_requests(&server).len(), 2);
}

#[test]
fn multi_source_enrichment_budget_and_turn_yielding_prevent_non_jobicy_starvation() {
    let cache = tempfile::tempdir().unwrap();
    let server = Server::new(|path| {
        if path.contains("/jobicy.com/api/jobs") {
            return json!({
                "jobs": [jobicy_job(101)],
                "hasMore": false,
                "nextCursor": null
            })
            .to_string();
        }
        if path.contains("/boards-api.greenhouse.io/v1/boards/cloudflare/jobs") {
            return json!({
                "jobs": [
                    {
                        "id": 201,
                        "title": "Cloudflare Remote Engineer",
                        "absolute_url": "https://boards.greenhouse.io/cloudflare/jobs/201",
                        "location": { "name": "Remote" },
                        "company_name": "Cloudflare",
                        "first_published": "2026-10-15T00:00:00Z",
                        "updated_at": "2026-10-15T00:00:00Z",
                        "content": "<p>Estimated annual salary of $184,000-$230,000.</p>"
                    }
                ]
            })
            .to_string();
        }
        if path.contains("/boards-api.greenhouse.io/v1/boards/bitwarden/jobs") {
            if path.contains("?content=true") {
                return json!({
                    "jobs": [
                        {
                            "id": 301,
                            "title": "Bitwarden Stale Role",
                            "absolute_url": "https://boards.greenhouse.io/bitwarden/jobs/301",
                            "location": { "name": "Remote" },
                            "company_name": "Bitwarden",
                            "first_published": "2026-09-01T00:00:00Z",
                            "updated_at": "2026-09-01T00:00:00Z",
                            "content": "<p>No pay info</p>"
                        }
                    ]
                })
                .to_string();
            }
            // Delay bitwarden detail
            std::thread::sleep(Duration::from_millis(50));
            return json!({"id": 301, "pay_input_ranges": []}).to_string();
        }
        if path.contains("/wday/cxs/") {
            if path.contains("/jobs") {
                return json!({
                    "total": 1,
                    "jobPostings": [
                        {
                            "title": "NVIDIA Remote Architect",
                            "externalPath": "/job/401",
                            "bulletFields": ["JR401"],
                            "locationsText": "6 Locations",
                            "postedOn": "Posted Yesterday"
                        }
                    ]
                })
                .to_string();
            }
            if path.contains("/job/") {
                return json!({
                    "jobPostingInfo": {
                        "jobReqId": "JR401",
                        "timeType": "Full time",
                        "location": "US, CA, Santa Clara",
                        "additionalLocations": ["US, Remote"],
                        "startDate": "2026-10-24",
                        "jobDescription": "<p>The base salary range is 190,000 USD - 250,000 USD per year.</p>"
                    }
                })
                .to_string();
            }
        }
        "<html>Fixture careers</html>".into()
    });

    let sources = vec![
        json!({
            "id": "jobicy",
            "kind": "search",
            "name": "Jobicy",
            "identifier": "jobicy",
            "config": { "__provider_cache_dir": cache.path().to_str().unwrap() }
        }),
        json!({
            "id": "cloudflare",
            "kind": "greenhouse",
            "name": "Cloudflare",
            "identifier": "cloudflare",
            "config": {}
        }),
        json!({
            "id": "bitwarden",
            "kind": "greenhouse",
            "name": "Bitwarden",
            "identifier": "bitwarden",
            "config": {}
        }),
        json!({
            "id": "nvidia",
            "kind": "workday",
            "name": "NVIDIA",
            "identifier": "https://nvidia.myworkdayjobs.com/NVIDIAExternalCareerSite",
            "config": { "max_pages": 1 }
        }),
    ];

    let events = run_worker_config(
        &server,
        sources,
        json!({
            "matching": {
                "workplaces": ["remote"],
                "compensation": {
                    "min_minor": 6000001,
                    "currency": "USD",
                    "interval": "year",
                    "match_mode": "advertised_amount"
                }
            }
        }),
        20,
        0,
        false,
        json!({
            "per_host_delay_ms": 0,
            "request_timeout_s": 5,
            "enrichment": { "timeout_ms": 5000, "max_requests": 5 }
        }),
    );

    let mut accepted_by_source = std::collections::HashMap::new();
    for event in &events {
        if let WorkerEvent::Job { source_id, .. } = event {
            *accepted_by_source.entry(source_id.clone()).or_insert(0) += 1;
        }
    }

    assert!(
        accepted_by_source.get("jobicy").copied().unwrap_or(0) >= 1,
        "Jobicy must emit jobs: {accepted_by_source:?}"
    );
    assert!(
        accepted_by_source.get("cloudflare").copied().unwrap_or(0) >= 1,
        "Cloudflare must not be starved: {accepted_by_source:?}"
    );
    assert!(
        accepted_by_source.get("nvidia").copied().unwrap_or(0) >= 1,
        "NVIDIA must emit remote jobs: {accepted_by_source:?}"
    );
}

#[test]
fn no_company_heuristic_discovery_orders_abdc_and_skips_unqualified_details() {
    let detail_html = |title: &str, company: &str, loc: &str, date: &str| {
        format!(
            r#"<!DOCTYPE html><html><head><script type="application/ld+json">{{"@context":"https://schema.org","@type":"JobPosting","title":"{title}","hiringOrganization":{{"name":"{company}"}},"datePosted":"{date}","jobLocation":{{"address":{{"addressCountry":"US","addressLocality":"{loc}"}}}},"description":"Engineering role"}}</script></head><body><h1>{title}</h1></body></html>"#
        )
    };

    let server = Server::new(move |path| {
        match path {
        "/www.themuse.com/api/public/jobs?page=0" => json!({
            "page": 0, "page_count": 1,
            "results": [
                muse_item_custom(101, "Software Engineer", "Global A", Some("2026-10-15T12:00:00Z"), vec!["Toronto, Canada"]),
                muse_item_custom(102, "Software Engineer", "Stale B", Some("2026-10-14T12:00:00Z"), vec!["Austin, TX"]),
                muse_item_custom(103, "Accountant", "Finance C", Some("2026-10-15T12:00:00Z"), vec!["Austin, TX"]),
                muse_item_custom(104, "Software Engineer", "Employer A", Some("2026-10-15T12:00:00Z"), vec!["Austin, TX"]),
                muse_item_custom(105, "Data Engineer", "Employer B", Some("2026-10-15T12:00:00Z"), vec!["New York, NY"]),
                muse_item_custom(106, "Software Engineer", "Employer C", Some("2026-10-15T12:00:00Z"), vec![]),
                muse_item_custom(107, "Software Engineer", "Employer D", None, vec!["Austin, TX"]),
            ]
        }).to_string(),
        "/www.themuse.com/jobs/employer/104" => detail_html("Software Engineer", "Employer A", "Austin", "2026-10-15T12:00:00Z"),
        "/www.themuse.com/jobs/employer/105" => detail_html("Data Engineer", "Employer B", "New York", "2026-10-15T12:00:00Z"),
        "/www.themuse.com/jobs/employer/106" => detail_html("Software Engineer", "Employer C", "Austin", "2026-10-15T12:00:00Z"),
        "/www.themuse.com/jobs/employer/107" => detail_html("Software Engineer", "Employer D", "Austin", "2026-10-15T12:00:00Z"),
        other => panic!("unexpected request {other}"),
    }
    });

    let filters = json!({
        "matching": {
            "target_roles": ["Software Engineer", "Data Engineer"],
            "countries": ["US"],
            "location_include_unknown": false,
            "posting_date": {
                "preset": "past_24h",
                "timezone": "America/New_York",
                "include_unknown": false
            }
        }
    });

    let events = run_worker_config(
        &server,
        vec![search_source(
            "themuse",
            json!({"follow_detail": true, "max_pages": 10}),
        )],
        filters.clone(),
        10,
        0,
        false,
        worker_limits(),
    );

    let accepted: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            WorkerEvent::Job { company, .. } => Some(company.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        accepted,
        vec!["Employer A", "Employer B", "Employer D", "Employer C"]
    );

    let paths = feed_requests(&server);
    assert_eq!(
        paths,
        vec![
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.themuse.com/jobs/employer/104",
            "/www.themuse.com/jobs/employer/105",
            "/www.themuse.com/jobs/employer/107",
            "/www.themuse.com/jobs/employer/106",
        ]
    );

    // With cap 1, only A is emitted and no later detail request starts
    let server2 = Server::new(move |path| {
        match path {
        "/www.themuse.com/api/public/jobs?page=0" => json!({
            "page": 0, "page_count": 1,
            "results": [
                muse_item_custom(101, "Software Engineer", "Global A", Some("2026-10-15T12:00:00Z"), vec!["Toronto, Canada"]),
                muse_item_custom(102, "Software Engineer", "Stale B", Some("2026-10-14T12:00:00Z"), vec!["Austin, TX"]),
                muse_item_custom(103, "Accountant", "Finance C", Some("2026-10-15T12:00:00Z"), vec!["Austin, TX"]),
                muse_item_custom(104, "Software Engineer", "Employer A", Some("2026-10-15T12:00:00Z"), vec!["Austin, TX"]),
                muse_item_custom(105, "Data Engineer", "Employer B", Some("2026-10-15T12:00:00Z"), vec!["New York, NY"]),
                muse_item_custom(106, "Software Engineer", "Employer C", Some("2026-10-15T12:00:00Z"), vec![]),
                muse_item_custom(107, "Software Engineer", "Employer D", None, vec!["Austin, TX"]),
            ]
        }).to_string(),
        "/www.themuse.com/jobs/employer/104" => detail_html("Software Engineer", "Employer A", "Austin", "2026-10-15T12:00:00Z"),
        other => panic!("unexpected request {other} with cap 1"),
    }
    });

    let events2 = run_worker_config(
        &server2,
        vec![search_source(
            "themuse",
            json!({"follow_detail": true, "max_pages": 10}),
        )],
        filters,
        1,
        0,
        false,
        worker_limits(),
    );

    let accepted2: Vec<String> = events2
        .iter()
        .filter_map(|e| match e {
            WorkerEvent::Job { company, .. } => Some(company.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(accepted2, vec!["Employer A"]);
    let paths2 = feed_requests(&server2);
    assert_eq!(
        paths2,
        vec![
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.themuse.com/jobs/employer/104",
        ]
    );
}

fn muse_item_custom(
    id: u64,
    title: &str,
    company: &str,
    publication: Option<&str>,
    locations: Vec<&str>,
) -> Value {
    json!({
        "id": id, "name": title, "company": {"name": company},
        "refs": {"landing_page": format!("https://www.themuse.com/jobs/employer/{id}")},
        "contents": "<p>Fictional description.</p>",
        "publication_date": publication,
        "locations": locations.into_iter().map(|name| json!({"name": name})).collect::<Vec<_>>(),
    })
}

#[test]
fn search_detail_timeout_does_not_abort_session_or_drop_subsequent_candidates() {
    use scrape_core::protocol::CoverageTermination;

    let server = Server::new(|path| {
        match path {
        "/www.themuse.com/api/public/jobs?page=0" => json!({
            "page": 0, "page_count": 1,
            "results": (1..=3).map(|id| muse_item_custom(
                id, "Software Engineer", "Fixture Employer",
                Some("2026-10-15T12:00:00Z"), vec!["Austin, TX"]
            )).collect::<Vec<_>>()
        }).to_string(),
        "/www.themuse.com/jobs/employer/1" => {
            thread::sleep(Duration::from_secs(16));
            "<html>Late detail</html>".into()
        }
        "/www.themuse.com/jobs/employer/2" => r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"JobPosting","title":"Software Engineer","baseSalary":{"@type":"MonetaryAmount","currency":"USD","value":{"@type":"QuantitativeValue","value":160000,"unitText":"YEAR"}}}</script>"#.into(),
        "/www.themuse.com/jobs/employer/3" => "<html>No additional evidence</html>".into(),
        other => panic!("unexpected request {other}"),
    }
    });
    let events = run_worker_config(
        &server,
        vec![search_source(
            "themuse",
            json!({"follow_detail":true,"max_pages":1}),
        )],
        json!({"matching":{"target_roles":["Software Engineer"],"countries":["US"],
            "location_include_unknown":false,
            "posting_date":{"preset":"past_24h","timezone":"UTC","include_unknown":false}}}),
        40,
        0,
        false,
        json!({"per_host_delay_ms":0,"request_timeout_s":20,
            "enrichment":{"timeout_ms":30000,"max_requests":10}}),
    );
    assert_eq!(scans(&events)[0].examined, 3, "{events:?}");
    assert_eq!(scans(&events)[0].accepted, 3, "{events:?}");
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorkerEvent::Warning {
                termination: Some(CoverageTermination::TimeLimit),
                ..
            }
        )),
        "{events:?}"
    );
    assert!(
        events.iter().any(|event| matches!(event,
            WorkerEvent::Job { url, salary: Some(_), .. }
            if url.ends_with("/2")
        )),
        "{events:?}"
    );
    assert_eq!(
        feed_requests(&server),
        [
            "/www.themuse.com/api/public/jobs?page=0",
            "/www.themuse.com/jobs/employer/1",
            "/www.themuse.com/jobs/employer/2",
            "/www.themuse.com/jobs/employer/3",
        ]
    );
}

#[test]
fn search_detail_bounds_preserve_feed_evidence_and_strict_matching() {
    use scrape_core::protocol::CoverageTermination;

    for time_bound in [false, true] {
        let server = Server::new(move |path| {
            match path {
            "/www.themuse.com/api/public/jobs?page=0" => json!({
                "page":0,"page_count":1,"results": [
                    muse_item_custom(1,"Software Engineer","Fixture Employer",Some("2026-10-15T12:00:00Z"),vec!["Austin, TX"]),
                    muse_item_custom(2,"Software Engineer","Fixture Employer",Some("2026-10-15T12:00:00Z"),vec!["Austin, TX"]),
                    muse_item_custom(3,"Software Engineer","Fixture Employer",None,vec!["Austin, TX"])
                ]
            }).to_string(),
            "/www.themuse.com/jobs/employer/1" => {
                if time_bound {
                    thread::sleep(Duration::from_millis(300));
                }
                "<html>No additional evidence</html>".into()
            }
            other => panic!("unexpected request {other}"),
        }
        });
        let events = run_worker_config(
            &server,
            vec![search_source(
                "themuse",
                json!({"follow_detail":true,"max_pages":1}),
            )],
            json!({"matching":{"target_roles":["Software Engineer"],"countries":["US"],
                "location_include_unknown":false,
                "posting_date":{"preset":"past_24h","timezone":"UTC","include_unknown":false}}}),
            40,
            0,
            false,
            json!({"per_host_delay_ms":0,"request_timeout_s":3,
                "enrichment":{"timeout_ms":if time_bound {200} else {5000},"max_requests":1}}),
        );
        assert_eq!(scans(&events)[0].examined, 3, "{events:?}");
        assert_eq!(scans(&events)[0].accepted, 2, "{events:?}");
        assert_eq!(scans(&events)[0].unresolved, 1, "{events:?}");
        let expected = if time_bound {
            CoverageTermination::TimeLimit
        } else {
            CoverageTermination::RequestLimit
        };
        assert!(events.iter().any(|event| matches!(event,
            WorkerEvent::Warning { termination: Some(termination), .. } if *termination == expected
        )), "{events:?}");
        assert_eq!(
            feed_requests(&server),
            [
                "/www.themuse.com/api/public/jobs?page=0",
                "/www.themuse.com/jobs/employer/1",
            ]
        );
    }
}
