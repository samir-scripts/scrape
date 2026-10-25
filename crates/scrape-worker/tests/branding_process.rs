//! Authored provider contracts and a separately captured public logo asset.
//! No live career sites or user storage.
use scrape_core::{
    branding::{LogoMethod, LogoStatus},
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

fn run(kind: &str, mode: &str) -> (Vec<WorkerEvent>, Vec<String>) {
    let invalid_selector = mode == "invalid-selector";
    let no_hint = mode == "empty-no-hint";
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let paths = Arc::new(Mutex::new(Vec::new()));
    let captured = paths.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let kind = kind.to_string();
    let mode = mode.to_string();
    let server_mode = mode.clone();
    let k = kind.clone();
    let server = thread::spawn(move || {
        while !stopping.load(Ordering::SeqCst) {
            if let Ok((mut socket, _)) = listener.accept() {
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = socket.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap();
                captured.lock().unwrap().push(path.to_owned());
                let mut status = "200 OK";
                let body = if path.ends_with("robots.txt") {
                    b"User-agent: *\nAllow: /".to_vec()
                } else if path.ends_with("/logo.png") {
                    match server_mode.as_str() {
                        "broken" => {
                            status = "404 Not Found";
                            Vec::new()
                        }
                        "decode" => b"<html>Access challenge</html>".to_vec(),
                        _ => include_bytes!("fixtures/cloudflare-logo.jpg").to_vec(),
                    }
                } else if path.ends_with("?content=true") {
                    json!({"jobs":(0..2).map(|i|json!({"id":i,"title":"Engineer","absolute_url":format!("https://fixture.test/jobs/{i}"),"company_name":"Cloudflare"})).collect::<Vec<_>>()}).to_string().into_bytes()
                } else if path.contains("?pay_transparency=true") {
                    let id = path
                        .rsplit('/')
                        .next()
                        .unwrap()
                        .split('?')
                        .next()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap();
                    json!({"id":id,"title":"Engineer","absolute_url":format!("https://fixture.test/jobs/{id}")}).to_string().into_bytes()
                } else if path.starts_with("/api.lever.co/") {
                    json!((0..2).map(|i|json!({"id":i.to_string(),"text":"Engineer","hostedUrl":format!("https://fixture.test/jobs/{i}")})).collect::<Vec<_>>()).to_string().into_bytes()
                } else if path.starts_with("/api.ashbyhq.com/") {
                    json!({"jobs":(0..2).map(|i|json!({"id":i.to_string(),"title":"Engineer","jobUrl":if server_mode.starts_with("ashby-theme") {format!("https://jobs.ashbyhq.com/fixture/{i}")} else {format!("https://fixture.test/jobs/{i}")}})).collect::<Vec<_>>()} ).to_string().into_bytes()
                } else if path.starts_with("/jobs.ashbyhq.com/") {
                    let organization = if server_mode == "ashby-theme-mismatch" {
                        "Another Employer"
                    } else {
                        "Cloudflare"
                    };
                    let data = if server_mode == "ashby-theme-favicon-only" {
                        String::new()
                    } else {
                        format!(
                            "<script>window.__appData = {};</script>",
                            json!({
                                "organization": {
                                    "name": organization,
                                    "organizationId": "org-id",
                                    "theme": {
                                        "logoSquareImageUrl": "https://app.ashbyhq.com/api/images/org-theme-logo/org-id/theme-id/logo.png",
                                        "logoWordmarkImageUrl": null
                                    }
                                }
                            })
                        )
                    };
                    format!("<link rel='icon' href='https://cdn.ashbyprd.com/cdn_assets/x/favicon.png'>{data}").into_bytes()
                } else if path.starts_with("/fixture.test/jobs/") || path == "/fixture.test/board" {
                    if server_mode == "board" && path != "/fixture.test/board" {
                        b"<html>No logo on posting</html>".to_vec()
                    } else {
                        b"<img alt='Greenhouse Logo' src='/platform.png'><img alt='Cloudflare Logo' src='/logo.png'>".to_vec()
                    }
                } else if kind == "selector" {
                    b"<img class='employer' src='/logo.png'><div class='job'><a href='/jobs/1'>Engineer</a></div><div class='job'><a href='/jobs/2'>Engineer</a></div>".to_vec()
                } else {
                    let jobs:Vec<Value>=(0..2).map(|i|json!({"@type":"JobPosting","identifier":i.to_string(),"title":"Engineer","url":format!("/jobs/{i}"),"hiringOrganization":if matches!(server_mode.as_str(),"empty-employer"|"empty-no-hint") {json!({"name":"","logo":"/logo.png"})}else if server_mode=="missing" {json!({"name":"Cloudflare"})}else{json!({"name":"Cloudflare","logo":"/logo.png"})}})).collect();
                    format!("<img alt='Greenhouse Logo' src='/platform.png'><script type='application/ld+json'>{}</script>",json!(jobs)).into_bytes()
                };
                let _=write!(socket,"HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",body.len());
                let _ = socket.write_all(&body);
            } else {
                thread::sleep(Duration::from_millis(3));
            }
        }
    });
    let mut config = json!({"version":PROTOCOL_VERSION,"sources":[{"id":"source","kind":k,"name":"Engineering jobs","identifier":if matches!(k.as_str(),"jsonld"|"selector") {"https://fixture.test/careers"}else{"fixture"},"config":{"__company":"Cloudflare","url":"https://fixture.test/board","__workflow_spec":{"max_jobs_per_run":2},"item":".job","title":"a","link":"a","employer_logo":if invalid_selector {"["}else{".employer"}}}],"limits":{"per_host_delay_ms":0,"request_timeout_s":2}});
    if no_hint {
        config["sources"][0]["config"]
            .as_object_mut()
            .unwrap()
            .remove("__company");
    }
    if mode.starts_with("ashby-theme") {
        config["sources"][0]["config"]["url"] = json!("https://jobs.ashbyhq.com/fixture/0");
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &origin])
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
    let output = child.wait_with_output().unwrap();
    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let paths = paths.lock().unwrap().clone();
    (events, paths)
}
#[test]
fn all_extraction_paths_validate_one_shared_asset_and_ignore_workflow_display_names() {
    for kind in ["greenhouse", "lever", "ashby", "jsonld", "selector"] {
        let (events, paths) = run(kind, "valid");
        let jobs: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, WorkerEvent::Job { .. }))
            .collect();
        assert_eq!(jobs.len(), 2, "{kind}");
        assert!(
            jobs.iter()
                .all(|e| matches!(e,WorkerEvent::Job{company,..} if company=="Cloudflare")),
            "{kind}"
        );
        let reports: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let WorkerEvent::Branding { logo, .. } = e {
                    Some(logo)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(reports.len(), 1);
        let logo = reports[0];
        logo.validate().unwrap();
        assert_eq!(
            logo.status,
            LogoStatus::Verified,
            "{kind}: {:?}",
            logo.diagnostic
        );
        assert_eq!(
            logo.evidence.as_ref().unwrap().url,
            "https://fixture.test/logo.png"
        );
        assert!(logo
            .asset
            .as_ref()
            .unwrap()
            .starts_with("data:image/webp;base64,"));
        assert_eq!(
            paths.iter().filter(|p| p.ends_with("/logo.png")).count(),
            1,
            "{kind}: {paths:?}"
        );
        assert!(!paths.iter().any(|p| p.contains("platform.png")));
        if kind == "selector" {
            assert_eq!(
                logo.evidence.as_ref().unwrap().method,
                LogoMethod::ConfiguredSelector
            );
        }
    }
}

#[test]
fn ashby_theme_downloads_one_employer_asset_for_two_jobs_without_platform_artwork() {
    for mode in [
        "ashby-theme",
        "ashby-theme-mismatch",
        "ashby-theme-favicon-only",
    ] {
        let (events, paths) = run("ashby", mode);
        let jobs: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorkerEvent::Job {
                    company,
                    title,
                    url,
                    ..
                } => Some((company.as_str(), title.as_str(), url.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(jobs.len(), 2, "{mode}");
        for (index, (company, title, url)) in jobs.iter().enumerate() {
            assert_eq!(*company, "Cloudflare", "{mode}");
            assert_eq!(*title, "Engineer", "{mode}");
            assert_eq!(*url, format!("https://jobs.ashbyhq.com/fixture/{index}"));
        }
        let reports: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorkerEvent::Branding {
                    source_id,
                    company,
                    logo,
                } => {
                    assert_eq!(source_id, "source");
                    assert_eq!(company, "Cloudflare");
                    Some(logo)
                }
                _ => None,
            })
            .collect();
        assert_eq!(reports.len(), 1, "{mode}");
        let report = reports[0];
        report.validate().unwrap();
        let expected_asset_requests = usize::from(mode == "ashby-theme");
        if mode == "ashby-theme" {
            assert_eq!(report.status, LogoStatus::Verified);
            let evidence = report.evidence.as_ref().unwrap();
            assert_eq!(evidence.method, LogoMethod::Provider);
            assert_eq!(evidence.page_url, "https://jobs.ashbyhq.com/fixture/0");
            assert_eq!(
                evidence.url,
                "https://app.ashbyhq.com/api/images/org-theme-logo/org-id/theme-id/logo.png"
            );
            assert_eq!(
                report.asset.as_deref(),
                Some(
                    scrape_core::branding::cache_asset(include_bytes!(
                        "fixtures/cloudflare-logo.jpg"
                    ))
                    .unwrap()
                    .as_str()
                )
            );
        } else {
            assert_eq!(report.status, LogoStatus::MissingMetadata, "{mode}");
            assert!(report.asset.is_none());
            assert!(report.evidence.is_none());
        }
        let mut non_robots: Vec<_> = paths
            .iter()
            .filter(|path| !path.ends_with("/robots.txt"))
            .map(String::as_str)
            .collect();
        non_robots.sort_unstable();
        let mut expected = vec![
            "/api.ashbyhq.com/posting-api/job-board/fixture?includeCompensation=true",
            "/jobs.ashbyhq.com/fixture/0",
        ];
        if expected_asset_requests == 1 {
            expected.push("/app.ashbyhq.com/api/images/org-theme-logo/org-id/theme-id/logo.png");
        }
        expected.sort_unstable();
        assert_eq!(non_robots, expected, "{mode}: {paths:?}");
        assert!(!paths.iter().any(|path| path.contains("favicon")));
    }
}
#[test]
fn diagnostics_distinguish_missing_metadata_download_and_decoding_failures_without_losing_jobs() {
    for (mode, expected) in [
        ("missing", LogoStatus::MissingMetadata),
        ("broken", LogoStatus::DownloadFailed),
        ("decode", LogoStatus::DecodeFailed),
    ] {
        let (events, paths) = run("jsonld", mode);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, WorkerEvent::Job { .. }))
                .count(),
            2
        );
        let report = events
            .iter()
            .find_map(|e| {
                if let WorkerEvent::Branding { logo, .. } = e {
                    Some(logo)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(report.status, expected);
        assert!(report.asset.is_none());
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, WorkerEvent::Warning { .. }))
                .count(),
            usize::from(mode != "missing")
        );
        assert!(!paths.iter().any(|p| p.contains("platform.png")));
    }
}
#[test]
fn configured_board_page_can_supply_branding_missing_from_the_posting_page() {
    let (events, paths) = run("greenhouse", "board");
    let report = events
        .iter()
        .find_map(|e| {
            if let WorkerEvent::Branding { logo, .. } = e {
                Some(logo)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(report.status, LogoStatus::Verified);
    assert_eq!(
        report.evidence.as_ref().unwrap().page_url,
        "https://fixture.test/board"
    );
    assert_eq!(paths.iter().filter(|p| p.ends_with("/board")).count(), 1);
}

#[test]
fn invalid_employer_selectors_fail_before_any_provider_request() {
    let (events, paths) = run("jsonld", "invalid-selector");
    assert!(paths.is_empty());
    assert!(events.iter().any(|event|matches!(event,WorkerEvent::Error{message,..} if message.contains("Invalid employer_logo selector"))));
    assert!(!events
        .iter()
        .any(|event| matches!(event, WorkerEvent::Job { .. })));
}

#[test]
fn empty_provider_employer_names_use_the_explicit_hint_without_trusting_nameless_logo_metadata() {
    for (mode, expected) in [
        ("empty-employer", "Cloudflare"),
        ("empty-no-hint", "Unknown company"),
    ] {
        let (events, paths) = run("jsonld", mode);
        let jobs: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .collect();
        assert_eq!(jobs.len(), 2);
        assert!(jobs
            .iter()
            .all(|event| matches!(event,WorkerEvent::Job{company,..} if company==expected)));
        assert!(events.iter().any(|event|matches!(event,WorkerEvent::Branding{logo,..} if logo.status==LogoStatus::MissingMetadata)));
        assert!(!paths.iter().any(|path| path.ends_with("/logo.png")));
    }
}

#[test]
fn large_branding_page_under_2mib_succeeds_without_warning() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let server = thread::spawn(move || {
        while !stopping.load(Ordering::SeqCst) {
            if let Ok((mut socket, _)) = listener.accept() {
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut buffer = [0; 4096];
                let n = socket.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                let path = request.split_whitespace().nth(1).unwrap();
                let body = if path.ends_with("robots.txt") {
                    b"User-agent: *\nAllow: /".to_vec()
                } else {
                    format!(
                        "<html><head><title>Acme Corp</title></head><body><!-- {} --></body></html>",
                        "padding".repeat(100 * 1024)
                    )
                    .into_bytes()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes());
                let _ = socket.write_all(&body);
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    });

    let client = scrape_worker::client::HttpClient::new(1, 0)
        .with_replay_origin(&origin)
        .unwrap();
    let res = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(client.fetch_branding_page(&format!("{}/careers", origin)));
    stop.store(true, Ordering::SeqCst);
    let _ = server.join();
    assert!(
        res.is_ok(),
        "Page under 2 MiB should succeed: {:?}",
        res.err()
    );
    let (html, _) = res.unwrap();
    assert!(html.len() > 600 * 1024);
}

#[test]
fn slow_employer_artwork_does_not_starve_other_sources_or_duplicate_asset_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let paths = Arc::new(Mutex::new(Vec::<String>::new()));
    let requests = paths.clone();
    let server = thread::spawn(move || {
        let mut handlers = Vec::new();
        while !stopping.load(Ordering::SeqCst) {
            if let Ok((mut socket, _)) = listener.accept() {
                let requests = requests.clone();
                handlers.push(thread::spawn(move || {
                    socket.set_nonblocking(false).unwrap();
                    socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                    let mut buffer = [0; 4096];
                    let count = socket.read(&mut buffer).unwrap();
                    let request = String::from_utf8_lossy(&buffer[..count]);
                    let path = request.split_whitespace().nth(1).unwrap();
                    requests.lock().unwrap().push(path.to_string());
                    let body = if path.ends_with("/robots.txt") {
                        b"User-agent: *\nAllow: /".to_vec()
                    } else if path == "/slow.test/logo.svg" {
                        thread::sleep(Duration::from_secs(5));
                        b"<svg xmlns='http://www.w3.org/2000/svg' width='12' height='12'><path fill='red' d='M0 0h12v12H0z'/></svg>".to_vec()
                    } else if path == "/fast.test/logo.svg" {
                        b"<svg xmlns='http://www.w3.org/2000/svg' width='24' height='12'><path fill='blue' d='M0 0h24v12H0z'/></svg>".to_vec()
                    } else {
                        let slow = path.ends_with("/slow");
                        let company = if slow { "Slow Employer" } else { "Fast Employer" };
                        let asset = if slow { "https://slow.test/logo.svg" } else { "https://fast.test/logo.svg" };
                        format!("<script type='application/ld+json'>{}</script>", json!({
                            "@type":"JobPosting", "identifier":path, "title":"Engineer",
                            "url":format!("https://board.test/apply{path}"),
                            "hiringOrganization":{"name":company,"logo":asset}
                        })).into_bytes()
                    };
                    let _ = write!(socket, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = socket.write_all(&body);
                }));
            } else {
                thread::sleep(Duration::from_millis(3));
            }
        }
        for handler in handlers {
            handler.join().unwrap();
        }
    });
    let sources: Vec<_> = ["slow", "fast", "fast-copy"]
        .into_iter()
        .map(|id| {
            json!({
                "id":id, "kind":"jsonld", "name":id,
                "identifier":format!("https://board.test/{id}"),
                "config":{}
            })
        })
        .collect();
    let config = json!({"version":PROTOCOL_VERSION,"sources":sources,"limits":{"per_host_delay_ms":0,"request_timeout_s":2}});
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args(["--stdin", "--replay-origin", &origin])
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
    let output = child.wait_with_output().unwrap();
    stop.store(true, Ordering::SeqCst);
    server.join().unwrap();
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
    let reports: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            WorkerEvent::Branding {
                source_id,
                company,
                logo,
            } => Some((source_id.as_str(), company.as_str(), logo)),
            _ => None,
        })
        .collect();
    assert_eq!(reports.len(), 3);
    assert_eq!(
        reports
            .iter()
            .find(|(id, _, _)| *id == "slow")
            .unwrap()
            .2
            .status,
        LogoStatus::DownloadFailed
    );
    for (id, company, logo) in &reports {
        if *id != "slow" {
            assert_eq!(*company, "Fast Employer");
            assert_eq!(logo.status, LogoStatus::Verified);
            assert_eq!(
                logo.evidence.as_ref().unwrap().url,
                "https://fast.test/logo.svg"
            );
            logo.validate().unwrap();
        }
    }
    assert_eq!(
        paths
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.as_str() == "/fast.test/logo.svg")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, WorkerEvent::Job { .. }))
            .count(),
        3
    );
}
