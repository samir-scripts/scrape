//! Actual bounded HTML crawl through the worker NDJSON/ingestion boundary.
use scrape_core::{
    protocol::{CoverageTermination, ScanOutcome, SourceScanStats},
    WorkerEvent, PROTOCOL_VERSION,
};
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

struct Response {
    status: &'static str,
    headers: String,
    body: String,
}
impl Response {
    fn html(body: impl Into<String>) -> Self {
        Self {
            status: "200 OK",
            headers: String::new(),
            body: body.into(),
        }
    }
    fn status(status: &'static str) -> Self {
        Self {
            status,
            headers: String::new(),
            body: "Unavailable fixture".into(),
        }
    }
}
struct Server {
    origin: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(handler: impl Fn(&str) -> Response + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let mut buf = [0; 8192];
                        let n = match stream.read(&mut buf) {
                            Ok(n) => n,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                                ) =>
                            {
                                continue;
                            }
                            Err(error) => panic!("{error}"),
                        };
                        if n == 0 {
                            continue;
                        }
                        let request = String::from_utf8_lossy(&buf[..n]);
                        let path = request.split_whitespace().nth(1).unwrap().to_string();
                        seen.lock().unwrap().push(path.clone());
                        let response = handler(&path);
                        let _ = write!(
                            stream,
                            "HTTP/1.1 {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response.status,
                            response.headers,
                            response.body.len(),
                            response.body
                        );
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(error) => panic!("{error}"),
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
    fn paths(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn allow() -> Response {
    Response::html("User-agent: *\nAllow: /\n")
}
fn posting(id: &str) -> String {
    format!(
        r#"<script type="application/ld+json">{{"@context":"https://schema.org","@type":"JobPosting","identifier":"{id}","title":"Fictional Crawl Engineer","url":"/jobs/{id}?team=platform","datePosted":"2026-10-15","hiringOrganization":{{"name":"Fictional Crawl Co"}},"description":"Build fictional pipelines."}}</script>"#
    )
}
fn source(server: &Server, id: &str, seed: &str, config: Value) -> Value {
    json!({"id":id,"name":"Fictional HTML crawl","kind":"selector","identifier":format!("{}{seed}",server.origin),"config":config})
}
fn run(sources: Vec<Value>, cap: u32) -> Vec<WorkerEvent> {
    run_with_request_timeout(sources, cap, 3)
}

fn run_with_request_timeout(
    sources: Vec<Value>,
    cap: u32,
    request_timeout_s: u64,
) -> Vec<WorkerEvent> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .env_remove("SCRAPE_CDP_WS_URL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let config = json!({"version":PROTOCOL_VERSION,"sources":sources,"max_jobs_per_run":cap,"run_started_at":"2026-10-16T00:00:00Z","limits":{"per_host_delay_ms":0,"request_timeout_s":request_timeout_s}});
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
fn stats(events: &[WorkerEvent]) -> &[SourceScanStats] {
    match events.last().unwrap() {
        WorkerEvent::Done { stats } => &stats.sources,
        _ => panic!("missing Done"),
    }
}
fn jobs(events: &[WorkerEvent]) -> Vec<&WorkerEvent> {
    events
        .iter()
        .filter(|event| matches!(event, WorkerEvent::Job { .. }))
        .collect()
}
fn crawl(pages: u32, depth: u32) -> Value {
    json!({"crawl":{"max_pages":pages,"max_depth":depth,"extraction":"jsonld"}})
}

#[test]
fn breadth_first_crawl_preserves_job_identity_query_date_and_origin_bounds() {
    let external = Server::new(|_| Response::html(posting("outside")));
    let outside = external.origin.clone();
    let server = Server::new(move |path| match path {
        "/robots.txt" => allow(),
        "/seed?team=platform" => Response::html(format!(
            r#"<a href="/listing#one">Jobs</a><a href="/listing#two">Duplicate</a><a href="{outside}/outside">Outside</a><a href="mailto:someone@example.test">Mail</a><a href="http://user:secret@127.0.0.1/unsafe">Unsafe</a>"#
        )),
        "/listing" => Response::html(
            r#"<a href="/seed?team=platform#cycle">Back</a><a href="/jobs/42?team=platform#apply">Role</a><a href="/about">About</a>"#,
        ),
        "/jobs/42?team=platform" => {
            Response::html(format!("{}<a href='/listing'>Back</a>", posting("42")))
        }
        "/about" => Response::html("<p>Fictional company, no job fields.</p>"),
        other => panic!("unexpected request {other}"),
    });
    let events = run(
        vec![source(
            &server,
            "crawl",
            "/seed?team=platform#start",
            crawl(20, 2),
        )],
        20,
    );
    assert_eq!(jobs(&events).len(), 1);
    match jobs(&events)[0] {
        WorkerEvent::Job {
            external_id,
            title,
            url,
            date_posted,
            company,
            ..
        } => {
            assert_eq!(external_id, "42");
            assert_eq!(title, "Fictional Crawl Engineer");
            assert_eq!(company, "Fictional Crawl Co");
            assert_eq!(url, &format!("{}/jobs/42?team=platform", server.origin));
            assert_eq!(date_posted.as_deref(), Some("2026-10-15"));
        }
        _ => unreachable!(),
    }
    assert_eq!(
        server.paths(),
        [
            "/robots.txt",
            "/seed?team=platform",
            "/listing",
            "/jobs/42?team=platform",
            "/about"
        ]
    );
    assert!(external.paths().is_empty());
    assert_eq!(
        stats(&events)[0].termination,
        CoverageTermination::Exhausted
    );
}

#[test]
fn depth_and_page_caps_report_partial_without_fetching_beyond_bounds() {
    for (pages, depth, expected) in [
        (20, 1, vec!["/robots.txt", "/seed", "/listing"]),
        (1, 2, vec!["/robots.txt", "/seed"]),
        (20, 0, vec!["/robots.txt", "/seed"]),
    ] {
        let server = Server::new(|path| match path {
            "/robots.txt" => allow(),
            "/seed" => Response::html("<a href='/listing'>Listing</a>"),
            "/listing" => Response::html("<a href='/jobs/42'>Role</a>"),
            _ => panic!("request beyond crawl cap: {path}"),
        });
        let events = run(
            vec![source(&server, "bounded", "/seed", crawl(pages, depth))],
            20,
        );
        assert!(jobs(&events).is_empty());
        assert_eq!(
            server.paths(),
            expected,
            "crawl pages={pages}, depth={depth}, events={events:?}"
        );
        assert_eq!(
            stats(&events)[0].termination,
            CoverageTermination::PageLimit
        );
        assert_eq!(stats(&events)[0].outcome, Some(ScanOutcome::Partial));
    }
    let server = Server::new(|path| {
        if path == "/robots.txt" {
            allow()
        } else {
            Response::html(posting("42"))
        }
    });
    let events = run(vec![source(&server, "leaf", "/seed", crawl(1, 0))], 20);
    assert_eq!(jobs(&events).len(), 1);
    assert_eq!(
        stats(&events)[0].termination,
        CoverageTermination::Exhausted
    );
    assert_eq!(server.paths(), ["/robots.txt", "/seed"]);
}

#[test]
fn later_access_failures_retain_valid_earlier_jobs_without_bypass() {
    for (path, termination) in [
        ("/missing", CoverageTermination::Failed),
        ("/forbidden", CoverageTermination::NeedsLogin),
        ("/login", CoverageTermination::NeedsLogin),
        ("/challenge", CoverageTermination::Failed),
    ] {
        let server = Server::new(move |request| match request {
            "/robots.txt" => allow(),
            "/seed" => Response::html(format!("{}<a href='{path}'>Next</a>", posting("42"))),
            "/missing" => Response::status("404 Not Found"),
            "/forbidden" => Response::status("403 Forbidden"),
            "/login" => Response::html("<title>Sign in</title><form><input type=password></form>"),
            "/challenge" => Response::html("<title>Just a moment...</title>"),
            other => panic!("unexpected bypass request {other}"),
        });
        let events = run(vec![source(&server, "partial", "/seed", crawl(20, 2))], 20);
        assert_eq!(jobs(&events).len(), 1);
        assert_eq!(stats(&events)[0].termination, termination, "{path}");
        assert_eq!(stats(&events)[0].outcome, Some(ScanOutcome::Partial));
        assert_eq!(
            server.paths(),
            ["/robots.txt", "/seed", path],
            "failed page {path}; events={events:?}"
        );
    }
}

#[test]
fn robots_denial_and_cross_origin_redirect_never_fetch_targets() {
    let denied = Server::new(|path| match path {
        "/robots.txt" => Response::html("User-agent: *\nDisallow: /seed\n"),
        _ => panic!("robots denied page fetched"),
    });
    let events = run(vec![source(&denied, "denied", "/seed", crawl(20, 2))], 20);
    assert!(jobs(&events).is_empty());
    assert_eq!(stats(&events)[0].termination, CoverageTermination::Failed);
    assert_eq!(denied.paths(), ["/robots.txt"]);
    let external = Server::new(|_| panic!("cross-origin redirect followed"));
    let location = format!("Location: {}/outside\r\n", external.origin);
    let redirect = Server::new(move |path| match path {
        "/robots.txt" => allow(),
        "/seed" => Response::html(format!("{}<a href='/redirect'>More</a>", posting("42"))),
        "/redirect" => Response {
            status: "302 Found",
            headers: location.clone(),
            body: String::new(),
        },
        _ => panic!("unexpected request"),
    });
    let events = run(
        vec![source(&redirect, "redirect", "/seed", crawl(20, 2))],
        20,
    );
    assert_eq!(jobs(&events).len(), 1);
    assert_eq!(stats(&events)[0].termination, CoverageTermination::Failed);
    assert_eq!(redirect.paths(), ["/robots.txt", "/seed", "/redirect"]);
    assert!(external.paths().is_empty());
}

#[test]
fn selector_crawl_reuses_configured_fields_and_rejects_invalid_config_before_network() {
    let server = Server::new(|path| {
        match path {
        "/robots.txt"=>allow(),
        "/seed"=>Response::html("<a href='/listing'>Listing</a>"),
        "/listing"=>Response::html("<article data-id='42'><h2>Fictional Selector Engineer</h2><a href='/jobs/42?team=platform'>Apply</a><time datetime='2026-10-15'></time></article>"),
        "/jobs/42?team=platform"=>Response::html("No additional posting fields"),
        _=>panic!("unexpected request {path}"),
    }
    });
    let config = json!({"crawl":{"extraction":"selectors","max_pages":20,"max_depth":2},"item":"article","title":"h2","link":"a","id":".","id_attribute":"data-id","date":"time"});
    let events = run(vec![source(&server, "selectors", "/seed", config)], 20);
    assert_eq!(jobs(&events).len(), 1);
    assert_eq!(
        stats(&events)[0].termination,
        CoverageTermination::Exhausted,
        "events={events:?}; requests={:?}",
        server.paths()
    );
    assert_eq!(
        server.paths(),
        ["/robots.txt", "/seed", "/listing", "/jobs/42?team=platform"]
    );
    for config in [
        json!({"crawl":{"extraction":"selectors"}}),
        json!({"crawl":{"extraction":"selectors"},"item":"[","title":"h2","link":"a"}),
        json!({"crawl":{"extraction":"jsonld","max_pages":0}}),
        json!({"crawl":{"extraction":"jsonld","max_depth":6}}),
        json!({"crawl":{"extraction":"jsonld","unknown":true}}),
        json!({"crawl":{"extraction":"jsonld"},"next_page":"a"}),
        json!({"crawl":{"extraction":"jsonld"},"mode":"browser"}),
        json!({"crawl":{"extraction":"jsonld"},"mode":"script","script":{"runtime":"js","code":"ignored"}}),
    ] {
        let isolated = Server::new(|_| panic!("invalid config issued network request"));
        let events = run(vec![source(&isolated, "invalid", "/seed", config)], 20);
        assert!(jobs(&events).is_empty());
        assert_eq!(stats(&events)[0].termination, CoverageTermination::Failed);
        assert!(isolated.paths().is_empty());
    }
    for identifier in [
        "file:///tmp/seed",
        "javascript:alert(1)",
        "https://username:fictional-secret@careers.test/",
    ] {
        let events = run(
            vec![
                json!({"id":"unsafe","name":"Unsafe fixture","kind":"selector","identifier":identifier,"config":crawl(20,2)}),
            ],
            20,
        );
        assert!(jobs(&events).is_empty());
        assert_eq!(stats(&events)[0].termination, CoverageTermination::Failed);
    }
}

#[test]
fn resumable_crawls_share_acceptance_budget_without_fetching_later_pages() {
    let server = Server::new(|path| match path {
        "/robots.txt" => allow(),
        "/a" => Response::html("<a href='/a-listing'>Listing</a>"),
        "/b" => Response::html(posting("b")),
        "/a-listing" => Response::html(format!("{}<a href='/unneeded'>More</a>", posting("a"))),
        "/unneeded" => panic!("request scheduled after acceptance cap"),
        _ => panic!("unexpected request {path}"),
    });
    let events = run(
        vec![
            source(&server, "a", "/a", crawl(20, 2)),
            source(&server, "b", "/b", crawl(20, 2)),
        ],
        2,
    );
    let ids: Vec<_> = jobs(&events)
        .into_iter()
        .map(|event| match event {
            WorkerEvent::Job { external_id, .. } => external_id.as_str(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        ids,
        ["b", "a"],
        "worker events: {events:?}; requests: {:?}",
        server.paths()
    );
    assert_eq!(server.paths(), ["/robots.txt", "/a", "/b", "/a-listing"]);
    assert!(stats(&events).iter().any(
        |source| source.source_id == "a" && source.termination == CoverageTermination::JobLimit
    ));
}

#[test]
fn later_robots_denial_and_navigation_timeout_preserve_earlier_postings() {
    let denied = Server::new(|path| match path {
        "/robots.txt" => Response::html("User-agent: *\nDisallow: /private\n"),
        "/seed" => Response::html(format!("{}<a href='/private'>Private</a>", posting("42"))),
        _ => panic!("robots-denied destination was fetched"),
    });
    let events = run(
        vec![source(&denied, "later-denied", "/seed", crawl(20, 2))],
        20,
    );
    assert_eq!(jobs(&events).len(), 1);
    assert_eq!(stats(&events)[0].termination, CoverageTermination::Failed);
    assert_eq!(stats(&events)[0].outcome, Some(ScanOutcome::Partial));
    assert_eq!(denied.paths(), ["/robots.txt", "/seed"]);

    let slow = Server::new(|path| match path {
        "/robots.txt" => allow(),
        "/seed" => Response::html(format!("{}<a href='/slow'>Slow</a>", posting("42"))),
        "/slow" => {
            thread::sleep(Duration::from_secs(6));
            Response::html("<a href='/must-not-fetch'>Later</a>")
        }
        _ => panic!("request after navigation deadline"),
    });
    let mut config = crawl(20, 2);
    config["navigation_timeout_s"] = json!(5);
    let events = run_with_request_timeout(vec![source(&slow, "timeout", "/seed", config)], 20, 8);
    assert_eq!(jobs(&events).len(), 1);
    assert_eq!(
        stats(&events)[0].termination,
        CoverageTermination::TimeLimit,
        "events={events:?}"
    );
    assert_eq!(stats(&events)[0].outcome, Some(ScanOutcome::Partial));
    assert_eq!(slow.paths(), ["/robots.txt", "/seed", "/slow"]);
}

#[test]
fn crawl_reuses_fetched_html_for_explicit_detail_enrichment_within_page_caps() {
    for same_page in [true, false] {
        let server = Server::new(move |path| {
            match path {
            "/robots.txt" => allow(),
            "/seed" => Response::html(format!("<article><h2>Fictional Detail Engineer</h2><a href='{}'>Apply</a><p class=description>Seed detail evidence.</p></article>", if same_page { "/seed#apply" } else { "/detail#apply" })),
            "/detail" => Response::html("<p class=description>Detail page evidence.</p>"),
            _ => panic!("unexpected detail request {path}"),
        }
        });
        let config = json!({
            "crawl":{"extraction":"selectors","max_pages":if same_page {1} else {2},"max_depth":if same_page {0} else {1}},
            "item":"article","title":"h2","link":"a","follow_detail":true,
            "detail_fields":{"description":".description","description_attribute":"text"}
        });
        let events = run(vec![source(&server, "details", "/seed", config)], 20);
        assert_eq!(jobs(&events).len(), 1);
        match jobs(&events)[0] {
            WorkerEvent::Job {
                description_text, ..
            } => assert_eq!(
                description_text.as_deref(),
                Some(if same_page {
                    "Seed detail evidence."
                } else {
                    "Detail page evidence."
                })
            ),
            _ => unreachable!(),
        }
        assert_eq!(
            stats(&events)[0].termination,
            CoverageTermination::Exhausted
        );
        assert_eq!(
            server.paths(),
            if same_page {
                vec!["/robots.txt", "/seed"]
            } else {
                vec!["/robots.txt", "/seed", "/detail"]
            }
        );
    }
}
