use base64::Engine;
use scrape_core::browser::{
    chromium_proxy_args, resolve_browser_runtime, CdpWsClient, ResolvedBrowserRuntime,
};
use scrape_core::protocol::HttpProxyConfig;
use scrape_worker::browser::cdp::BrowserPool;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(5);
const USERNAME: &str = "fictional-proxy-user";
const PASSWORD: &str = "fictional-proxy-password";

fn proxy(server: String) -> HttpProxyConfig {
    HttpProxyConfig {
        server,
        username: Some(USERNAME.into()),
        password: Some(PASSWORD.into()),
    }
}

fn read_headers(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
        assert!(bytes.len() < 32768);
    }
    String::from_utf8(bytes).unwrap()
}

fn accept_ws(stream: &mut TcpStream) {
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    stream.set_write_timeout(Some(TIMEOUT)).unwrap();
    read_headers(stream);
    // The in-repo CDP client needs the upgrade status; no browser connects to
    // this deterministic transport fixture.
    stream.write_all(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n").unwrap();
}

fn read_message(stream: &mut TcpStream) -> Value {
    let mut header = [0; 2];
    stream.read_exact(&mut header).unwrap();
    let mut len = (header[1] & 0x7f) as usize;
    if len == 126 {
        let mut ext = [0; 2];
        stream.read_exact(&mut ext).unwrap();
        len = u16::from_be_bytes(ext) as usize;
    } else if len == 127 {
        let mut ext = [0; 8];
        stream.read_exact(&mut ext).unwrap();
        len = u64::from_be_bytes(ext) as usize;
    }
    assert!(len < 65536);
    let mut mask = [0; 4];
    if header[1] & 0x80 != 0 {
        stream.read_exact(&mut mask).unwrap();
    }
    let mut bytes = vec![0; len];
    stream.read_exact(&mut bytes).unwrap();
    if header[1] & 0x80 != 0 {
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }
    serde_json::from_slice(&bytes).unwrap()
}

fn send_message(stream: &mut TcpStream, message: Value) {
    let bytes = serde_json::to_vec(&message).unwrap();
    let mut frame = vec![0x81];
    if bytes.len() < 126 {
        frame.push(bytes.len() as u8);
    } else {
        frame.push(126);
        frame.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    }
    frame.extend(bytes);
    stream.write_all(&frame).unwrap();
}

fn reply(stream: &mut TcpStream, command: &Value) {
    send_message(stream, json!({"id": command["id"], "result": {}}));
}

fn mock_client(
    run: impl FnOnce(TcpStream) + Send + 'static,
) -> (CdpWsClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        accept_ws(&mut stream);
        run(stream);
    });
    let client =
        CdpWsClient::connect(&format!("ws://{address}/devtools/browser/fixture"), TIMEOUT).unwrap();
    (client, server)
}

fn challenge(stream: &mut TcpStream, source: &str, request: &str) -> Value {
    send_message(
        stream,
        json!({"method": "Fetch.authRequired", "sessionId": "page-session",
        "params": {"requestId": request, "authChallenge": {"source": source}}}),
    );
    let answer = read_message(stream);
    assert_eq!(answer["method"], "Fetch.continueWithAuth");
    assert_eq!(answer["sessionId"], "page-session");
    assert_eq!(answer["params"]["requestId"], request);
    answer
}

#[test]
fn cdp_proxy_auth_preserves_events_and_never_sends_proxy_credentials_to_server() {
    let unrelated = json!({"method": "Page.loadEventFired", "sessionId": "page-session", "params": {"timestamp": 7}});
    let retained = unrelated.clone();
    let (mut client, server) = mock_client(move |mut stream| {
        let enable = read_message(&mut stream);
        assert_eq!(enable["method"], "Fetch.enable");
        assert_eq!(enable["params"]["handleAuthRequests"], true);
        reply(&mut stream, &enable);
        let navigate = read_message(&mut stream);
        assert_eq!(navigate["method"], "Page.navigate");
        send_message(&mut stream, unrelated);
        let server_auth = challenge(&mut stream, "Server", "origin-request");
        assert_eq!(
            server_auth["params"]["authChallengeResponse"],
            json!({"response": "Default"})
        );
        assert!(!server_auth.to_string().contains(USERNAME));
        assert!(!server_auth.to_string().contains(PASSWORD));
        reply(&mut stream, &server_auth);
        let proxy_auth = challenge(&mut stream, "Proxy", "proxy-request");
        assert_eq!(
            proxy_auth["params"]["authChallengeResponse"],
            json!({
            "response": "ProvideCredentials", "username": USERNAME, "password": PASSWORD})
        );
        reply(&mut stream, &proxy_auth);
        let rejected = challenge(&mut stream, "Proxy", "proxy-request");
        assert_eq!(
            rejected["params"]["authChallengeResponse"],
            json!({"response": "CancelAuth"})
        );
        reply(&mut stream, &rejected);
        send_message(
            &mut stream,
            json!({"method": "Fetch.requestPaused", "sessionId": "page-session",
            "params": {"requestId": "resource-request"}}),
        );
        let resumed = read_message(&mut stream);
        assert_eq!(resumed["method"], "Fetch.continueRequest");
        reply(&mut stream, &resumed);
        // An event in another target must not receive this target's credentials.
        send_message(
            &mut stream,
            json!({"method": "Fetch.authRequired", "sessionId": "other-session",
            "params": {"requestId": "unrelated", "authChallenge": {"source": "Proxy"}}}),
        );
        reply(&mut stream, &navigate);
    });
    client
        .enable_proxy_auth(
            &proxy("http://127.0.0.1:8080".into()),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap();
    client
        .call(
            "Page.navigate",
            json!({"url": "http://fixture.invalid/jobs"}),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap();
    assert_eq!(client.take_event(), Some(retained));
    assert_eq!(client.take_event().unwrap()["sessionId"], "other-session");
    assert!(client.take_event().is_none());
    server.join().unwrap();
}

#[test]
fn cdp_proxy_auth_failures_never_echo_credentials() {
    let (mut client, server) = mock_client(|mut stream| {
        let enable = read_message(&mut stream);
        reply(&mut stream, &enable);
        let _navigate = read_message(&mut stream);
        let answer = challenge(&mut stream, "Proxy", "request");
        send_message(
            &mut stream,
            json!({"id": answer["id"], "error": {
            "message": format!("rejected {USERNAME} {PASSWORD}")}}),
        );
    });
    client
        .enable_proxy_auth(
            &proxy("http://127.0.0.1:8080".into()),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap();
    let error = client
        .call(
            "Page.navigate",
            json!({"url": "http://fixture.invalid/jobs"}),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap_err();
    assert_eq!(error, "Chromium proxy authentication command failed");
    assert!(!error.contains(USERNAME));
    assert!(!error.contains(PASSWORD));
    server.join().unwrap();
}

#[test]
fn cdp_proxy_enable_failure_is_secret_free() {
    let (mut client, server) = mock_client(|mut stream| {
        let enable = read_message(&mut stream);
        send_message(
            &mut stream,
            json!({"id": enable["id"], "error": {
            "message": format!("rejected {USERNAME} {PASSWORD}")}}),
        );
    });
    let error = client
        .enable_proxy_auth(
            &proxy("http://127.0.0.1:8080".into()),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap_err();
    assert_eq!(error, "Failed to enable Chromium proxy authentication");
    server.join().unwrap();
}

#[test]
fn cdp_navigation_errors_cannot_echo_proxy_credentials() {
    let (mut client, server) = mock_client(|mut stream| {
        let enable = read_message(&mut stream);
        reply(&mut stream, &enable);
        let navigate = read_message(&mut stream);
        send_message(
            &mut stream,
            json!({"id": navigate["id"], "error": {
            "message": format!("rejected {USERNAME} {PASSWORD}")}}),
        );
    });
    client
        .enable_proxy_auth(
            &proxy("http://127.0.0.1:8080".into()),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap();
    let error = client
        .call(
            "Page.navigate",
            json!({"url": "http://fixture.invalid/jobs"}),
            Some("page-session"),
            TIMEOUT,
        )
        .unwrap_err();
    assert_eq!(
        error,
        "CDP Page.navigate failed (proxy authentication active)"
    );
    assert!(!error.contains(USERNAME));
    assert!(!error.contains(PASSWORD));
    server.join().unwrap();
}

#[test]
fn chromium_arguments_are_endpoint_only_and_external_cdp_is_rejected() {
    let config = proxy("http://127.0.0.1:8080".into());
    let args = chromium_proxy_args(&config).unwrap();
    assert!(args
        .iter()
        .any(|arg| arg == "--proxy-server=http://127.0.0.1:8080/"
            || arg == "--proxy-server=http://127.0.0.1:8080"));
    assert!(args
        .iter()
        .any(|arg| arg == "--proxy-bypass-list=<-loopback>"));
    assert!(!args.join(" ").contains(USERNAME));
    assert!(!args.join(" ").contains(PASSWORD));
    assert!(BrowserPool::with_ws_url(1, "ws://127.0.0.1:1".into())
        .with_proxy(Some(&config))
        .is_err());
}

struct BrowserChild(Child);
impl Drop for BrowserChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct ProxyFixture {
    address: std::net::SocketAddr,
    authenticated: Arc<AtomicUsize>,
    challenged: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl ProxyFixture {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let authenticated = Arc::new(AtomicUsize::new(0));
        let challenged = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (seen_auth, seen_challenge, stopping) =
            (authenticated.clone(), challenged.clone(), stop.clone());
        let thread = thread::spawn(move || {
            let mut connections = Vec::new();
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let seen_auth = seen_auth.clone();
                        let seen_challenge = seen_challenge.clone();
                        connections.push(thread::spawn(move || {
                            stream.set_read_timeout(Some(TIMEOUT)).unwrap();
                            // Chromium also opens speculative connections which
                            // it may close without issuing a request.
                            let mut bytes = Vec::new();
                            loop {
                                let mut byte = [0];
                                if stream.read_exact(&mut byte).is_err() { return; }
                                bytes.push(byte[0]);
                                if bytes.ends_with(b"\r\n\r\n") { break; }
                                if bytes.len() >= 32768 { return; }
                            }
                            let request = String::from_utf8(bytes).unwrap();
                            let expected = format!("proxy-authorization: basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{USERNAME}:{PASSWORD}")));
                            if !request.to_ascii_lowercase().contains(&expected.to_ascii_lowercase()) {
                                seen_challenge.fetch_add(1, Ordering::SeqCst);
                                let _ = stream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"fixture\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                            } else if request.starts_with("GET http://proxy-fixture.invalid/jobs ") {
                                seen_auth.fetch_add(1, Ordering::SeqCst);
                                let body = "<!doctype html><title>Proxy fixture</title><h1 id=job>Fictional Proxy Engineer</h1>";
                                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                                let _ = stream.write_all(response.as_bytes());
                            } else {
                                let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                            }
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("loopback proxy accept failed: {e}"),
                }
            }
            for connection in connections {
                connection.join().unwrap();
            }
        });
        Self {
            address,
            authenticated,
            challenged,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for ProxyFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[test]
#[ignore = "requires installed Chromium; run explicitly with --ignored --exact"]
fn chromium_authenticated_loopback_proxy() {
    let executable =
        match resolve_browser_runtime().expect("install Chromium or set SCRAPE_BROWSER_PATH") {
            ResolvedBrowserRuntime::Executable { path, .. } => path,
            ResolvedBrowserRuntime::WebSocket(_) => {
                panic!("fixture requires an app-launched browser, unset SCRAPE_CDP_WS_URL")
            }
        };
    let fixture = ProxyFixture::start();
    let config = proxy(format!("http://{}", fixture.address));
    let profile = tempfile::tempdir().unwrap();
    let mut child = BrowserChild(
        Command::new(executable)
            .args(chromium_proxy_args(&config).unwrap())
            .args([
                "--headless=new",
                "--remote-debugging-port=0",
                "--no-first-run",
                "--disable-background-networking",
                "--disable-extensions",
                "--disable-sync",
            ])
            .arg(format!("--user-data-dir={}", profile.path().display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stderr = child.0.stderr.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if let Some((_, endpoint)) = line.split_once("DevTools listening on ") {
                let _ = tx.send(endpoint.trim().to_string());
                break;
            }
        }
    });
    let endpoint = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let mut client = CdpWsClient::connect(&endpoint, TIMEOUT).unwrap();
    let target = client
        .call(
            "Target.createTarget",
            json!({"url": "about:blank"}),
            None,
            TIMEOUT,
        )
        .unwrap();
    let session = client
        .call(
            "Target.attachToTarget",
            json!({"targetId": target["targetId"], "flatten": true}),
            None,
            TIMEOUT,
        )
        .unwrap();
    let sid = session["sessionId"].as_str().unwrap();
    client
        .enable_proxy_auth(&config, Some(sid), TIMEOUT)
        .unwrap();
    let navigation = client
        .call(
            "Page.navigate",
            json!({"url": "http://proxy-fixture.invalid/jobs"}),
            Some(sid),
            TIMEOUT,
        )
        .unwrap();
    assert!(
        navigation.get("errorText").is_none(),
        "proxy navigation failed"
    );
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let result = client.call("Runtime.evaluate", json!({"expression": "document.querySelector('#job')?.textContent", "returnByValue": true}), Some(sid), TIMEOUT).unwrap();
        if result.pointer("/result/value").and_then(Value::as_str)
            == Some("Fictional Proxy Engineer")
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "authenticated proxy fixture never rendered"
        );
        thread::sleep(Duration::from_millis(20));
    }
    assert!(fixture.challenged.load(Ordering::SeqCst) > 0);
    assert_eq!(fixture.authenticated.load(Ordering::SeqCst), 1);
    drop(fixture);
    // An unreachable configured proxy must not silently fall back to a direct
    // connection, including Chromium's normally bypassed loopback addresses.
    let origin = TcpListener::bind("127.0.0.1:0").unwrap();
    origin.set_nonblocking(true).unwrap();
    let address = origin.local_addr().unwrap();
    let stopped = Arc::new(AtomicBool::new(false));
    let direct_requests = Arc::new(AtomicUsize::new(0));
    let (stop_origin, seen_direct) = (stopped.clone(), direct_requests.clone());
    let origin_thread = thread::spawn(move || {
        while !stop_origin.load(Ordering::SeqCst) {
            match origin.accept() {
                Ok((mut stream, _)) => {
                    seen_direct.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("origin fixture accept failed: {e}"),
            }
        }
    });
    let unavailable = client.call(
        "Page.navigate",
        json!({"url": format!("http://{address}/must-not-connect-directly")}),
        Some(sid),
        TIMEOUT,
    );
    stopped.store(true, Ordering::SeqCst);
    origin_thread.join().unwrap();
    let unavailable = unavailable.unwrap();
    assert!(
        unavailable.get("errorText").is_some(),
        "unavailable proxy must fail navigation"
    );
    assert_eq!(direct_requests.load(Ordering::SeqCst), 0);
}
