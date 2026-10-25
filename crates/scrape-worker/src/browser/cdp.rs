//! Direct Rust Chrome DevTools Protocol (CDP) browser pool and single-turn page renderer.
use super::discovery::resolve_runtime;
use super::stealth::{
    user_agent_override_params, BLOCKED_RESOURCE_URL_PATTERNS, STEALTH_INIT_SCRIPT,
};
use crate::client::{ClientError, HttpClient};
use scrape_core::browser::{
    chromium_proxy_args, is_login_wall, BrowserCookie, CdpWsClient, ResolvedBrowserRuntime,
    NEEDS_LOGIN_MESSAGE,
};
use scrape_core::locator::QueryEngine;
use scrape_core::protocol::HttpProxyConfig;
use scrape_core::static_source::StaticSourceConfig;
use scrape_core::WorkerSource;
use scraper::Html;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::Instant;
use url::Url;

pub struct RenderedPage {
    pub html: String,
    pub actual_url: String,
    pub pages_consumed: u32,
    pub page_limit_reached: bool,
}

#[derive(Deserialize)]
struct DomSnapshot {
    #[serde(rename = "readyState", default)]
    ready_state: String,
    #[serde(default)]
    href: String,
    #[serde(default)]
    title: String,
    #[serde(rename = "hasPassword", default)]
    has_password: bool,
    #[serde(default)]
    html: String,
}

const SNAPSHOT_JS: &str = r#"JSON.stringify({
  readyState: document.readyState || "complete",
  href: String(location.href || ""),
  title: String(document.title || ""),
  hasPassword: Boolean(document.querySelector('input[type="password"]')),
  html: document.documentElement ? document.documentElement.outerHTML : ""
})"#;

pub struct BrowserPool {
    semaphore: Arc<Semaphore>,
    ws_url_override: Option<String>,
    ws_client: Mutex<Option<CdpWsClient>>,
    child: Mutex<Option<Child>>,
    user_data_dir: Mutex<Option<tempfile::TempDir>>,
    http_proxy: Option<HttpProxyConfig>,
}

impl BrowserPool {
    pub fn new(max_contexts: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_contexts.clamp(1, 2))),
            ws_url_override: None,
            ws_client: Mutex::new(None),
            child: Mutex::new(None),
            user_data_dir: Mutex::new(None),
            http_proxy: None,
        }
    }

    pub fn with_ws_url(max_contexts: usize, ws_url: String) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_contexts.clamp(1, 2))),
            ws_url_override: Some(ws_url),
            ws_client: Mutex::new(None),
            child: Mutex::new(None),
            user_data_dir: Mutex::new(None),
            http_proxy: None,
        }
    }

    pub fn with_proxy(mut self, proxy: Option<&HttpProxyConfig>) -> Result<Self, ClientError> {
        self.http_proxy = proxy
            .cloned()
            .map(HttpProxyConfig::validate)
            .transpose()
            .map_err(ClientError::Configuration)?;
        if self.http_proxy.is_some() && self.ws_url_override.is_some() {
            return Err(ClientError::Configuration(
                "Proxy routing requires app-launched Chromium; external CDP is unsupported".into(),
            ));
        }
        Ok(self)
    }

    fn with_client<F, T>(&self, timeout: Duration, f: F) -> Result<T, ClientError>
    where
        F: FnOnce(&mut CdpWsClient) -> Result<T, ClientError>,
    {
        let mut guard = self
            .ws_client
            .lock()
            .map_err(|_| ClientError::Extraction("Browser pool lock poisoned".into()))?;
        if guard.is_none() {
            let client = self.connect_or_launch(timeout)?;
            *guard = Some(client);
        }
        let client = guard.as_mut().unwrap();
        f(client)
    }

    fn connect_or_launch(&self, timeout: Duration) -> Result<CdpWsClient, ClientError> {
        if let Some(ws_url) = &self.ws_url_override {
            return CdpWsClient::connect(ws_url, timeout).map_err(ClientError::Extraction);
        }
        match resolve_runtime()? {
            ResolvedBrowserRuntime::WebSocket(ws_url) => {
                if self.http_proxy.is_some() {
                    return Err(ClientError::Configuration(
                        "Proxy routing requires app-launched Chromium; external CDP is unsupported"
                            .into(),
                    ));
                }
                CdpWsClient::connect(&ws_url, timeout).map_err(ClientError::Extraction)
            }
            ResolvedBrowserRuntime::Executable { path, .. } => {
                self.launch_headless_executable(&path, timeout)
            }
        }
    }

    fn launch_headless_executable(
        &self,
        executable: &Path,
        timeout: Duration,
    ) -> Result<CdpWsClient, ClientError> {
        let temp_dir = tempfile::tempdir().map_err(|e| {
            ClientError::Extraction(format!("Failed creating ephemeral browser profile: {e}"))
        })?;
        let mut cmd = Command::new(executable);
        if let Some(proxy) = &self.http_proxy {
            cmd.args(chromium_proxy_args(proxy).map_err(ClientError::Configuration)?);
        }
        cmd.arg("--headless=new")
            .arg("--remote-debugging-port=0")
            .arg(format!("--user-data-dir={}", temp_dir.path().display()))
            .arg("--disable-gpu")
            .arg("--disable-dev-shm-usage")
            .arg("--disable-extensions")
            .arg("--disable-background-networking")
            .arg("--disable-sync")
            .arg("--disable-translate")
            .arg("--disable-default-apps")
            .arg("--no-first-run")
            .arg("--mute-audio")
            .arg("--hide-scrollbars")
            .arg("--metrics-recording-only")
            .arg("--js-flags=--max-old-space-size=128")
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let mut child = cmd.spawn().map_err(|e| {
            ClientError::Extraction(format!(
                "Failed launching browser executable {}: {e}",
                executable.display()
            ))
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ClientError::Extraction("Failed capturing browser stderr for DevTools endpoint".into())
        })?;

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(idx) = line.find("DevTools listening on ws://") {
                    let ws = line[idx + "DevTools listening on ".len()..]
                        .trim()
                        .to_string();
                    let _ = tx.send(Ok(ws));
                    return;
                }
            }
            let _ = tx.send(Err(
                "Browser process exited before reporting a DevTools WebSocket endpoint".to_string(),
            ));
        });

        let startup_timeout = timeout.min(Duration::from_secs(10));
        let ws_url = match rx.recv_timeout(startup_timeout) {
            Ok(Ok(url)) => url,
            Ok(Err(msg)) => {
                kill_browser_child(&mut child);
                return Err(ClientError::Extraction(msg));
            }
            Err(_) => {
                kill_browser_child(&mut child);
                return Err(ClientError::TimeLimit(
                    "Browser startup timed out waiting for DevTools endpoint".into(),
                ));
            }
        };

        let ws_client = match CdpWsClient::connect(&ws_url, startup_timeout) {
            Ok(c) => c,
            Err(e) => {
                kill_browser_child(&mut child);
                return Err(ClientError::Extraction(e));
            }
        };

        if let Ok(mut child_guard) = self.child.lock() {
            *child_guard = Some(child);
        }
        if let Ok(mut dir_guard) = self.user_data_dir.lock() {
            *dir_guard = Some(temp_dir);
        }
        Ok(ws_client)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fetch_rendered_page(
        &self,
        client: &HttpClient,
        requested_url: &Url,
        source: &WorkerSource,
        config: &StaticSourceConfig,
        session_cookies: &[BrowserCookie],
        remaining_pages: u32,
        navigation_timeout: Duration,
        source_deadline: Instant,
    ) -> Result<RenderedPage, ClientError> {
        let deadline = source_deadline.min(Instant::now() + navigation_timeout);
        if Instant::now() >= deadline {
            return Err(ClientError::TimeLimit(
                "Browser navigation reached its time limit before acquiring a context".into(),
            ));
        }

        let _permit = tokio::time::timeout_at(deadline, self.semaphore.acquire())
            .await
            .map_err(|_| {
                ClientError::TimeLimit("Timed out waiting for an available browser context".into())
            })?
            .map_err(|_| ClientError::Extraction("Browser context semaphore closed".into()))?;

        let transport_url = client
            .prepare_navigation(requested_url.as_str(), self.ws_url_override.is_none())
            .await?;
        let item_query = QueryEngine::parse_elements(
            source
                .config
                .get("item")
                .and_then(Value::as_str)
                .unwrap_or(".job-item"),
        )
        .map_err(ClientError::Configuration)?;

        let remaining_time = || {
            deadline
                .into_std()
                .checked_duration_since(std::time::Instant::now())
                .ok_or_else(|| {
                    ClientError::TimeLimit("Browser navigation reached its time limit".into())
                })
        };

        let initial_timeout = remaining_time()?;
        self.with_client(initial_timeout, |ws| {
            let ctx_res = call_cdp(
                ws,
                "Target.createBrowserContext",
                json!({ "disposeOnDetach": true }),
                None,
                remaining_time()?,
            )?;
            let context_id = ctx_res
                .get("browserContextId")
                .and_then(Value::as_str)
                .unwrap_or("default-ctx")
                .to_string();

            let target_res = match call_cdp(
                ws,
                "Target.createTarget",
                json!({
                    "url": "about:blank",
                    "browserContextId": context_id,
                }),
                None,
                remaining_time()?,
            ) {
                Ok(res) => res,
                Err(e) => {
                    let _ = ws.call(
                        "Target.disposeBrowserContext",
                        json!({ "browserContextId": context_id }),
                        None,
                        Duration::from_millis(500),
                    );
                    return Err(e);
                }
            };
            let target_id = target_res
                .get("targetId")
                .and_then(Value::as_str)
                .unwrap_or("default-target")
                .to_string();

            let result = render_inside_target(
                ws,
                client,
                requested_url,
                &transport_url,
                &target_id,
                &item_query,
                config,
                session_cookies,
                remaining_pages,
                &remaining_time,
                self.http_proxy.as_ref(),
            );

            // Always close target and dispose context immediately after the turn!
            let _ = ws.call(
                "Target.closeTarget",
                json!({ "targetId": target_id }),
                None,
                Duration::from_millis(500),
            );
            let _ = ws.call(
                "Target.disposeBrowserContext",
                json!({ "browserContextId": context_id }),
                None,
                Duration::from_millis(500),
            );
            ws.clear_proxy_auth();
            // This renderer has no notification consumer beyond the turn.
            while ws.take_event().is_some() {}

            result
        })
    }
}

impl Drop for BrowserPool {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.ws_client.lock() {
            if let Some(mut ws) = guard.take() {
                let _ = ws.call("Browser.close", json!({}), None, Duration::from_millis(500));
            }
        }
        if let Ok(mut child_guard) = self.child.lock() {
            if let Some(mut child) = child_guard.take() {
                kill_browser_child(&mut child);
            }
        }
    }
}

fn kill_browser_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id();
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[allow(clippy::too_many_arguments)]
fn render_inside_target(
    ws: &mut CdpWsClient,
    client: &HttpClient,
    requested_url: &Url,
    transport_url: &Url,
    target_id: &str,
    item_query: &QueryEngine,
    config: &StaticSourceConfig,
    session_cookies: &[BrowserCookie],
    remaining_pages: u32,
    remaining_time: &dyn Fn() -> Result<Duration, ClientError>,
    http_proxy: Option<&HttpProxyConfig>,
) -> Result<RenderedPage, ClientError> {
    let attach_res = call_cdp(
        ws,
        "Target.attachToTarget",
        json!({ "targetId": target_id, "flatten": true }),
        None,
        remaining_time()?,
    )?;
    let session_id = attach_res
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let sid = session_id.as_deref();
    if let Some(proxy) = http_proxy {
        ws.enable_proxy_auth(proxy, sid, remaining_time()?)
            .map_err(ClientError::Extraction)?;
    }

    call_cdp(ws, "Page.enable", json!({}), sid, remaining_time()?)?;
    call_cdp(ws, "Network.enable", json!({}), sid, remaining_time()?)?;
    call_cdp(
        ws,
        "Network.setBlockedURLs",
        json!({ "urls": BLOCKED_RESOURCE_URL_PATTERNS }),
        sid,
        remaining_time()?,
    )?;
    call_cdp(
        ws,
        "Emulation.setUserAgentOverride",
        user_agent_override_params(),
        sid,
        remaining_time()?,
    )?;
    call_cdp(
        ws,
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": STEALTH_INIT_SCRIPT }),
        sid,
        remaining_time()?,
    )?;

    if !session_cookies.is_empty() {
        let cdp_cookies: Vec<Value> = session_cookies
            .iter()
            .map(|c| {
                let mut obj = json!({
                    "name": c.name,
                    "value": c.value,
                    "domain": c.domain,
                    "path": c.path,
                    "secure": c.secure,
                    "httpOnly": c.http_only,
                });
                if let Some(exp) = c.expires {
                    obj["expires"] = json!(exp);
                }
                if let Some(same_site) = &c.same_site {
                    obj["sameSite"] = json!(same_site);
                }
                obj
            })
            .collect();
        call_cdp(
            ws,
            "Network.setCookies",
            json!({ "cookies": cdp_cookies }),
            sid,
            remaining_time()?,
        )?;
    }

    let navigation = call_cdp(
        ws,
        "Page.navigate",
        json!({ "url": transport_url.as_str() }),
        sid,
        remaining_time()?,
    )?;
    if http_proxy.is_some() && navigation.get("errorText").is_some() {
        return Err(ClientError::Extraction(
            "Chromium could not navigate through the configured proxy".into(),
        ));
    }

    let (mut snapshot, mut document) =
        wait_for_ready_snapshot(ws, sid, config.wait_for.as_ref(), remaining_time)?;
    if http_proxy.is_some() && snapshot.href.starts_with("chrome-error:") {
        return Err(ClientError::Extraction(
            "Chromium could not load the page through the configured proxy".into(),
        ));
    }

    let canonical_href = client.canonical_url(requested_url.as_str(), &snapshot.href);
    let has_items = !item_query.elements(&document, None).is_empty();
    if is_login_wall(
        requested_url,
        &canonical_href,
        &snapshot.title,
        snapshot.has_password,
        has_items,
    ) {
        return Err(ClientError::NeedsLogin(NEEDS_LOGIN_MESSAGE.to_string()));
    }

    let mut pages_consumed = 1u32;
    let mut page_limit_reached = false;

    if let (Some(load_more_query), Some(load_more_raw)) =
        (&config.load_more, config.load_more_raw.as_deref())
    {
        loop {
            if load_more_query.elements(&document, None).is_empty() {
                break;
            }
            if pages_consumed >= remaining_pages {
                page_limit_reached = true;
                break;
            }
            let before_count = item_query.elements(&document, None).len();
            let click_js = build_click_expression(load_more_raw);
            let click_res = call_cdp(
                ws,
                "Runtime.evaluate",
                json!({
                    "expression": click_js,
                    "returnByValue": true,
                }),
                sid,
                remaining_time()?,
            )?;
            let clicked = click_res
                .pointer("/result/value")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !clicked {
                break;
            }
            pages_consumed += 1;

            let (next_snap, next_doc) =
                wait_for_ready_snapshot(ws, sid, config.wait_for.as_ref(), remaining_time)?;
            let after_count = item_query.elements(&next_doc, None).len();
            snapshot = next_snap;
            document = next_doc;
            if after_count <= before_count {
                break;
            }
        }
    }

    let final_url = client.canonical_url(requested_url.as_str(), &snapshot.href);
    Ok(RenderedPage {
        html: snapshot.html,
        actual_url: final_url,
        pages_consumed,
        page_limit_reached,
    })
}

fn wait_for_ready_snapshot(
    ws: &mut CdpWsClient,
    sid: Option<&str>,
    wait_for: Option<&QueryEngine>,
    remaining_time: &dyn Fn() -> Result<Duration, ClientError>,
) -> Result<(DomSnapshot, Html), ClientError> {
    loop {
        let eval_res = call_cdp(
            ws,
            "Runtime.evaluate",
            json!({
                "expression": SNAPSHOT_JS,
                "returnByValue": true,
            }),
            sid,
            remaining_time()?,
        )?;
        let raw_json = eval_res
            .pointer("/result/value")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ClientError::Extraction("CDP Runtime.evaluate did not return DOM snapshot".into())
            })?;
        let snapshot: DomSnapshot = serde_json::from_str(raw_json).map_err(|e| {
            ClientError::Extraction(format!("Malformed CDP DOM snapshot JSON: {e}"))
        })?;
        let doc = Html::parse_document(&snapshot.html);
        let ready = (snapshot.ready_state == "complete" || snapshot.ready_state == "interactive")
            && !snapshot.html.is_empty()
            && wait_for.is_none_or(|q| !q.elements(&doc, None).is_empty());
        if ready {
            return Ok((snapshot, doc));
        }
        let rem = remaining_time()?;
        std::thread::sleep(rem.min(Duration::from_millis(25)));
    }
}

fn build_click_expression(raw_locator: &str) -> String {
    let trimmed = raw_locator.trim();
    if trimmed.starts_with("xpath:") || trimmed.starts_with('/') || trimmed.starts_with('(') {
        let expr = trimmed.strip_prefix("xpath:").unwrap_or(trimmed).trim();
        let escaped = serde_json::to_string(expr).unwrap_or_else(|_| "\"\"".into());
        format!(
            "(() => {{ const el = document.evaluate({escaped}, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue; if (!el || el.disabled || (el.getAttribute && el.getAttribute('aria-disabled') === 'true')) return false; el.click(); return true; }})()"
        )
    } else {
        let escaped = serde_json::to_string(trimmed).unwrap_or_else(|_| "\"\"".into());
        format!(
            "(() => {{ const el = document.querySelector({escaped}); if (!el || el.disabled || (el.getAttribute && el.getAttribute('aria-disabled') === 'true')) return false; el.click(); return true; }})()"
        )
    }
}

fn call_cdp(
    ws: &mut CdpWsClient,
    method: &str,
    params: Value,
    session_id: Option<&str>,
    timeout: Duration,
) -> Result<Value, ClientError> {
    ws.call(method, params, session_id, timeout).map_err(|e| {
        if e.contains("timed out") {
            ClientError::TimeLimit(e)
        } else {
            ClientError::Extraction(e)
        }
    })
}
