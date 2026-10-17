use crate::error::{AppError, Result};
use scrape_core::{
    browser::{
        discover_browser_runtime, resolve_browser_runtime, BrowserCookie, CdpWsClient,
        StoredBrowserSession,
    },
    protocol::{
        BrowserRuntimeStatus, BrowserSessionStatus, BrowserSessionSummary, CoverageTermination,
    },
};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{LazyLock, Mutex},
    time::Duration,
};

struct ActiveLoginSession {
    ws_url: String,
    child: Option<Child>,
    profile_dir: Option<PathBuf>,
}

impl ActiveLoginSession {
    fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            unsafe {
                let _ = libc::killpg(child.id() as libc::pid_t, libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(dir) = self.profile_dir.take() {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

impl Drop for ActiveLoginSession {
    fn drop(&mut self) {
        self.shutdown();
    }
}

static ACTIVE_SESSIONS: LazyLock<Mutex<HashMap<String, ActiveLoginSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn session_dir_for_app(app: &tauri::AppHandle) -> Result<PathBuf> {
    use tauri::Manager;
    let app_dir = super::native_smoke::directory()
        .map(Ok)
        .unwrap_or_else(|| app.path().app_data_dir())
        .map_err(|e| AppError::Internal(format!("Failed to resolve app data dir: {e}")))?;
    Ok(app_dir.join("browser_sessions"))
}

pub fn ensure_private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    fs::write(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn session_file_path(session_dir: &Path, session_id: &str) -> Result<PathBuf> {
    let valid_id =
        StoredBrowserSession::validate_session_id(session_id).map_err(AppError::Validation)?;
    Ok(session_dir.join(format!("{valid_id}.json")))
}

pub fn detect_browser_runtime() -> BrowserRuntimeStatus {
    discover_browser_runtime()
}

fn parse_status(raw: &str, has_valid_cookie_file: bool) -> BrowserSessionStatus {
    match raw {
        "ready" if has_valid_cookie_file => BrowserSessionStatus::Ready,
        "expired" => BrowserSessionStatus::Expired,
        _ => BrowserSessionStatus::NeedsLogin,
    }
}

fn cookie_file_is_usable(session_dir: &Path, session_id: &str, domain: &str) -> bool {
    let Ok(path) = session_file_path(session_dir, session_id) else {
        return false;
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(stored) = serde_json::from_str::<StoredBrowserSession>(&raw) else {
        return false;
    };
    let now_epoch = chrono::Utc::now().timestamp() as f64;
    stored.cookies.iter().any(|cookie| {
        StoredBrowserSession::domain_matches(&cookie.domain, domain)
            && cookie
                .expires
                .is_none_or(|exp| exp <= 0.0 || exp > now_epoch)
    })
}

pub async fn list_browser_sessions(
    pool: &SqlitePool,
    session_dir: &Path,
) -> Result<Vec<BrowserSessionSummary>> {
    let rows = sqlx::query(
        "SELECT id, label, domain, status, created_at, updated_at, last_verified_at \
         FROM browser_sessions ORDER BY updated_at DESC, id ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.get("id");
        let label: String = row.get("label");
        let domain: String = row.get("domain");
        let raw_status: String = row.get("status");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");
        let last_verified_at: Option<String> = row.get("last_verified_at");
        let usable = cookie_file_is_usable(session_dir, &id, &domain);
        out.push(BrowserSessionSummary {
            id,
            label,
            domain,
            status: parse_status(&raw_status, usable),
            created_at,
            updated_at,
            last_verified_at,
        });
    }
    Ok(out)
}

fn spawn_headed_login_browser(
    session_dir: &Path,
    session_id: &str,
    login_url: &str,
) -> Result<ActiveLoginSession> {
    let executable_path = match resolve_browser_runtime().map_err(AppError::Validation)? {
        scrape_core::browser::ResolvedBrowserRuntime::WebSocket(ws_url) => {
            return Ok(ActiveLoginSession {
                ws_url,
                child: None,
                profile_dir: None,
            });
        }
        scrape_core::browser::ResolvedBrowserRuntime::Executable { path, .. } => path,
    };
    let profile_dir = session_dir.join("profiles").join(session_id);
    ensure_private_dir(&profile_dir)?;

    let mut cmd = Command::new(&executable_path);
    cmd.args([
        "--remote-debugging-port=0",
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-background-networking",
        "--disable-sync",
        "--window-size=1280,900",
        &format!("--user-data-dir={}", profile_dir.display()),
        login_url,
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Internal(format!("Failed to launch browser session: {e}")))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Internal("Failed to capture browser stderr".into()))?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(std::result::Result::ok) {
            if let Some(idx) = line.find("ws://127.0.0.1:") {
                let ws = line[idx..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let _ = tx.send(ws);
                break;
            } else if let Some(idx) = line.find("ws://[::1]:") {
                let ws = line[idx..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let _ = tx.send(ws);
                break;
            }
        }
    });

    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(ws_url) if !ws_url.is_empty() => Ok(ActiveLoginSession {
            ws_url,
            child: Some(child),
            profile_dir: Some(profile_dir),
        }),
        _ => {
            let mut failed = ActiveLoginSession {
                ws_url: String::new(),
                child: Some(child),
                profile_dir: Some(profile_dir),
            };
            failed.shutdown();
            Err(AppError::Internal(
                "Timed out waiting for the browser DevTools endpoint".into(),
            ))
        }
    }
}

pub async fn open_browser_session(
    pool: &SqlitePool,
    session_dir: &Path,
    label: &str,
    login_url: &str,
) -> Result<BrowserSessionSummary> {
    let trimmed_label = label.trim();
    if trimmed_label.is_empty() {
        return Err(AppError::Validation(
            "Session label must not be blank.".into(),
        ));
    }
    let normalized_url =
        scrape_core::identity::normalized_detail_url(login_url).map_err(AppError::Validation)?;
    let parsed_url = url::Url::parse(&normalized_url)
        .map_err(|e| AppError::Validation(format!("Invalid login URL: {e}")))?;
    let raw_host = parsed_url
        .host_str()
        .ok_or_else(|| AppError::Validation("Login URL must include a valid host.".into()))?;
    let host = StoredBrowserSession::normalize_domain(raw_host).map_err(AppError::Validation)?;

    ensure_private_dir(session_dir)?;

    let existing: Option<(String, String)> = sqlx::query_as(
        "SELECT id, created_at FROM browser_sessions WHERE lower(label) = lower(?) AND lower(domain) = lower(?) LIMIT 1",
    )
    .bind(trimmed_label)
    .bind(&host)
    .fetch_optional(pool)
    .await?;

    let now = scrape_core::now_iso();
    let (session_id, created_at) = existing.unwrap_or_else(|| {
        (
            format!("session-{}", uuid::Uuid::new_v4().simple()),
            now.clone(),
        )
    });
    StoredBrowserSession::validate_session_id(&session_id).map_err(AppError::Validation)?;
    let storage_relpath = format!("browser_sessions/{session_id}.json");

    let active = spawn_headed_login_browser(session_dir, &session_id, &normalized_url)?;
    {
        let mut guard = ACTIVE_SESSIONS.lock().unwrap();
        if let Some(mut previous) = guard.insert(session_id.clone(), active) {
            previous.shutdown();
        }
    }

    sqlx::query(
        "INSERT INTO browser_sessions (id, label, domain, storage_relpath, status, created_at, updated_at, last_verified_at) \
         VALUES (?, ?, ?, ?, 'needs_login', ?, ?, NULL) \
         ON CONFLICT(id) DO UPDATE SET \
           label = excluded.label, \
           domain = excluded.domain, \
           storage_relpath = excluded.storage_relpath, \
           status = 'needs_login', \
           updated_at = excluded.updated_at",
    )
    .bind(&session_id)
    .bind(trimmed_label)
    .bind(&host)
    .bind(&storage_relpath)
    .bind(&created_at)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(BrowserSessionSummary {
        id: session_id,
        label: trimmed_label.to_string(),
        domain: host,
        status: BrowserSessionStatus::NeedsLogin,
        created_at,
        updated_at: now,
        last_verified_at: None,
    })
}

fn parse_cdp_cookies(payload: &Value, session_domain: &str) -> Vec<BrowserCookie> {
    let Some(items) = payload.pointer("/cookies").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut cookies = Vec::new();
    for item in items {
        let Some(name) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(value) = item.get("value").and_then(Value::as_str) else {
            continue;
        };
        let domain = item
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or(session_domain);
        if name.trim().is_empty() || !StoredBrowserSession::domain_matches(domain, session_domain) {
            continue;
        }
        let path = item
            .get("path")
            .and_then(Value::as_str)
            .filter(|p| !p.trim().is_empty())
            .unwrap_or("/")
            .to_string();
        let secure = item.get("secure").and_then(Value::as_bool).unwrap_or(false);
        let http_only = item
            .get("httpOnly")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let same_site = item
            .get("sameSite")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let expires = item
            .get("expires")
            .and_then(Value::as_f64)
            .filter(|exp| *exp > 0.0);
        cookies.push(BrowserCookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            path,
            secure,
            http_only,
            same_site,
            expires,
        });
    }
    cookies
}

pub async fn save_browser_session(
    pool: &SqlitePool,
    session_dir: &Path,
    session_id: &str,
) -> Result<BrowserSessionSummary> {
    let valid_id =
        StoredBrowserSession::validate_session_id(session_id).map_err(AppError::Validation)?;
    let row =
        sqlx::query("SELECT id, label, domain, created_at FROM browser_sessions WHERE id = ?")
            .bind(&valid_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Browser session {valid_id} not found")))?;

    let label: String = row.get("label");
    let domain: String = row.get("domain");
    let created_at: String = row.get("created_at");

    let mut active = {
        let mut guard = ACTIVE_SESSIONS.lock().unwrap();
        guard.remove(&valid_id)
    };
    if active.is_none() {
        if let Ok(ws_url) = std::env::var("SCRAPE_CDP_WS_URL") {
            if !ws_url.trim().is_empty() {
                active = Some(ActiveLoginSession {
                    ws_url: ws_url.trim().to_string(),
                    child: None,
                    profile_dir: None,
                });
            }
        }
    }
    let mut active = active.ok_or_else(|| {
        AppError::Validation(
            "Open the login window and sign in before saving the browser session.".into(),
        )
    })?;

    let timeout = Duration::from_secs(5);
    let mut client = CdpWsClient::connect(&active.ws_url, timeout).map_err(AppError::Validation)?;
    let cookie_payload = client
        .call("Storage.getCookies", json!({}), None, timeout)
        .or_else(|_| client.call("Network.getAllCookies", json!({}), None, timeout))
        .map_err(AppError::Validation)?;
    drop(client);
    active.shutdown();

    let cookies = parse_cdp_cookies(&cookie_payload, &domain);
    if cookies.is_empty() {
        return Err(AppError::Validation(format!(
            "No cookies matching {domain} were found. Sign in inside the browser window and try again."
        )));
    }

    let now = scrape_core::now_iso();
    let stored = StoredBrowserSession {
        id: valid_id.clone(),
        domain: domain.clone(),
        saved_at: now.clone(),
        cookies,
    };
    let file_path = session_file_path(session_dir, &valid_id)?;
    let bytes = serde_json::to_vec_pretty(&stored)?;
    write_private_file(&file_path, &bytes)?;

    sqlx::query(
        "UPDATE browser_sessions SET status = 'ready', updated_at = ?, last_verified_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(&valid_id)
    .execute(pool)
    .await?;

    Ok(BrowserSessionSummary {
        id: valid_id,
        label,
        domain,
        status: BrowserSessionStatus::Ready,
        created_at,
        updated_at: now.clone(),
        last_verified_at: Some(now),
    })
}

pub async fn clear_browser_session(
    pool: &SqlitePool,
    session_dir: &Path,
    session_id: &str,
) -> Result<()> {
    let valid_id =
        StoredBrowserSession::validate_session_id(session_id).map_err(AppError::Validation)?;
    if let Some(mut active) = ACTIVE_SESSIONS.lock().unwrap().remove(&valid_id) {
        active.shutdown();
    }
    if let Ok(path) = session_file_path(session_dir, &valid_id) {
        let _ = fs::remove_file(path);
    }
    let profile_dir = session_dir.join("profiles").join(&valid_id);
    if profile_dir.exists() {
        let _ = fs::remove_dir_all(profile_dir);
    }
    sqlx::query("DELETE FROM browser_sessions WHERE id = ?")
        .bind(&valid_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn validate_session_ready(
    pool: &SqlitePool,
    session_dir: &Path,
    session_id: &str,
) -> Result<()> {
    let valid_id =
        StoredBrowserSession::validate_session_id(session_id).map_err(AppError::Validation)?;
    let row = sqlx::query("SELECT label, domain, status FROM browser_sessions WHERE id = ?")
        .bind(&valid_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Browser session '{valid_id}' does not exist. Open and save the login session before running."
            ))
        })?;
    let label: String = row.get("label");
    let domain: String = row.get("domain");
    let status: String = row.get("status");
    if status != "ready" || !cookie_file_is_usable(session_dir, &valid_id, &domain) {
        return Err(AppError::Validation(format!(
            "Browser session '{label}' ({domain}) requires login. Authenticate and save the session before running."
        )));
    }
    Ok(())
}

pub async fn reconcile_source_session_after_scan(
    pool: &SqlitePool,
    session_dir: &Path,
    config: &Value,
    termination: Option<CoverageTermination>,
) -> Result<()> {
    if termination != Some(CoverageTermination::NeedsLogin) {
        return Ok(());
    }
    let Some(session_id) = config
        .get("session_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return Ok(());
    };
    if let Ok(valid_id) = StoredBrowserSession::validate_session_id(session_id) {
        let now = scrape_core::now_iso();
        let _ = sqlx::query(
            "UPDATE browser_sessions SET status = 'needs_login', updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(&valid_id)
        .execute(pool)
        .await;
        if let Ok(path) = session_file_path(session_dir, &valid_id) {
            let _ = fs::remove_file(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha1::{Digest, Sha1};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn accept_ws_handshake(stream: &mut TcpStream) {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            buf.push(byte[0]);
        }
        let req = String::from_utf8_lossy(&buf);
        let key = req
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.trim().eq_ignore_ascii_case("sec-websocket-key") {
                    Some(value.trim().to_string())
                } else {
                    None
                }
            })
            .unwrap();
        let mut sha1 = Sha1::new();
        sha1.update(key.as_bytes());
        sha1.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let accept = BASE64.encode(sha1.finalize());
        let resp = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        stream.write_all(resp.as_bytes()).unwrap();
    }

    fn read_ws_frame(stream: &mut TcpStream) -> Option<Value> {
        let mut header = [0u8; 2];
        if stream.read_exact(&mut header).is_err() {
            return None;
        }
        let opcode = header[0] & 0x0f;
        if opcode == 0x8 {
            return None;
        }
        let masked = (header[1] & 0x80) != 0;
        let mut len = (header[1] & 0x7f) as usize;
        if len == 126 {
            let mut ext = [0u8; 2];
            stream.read_exact(&mut ext).ok()?;
            len = u16::from_be_bytes(ext) as usize;
        }
        let mut mask = [0u8; 4];
        if masked {
            stream.read_exact(&mut mask).ok()?;
        }
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).ok()?;
        if masked {
            for (i, b) in payload.iter_mut().enumerate() {
                *b ^= mask[i % 4];
            }
        }
        serde_json::from_slice(&payload).ok()
    }

    fn write_ws_frame(stream: &mut TcpStream, msg: &Value) {
        let payload = serde_json::to_vec(msg).unwrap();
        let mut frame = vec![0x81];
        if payload.len() <= 125 {
            frame.push(payload.len() as u8);
        } else {
            frame.push(126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        frame.extend_from_slice(&payload);
        let _ = stream.write_all(&frame);
    }

    #[tokio::test]
    async fn browser_session_lifecycle_persists_private_domain_scoped_cookies_and_clears() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let session_dir = dir.path().join("browser_sessions");
        let pool = crate::db::init_db(&db_path).await.unwrap();

        let ws_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let ws_addr = ws_listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = ws_listener.accept() {
                accept_ws_handshake(&mut stream);
                while let Some(msg) = read_ws_frame(&mut stream) {
                    let id = msg.get("id").and_then(Value::as_u64).unwrap_or(1);
                    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
                    if method == "Storage.getCookies" {
                        write_ws_frame(
                            &mut stream,
                            &json!({
                                "id": id,
                                "result": {
                                    "cookies": [
                                        {
                                            "name": "portal_auth",
                                            "value": "secret-session-cookie",
                                            "domain": ".Workday.Example.com",
                                            "path": "/",
                                            "secure": true,
                                            "httpOnly": true,
                                            "expires": 4102444800.0
                                        },
                                        {
                                            "name": "other_tracker",
                                            "value": "ignore-me",
                                            "domain": ".other-site.example.org",
                                            "path": "/"
                                        }
                                    ]
                                }
                            }),
                        );
                    }
                }
            }
        });

        let opened = {
            let active = ActiveLoginSession {
                ws_url: format!("ws://{ws_addr}/devtools/browser/mock"),
                child: None,
                profile_dir: None,
            };
            ensure_private_dir(&session_dir).unwrap();
            let session_id = "session-workday-1".to_string();
            ACTIVE_SESSIONS
                .lock()
                .unwrap()
                .insert(session_id.clone(), active);
            let now = scrape_core::now_iso();
            sqlx::query(
                "INSERT INTO browser_sessions (id, label, domain, storage_relpath, status, created_at, updated_at) \
                 VALUES (?, 'Workday Portal', 'workday.example.com', 'browser_sessions/session-workday-1.json', 'needs_login', ?, ?)",
            )
            .bind(&session_id)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await
            .unwrap();
            session_id
        };

        let saved = save_browser_session(&pool, &session_dir, &opened)
            .await
            .unwrap();
        assert_eq!(saved.status, BrowserSessionStatus::Ready);
        assert_eq!(saved.domain, "workday.example.com");

        let cookie_file = session_file_path(&session_dir, &opened).unwrap();
        assert!(cookie_file.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir_mode = fs::metadata(&session_dir).unwrap().permissions().mode() & 0o777;
            let file_mode = fs::metadata(&cookie_file).unwrap().permissions().mode() & 0o777;
            assert_eq!(dir_mode, 0o700);
            assert_eq!(file_mode, 0o600);
        }

        let stored: StoredBrowserSession =
            serde_json::from_str(&fs::read_to_string(&cookie_file).unwrap()).unwrap();
        assert_eq!(stored.cookies.len(), 1);
        assert_eq!(stored.cookies[0].name, "portal_auth");

        validate_session_ready(&pool, &session_dir, &opened)
            .await
            .unwrap();

        reconcile_source_session_after_scan(
            &pool,
            &session_dir,
            &json!({ "session_id": opened }),
            Some(CoverageTermination::NeedsLogin),
        )
        .await
        .unwrap();
        assert!(!cookie_file.exists());
        let sessions = list_browser_sessions(&pool, &session_dir).await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].status, BrowserSessionStatus::NeedsLogin);

        clear_browser_session(&pool, &session_dir, &opened)
            .await
            .unwrap();
        assert!(list_browser_sessions(&pool, &session_dir)
            .await
            .unwrap()
            .is_empty());
        let _ = server.join();
    }
}
