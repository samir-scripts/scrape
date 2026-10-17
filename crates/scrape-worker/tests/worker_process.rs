use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn unavailable_extraction_modes_fail_explicitly_before_static_fallback() {
    for mode in ["browser", "script"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
            .arg("--stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let config = serde_json::json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[{"id":"fixture","name":"Fixture","kind":"selector","identifier":"http://127.0.0.1:9/must-not-fetch","config":{"mode":mode,"item":"article","title":"h2","link":"a"}}]});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&config).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        let events: Vec<scrape_core::WorkerEvent> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(events.iter().any(|event| matches!(event, scrape_core::WorkerEvent::Error { message, retryable:false, .. } if message.contains("extraction is unavailable"))));
        assert!(!events
            .iter()
            .any(|event| matches!(event, scrape_core::WorkerEvent::Job { .. })));
        let done = events
            .iter()
            .find_map(|event| {
                if let scrape_core::WorkerEvent::Done { stats } = event {
                    Some(stats)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(done.sources_failed, 1);
        assert_eq!(done.sources[0].examined, 0);
        assert_eq!(
            done.sources[0].termination,
            scrape_core::protocol::CoverageTermination::Failed
        );
    }
}

#[test]
fn invalid_cli_job_limits_fail_before_provider_requests() {
    for limit in ["0", "10001"] {
        let output = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
            .args([
                "--source",
                "lever:unused-fixture",
                "--max-jobs-per-run",
                limit,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("max_jobs_per_run"));
    }
}

#[test]
fn invalid_filters_fail_before_extraction() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&serde_json::json!({"version": scrape_core::PROTOCOL_VERSION, "sources": [], "filters": {"query": "title LIKE"}})).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid workflow filters"));
    assert!(output.stdout.is_empty());
}

#[test]
fn cli_url_survives_http_extraction_and_ndjson() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut served_page = false;
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    // Poll accept without making the subsequent HTTP read race.
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = [0; 4096];
                    let n = stream.read(&mut buffer).unwrap();
                    let request = String::from_utf8_lossy(&buffer[..n]);
                    let body = if request.starts_with("GET /robots.txt ") {
                        "User-agent: *\nAllow: /".to_string()
                    } else {
                        assert!(
                            request.starts_with("GET /careers?team=engineering "),
                            "{request}"
                        );
                        served_page = true;
                        r#"<script type="application/ld+json">[
                            {"@type":"JobPosting","title":"Fixture Engineer","identifier":"42","url":"/detail?gh_jid=42","hiringOrganization":{"name":"Fixture Co"}},
                            {"@type":"JobPosting","title":"Unsafe","url":"javascript:alert(1)"},
                            {"@type":"JobPosting","title":"Missing destination"}
                        ]</script>"#.to_string()
                    };
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                    if served_page {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(e) => panic!("{e}"),
            }
        }
        assert!(served_page);
    });
    let output = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .args([
            "--source",
            &format!("jsonld:http://{address}/careers?team=engineering"),
        ])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<scrape_core::WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    match &events[0] {
        scrape_core::WorkerEvent::Job {
            title,
            external_id,
            url,
            ..
        } => {
            assert_eq!(title, "Fixture Engineer");
            assert_eq!(external_id, "42");
            assert_eq!(url, &format!("http://{address}/detail?gh_jid=42"));
        }
        _ => panic!("Expected posting"),
    }
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, scrape_core::WorkerEvent::Job { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                scrape_core::WorkerEvent::Error {
                    retryable: false,
                    ..
                }
            ))
            .count(),
        2
    );
    assert!(matches!(
        events.last(),
        Some(scrape_core::WorkerEvent::Done { .. })
    ));
}

#[test]
fn source_specific_invalid_filters_fail_even_when_global_filters_are_valid() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&serde_json::to_vec(&serde_json::json!({"version": scrape_core::PROTOCOL_VERSION, "sources": [], "filters_by_source": {"workflow": {"query": "title LIKE"}}})).unwrap()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid workflow filters"));
    assert!(output.stdout.is_empty());
}

#[test]
fn invalid_enrichment_limits_and_incompatible_protocol_fail_before_extraction() {
    for config in [
        serde_json::json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[],"limits":{"enrichment":{"max_requests":101}}}),
        serde_json::json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[],"limits":{"enrichment":{"timeout_ms":0}}}),
        serde_json::json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[],"limits":{"enrichment":{"timeout_ms":45001}}}),
        serde_json::json!({"version":scrape_core::PROTOCOL_VERSION-1,"sources":[]}),
    ] {
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
            .arg("--stdin")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&config).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(
            message.contains("limits.enrichment")
                || message.contains("Unsupported protocol version"),
            "{message}"
        );
    }
}

#[test]
fn extraction_limits_and_protocol_are_validated_before_any_source_work() {
    let version = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--protocol-version")
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        scrape_core::PROTOCOL_VERSION.to_string()
    );
    for (key, value) in [
        ("max_pages_per_source", 0),
        ("max_pages_per_source", 101),
        ("navigation_timeout_s", 4),
        ("navigation_timeout_s", 121),
        ("max_source_duration_s", 9),
        ("max_source_duration_s", 601),
        ("max_workflow_duration_s", 29),
        ("max_workflow_duration_s", 1801),
        ("max_browser_processes", 2),
        ("max_browser_contexts", 3),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
            .arg("--stdin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let config = serde_json::json!({"version":scrape_core::PROTOCOL_VERSION,"sources":[],"limits":{key:value}});
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&config).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success(), "accepted {key}={value}");
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(key),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"version":6,"sources":[]}"#)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Unsupported protocol version"));
}
