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
