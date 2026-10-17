use base64::Engine;
use scrape_core::protocol::{CoverageTermination, ScanOutcome};
use scrape_core::WorkerEvent;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }
    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

fn ws_accept_handshake(stream: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let header_end = loop {
        let n = stream.read(&mut tmp).unwrap();
        assert!(n > 0, "unexpected EOF during WS handshake");
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
    };
    let req = String::from_utf8_lossy(&buf[..header_end]);
    let key = req
        .lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case("sec-websocket-key")
                .then(|| v.trim().to_string())
        })
        .expect("missing Sec-WebSocket-Key");
    let mut combined = key.into_bytes();
    combined.extend_from_slice(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let accept = base64::engine::general_purpose::STANDARD.encode(sha1_digest(&combined));
    let resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream.write_all(resp.as_bytes()).unwrap();
    buf.split_off(header_end)
}

fn read_exact_buffered(
    stream: &mut TcpStream,
    leftover: &mut Vec<u8>,
    out: &mut [u8],
) -> std::io::Result<()> {
    let take = leftover.len().min(out.len());
    if take > 0 {
        out[..take].copy_from_slice(&leftover[..take]);
        leftover.drain(..take);
    }
    if take < out.len() {
        stream.read_exact(&mut out[take..])?;
    }
    Ok(())
}

fn ws_read_text(stream: &mut TcpStream, leftover: &mut Vec<u8>) -> Option<String> {
    let mut header = [0u8; 2];
    if read_exact_buffered(stream, leftover, &mut header).is_err() {
        return None;
    }
    let opcode = header[0] & 0x0F;
    if opcode == 0x8 {
        return None;
    }
    let masked = (header[1] & 0x80) != 0;
    let mut len = (header[1] & 0x7F) as usize;
    if len == 126 {
        let mut ext = [0u8; 2];
        read_exact_buffered(stream, leftover, &mut ext).ok()?;
        len = u16::from_be_bytes(ext) as usize;
    } else if len == 127 {
        let mut ext = [0u8; 8];
        read_exact_buffered(stream, leftover, &mut ext).ok()?;
        len = u64::from_be_bytes(ext) as usize;
    }
    let mut mask = [0u8; 4];
    if masked {
        read_exact_buffered(stream, leftover, &mut mask).ok()?;
    }
    let mut payload = vec![0u8; len];
    read_exact_buffered(stream, leftover, &mut payload).ok()?;
    if masked {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i & 3];
        }
    }
    String::from_utf8(payload).ok()
}

fn ws_write_text(stream: &mut TcpStream, text: &str) {
    let bytes = text.as_bytes();
    let mut frame = Vec::with_capacity(10 + bytes.len());
    frame.push(0x81); // FIN + Text
    if bytes.len() <= 125 {
        frame.push(bytes.len() as u8);
    } else if bytes.len() <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(bytes);
    let _ = stream.write_all(&frame);
}

fn accept_with_timeout(listener: &TcpListener, timeout: Duration) -> Option<TcpStream> {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                return Some(stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    }
    None
}

#[derive(Default)]
struct MockCdpLog {
    methods: Vec<String>,
    blocked_urls: Vec<String>,
    stealth_scripts: Vec<String>,
    user_agents: Vec<String>,
    cookies_set: Vec<Value>,
    max_concurrent_contexts: usize,
    active_contexts: HashSet<String>,
    active_targets: HashSet<String>,
}

#[test]
fn browser_cdp_blocks_resources_injects_stealth_paginates_and_closes_targets() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let cdp_log = Arc::new(Mutex::new(MockCdpLog::default()));
    let cdp_log_server = cdp_log.clone();

    let server = thread::spawn(move || {
        let Some(mut stream) = accept_with_timeout(&listener, Duration::from_secs(2)) else {
            return;
        };
        let mut leftover = ws_accept_handshake(&mut stream);
        let mut current_url = String::new();
        let mut eval_polls = 0usize;
        let mut load_more_clicked = false;
        let mut seq = 0usize;

        while let Some(msg) = ws_read_text(&mut stream, &mut leftover) {
            let req: Value = serde_json::from_str(&msg).unwrap();
            let id = req["id"].as_u64().unwrap();
            let method = req["method"].as_str().unwrap().to_string();
            let params = &req["params"];
            let mut log = cdp_log_server.lock().unwrap();
            log.methods.push(method.clone());

            let result = match method.as_str() {
                "Target.createBrowserContext" => {
                    seq += 1;
                    let ctx_id = format!("ctx-{seq}");
                    log.active_contexts.insert(ctx_id.clone());
                    log.max_concurrent_contexts =
                        log.max_concurrent_contexts.max(log.active_contexts.len());
                    json!({ "browserContextId": ctx_id })
                }
                "Target.disposeBrowserContext" => {
                    if let Some(ctx_id) = params["browserContextId"].as_str() {
                        log.active_contexts.remove(ctx_id);
                    }
                    json!({})
                }
                "Target.createTarget" => {
                    seq += 1;
                    let target_id = format!("target-{seq}");
                    log.active_targets.insert(target_id.clone());
                    json!({ "targetId": target_id })
                }
                "Target.attachToTarget" => {
                    seq += 1;
                    json!({ "sessionId": format!("session-{seq}") })
                }
                "Target.closeTarget" => {
                    if let Some(target_id) = params["targetId"].as_str() {
                        log.active_targets.remove(target_id);
                    }
                    json!({ "success": true })
                }
                "Network.setBlockedURLs" => {
                    if let Some(urls) = params["urls"].as_array() {
                        for u in urls.iter().filter_map(Value::as_str) {
                            log.blocked_urls.push(u.to_string());
                        }
                    }
                    json!({})
                }
                "Emulation.setUserAgentOverride" => {
                    if let Some(ua) = params["userAgent"].as_str() {
                        log.user_agents.push(ua.to_string());
                    }
                    json!({})
                }
                "Page.addScriptToEvaluateOnNewDocument" => {
                    if let Some(src) = params["source"].as_str() {
                        log.stealth_scripts.push(src.to_string());
                    }
                    json!({ "identifier": "1" })
                }
                "Page.navigate" => {
                    current_url = params["url"].as_str().unwrap_or_default().to_string();
                    eval_polls = 0;
                    load_more_clicked = false;
                    json!({ "frameId": "frame-1" })
                }
                "Runtime.evaluate" => {
                    let expr = params["expression"].as_str().unwrap_or_default();
                    if expr.contains(".click()") {
                        load_more_clicked = true;
                        json!({ "result": { "type": "boolean", "value": true } })
                    } else {
                        eval_polls += 1;
                        let html = if current_url.ends_with("/jobs?page=2") {
                            r#"<html><body><main id="ready"><article data-id="job-3"><h2 class="title">Principal Systems Engineer</h2><a href="https://fixture.test/jobs/3">Apply</a></article></main></body></html>"#
                        } else if eval_polls == 1 {
                            // First poll: skeleton before #ready exists
                            r#"<html><body><div class="loading">Loading...</div></body></html>"#
                        } else if !load_more_clicked {
                            r#"<html><body><main id="ready"><article data-id="job-1"><h2 class="title">Staff Rust Engineer</h2><a href="https://fixture.test/jobs/1">Apply</a></article><button class="load-more">More</button></main></body></html>"#
                        } else {
                            r#"<html><body><main id="ready"><article data-id="job-1"><h2 class="title">Staff Rust Engineer</h2><a href="https://fixture.test/jobs/1">Apply</a></article><article data-id="job-2"><h2 class="title">Distributed Data Engineer</h2><a href="https://fixture.test/jobs/2">Apply</a></article><a class="next" href="https://fixture.test/jobs?page=2">Next</a></main></body></html>"#
                        };
                        let snapshot = json!({
                            "readyState": "complete",
                            "href": current_url,
                            "title": "Careers",
                            "hasPassword": false,
                            "html": html
                        });
                        json!({ "result": { "type": "string", "value": snapshot.to_string() } })
                    }
                }
                "Browser.close" => {
                    drop(log);
                    ws_write_text(&mut stream, &json!({ "id": id, "result": {} }).to_string());
                    break;
                }
                _ => json!({}),
            };
            drop(log);
            ws_write_text(
                &mut stream,
                &json!({ "id": id, "result": result }).to_string(),
            );
        }
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .env(
            "SCRAPE_CDP_WS_URL",
            format!("ws://{addr}/devtools/browser/mock"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "browser-src",
            "name": "Fixture Corp",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "browser",
                "item": "//article",
                "id": ".",
                "id_attribute": "data-id",
                "title": ".title",
                "link": "a",
                "wait_for": "#ready",
                "load_more": "button.load-more",
                "next_page": "a.next@href",
                "max_pages": 5
            }
        }]
    });

    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    server.join().unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    let titles: Vec<String> = events
        .iter()
        .filter_map(|e| match e {
            WorkerEvent::Job { title, .. } => Some(title.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        titles,
        vec![
            "Staff Rust Engineer",
            "Distributed Data Engineer",
            "Principal Systems Engineer"
        ]
    );

    let log = cdp_log.lock().unwrap();
    let nav_idx = log
        .methods
        .iter()
        .position(|m| m == "Page.navigate")
        .expect("Page.navigate must be called");
    let block_idx = log
        .methods
        .iter()
        .position(|m| m == "Network.setBlockedURLs")
        .expect("Network.setBlockedURLs must be called");
    let stealth_idx = log
        .methods
        .iter()
        .position(|m| m == "Page.addScriptToEvaluateOnNewDocument")
        .expect("stealth script must be injected");
    assert!(
        block_idx < nav_idx,
        "resource blocking must precede navigation"
    );
    assert!(
        stealth_idx < nav_idx,
        "stealth injection must precede navigation"
    );
    assert!(log.blocked_urls.iter().any(|u| u.contains(".png")));
    assert!(log.blocked_urls.iter().any(|u| u.contains(".woff2")));
    assert!(log.blocked_urls.iter().any(|u| u.contains(".css")));
    assert!(log.stealth_scripts.iter().any(|s| s.contains("webdriver")));
    assert!(log.max_concurrent_contexts <= 2);
    assert!(log.active_targets.is_empty(), "all targets must be closed");
    assert!(
        log.active_contexts.is_empty(),
        "all contexts must be disposed"
    );
}

#[test]
fn browser_cdp_detects_login_redirect_and_injects_scoped_session_cookies() {
    let temp_dir = tempfile::tempdir().unwrap();
    let session_path = temp_dir.path().join("corp-auth.json");
    std::fs::write(
        &session_path,
        serde_json::to_vec(&json!({
            "id": "corp-auth",
            "domain": "fixture.test",
            "saved_at": "2026-10-17T00:00:00Z",
            "cookies": [{
                "name": "corp_session",
                "value": "secret-cookie-token",
                "domain": ".fixture.test",
                "path": "/",
                "secure": true,
                "http_only": true
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    // 1. Authenticated run with valid session cookie -> injects cookie and extracts job.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let cdp_log = Arc::new(Mutex::new(MockCdpLog::default()));
    let cdp_log_server = cdp_log.clone();

    let server = thread::spawn(move || {
        let Some(mut stream) = accept_with_timeout(&listener, Duration::from_secs(2)) else {
            return;
        };
        let mut leftover = ws_accept_handshake(&mut stream);
        let mut has_auth_cookie = false;

        while let Some(msg) = ws_read_text(&mut stream, &mut leftover) {
            let req: Value = serde_json::from_str(&msg).unwrap();
            let id = req["id"].as_u64().unwrap();
            let method = req["method"].as_str().unwrap();
            let params = &req["params"];

            let result = match method {
                "Target.createBrowserContext" => json!({ "browserContextId": "ctx-1" }),
                "Target.createTarget" => json!({ "targetId": "target-1" }),
                "Target.attachToTarget" => json!({ "sessionId": "session-1" }),
                "Target.closeTarget" => json!({ "success": true }),
                "Network.setCookies" => {
                    let cookies = params["cookies"].as_array().cloned().unwrap_or_default();
                    has_auth_cookie = cookies.iter().any(|c| {
                        c["name"] == "corp_session" && c["value"] == "secret-cookie-token"
                    });
                    cdp_log_server.lock().unwrap().cookies_set = cookies;
                    json!({})
                }
                "Page.navigate" => json!({ "frameId": "f1" }),
                "Runtime.evaluate" => {
                    let snapshot = if has_auth_cookie {
                        json!({
                            "readyState": "complete",
                            "href": "https://jobs.fixture.test/board",
                            "title": "Internal Board",
                            "hasPassword": false,
                            "html": r#"<html><body><article><h2 class="title">Authenticated Engineer</h2><a href="https://jobs.fixture.test/jobs/99">Apply</a></article></body></html>"#
                        })
                    } else {
                        json!({
                            "readyState": "complete",
                            "href": "https://jobs.fixture.test/login",
                            "title": "Sign In",
                            "hasPassword": true,
                            "html": r#"<html><body><form><input type="password" name="pw"></form></body></html>"#
                        })
                    };
                    json!({ "result": { "type": "string", "value": snapshot.to_string() } })
                }
                "Browser.close" => {
                    ws_write_text(&mut stream, &json!({ "id": id, "result": {} }).to_string());
                    break;
                }
                _ => json!({}),
            };
            ws_write_text(
                &mut stream,
                &json!({ "id": id, "result": result }).to_string(),
            );
        }
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .env(
            "SCRAPE_CDP_WS_URL",
            format!("ws://{addr}/devtools/browser/mock"),
        )
        .env("SCRAPE_SESSION_DIR", temp_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "auth-src",
            "name": "Fixture Internal",
            "kind": "selector",
            "identifier": "https://jobs.fixture.test/board",
            "config": {
                "mode": "browser",
                "session_id": "corp-auth",
                "item": "article",
                "title": ".title",
                "link": "a"
            }
        }]
    });
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    server.join().unwrap();
    assert!(output.status.success());
    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert!(events.iter().any(|e| matches!(
        e,
        WorkerEvent::Job { title, .. } if title == "Authenticated Engineer"
    )));
    assert_eq!(cdp_log.lock().unwrap().cookies_set.len(), 1);

    // 2. Login redirect / missing session -> NeedsLogin termination without opening headed window.
    let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr2 = listener2.local_addr().unwrap();
    let server2 = thread::spawn(move || {
        let Some(mut stream) = accept_with_timeout(&listener2, Duration::from_secs(2)) else {
            return;
        };
        let mut leftover = ws_accept_handshake(&mut stream);
        while let Some(msg) = ws_read_text(&mut stream, &mut leftover) {
            let req: Value = serde_json::from_str(&msg).unwrap();
            let id = req["id"].as_u64().unwrap();
            let method = req["method"].as_str().unwrap();
            let result = match method {
                "Target.createBrowserContext" => json!({ "browserContextId": "ctx-1" }),
                "Target.createTarget" => json!({ "targetId": "target-1" }),
                "Target.attachToTarget" => json!({ "sessionId": "session-1" }),
                "Target.closeTarget" => json!({ "success": true }),
                "Page.navigate" => json!({ "frameId": "f1" }),
                "Runtime.evaluate" => {
                    let snapshot = json!({
                        "readyState": "complete",
                        "href": "https://jobs.fixture.test/login?redirect=/board",
                        "title": "Sign In",
                        "hasPassword": true,
                        "html": r#"<html><body><form><input type="password"></form></body></html>"#
                    });
                    json!({ "result": { "type": "string", "value": snapshot.to_string() } })
                }
                "Browser.close" => {
                    ws_write_text(&mut stream, &json!({ "id": id, "result": {} }).to_string());
                    break;
                }
                _ => json!({}),
            };
            ws_write_text(
                &mut stream,
                &json!({ "id": id, "result": result }).to_string(),
            );
        }
    });

    let mut child2 = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .env(
            "SCRAPE_CDP_WS_URL",
            format!("ws://{addr2}/devtools/browser/mock"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let config2 = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "expired-src",
            "name": "Fixture Internal",
            "kind": "selector",
            "identifier": "https://jobs.fixture.test/board",
            "config": {
                "mode": "browser",
                "item": "article",
                "title": ".title",
                "link": "a"
            }
        }]
    });
    child2
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config2).unwrap())
        .unwrap();
    let output2 = child2.wait_with_output().unwrap();
    server2.join().unwrap();
    let events2: Vec<WorkerEvent> = String::from_utf8(output2.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let done = events2
        .iter()
        .find_map(|e| match e {
            WorkerEvent::Done { stats } => Some(stats),
            _ => None,
        })
        .unwrap();
    assert_eq!(done.sources[0].termination, CoverageTermination::NeedsLogin);
    assert_eq!(done.sources[0].outcome, Some(ScanOutcome::Failed));
}

#[test]
fn browser_cdp_enforces_page_limit_and_navigation_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let server = thread::spawn(move || {
        let Some(mut stream) = accept_with_timeout(&listener, Duration::from_secs(2)) else {
            return;
        };
        let mut leftover = ws_accept_handshake(&mut stream);
        let mut clicks = 0usize;

        while let Some(msg) = ws_read_text(&mut stream, &mut leftover) {
            let req: Value = serde_json::from_str(&msg).unwrap();
            let id = req["id"].as_u64().unwrap();
            let method = req["method"].as_str().unwrap();
            let params = &req["params"];

            let result = match method {
                "Target.createBrowserContext" => json!({ "browserContextId": "ctx-1" }),
                "Target.createTarget" => json!({ "targetId": "target-1" }),
                "Target.attachToTarget" => json!({ "sessionId": "session-1" }),
                "Target.closeTarget" => json!({ "success": true }),
                "Page.navigate" => json!({ "frameId": "f1" }),
                "Runtime.evaluate" => {
                    let expr = params["expression"].as_str().unwrap_or_default();
                    if expr.contains(".click()") {
                        clicks += 1;
                        json!({ "result": { "type": "boolean", "value": true } })
                    } else {
                        let mut items = String::new();
                        for idx in 1..=(clicks + 1) {
                            items.push_str(&format!(
                                r#"<article><h2 class="title">Engineer {idx}</h2><a href="https://fixture.test/jobs/{idx}">Apply</a></article>"#
                            ));
                        }
                        let html = format!(
                            r#"<html><body>{items}<button class="load-more">More</button></body></html>"#
                        );
                        let snapshot = json!({
                            "readyState": "complete",
                            "href": "https://fixture.test/jobs",
                            "title": "Jobs",
                            "hasPassword": false,
                            "html": html
                        });
                        json!({ "result": { "type": "string", "value": snapshot.to_string() } })
                    }
                }
                "Browser.close" => {
                    ws_write_text(&mut stream, &json!({ "id": id, "result": {} }).to_string());
                    break;
                }
                _ => json!({}),
            };
            ws_write_text(
                &mut stream,
                &json!({ "id": id, "result": result }).to_string(),
            );
        }
        assert_eq!(
            clicks, 1,
            "with max_pages=2 (1 initial + 1 load_more), must click at most once"
        );
    });

    let mut child = Command::new(env!("CARGO_BIN_EXE_scrape-worker"))
        .arg("--stdin")
        .env(
            "SCRAPE_CDP_WS_URL",
            format!("ws://{addr}/devtools/browser/mock"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let config = json!({
        "version": scrape_core::PROTOCOL_VERSION,
        "sources": [{
            "id": "bounded-src",
            "name": "Bounded Fixture",
            "kind": "selector",
            "identifier": "https://fixture.test/jobs",
            "config": {
                "mode": "browser",
                "item": "article",
                "title": ".title",
                "link": "a",
                "load_more": "button.load-more",
                "max_pages": 2
            }
        }]
    });
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&config).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    server.join().unwrap();

    let events: Vec<WorkerEvent> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let done = events
        .iter()
        .find_map(|e| match e {
            WorkerEvent::Done { stats } => Some(stats),
            _ => None,
        })
        .unwrap();
    assert_eq!(done.sources[0].accepted, 2);
    assert_eq!(done.sources[0].termination, CoverageTermination::PageLimit);
    assert_eq!(done.sources[0].outcome, Some(ScanOutcome::Partial));

    // Also verify navigation_timeout termination when wait_for (#never-appears) never resolves:
    let listener2 = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr2 = listener2.local_addr().unwrap();
    let server2 = thread::spawn(move || {
        let Some(mut stream) = accept_with_timeout(&listener2, Duration::from_secs(2)) else {
            return;
        };
        let mut leftover = ws_accept_handshake(&mut stream);
        while let Some(msg) = ws_read_text(&mut stream, &mut leftover) {
            let req: Value = serde_json::from_str(&msg).unwrap();
            let id = req["id"].as_u64().unwrap();
            let method = req["method"].as_str().unwrap();
            let result = match method {
                "Target.createBrowserContext" => json!({ "browserContextId": "ctx-timeout" }),
                "Target.createTarget" => json!({ "targetId": "target-timeout" }),
                "Target.attachToTarget" => json!({ "sessionId": "session-timeout" }),
                "Target.closeTarget" => json!({ "success": true }),
                "Page.navigate" => json!({ "frameId": "f1" }),
                "Runtime.evaluate" => {
                    let snapshot = json!({
                        "readyState": "complete",
                        "href": "https://fixture.test/slow",
                        "title": "Loading",
                        "hasPassword": false,
                        "html": "<html><body><div>Still loading...</div></body></html>"
                    });
                    json!({ "result": { "type": "string", "value": snapshot.to_string() } })
                }
                "Browser.close" => {
                    ws_write_text(&mut stream, &json!({ "id": id, "result": {} }).to_string());
                    break;
                }
                _ => json!({}),
            };
            ws_write_text(
                &mut stream,
                &json!({ "id": id, "result": result }).to_string(),
            );
        }
    });

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let err = rt.block_on(async {
        let pool = scrape_worker::browser::BrowserPool::with_ws_url(
            2,
            format!("ws://{addr2}/devtools/browser/mock"),
        );
        let client = scrape_worker::HttpClient::new(1, 0);
        let source = scrape_core::WorkerSource {
            id: "slow".into(),
            kind: scrape_core::SourceKind::Selector,
            name: "Slow".into(),
            identifier: "https://fixture.test/slow".into(),
            config: json!({
                "mode": "browser",
                "item": "article",
                "title": "h2",
                "link": "a",
                "wait_for": "#never-appears"
            }),
        };
        let static_cfg = scrape_core::static_source::StaticSourceConfig::parse(
            &source.config,
            &scrape_core::WorkerLimits::default(),
        )
        .unwrap();
        let url = url::Url::parse(&source.identifier).unwrap();
        let res = pool
            .fetch_rendered_page(
                &client,
                &url,
                &source,
                &static_cfg,
                &[],
                1,
                Duration::from_millis(120),
                tokio::time::Instant::now() + Duration::from_secs(5),
            )
            .await;
        drop(pool);
        res.err().unwrap()
    });
    server2.join().unwrap();
    assert!(
        matches!(err, scrape_worker::ClientError::TimeLimit(_)),
        "expected TimeLimit error, got: {err:?}"
    );
}
