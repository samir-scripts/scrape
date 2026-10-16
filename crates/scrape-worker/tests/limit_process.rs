//! Actual worker processes against isolated HTTP fixtures; no public providers.
use scrape_core::{protocol::SourceScanStats, WorkerEvent, PROTOCOL_VERSION};
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
    requests: Arc<Mutex<Vec<String>>>,
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
                        write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            body.len(),
                            body
                        )
                        .unwrap();
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
fn run(
    server: &Server,
    sources: Vec<Value>,
    filters: Value,
    blocked: Vec<String>,
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
    let config = json!({"version": PROTOCOL_VERSION, "sources": sources, "filters": filters, "blocked_postings": blocked, "limits": {"per_host_delay_ms": 0, "request_timeout_s": 3}});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn source(kind: &str, id: &str, limit: Option<u32>) -> Value {
    json!({"id": id, "kind": kind, "name": "Fixture", "identifier": if matches!(kind,"jsonld"|"selector") {"https://fixture.test/careers"} else {id}, "config": {"__workflow_spec": {"max_jobs_per_run": limit}, "follow_links": if kind == "jsonld" { Value::String("a.detail".into()) } else { Value::Null }}})
}
fn summary(events: &[WorkerEvent]) -> &[SourceScanStats] {
    match events.last().unwrap() {
        WorkerEvent::Done { stats } => &stats.sources,
        _ => panic!("missing Done"),
    }
}
fn jobs(events: &[WorkerEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, WorkerEvent::Job { .. }))
        .count()
}

#[test]
fn all_adapters_stop_at_their_default_or_configured_acceptance_limit() {
    for kind in ["lever", "ashby", "selector", "jsonld", "greenhouse"] {
        let server = Server::new(move |path| {
            if path.starts_with("/fixture.test/jobs/") {
                return r#"<img alt="Fixture Logo" src="/company.svg">"#.into();
            }
            if kind == "greenhouse" && !path.ends_with("?content=true") {
                let id: u64 = path
                    .split('/')
                    .next_back()
                    .unwrap()
                    .split('?')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                return json!({"id":id,"title":"Engineer","absolute_url":format!("https://fixture.test/jobs/{id}")}).to_string();
            }
            let records: Vec<Value> = (0..120).map(|id| match kind {
                "lever" => json!({"id":id.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/jobs/{id}")}),
                "ashby" => json!({"id":id.to_string(),"title":"Engineer","jobUrl":format!("https://fixture.test/jobs/{id}")}),
                "greenhouse" => json!({"id":id,"title":"Engineer","absolute_url":format!("https://fixture.test/jobs/{id}")}),
                _ => json!({"@type":"JobPosting","identifier":id.to_string(),"title":"Engineer","url":format!("/jobs/{id}")}),
            }).collect();
            match kind {
                "lever" => json!(records).to_string(),
                "ashby" | "greenhouse" => json!({"jobs":records}).to_string(),
                "jsonld" => format!("<script type=\"application/ld+json\">{}</script><a class=detail href=/unneeded>More</a>", json!(records)),
                _ => (0..120).map(|id| format!("<div class=job-item><a class=job-title href=/jobs/{id}>Engineer</a></div>")).collect(),
            }
        });
        let limit = if kind == "greenhouse" { Some(3) } else { None };
        let events = run(
            &server,
            vec![source(kind, "board", limit)],
            json!({}),
            vec![],
            false,
        );
        let count = limit.unwrap_or(20) as usize;
        assert_eq!(jobs(&events), count, "{kind}");
        assert_eq!(summary(&events)[0].examined, count as u64, "{kind}");
        assert!(summary(&events)[0].limit_reached);
        let requests = server.requests.lock().unwrap();
        if matches!(kind, "greenhouse" | "lever" | "ashby") {
            assert_eq!(
                requests
                    .iter()
                    .filter(|path| path.starts_with("/fixture.test/jobs/"))
                    .count(),
                1,
                "exactly one branding page per workflow, not per job"
            );
            assert!(
                events
                    .iter()
                    .filter_map(|event| match event {
                        WorkerEvent::Job {
                            company_icon_url, ..
                        } => Some(company_icon_url.as_deref()),
                        _ => None,
                    })
                    .all(|logo| logo == Some("https://fixture.test/company.svg")),
                "{kind} must emit the provider logo through the real worker process"
            );
        }
        assert!(!requests.iter().any(|path| path.contains("unneeded")));
        if kind == "greenhouse" {
            assert_eq!(
                requests.len(),
                6,
                "listing, exactly three details, one robots check and one branding page"
            );
        }
    }
}

#[test]
fn rejected_invalid_duplicate_and_blocked_postings_do_not_consume_the_budget() {
    let server = Server::new(|_| {
        let mut records = vec![
            json!({"id":"reject","text":"Sales","hostedUrl":"https://fixture.test/reject"}),
            json!({"id":"unsafe","text":"Engineer","hostedUrl":"javascript:bad"}),
            json!({"id":"blocked","text":"Engineer","hostedUrl":"https://fixture.test/blocked"}),
        ];
        records.push(records[2].clone());
        records.extend((0..100).map(|id| json!({"id":id.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/{id}")})));
        json!(records).to_string()
    });
    let block = scrape_core::PostingIdentity::for_posting(
        Some(scrape_core::SourceKind::Lever),
        Some("board"),
        Some("blocked"),
        "https://fixture.test/blocked",
    )
    .unwrap()
    .key;
    let events = run(
        &server,
        vec![source("lever", "board", Some(5))],
        json!({"include_keywords":["Engineer"]}),
        vec![block],
        false,
    );
    assert_eq!(jobs(&events), 5);
    let scan = &summary(&events)[0];
    assert_eq!(
        (
            scan.examined,
            scan.matched,
            scan.blocked,
            scan.duplicates,
            scan.invalid
        ),
        (9, 6, 1, 1, 1)
    );
}

#[test]
fn preview_is_capped_at_ten_or_the_lower_limit_and_global_budgets_are_independent() {
    let server = Server::new(|_| {
        json!((0..100).map(|id| json!({"id":id.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/{id}")})).collect::<Vec<_>>()).to_string()
    });
    for (limit, expected) in [(Some(5), 5), (Some(50), 10), (None, 10)] {
        let events = run(
            &server,
            vec![source("lever", "one", limit)],
            json!({}),
            vec![],
            true,
        );
        assert_eq!(jobs(&events), expected);
        assert!(summary(&events)[0].limit_reached);
    }
    let events = run(
        &server,
        vec![source("lever", "one", None), source("lever", "two", None)],
        json!({}),
        vec![],
        false,
    );
    assert_eq!(jobs(&events), 40);
    assert_eq!(summary(&events).len(), 2);
    assert!(summary(&events)
        .iter()
        .all(|s| s.limit == 20 && s.limit_reached));
}
