use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
        .write_all(br#"{"version":2,"sources":[],"filters":{"query":"title LIKE"}}"#)
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
    child.stdin.take().unwrap().write_all(br#"{"version":2,"sources":[],"filters_by_source":{"workflow":{"query":"title LIKE"}}}"#).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Invalid workflow filters"));
    assert!(output.stdout.is_empty());
}
