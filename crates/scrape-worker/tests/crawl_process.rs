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
    time::{Duration, Instant},
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
    arrivals: Arc<Mutex<Vec<Instant>>>,
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
        let arrivals = Arc::new(Mutex::new(Vec::new()));
        let arrival_log = arrivals.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
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
                        arrival_log.lock().unwrap().push(Instant::now());
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
            arrivals,
            stop,
            thread: Some(thread),
        }
    }
    fn paths(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
    fn arrivals(&self) -> Vec<Instant> {
        self.arrivals.lock().unwrap().clone()
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
fn run_with_filters(sources: Vec<Value>, cap: u32, filters: Value) -> Vec<WorkerEvent> {
    run_with_replay(sources, cap, filters, None)
}
fn run_with_replay(
    sources: Vec<Value>,
    cap: u32,
    filters: Value,
    replay: Option<&str>,
) -> Vec<WorkerEvent> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scrape-worker"));
    command.arg("--stdin");
    if let Some(origin) = replay {
        command.args(["--replay-origin", origin]);
    }
    let mut child = command
        .env_remove("SCRAPE_CDP_WS_URL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let config = json!({
        "version": PROTOCOL_VERSION,
        "sources": sources,
        "filters": filters,
        "max_jobs_per_run": cap,
        "run_started_at": "2026-10-16T00:00:00Z",
        "limits": { "per_host_delay_ms": 0, "request_timeout_s": 5 }
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
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn run_with_request_timeout(
    sources: Vec<Value>,
    cap: u32,
    request_timeout_s: u64,
) -> Vec<WorkerEvent> {
    run_with_limits(sources, cap, request_timeout_s, None)
}
fn run_with_limits(
    sources: Vec<Value>,
    cap: u32,
    request_timeout_s: u64,
    max_source_duration_s: Option<u64>,
) -> Vec<WorkerEvent> {
    let mut limits = json!({"per_host_delay_ms":0,"request_timeout_s":request_timeout_s});
    if let Some(duration) = max_source_duration_s {
        limits["max_source_duration_s"] = json!(duration);
    }
    run_with_custom_limits(sources, cap, json!({}), limits)
}
fn run_with_custom_limits(
    sources: Vec<Value>,
    cap: u32,
    filters: Value,
    limits: Value,
) -> Vec<WorkerEvent> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .env_remove("SCRAPE_CDP_WS_URL")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let config = json!({"version":PROTOCOL_VERSION,"sources":sources,"filters":filters,"max_jobs_per_run":cap,"run_started_at":"2026-10-16T00:00:00Z","limits":limits});
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
fn plain_html_detail(title: &str, location: Option<&str>, date: Option<&str>) -> String {
    let location = location
        .map(|text| format!("<div class='location'>{text}</div>"))
        .unwrap_or_default();
    let date = date
        .map(|text| format!("<time datetime='{text}'>Posted</time>"))
        .unwrap_or_default();
    format!(
        "<html><h1>{title}</h1><main><div class='job-description'><p>Build dependable software systems, distributed services, and platform APIs for customers.</p></div>{location}<div class='salary'>$100,000 - $140,000 per year</div>{date}</main></html>"
    )
}

#[test]
fn public_html_crawl_falls_back_to_card_and_detail_when_jsonld_is_incomplete() {
    let server = Server::new(|path| match path {
        "/careers" => Response::html("<a href='/openings'>Open positions</a>"),
        "/openings" => Response::html(
            r#"<script type="application/ld+json">{"@type":"JobPosting","title":"Software Engineer"}</script><article><a href="/jobs/role">Software Engineer</a><div class="location">Austin, TX</div><time datetime="2026-10-15T20:00:00Z">Today</time></article>"#,
        ),
        "/jobs/role" => Response::html(
            r#"<script type="application/ld+json">{"@type":"JobPosting","title":"Software Engineer","datePosted":"2026-10-15T20:00:00Z","description":"Responsibilities include building dependable services and maintaining production software systems.","jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressLocality":"Austin","addressRegion":"TX","addressCountry":"US"}},"hiringOrganization":{"name":"Fixture Company"}}</script>"#,
        ),
        other => panic!("unexpected request {other}"),
    });
    let mut config = crawl(10, 2);
    config["crawl"]["extraction"] = json!("public_html");
    config["per_host_delay_ms"] = json!(0);
    config["jitter_ms"] = json!(0);
    config["ignore_robots"] = json!(true);
    let filters = json!({"matching": {
        "target_roles": ["Software Engineer"],
        "posting_date": {"preset": "past_24h", "timezone": "UTC", "include_unknown": false},
        "countries": ["US"],
        "location_include_unknown": false
    }});
    let events = run_with_filters(
        vec![source(&server, "incomplete-jsonld", "/careers", config)],
        10,
        filters,
    );

    let accepted = jobs(&events);
    assert_eq!(accepted.len(), 1, "events: {events:?}");
    assert!(matches!(
        accepted[0],
        WorkerEvent::Job { title, url, .. }
            if title == "Software Engineer" && url.ends_with("/jobs/role")
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkerEvent::Error { .. })),
        "incomplete structured data should not mask valid public HTML evidence: {events:?}"
    );
    assert_eq!(stats(&events)[0].invalid, 0);
}

#[test]
fn plain_html_crawl_extracts_postings_and_enforces_date_and_us_evidence() {
    use std::collections::HashMap;

    let cases = [
        (
            "boundary",
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-15T00:00:00Z"),
        ),
        (
            "newer",
            "Software Engineer",
            Some("Seattle, WA"),
            Some("2026-10-15T23:59:59Z"),
        ),
        (
            "older",
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-14T23:59:59.999Z"),
        ),
        (
            "future",
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-16T00:00:00.001Z"),
        ),
        (
            "missing-date",
            "Software Engineer",
            Some("Austin, TX"),
            None,
        ),
        (
            "invalid-date",
            "Software Engineer",
            Some("Austin, TX"),
            Some("not-a-date"),
        ),
        (
            "canada",
            "Software Engineer",
            Some("Toronto, ON, Canada"),
            Some("2026-10-15T20:00:00Z"),
        ),
        (
            "unknown",
            "Software Engineer",
            Some("Remote"),
            Some("2026-10-15T20:00:00Z"),
        ),
        (
            "remote-us",
            "Software Engineer",
            Some("Remote, United States"),
            Some("2026-10-15T20:00:00Z"),
        ),
        (
            "remote-only",
            "Software Engineer",
            Some("Remote"),
            Some("2026-10-15T20:00:00Z"),
        ),
        (
            "unrelated",
            "Accounting Manager",
            Some("Austin, TX"),
            Some("2026-10-15T20:00:00Z"),
        ),
    ];
    let links = cases
        .iter()
        .map(|(id, title, _, _)| format!("<article><a href='/jobs/{id}'>{title}</a></article>"))
        .collect::<String>();
    let pages: HashMap<_, _> = cases
        .iter()
        .map(|(id, title, location, date)| {
            (
                format!("/jobs/{id}"),
                plain_html_detail(title, *location, *date),
            )
        })
        .collect();
    let server = Server::new(move |path| match path {
        "/careers" => Response::html("<a href='/openings'>Open positions</a>"),
        "/openings" => Response::html(links.clone()),
        other => Response::html(
            pages
                .get(other)
                .unwrap_or_else(|| panic!("unexpected request {other}"))
                .clone(),
        ),
    });

    let mut config = crawl(20, 2);
    config["crawl"]["extraction"] = json!("public_html");
    config["per_host_delay_ms"] = json!(0);
    config["jitter_ms"] = json!(0);
    config["ignore_robots"] = json!(true);
    let filters = json!({"matching": {
        "target_roles": ["Software Engineer"],
        "posting_date": {"preset": "past_24h", "timezone": "UTC", "include_unknown": false},
        "countries": ["US"],
        "location_include_unknown": false
    }});
    let events = run_with_filters(
        vec![source(&server, "plain-html", "/careers", config)],
        20,
        filters,
    );

    let accepted: Vec<_> = jobs(&events)
        .into_iter()
        .map(|event| match event {
            WorkerEvent::Job {
                title,
                url,
                date_posted,
                location,
                locations,
                description_text,
                ..
            } => (
                url,
                title,
                date_posted,
                location,
                locations,
                description_text,
            ),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        accepted.len(),
        3,
        "accepted postings: {accepted:?}; events: {events:?}"
    );
    for (url, title, date, location, locations, description) in &accepted {
        let id = url.rsplit('/').next().unwrap();
        assert!(
            url.ends_with(&format!("/jobs/{id}")),
            "wrong original URL: {url}"
        );
        assert_eq!(*title, "Software Engineer");
        assert!(date.is_some());
        assert!(
            location.contains("Austin")
                || location.contains("Seattle")
                || location.contains("Remote")
        );
        assert!(
            locations
                .iter()
                .any(|loc| loc.country_code.as_deref() == Some("US")),
            "US evidence missing for {id}: {locations:?}"
        );
        assert!(description
            .as_deref()
            .unwrap_or("")
            .contains("dependable software systems"));
    }
    let ids: Vec<_> = accepted
        .iter()
        .map(|(url, ..)| url.rsplit('/').next().unwrap())
        .collect();
    assert!(
        ids.contains(&"boundary") && ids.contains(&"newer") && ids.contains(&"remote-us"),
        "wrong accepted identities: {ids:?}"
    );
    assert_eq!(
        accepted
            .iter()
            .find(|(url, ..)| url.ends_with("/jobs/boundary"))
            .unwrap()
            .2
            .as_deref(),
        Some("2026-10-15T00:00:00Z")
    );
    for (id, ..) in cases
        .iter()
        .filter(|(id, ..)| !["boundary", "newer", "remote-us"].contains(id))
    {
        assert!(!ids.contains(id), "ineligible job was accepted: {id}");
    }
    let stats = &stats(&events)[0];
    assert_eq!(stats.accepted, 3);
    assert!(
        stats.rejected + stats.unresolved >= 8,
        "ineligible candidates lack accounting: {stats:?}"
    );
    let paths = server.paths();
    assert_eq!(paths[0], "/careers");
    assert_eq!(paths[1], "/openings");
    for (id, ..) in cases {
        assert!(
            paths.contains(&format!("/jobs/{id}")),
            "crawler did not visit detail page {id}: {paths:?}"
        );
    }
    assert!(!paths.contains(&"/robots.txt".to_string()));
}

#[test]
fn static_crawl_applies_source_delay_and_ignores_robots_for_all_pages() {
    let server = Server::new(|path| {
        match path {
        "/robots.txt" => Response::html("User-agent: *\nDisallow: /jobs/blocked\n"),
        "/seed" => Response::html("<a href='/careers'>Careers</a>"),
        "/careers" => Response::html(
            "<a href='/jobs/allowed'>Software Engineer role</a><a href='/jobs/blocked'>Software Engineer role</a>",
        ),
        "/jobs/allowed" | "/jobs/blocked" => Response::html(
            "<h1>Software Engineer</h1><main><p>Build software systems and services.</p></main><div class='location'>United States</div><time datetime='2026-10-15T20:00:00Z'>Today</time>",
        ),
        other => panic!("unexpected request {other}"),
    }
    });
    let mut config = crawl(4, 2);
    config["crawl"]["extraction"] = json!("public_html");
    config["per_host_delay_ms"] = json!(1000);
    config["jitter_ms"] = json!(0);
    config["ignore_robots"] = json!(true);
    let events = run(vec![source(&server, "crawler-policy", "/seed", config)], 4);
    let paths = server.paths();
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(
        paths.contains(&"/jobs/blocked".to_string()),
        "robots-disallowed child was skipped: {paths:?}"
    );
    assert!(
        !paths.contains(&"/robots.txt".to_string()),
        "ignore_robots still fetched robots.txt: {paths:?}"
    );
    let arrivals = server.arrivals();
    assert_eq!(arrivals.len(), paths.len());
    for pair in arrivals.windows(2) {
        assert!(
            pair[1].duration_since(pair[0]) >= Duration::from_millis(980),
            "source delay not applied across crawl requests: {:?}",
            pair[1].duration_since(pair[0])
        );
    }
    assert_eq!(
        stats(&events)[0].termination,
        CoverageTermination::Exhausted
    );
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
    assert!(events.iter().any(|event| matches!(
        event,
        WorkerEvent::Warning { message, termination: Some(CoverageTermination::TimeLimit), .. }
            if message == "Website crawl navigation reached its time limit"
    )));
    assert_eq!(slow.paths(), ["/robots.txt", "/seed", "/slow"]);
}

#[test]
fn source_deadline_wins_over_navigation_timeout_and_preserves_earlier_postings() {
    let slow = Server::new(|path| match path {
        "/robots.txt" => allow(),
        "/seed" => Response::html(format!("{}<a href='/slow'>Slow</a>", posting("42"))),
        "/slow" => {
            thread::sleep(Duration::from_secs(11));
            Response::html("<a href='/must-not-fetch'>Later</a>")
        }
        _ => panic!("request after source deadline"),
    });
    let mut config = crawl(20, 2);
    config["navigation_timeout_s"] = json!(30);
    let events = run_with_limits(
        vec![source(&slow, "source-timeout", "/seed", config)],
        20,
        20,
        Some(10),
    );
    assert_eq!(jobs(&events).len(), 1, "events={events:?}");
    assert_eq!(
        stats(&events)[0].termination,
        CoverageTermination::TimeLimit
    );
    assert_eq!(stats(&events)[0].outcome, Some(ScanOutcome::Partial));
    assert_eq!(slow.paths(), ["/robots.txt", "/seed", "/slow"]);
    assert!(events.iter().any(|event| matches!(
        event,
        WorkerEvent::Warning { message, termination: Some(CoverageTermination::TimeLimit), .. }
            if message == "Static source duration limit reached; remaining candidates/pages were not examined"
    )));
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

#[test]
fn role_and_keyword_guided_crawl_prioritizes_relevant_links_and_scrapes_majority_requirements() {
    let server = Server::new(|path| match path {
        "/robots.txt" => allow(),
        "/careers" => Response::html(
            r#"
            <html>
            <body>
                <div class="header">
                    <a href="/privacy">Privacy Policy</a>
                    <a href="/terms">Terms of Service</a>
                    <a href="/about">About Our Company</a>
                </div>
                <div class="jobs">
                    <div class="job-card">
                        <a href="/jobs/sales-intern">Sales Intern</a>
                        <span class="location">Remote</span>
                    </div>
                    <div class="job-card">
                        <a href="/jobs/lead-recruiter">Lead Recruiter</a>
                        <span class="location">New York, NY</span>
                    </div>
                    <div class="job-card">
                        <a href="/jobs/software-engineer">Senior Software Engineer</a>
                        <span class="location">Austin, TX</span>
                    </div>
                </div>
            </body>
            </html>
        "#,
        ),
        "/jobs/software-engineer" => Response::html(
            r#"
            <html>
            <body>
                <main>
                    <h1>Senior Software Engineer</h1>
                    <div class="location">Austin, TX</div>
                    <div class="salary">$160,000 - $200,000 per year</div>
                    <div class="job-description">
                        <p>We are looking for a Senior Software Engineer to build distributed Rust database platforms.</p>
                        <p>Responsibilities include designing high-throughput ingestion pipelines and microservices.</p>
                    </div>
                </main>
            </body>
            </html>
        "#,
        ),
        "/jobs/sales-intern" => Response::html(
            r#"
            <html><body><h1>Sales Intern</h1><p>Cold outreach and lead qualification.</p></body></html>
        "#,
        ),
        "/jobs/lead-recruiter" => Response::html(
            r#"
            <html><body><h1>Lead Recruiter</h1><p>Source candidates and manage pipelines.</p></body></html>
        "#,
        ),
        "/privacy" | "/terms" | "/about" => Response::html("<html><body>Boilerplate</body></html>"),
        other => panic!("Unexpected request: {other}"),
    });

    let crawl_cfg = json!({
        "crawl": {
            "max_pages": 4,
            "max_depth": 2,
            "extraction": "public_html"
        }
    });

    let filters = json!({
        "matching": {
            "target_roles": ["Software Engineer"],
            "keywords": {
                "any": ["Rust", "Distributed"],
                "all": [],
                "exclude": ["Recruiter"],
                "fields": ["title", "description"]
            },
            "countries": ["US"],
            "posting_date": {
                "preset": "past_24h",
                "timezone": "America/New_York",
                "include_unknown": false
            }
        }
    });

    let events = run_with_filters(
        vec![source(&server, "html-crawl", "/careers", crawl_cfg)],
        10,
        filters,
    );

    assert!(
        jobs(&events).is_empty(),
        "undated HTML posting passed strict date eligibility: {events:?}"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorkerEvent::Warning { message, .. } if message.contains("posting_date")
        )),
        "undated candidate lacks posting-date rejection/unresolved evidence: {events:?}"
    );
    let paths = server.paths();
    assert!(
        paths.contains(&"/jobs/software-engineer".to_string()),
        "The prioritized software engineering detail page must have been visited; paths={paths:?}"
    );
}

#[test]
fn opaque_posting_card_with_lever_heading_is_extracted() {
    let server = Server::new(|path| {
        match path {
        "/careers" => Response::html(
            "<div class='posting'><a class='posting-title' href='/employer/7bc9'><h5>Software Engineer</h5></a><span class='location'>Austin, TX</span></div>",
        ),
        "/employer/7bc9" => Response::html(
            "<main><div class='posting-headline'><h2>Software Engineer</h2></div><div class='location'>Austin, TX</div><time itemprop='datePosted' datetime='2026-10-15T12:00:00Z'>Posted October 15</time><div class='job-description'>Responsibilities include building reliable distributed software systems and developing engineering infrastructure.</div></main>",
        ),
        other => panic!("unexpected request {other}"),
    }
    });
    let events = run_with_filters(
        vec![source(&server, "opaque", "/careers", regression_crawl())],
        10,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    assert_eq!(server.paths(), ["/careers", "/employer/7bc9"]);
}

#[test]
fn apply_anchor_does_not_consume_later_meaningful_title() {
    let server = Server::new(|path| {
        match path {
        "/careers" => Response::html("<article><a href='/jobs/123'>Apply</a><a href='/jobs/123'>Software Engineer</a></article>"),
        "/jobs/123" => Response::html(plain_html_detail("Software Engineer", Some("Austin, TX"), Some("2026-10-15T12:00:00Z"))),
        other => panic!("unexpected request {other}"),
    }
    });
    let events = run_with_filters(
        vec![source(&server, "apply", "/careers", regression_crawl())],
        10,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    assert_eq!(server.paths(), ["/careers", "/jobs/123"]);
}

#[test]
fn detail_structured_date_and_geography_are_preserved() {
    let server = Server::new(|path| match path {
        "/careers" => {
            Response::html("<article><a href='/jobs/123'>Software Engineer</a></article>")
        }
        "/jobs/123" => Response::html(
            r#"<main><h1>Software Engineer</h1><script type="application/ld+json">{"@type":"JobPosting","title":"Software Engineer","url":"/jobs/123","datePosted":"2026-10-15T12:00:00Z","hiringOrganization":{"name":"Fixture Employer"},"jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressLocality":"Austin","addressRegion":"TX","addressCountry":"US"}},"description":"Build reliable distributed software systems."}</script></main>"#,
        ),
        other => panic!("unexpected request {other}"),
    });
    let events = run_with_filters(
        vec![source(
            &server,
            "structured",
            "/careers",
            regression_crawl(),
        )],
        10,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    assert_eq!(server.paths(), ["/careers", "/jobs/123"]);
}

fn regression_crawl() -> Value {
    json!({"crawl":{"max_pages":6,"max_depth":2,"extraction":"public_html"},"ignore_robots":true,"per_host_delay_ms":0,"jitter_ms":0})
}

fn regression_filters() -> Value {
    json!({"matching":{"target_roles":["Software Engineer"],"countries":["US"],"location_include_unknown":false,"posting_date":{"preset":"past_24h","timezone":"UTC","include_unknown":false}}})
}

#[test]
fn structured_detail_requires_original_date_and_same_job_geography() {
    for (name, evidence, expected) in [
        (
            "single-no-url",
            json!({"datePosted":"2026-10-15T12:00:00Z","jobLocation":{"address":{"addressCountry":"US"}}}),
            1,
        ),
        (
            "old",
            json!({"datePosted":"2026-10-14T12:00:00Z","jobLocation":{"address":{"addressCountry":"US"}}}),
            0,
        ),
        (
            "missing-date",
            json!({"jobLocation":{"address":{"addressCountry":"US"}}}),
            0,
        ),
        (
            "wrong-country",
            json!({"datePosted":"2026-10-15T12:00:00Z","jobLocation":{"address":{"addressCountry":"CA"}}}),
            0,
        ),
        (
            "updated-only",
            json!({"dateModified":"2026-10-15T12:00:00Z","jobLocation":{"address":{"addressCountry":"US"}}}),
            0,
        ),
    ] {
        let mut record = evidence;
        record["@type"] = json!("JobPosting");
        record["title"] = json!("Software Engineer");
        record["description"] = json!("Responsibilities include building reliable distributed software services and maintaining production engineering infrastructure.");
        let detail = format!("<main><h1>Software Engineer</h1><script type='application/ld+json'>{record}</script><time datetime='2026-10-15T12:00:00Z'>Updated</time></main>");
        let server = Server::new(move |path| match path {
            "/careers" => {
                Response::html("<article><a href='/jobs/123'>Software Engineer</a></article>")
            }
            "/jobs/123" => Response::html(detail.clone()),
            other => panic!("unexpected request {other}"),
        });
        let events = run_with_filters(
            vec![source(&server, name, "/careers", regression_crawl())],
            10,
            regression_filters(),
        );
        assert_eq!(jobs(&events).len(), expected, "{name}: {events:?}");
        assert_eq!(server.paths(), ["/careers", "/jobs/123"], "{name}");
    }
}

#[test]
fn unrelated_or_conflicting_structured_posting_cannot_repair_target() {
    for conflicting in [false, true] {
        let other_url = if conflicting {
            "/jobs/123"
        } else {
            "/jobs/456"
        };
        let target_date = if conflicting {
            r#","datePosted":"2026-10-14T12:00:00Z""#
        } else {
            ""
        };
        let detail = format!(
            r#"<main><h1>Software Engineer</h1><script type="application/ld+json">[
            {{"@type":"JobPosting","title":"Software Engineer","url":"/jobs/123","description":"Responsibilities include maintaining reliable distributed software systems and engineering infrastructure."{target_date}}},
            {{"@type":"JobPosting","title":"Software Engineer","url":"{other_url}","datePosted":"2026-10-15T12:00:00Z","jobLocation":{{"address":{{"addressCountry":"US"}}}}}}
        ]</script></main>"#
        );
        let server = Server::new(move |path| match path {
            "/careers" => {
                Response::html("<article><a href='/jobs/123'>Software Engineer</a></article>")
            }
            "/jobs/123" => Response::html(detail.clone()),
            other => panic!("unexpected request {other}"),
        });
        let events = run_with_filters(
            vec![source(&server, "unrelated", "/careers", regression_crawl())],
            10,
            regression_filters(),
        );
        assert!(jobs(&events).is_empty(), "{events:?}");
        assert_eq!(stats(&events)[0].unresolved, 1, "{events:?}");
        assert_eq!(server.paths(), ["/careers", "/jobs/123"]);
    }
}

#[test]
fn self_contained_detail_uses_current_html_without_refetch_or_warning() {
    let server = Server::new(|path| match path {
        "/jobs/123" => Response::html(plain_html_detail(
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-15T12:00:00Z"),
        )),
        other => panic!("unexpected request {other}"),
    });
    let events = run_with_filters(
        vec![source(&server, "self", "/jobs/123", regression_crawl())],
        10,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    assert_eq!(server.paths(), ["/jobs/123"]);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkerEvent::Warning { .. })),
        "{events:?}"
    );
}

#[test]
fn six_navigation_bound_preserves_jobs_and_leaves_unvisited_candidates_unexamined() {
    let server = Server::new(|path| {
        if path == "/careers" {
            return Response::html((0..9).map(|id| format!("<article><a href='/jobs/{id}'>Software Engineer</a><a href='/jobs/{id}#apply'>Apply</a></article>")).collect::<String>());
        }
        if path.starts_with("/jobs/") {
            return Response::html(plain_html_detail(
                "Software Engineer",
                Some("Austin, TX"),
                Some("2026-10-15T12:00:00Z"),
            ));
        }
        panic!("unexpected request {path}");
    });
    let events = run_with_filters(
        vec![source(&server, "bounded", "/careers", regression_crawl())],
        40,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 5, "{events:?}");
    assert_eq!(stats(&events)[0].examined, 5, "{events:?}");
    assert_eq!(stats(&events)[0].unresolved, 0, "{events:?}");
    assert_eq!(server.paths().len(), 6);
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorkerEvent::Warning {
                termination: Some(CoverageTermination::PageLimit),
                ..
            }
        )),
        "{events:?}"
    );
}

#[test]
fn lever_tenant_scope_rejects_sibling_redirect_base_and_encoded_escapes() {
    let server = Server::new(|path| match path {
        "/jobs.lever.co/palantir" => Response::html(
            r#"<base href="https://jobs.lever.co/sibling/"><div class="posting"><a class="posting-title" href="palantir/opaque"><h5>Software Engineer</h5></a></div><article><a href="/sibling/jobs/999">Software Engineer</a></article><article><a href="/palantir/%2e%2e/sibling/jobs/escape">Software Engineer</a></article><article><a href="/palantir/jobs/redirect">Software Engineer</a></article>"#,
        ),
        "/jobs.lever.co/palantir/opaque" => Response::html(plain_html_detail(
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-15T12:00:00Z"),
        )),
        "/jobs.lever.co/palantir/jobs/redirect" => Response {
            status: "302 Found",
            headers: "Location: https://jobs.lever.co/sibling/jobs/redirected\r\n".into(),
            body: String::new(),
        },
        other => panic!("out-of-scope request {other}"),
    });
    let config = regression_crawl();
    let sources = vec![
        json!({"id":"scoped","name":"Palantir","kind":"selector","identifier":"https://jobs.lever.co/palantir","config":config}),
    ];
    let events = run_with_replay(sources, 10, regression_filters(), Some(&server.origin));
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    let paths = server.paths();
    assert_eq!(paths.len(), 3, "{paths:?}");
    assert!(paths.contains(&"/jobs.lever.co/palantir/opaque".into()));
    assert!(paths.contains(&"/jobs.lever.co/palantir/jobs/redirect".into()));
    assert!(!paths.iter().any(|path| path.contains("/sibling")));
}

#[test]
fn excluded_url_word_is_not_a_keyword_field_rejection() {
    let server = Server::new(|path| match path {
        "/careers" => Response::html(
            "<article><a href='/jobs/recruiter-platform'>Software Engineer</a></article>",
        ),
        "/jobs/recruiter-platform" => Response::html(plain_html_detail(
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-15T12:00:00Z"),
        )),
        other => panic!("unexpected request {other}"),
    });
    let mut filters = regression_filters();
    filters["matching"]["keywords"] =
        json!({"any":[],"all":[],"exclude":["recruiter"],"fields":["title","description"]});
    let events = run_with_filters(
        vec![source(&server, "url-only", "/careers", regression_crawl())],
        10,
        filters,
    );
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    assert_eq!(server.paths(), ["/careers", "/jobs/recruiter-platform"]);
}

#[test]
fn later_relevant_link_replaces_low_priority_unvisited_frontier_entry() {
    let server = Server::new(|path| {
        match path {
        "/seed" => Response::html("<a href='/directory'>Software Engineer openings</a><a href='/noise'>About our organization</a>"),
        "/directory" => Response::html("<a href='/engineering'>Software Engineer careers</a>"),
        "/engineering" => Response::html("<main><h1>Software Engineer</h1><div class='location'>Austin, TX</div><time itemprop='datePosted' datetime='2026-10-15T12:00:00Z'>Posted</time><div class='job-description'>Responsibilities include building reliable distributed software systems and maintaining production infrastructure for customers.</div></main>"),
        other => panic!("low-priority or unexpected request {other}"),
    }
    });
    let mut config = regression_crawl();
    config["crawl"]["max_pages"] = json!(3);
    config["crawl"]["max_depth"] = json!(3);
    let events = run_with_filters(
        vec![source(&server, "frontier", "/seed", config)],
        10,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 1, "{events:?}");
    assert_eq!(server.paths(), ["/seed", "/directory", "/engineering"]);
}

#[test]
fn html_candidate_priority_puts_known_geography_before_known_date() {
    let server = Server::new(|path| match path {
        "/careers" => Response::html(
            r#"<script type="application/ld+json">[
            {"@type":"JobPosting","title":"Software Engineer","url":"/jobs/unknown-geography","datePosted":"2026-10-15T12:00:00Z"},
            {"@type":"JobPosting","title":"Software Engineer","url":"/jobs/unknown-date","jobLocation":{"address":{"addressCountry":"US"}}}
        ]</script>"#,
        ),
        "/jobs/unknown-date" | "/jobs/unknown-geography" => Response::html(plain_html_detail(
            "Software Engineer",
            Some("Austin, TX"),
            Some("2026-10-15T12:00:00Z"),
        )),
        other => panic!("unexpected request {other}"),
    });
    let events = run_with_filters(
        vec![source(&server, "priority", "/careers", regression_crawl())],
        10,
        regression_filters(),
    );
    assert_eq!(jobs(&events).len(), 2, "{events:?}");
    assert_eq!(
        server.paths(),
        ["/careers", "/jobs/unknown-date", "/jobs/unknown-geography"]
    );
}

#[test]
fn structured_remote_eligibility_and_native_salary_survive_html_hydration() {
    let server = Server::new(|path| {
        match path {
        "/careers" => Response::html("<article><a href='/jobs/remote'>Software Engineer</a><div class='location'>New York, NY</div></article>"),
        "/jobs/remote" => Response::html(r#"<script type="application/ld+json">{"@type":"JobPosting","title":"Software Engineer","url":"/jobs/remote","datePosted":"2026-10-15T12:00:00Z","jobLocationType":"TELECOMMUTE","jobLocation":{"address":{"addressCountry":"CA"}},"applicantLocationRequirements":{"@type":"Country","name":"United States"},"baseSalary":{"@type":"MonetaryAmount","currency":"USD","value":{"@type":"QuantitativeValue","minValue":55,"maxValue":75,"unitText":"HOUR"}},"description":"Responsibilities include building reliable distributed software systems."}</script>"#),
        other => panic!("unexpected request {other}"),
    }
    });
    let events = run_with_filters(
        vec![source(
            &server,
            "remote-native",
            "/careers",
            regression_crawl(),
        )],
        10,
        regression_filters(),
    );
    let accepted = jobs(&events);
    assert_eq!(accepted.len(), 1, "{events:?}");
    let WorkerEvent::Job {
        salary: Some(salary),
        locations,
        workplace_type,
        ..
    } = accepted[0]
    else {
        panic!("{events:?}");
    };
    assert_eq!(*workplace_type, scrape_core::WorkplaceType::Remote);
    assert!(locations
        .iter()
        .all(|location| location.raw.starts_with("applicantLocationRequirements:")));
    assert_eq!(salary.ranges.len(), 1);
    assert_eq!(salary.ranges[0].min_minor, Some(5500));
    assert_eq!(salary.ranges[0].max_minor, Some(7500));
    assert_eq!(salary.ranges[0].currency.as_deref(), Some("USD"));
    assert_eq!(
        salary.ranges[0].interval,
        Some(scrape_core::SalaryInterval::Hour)
    );
}

#[test]
fn generic_time_is_not_original_publication_evidence() {
    let server = Server::new(|path| {
        match path {
        "/careers" => Response::html("<article><a href='/jobs/undated'>Software Engineer</a></article>"),
        "/jobs/undated" => Response::html("<main><h1>Software Engineer</h1><div class='location'>Austin, TX</div><time datetime='2026-10-15T12:00:00Z'>October 15</time><div class='job-description'>Responsibilities include building reliable distributed software systems and maintaining production engineering infrastructure.</div></main>"),
        other => panic!("unexpected request {other}"),
    }
    });
    let events = run_with_filters(
        vec![source(
            &server,
            "generic-time",
            "/careers",
            regression_crawl(),
        )],
        10,
        regression_filters(),
    );
    assert!(jobs(&events).is_empty(), "{events:?}");
    assert_eq!(stats(&events)[0].unresolved, 1, "{events:?}");
}

#[test]
fn public_html_crawl_detail_request_bound_preserves_remaining_candidates() {
    let server = Server::new(|path| match path {
        "/careers" => Response::html(
            "<article><a href='/jobs/1'>Software Engineer 1</a></article>\
             <article><a href='/jobs/2'>Software Engineer 2</a></article>\
             <article><a href='/jobs/3'>Software Engineer 3</a></article>",
        ),
        "/jobs/1" => Response::html(
            r#"<main><h1>Software Engineer 1</h1><script type="application/ld+json">{"@type":"JobPosting","title":"Software Engineer 1","url":"/jobs/1","datePosted":"2026-10-15T12:00:00Z","hiringOrganization":{"name":"Fixture Employer"},"jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressCountry":"US"}},"description":"Hydrated role 1"}</script></main>"#,
        ),
        "/jobs/2" | "/jobs/3" => Response::html("<html>No LD+JSON</html>"),
        other => panic!("unexpected request {other}"),
    });
    let crawl_config = regression_crawl();
    let events = run_with_custom_limits(
        vec![source(&server, "bound-test", "/careers", crawl_config)],
        10,
        regression_filters(),
        json!({
            "per_host_delay_ms": 0,
            "request_timeout_s": 5,
            "enrichment": { "timeout_ms": 10000, "max_requests": 1 }
        }),
    );
    assert_eq!(stats(&events)[0].examined, 3, "{events:?}");
    assert_eq!(stats(&events)[0].accepted, 1, "{events:?}");
    assert_eq!(stats(&events)[0].unresolved, 2, "{events:?}");
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorkerEvent::Warning {
                termination: Some(CoverageTermination::RequestLimit),
                ..
            }
        )),
        "{events:?}"
    );
}
#[test]
fn public_html_crawl_detail_timeout_preserves_remaining_candidates() {
    let server = Server::new(|path| match path {
        "/careers" => Response::html(
            "<article><a href='/jobs/1'>Software Engineer 1</a></article>\
             <article><a href='/jobs/2'>Software Engineer 2</a></article>",
        ),
        "/jobs/1" => {
            thread::sleep(Duration::from_millis(500));
            Response::html("<html>Late detail</html>")
        }
        "/jobs/2" => Response::html(
            r#"<main><h1>Software Engineer 2</h1><script type="application/ld+json">{"@type":"JobPosting","title":"Software Engineer 2","url":"/jobs/2","datePosted":"2026-10-15T12:00:00Z","hiringOrganization":{"name":"Fixture Employer"},"jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressCountry":"US"}},"description":"Hydrated role 2"}</script></main>"#,
        ),
        other => panic!("unexpected request {other}"),
    });
    let crawl_config = regression_crawl();
    let events = run_with_custom_limits(
        vec![source(&server, "timeout-test", "/careers", crawl_config)],
        10,
        regression_filters(),
        json!({
            "per_host_delay_ms": 0,
            "request_timeout_s": 5,
            "enrichment": { "timeout_ms": 300, "max_requests": 10 }
        }),
    );
    assert_eq!(stats(&events)[0].examined, 3, "{events:?}");
    assert_eq!(stats(&events)[0].unresolved, 2, "{events:?}");
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
}
