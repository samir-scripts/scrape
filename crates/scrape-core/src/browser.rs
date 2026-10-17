//! Shared browser runtime discovery, isolated session storage contracts, and RFC 6455 CDP WebSocket transport.
use crate::protocol::BrowserRuntimeStatus;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use url::Url;

pub const MISSING_BROWSER_RUNTIME_MESSAGE: &str = "No compatible Chromium browser found. Install Chrome, Brave, Edge, or Chromium, or configure the browser executable path in Scraper settings. Automatic background downloads are disabled.";

pub const NEEDS_LOGIN_MESSAGE: &str =
    "Session expired or login required; re-authenticate in workflow settings";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBrowserRuntime {
    WebSocket(String),
    Executable {
        path: PathBuf,
        version: Option<String>,
    },
}

fn default_cookie_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrowserCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_cookie_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredBrowserSession {
    pub id: String,
    pub domain: String,
    pub saved_at: String,
    #[serde(default)]
    pub cookies: Vec<BrowserCookie>,
}

impl StoredBrowserSession {
    pub fn validate_session_id(raw: &str) -> Result<String, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.len() > 64 {
            return Err("session_id: expected 1–64 ASCII characters".into());
        }
        if !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err("session_id: only ASCII letters, digits, '-' and '_' are allowed".into());
        }
        Ok(trimmed.to_string())
    }

    pub fn normalize_domain(raw: &str) -> Result<String, String> {
        let trimmed = raw.trim().trim_start_matches('.').to_ascii_lowercase();
        if trimmed.is_empty()
            || trimmed.contains('/')
            || trimmed.contains(':')
            || trimmed.contains('\0')
            || !trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
        {
            return Err("session domain: expected a valid hostname".into());
        }
        Ok(trimmed)
    }

    pub fn domain_matches(session_or_cookie_domain: &str, target_host: &str) -> bool {
        let domain = session_or_cookie_domain
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        let host = target_host
            .trim()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        if domain.is_empty() || host.is_empty() {
            return false;
        }
        host == domain || host.ends_with(&format!(".{domain}"))
    }

    pub fn cookies_for_url(
        &self,
        url: &Url,
        now_epoch_s: f64,
    ) -> Result<Vec<BrowserCookie>, String> {
        let host = url
            .host_str()
            .ok_or_else(|| "Target URL is missing a host".to_string())?;
        if !Self::domain_matches(&self.domain, host) {
            return Err(format!(
                "Browser session '{}' is scoped to domain '{}' and cannot be used for '{host}'",
                self.id, self.domain
            ));
        }
        let mut eligible = Vec::new();
        let mut saw_domain_match = false;
        for cookie in &self.cookies {
            if !Self::domain_matches(&cookie.domain, host) {
                continue;
            }
            saw_domain_match = true;
            if let Some(exp) = cookie.expires {
                if exp > 0.0 && exp <= now_epoch_s {
                    continue;
                }
            }
            eligible.push(cookie.clone());
        }
        if (self.cookies.is_empty() || (saw_domain_match && eligible.is_empty()))
            && !self.id.is_empty()
        {
            return Err(NEEDS_LOGIN_MESSAGE.to_string());
        }
        Ok(eligible)
    }
}

pub fn is_login_wall(
    requested_url: &Url,
    final_href: &str,
    page_title: &str,
    has_password_input: bool,
    has_candidate_items: bool,
) -> bool {
    if has_candidate_items {
        return false;
    }
    if has_password_input {
        return true;
    }
    let req_path = requested_url.path().to_ascii_lowercase();
    let final_lower = final_href.to_ascii_lowercase();
    let login_markers = [
        "/login", "/signin", "/sign-in", "/auth/", "/sso", "/oauth", "login?", "signin?",
    ];
    if login_markers
        .iter()
        .any(|m| final_lower.contains(m) && !req_path.contains(m))
    {
        return true;
    }
    let title_lower = page_title.trim().to_ascii_lowercase();
    matches!(
        title_lower.as_str(),
        "sign in" | "log in" | "login" | "authentication required" | "single sign-on"
    )
}

pub fn candidate_browser_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "macos")]
    {
        paths.extend([
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            PathBuf::from("/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"),
            PathBuf::from("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
        ]);
    }
    #[cfg(target_os = "linux")]
    {
        for bin in [
            "chromium",
            "chromium-browser",
            "google-chrome",
            "google-chrome-stable",
            "brave-browser",
            "microsoft-edge",
        ] {
            for dir in ["/usr/bin", "/usr/local/bin", "/snap/bin"] {
                paths.push(PathBuf::from(dir).join(bin));
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        for root in [
            std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into()),
            std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into()),
            std::env::var("LOCALAPPDATA").unwrap_or_default(),
        ] {
            if !root.is_empty() {
                paths.push(PathBuf::from(&root).join(r"Google\Chrome\Application\chrome.exe"));
                paths.push(
                    PathBuf::from(&root).join(r"BraveSoftware\Brave-Browser\Application\brave.exe"),
                );
                paths.push(PathBuf::from(&root).join(r"Microsoft\Edge\Application\msedge.exe"));
            }
        }
    }
    if let Some(path_env) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_env) {
            for bin in [
                "chromium",
                "chromium-browser",
                "google-chrome",
                "google-chrome-stable",
                "brave-browser",
                "microsoft-edge",
            ] {
                paths.push(dir.join(bin));
            }
        }
    }
    paths
}

fn probe_executable_version(path: &Path) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        // On macOS, read Info.plist next to Contents/MacOS/<Binary> to avoid spawning a GUI process during discovery.
        if let Some(contents_dir) = path.parent().and_then(Path::parent) {
            let plist = contents_dir.join("Info.plist");
            if let Ok(text) = std::fs::read_to_string(plist) {
                if let Some(idx) = text.find("CFBundleShortVersionString") {
                    if let Some(start) = text[idx..].find("<string>") {
                        let rest = &text[idx + start + 8..];
                        if let Some(end) = rest.find("</string>") {
                            let ver = rest[..end].trim();
                            if !ver.is_empty() {
                                return Some(ver.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub fn resolve_browser_runtime() -> Result<ResolvedBrowserRuntime, String> {
    if let Ok(ws_url) = std::env::var("SCRAPE_CDP_WS_URL") {
        let trimmed = ws_url.trim();
        if !trimmed.is_empty() {
            let parsed = Url::parse(trimmed)
                .map_err(|e| format!("SCRAPE_CDP_WS_URL: invalid WebSocket URL: {e}"))?;
            if !matches!(parsed.scheme(), "ws" | "wss") {
                return Err("SCRAPE_CDP_WS_URL: scheme must be ws:// or wss://".into());
            }
            return Ok(ResolvedBrowserRuntime::WebSocket(trimmed.to_string()));
        }
    }
    if let Ok(custom_path) = std::env::var("SCRAPE_BROWSER_PATH") {
        let trimmed = custom_path.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_file() {
                let version = probe_executable_version(&path);
                return Ok(ResolvedBrowserRuntime::Executable { path, version });
            }
            return Err(MISSING_BROWSER_RUNTIME_MESSAGE.to_string());
        }
    }
    for candidate in candidate_browser_paths() {
        if candidate.is_file() {
            let version = probe_executable_version(&candidate);
            return Ok(ResolvedBrowserRuntime::Executable {
                path: candidate,
                version,
            });
        }
    }
    Err(MISSING_BROWSER_RUNTIME_MESSAGE.to_string())
}

pub fn discover_browser_runtime() -> BrowserRuntimeStatus {
    match resolve_browser_runtime() {
        Ok(ResolvedBrowserRuntime::WebSocket(ws_url)) => BrowserRuntimeStatus {
            available: true,
            executable_path: Some(ws_url),
            version: Some("CDP WebSocket endpoint".into()),
            message: None,
        },
        Ok(ResolvedBrowserRuntime::Executable { path, version }) => BrowserRuntimeStatus {
            available: true,
            executable_path: Some(path.to_string_lossy().into_owned()),
            version,
            message: None,
        },
        Err(message) => {
            let configured_path = std::env::var("SCRAPE_BROWSER_PATH")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            BrowserRuntimeStatus {
                available: false,
                executable_path: configured_path,
                version: None,
                message: Some(message),
            }
        }
    }
}

/// Minimal RFC 6455 WebSocket JSON-RPC client for local loopback Chrome DevTools Protocol.
pub struct CdpWsClient {
    stream: TcpStream,
    leftover: Vec<u8>,
    next_id: u64,
}

impl CdpWsClient {
    pub fn connect(ws_url: &str, timeout: Duration) -> Result<Self, String> {
        let parsed = Url::parse(ws_url).map_err(|e| format!("Invalid CDP WebSocket URL: {e}"))?;
        if parsed.scheme() != "ws" {
            return Err(format!(
                "Unsupported CDP WebSocket scheme '{}'; expected ws://",
                parsed.scheme()
            ));
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| "CDP WebSocket URL is missing host".to_string())?;
        let port = parsed.port().unwrap_or(80);
        let addr: SocketAddr = (host, port)
            .to_socket_addrs()
            .map_err(|e| format!("Failed to resolve CDP endpoint {host}:{port}: {e}"))?
            .next()
            .ok_or_else(|| format!("No socket address resolved for CDP endpoint {host}:{port}"))?;

        let mut stream = TcpStream::connect_timeout(&addr, timeout)
            .map_err(|e| format!("Failed to connect to CDP WebSocket {addr}: {e}"))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| format!("CDP socket timeout error: {e}"))?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| format!("CDP socket timeout error: {e}"))?;

        let path = match parsed.query() {
            Some(q) => format!("{}?{q}", parsed.path()),
            None if parsed.path().is_empty() => "/".to_string(),
            None => parsed.path().to_string(),
        };
        let key_bytes = Sha256::digest(format!("scrape-cdp-{ws_url}").as_bytes());
        let sec_key = base64::engine::general_purpose::STANDARD.encode(&key_bytes[..16]);
        let request = format!(
            "GET {path} HTTP/1.1\r\n\
             Host: {host}:{port}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {sec_key}\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("Failed to send CDP WebSocket handshake: {e}"))?;
        stream
            .flush()
            .map_err(|e| format!("Failed to flush CDP WebSocket handshake: {e}"))?;

        let mut buf = Vec::new();
        let mut temp = [0u8; 1024];
        let header_end = loop {
            let n = stream
                .read(&mut temp)
                .map_err(|e| format!("Failed reading CDP WebSocket handshake: {e}"))?;
            if n == 0 {
                return Err("CDP WebSocket closed before completing handshake".into());
            }
            buf.extend_from_slice(&temp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
            if buf.len() > 16_384 {
                return Err("CDP WebSocket handshake headers exceeded 16 KiB".into());
            }
        };

        let headers = String::from_utf8_lossy(&buf[..header_end]);
        if !headers.starts_with("HTTP/1.1 101") {
            return Err(format!(
                "CDP WebSocket handshake rejected: {}",
                headers.lines().next().unwrap_or("unknown response")
            ));
        }
        let leftover = buf[header_end..].to_vec();
        Ok(Self {
            stream,
            leftover,
            next_id: 1,
        })
    }

    pub fn call(
        &mut self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
        timeout: Duration,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let mut payload = json!({
            "id": id,
            "method": method,
            "params": params,
        });
        if let Some(sid) = session_id {
            payload["sessionId"] = Value::String(sid.to_string());
        }
        let encoded = serde_json::to_vec(&payload)
            .map_err(|e| format!("Failed to encode CDP command {method}: {e}"))?;

        let deadline = Instant::now() + timeout;
        self.stream
            .set_write_timeout(Some(timeout))
            .map_err(|e| format!("CDP write timeout error: {e}"))?;
        self.write_masked_frame(0x81, &encoded)?;

        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| {
                    format!(
                        "CDP command {method} timed out after {}s",
                        timeout.as_secs()
                    )
                })?;
            self.stream
                .set_read_timeout(Some(remaining.max(Duration::from_millis(20))))
                .map_err(|e| format!("CDP read timeout error: {e}"))?;

            let text = self.read_text_frame().map_err(|e| {
                if e.contains("TimedOut") || e.contains("WouldBlock") || e.contains("timed out") {
                    format!(
                        "CDP command {method} timed out after {}s",
                        timeout.as_secs()
                    )
                } else {
                    e
                }
            })?;
            let message: Value = serde_json::from_str(&text)
                .map_err(|e| format!("Malformed CDP JSON message during {method}: {e}"))?;
            if message.get("id").and_then(Value::as_u64) == Some(id) {
                if let Some(err) = message.get("error") {
                    let msg = err
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown CDP error");
                    return Err(format!("CDP {method} failed: {msg}"));
                }
                return Ok(message.get("result").cloned().unwrap_or_else(|| json!({})));
            }
        }
    }

    fn write_masked_frame(&mut self, opcode_byte: u8, payload: &[u8]) -> Result<(), String> {
        let mask: [u8; 4] = [0x37, 0xfa, 0x21, 0x3d];
        let mut frame = Vec::with_capacity(14 + payload.len());
        frame.push(opcode_byte);
        if payload.len() < 126 {
            frame.push(0x80 | (payload.len() as u8));
        } else if payload.len() <= u16::MAX as usize {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        frame.extend_from_slice(&mask);
        for (i, byte) in payload.iter().enumerate() {
            frame.push(byte ^ mask[i % 4]);
        }
        self.stream
            .write_all(&frame)
            .map_err(|e| format!("Failed writing CDP WebSocket frame: {e}"))?;
        self.stream
            .flush()
            .map_err(|e| format!("Failed flushing CDP WebSocket frame: {e}"))
    }

    fn read_exact_buffered(&mut self, out: &mut [u8]) -> Result<(), String> {
        let mut filled = 0;
        if !self.leftover.is_empty() {
            let take = out.len().min(self.leftover.len());
            out[..take].copy_from_slice(&self.leftover[..take]);
            self.leftover.drain(..take);
            filled = take;
        }
        while filled < out.len() {
            let n = self
                .stream
                .read(&mut out[filled..])
                .map_err(|e| format!("CDP WebSocket read error ({:?}): {e}", e.kind()))?;
            if n == 0 {
                return Err("CDP WebSocket closed unexpectedly".into());
            }
            filled += n;
        }
        Ok(())
    }

    fn read_text_frame(&mut self) -> Result<String, String> {
        let mut message_bytes = Vec::new();
        loop {
            let mut header = [0u8; 2];
            self.read_exact_buffered(&mut header)?;
            let fin = (header[0] & 0x80) != 0;
            let opcode = header[0] & 0x0f;
            let masked = (header[1] & 0x80) != 0;
            let mut len = (header[1] & 0x7f) as usize;
            if len == 126 {
                let mut ext = [0u8; 2];
                self.read_exact_buffered(&mut ext)?;
                len = u16::from_be_bytes(ext) as usize;
            } else if len == 127 {
                let mut ext = [0u8; 8];
                self.read_exact_buffered(&mut ext)?;
                len = usize::try_from(u64::from_be_bytes(ext))
                    .map_err(|_| "CDP WebSocket frame exceeds addressable memory".to_string())?;
            }
            if len > 32 * 1024 * 1024 {
                return Err("CDP WebSocket frame exceeded 32 MiB safety limit".into());
            }
            let mut mask = [0u8; 4];
            if masked {
                self.read_exact_buffered(&mut mask)?;
            }
            let mut payload = vec![0u8; len];
            self.read_exact_buffered(&mut payload)?;
            if masked {
                for (i, b) in payload.iter_mut().enumerate() {
                    *b ^= mask[i % 4];
                }
            }
            match opcode {
                0x8 => return Err("CDP WebSocket closed by peer".into()),
                0x9 => {
                    // Reply to Ping with Pong
                    let _ = self.write_masked_frame(0x8A, &payload);
                    continue;
                }
                0xA => continue,
                0x0..=0x2 => {
                    message_bytes.extend_from_slice(&payload);
                    if fin {
                        return String::from_utf8(message_bytes)
                            .map_err(|e| format!("Invalid UTF-8 in CDP frame: {e}"));
                    }
                }
                other => return Err(format!("Unsupported CDP WebSocket opcode: {other}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_browser_session_enforces_domain_scope_and_expiry() {
        let session = StoredBrowserSession {
            id: "corp-auth".into(),
            domain: "fixture.test".into(),
            saved_at: "2026-10-17T00:00:00Z".into(),
            cookies: vec![BrowserCookie {
                name: "sid".into(),
                value: "secret".into(),
                domain: ".fixture.test".into(),
                path: "/".into(),
                secure: true,
                http_only: true,
                expires: Some(2_000_000_000.0),
                same_site: None,
            }],
        };

        let valid_url = Url::parse("https://jobs.fixture.test/board").unwrap();
        let cookies = session
            .cookies_for_url(&valid_url, 1_900_000_000.0)
            .unwrap();
        assert_eq!(cookies.len(), 1);

        let wrong_domain = Url::parse("https://other.example.com/board").unwrap();
        assert!(session
            .cookies_for_url(&wrong_domain, 1_900_000_000.0)
            .unwrap_err()
            .contains("scoped to domain"));

        assert!(session
            .cookies_for_url(&valid_url, 2_100_000_000.0)
            .unwrap_err()
            .contains("Session expired or login required"));
    }
}
