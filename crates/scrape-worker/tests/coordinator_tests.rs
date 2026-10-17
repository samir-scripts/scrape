use scrape_core::{WorkerEvent, PROTOCOL_VERSION};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
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
    _requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new(handler: impl Fn(&str) -> String + Send + 'static) -> Self {
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
                        let body = if path.ends_with("/robots.txt") {
                            "User-agent: *\nAllow: /".into()
                        } else {
                            handler(&path)
                        };
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
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
            _requests: requests,
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

    let config = json!({
        "version": PROTOCOL_VERSION,
        "sources": sources,
        "filters": {},
        "blocked_postings": [],
        "run_started_at": "2026-10-16T00:00:00Z",
        "limits": {
            "per_host_delay_ms": 0,
            "request_timeout_s": 3,
        },
        "max_jobs_per_run": max_jobs_per_run,
        "starting_source_index": starting_source_index,
    });

    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();

    let output = child.wait_with_output().unwrap();
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
