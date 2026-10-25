use reqwest::{Client, StatusCode};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use texting_robots::Robot;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use url::Url;

const USER_AGENT: &str = "Scrape/0.1.0 (local job tracker)";

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Source extraction failed: {0}")]
    Extraction(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Time limit reached: {0}")]
    TimeLimit(String),
    #[error("Login required: {0}")]
    NeedsLogin(String),
    #[error("Browser runtime missing: {0}")]
    BrowserRuntimeMissing(String),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("Disallowed by robots.txt: {0}")]
    RobotsDisallowed(String),
    #[error("HTTP error {0}: {1}")]
    HttpStatus(StatusCode, String),
    #[error("JSON decode error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    credential_client: Client,
    crawl_client: Client,
    replay_origin: Option<Url>,
    proxy_configured: bool,
    proxy: Option<scrape_core::protocol::HttpProxyConfig>,
    per_host_delay: Duration,
    jitter: Option<Duration>,
    respect_robots: bool,
    host_locks: Arc<Mutex<HashMap<String, Arc<Mutex<Instant>>>>>,
    robots_cache: Arc<Mutex<HashMap<String, Option<Arc<Robot>>>>>,
    json_cache: Arc<Mutex<HashMap<String, (Instant, serde_json::Value)>>>,
    timeout: Duration,
}

fn build_client(
    timeout: Duration,
    proxy: Option<&scrape_core::protocol::HttpProxyConfig>,
    redirects: bool,
) -> Result<Client, ClientError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,application/json;q=0.8,*/*;q=0.7",
        ),
    );
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
    );

    let mut builder = Client::builder()
        .timeout(timeout)
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .gzip(true);
    if !redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    if let Some(proxy) = proxy {
        let mut configured = reqwest::Proxy::all(&proxy.server)
            .map_err(|_| ClientError::Configuration("Invalid HTTP proxy configuration".into()))?
            .no_proxy(None);
        if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
            configured = configured.basic_auth(username, password);
        }
        builder = builder.proxy(configured);
    }
    builder
        .build()
        .map_err(|_| ClientError::Configuration("HTTP transport initialization failed".into()))
}

impl HttpClient {
    pub fn new(timeout_s: u64, per_host_delay_ms: u64) -> Self {
        let timeout = Duration::from_secs(timeout_s);
        let default_jitter = if per_host_delay_ms > 0 {
            Some(Duration::from_millis((per_host_delay_ms / 3).clamp(50, 500)))
        } else {
            None
        };
        Self {
            client: build_client(timeout, None, true).expect("Failed to build reqwest client"),
            credential_client: build_client(timeout, None, false)
                .expect("Failed to build credential-safe reqwest client"),
            crawl_client: build_client(timeout, None, false)
                .expect("Failed to build crawl reqwest client"),
            replay_origin: None,
            proxy_configured: false,
            proxy: None,
            per_host_delay: Duration::from_millis(per_host_delay_ms),
            jitter: default_jitter,
            respect_robots: true,
            host_locks: Arc::new(Mutex::new(HashMap::new())),
            robots_cache: Arc::new(Mutex::new(HashMap::new())),
            json_cache: Arc::new(Mutex::new(HashMap::new())),
            timeout,
        }
    }

    pub fn for_static_source(&self, config: &scrape_core::static_source::StaticSourceConfig) -> Self {
        let mut cloned = self.clone();
        if let Some(delay) = config.per_host_delay_ms {
            cloned.per_host_delay = Duration::from_millis(delay);
        }
        if let Some(jitter) = config.jitter_ms {
            cloned.jitter = Some(Duration::from_millis(jitter));
        }
        if config.ignore_robots {
            cloned.respect_robots = false;
        }
        cloned
    }

    pub fn with_jitter_ms(mut self, jitter_ms: Option<u64>) -> Self {
        self.jitter = jitter_ms.map(Duration::from_millis);
        self
    }

    pub fn with_respect_robots(mut self, respect: bool) -> Self {
        self.respect_robots = respect;
        self
    }

    pub fn with_proxy(
        mut self,
        proxy: Option<&scrape_core::protocol::HttpProxyConfig>,
    ) -> Result<Self, ClientError> {
        let proxy = proxy
            .cloned()
            .map(scrape_core::protocol::HttpProxyConfig::validate_with_socks)
            .transpose()
            .map_err(ClientError::Configuration)?;
        self.client = build_client(self.timeout, proxy.as_ref(), true)?;
        self.credential_client = build_client(self.timeout, proxy.as_ref(), false)?;
        self.crawl_client = build_client(self.timeout, proxy.as_ref(), false)?;
        self.proxy = proxy;
        self.proxy_configured = self.proxy.is_some();
        Ok(self)
    }

    fn transport_failure(&self) -> ClientError {
        ClientError::Extraction(if self.proxy_configured {
            "HTTP proxy request failed".into()
        } else {
            "HTTP request failed".into()
        })
    }

    fn request_failure(&self, error: ClientError) -> ClientError {
        match error {
            ClientError::HttpStatus(status, _) => {
                ClientError::HttpStatus(status, "HTTP response".into())
            }
            _ => self.transport_failure(),
        }
    }

    /// Fetch a static page while refusing to follow redirects outside its origin.
    /// Each hop is independently checked against robots and destination pacing.
    pub async fn fetch_text_same_origin_at(
        &self,
        url: &str,
        origin: &Url,
        check_robots: bool,
    ) -> Result<(String, String), ClientError> {
        let mut requested = Url::parse(url)?;
        if !Self::same_origin(origin, &requested) {
            return Err(ClientError::Configuration(
                "Crawl destination is outside the seed origin".into(),
            ));
        }
        for redirect_count in 0..=5 {
            if check_robots {
                self.check_robots(&requested).await?;
            }
            let _guard = self.rate_limit(requested.host_str().unwrap_or("")).await;
            let transport = self.transport_url(requested.as_str())?;
            let response = self
                .crawl_client
                .get(transport)
                .send()
                .await
                .map_err(|_| self.transport_failure())?;
            if response.status().is_redirection() {
                if redirect_count == 5 {
                    return Err(ClientError::Extraction(
                        "Crawl redirect limit reached".into(),
                    ));
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .ok_or_else(|| {
                        ClientError::Extraction(
                            "Crawl redirect did not include a destination".into(),
                        )
                    })?;
                let location = location.to_str().map_err(|_| {
                    ClientError::Extraction("Crawl redirect destination was invalid".into())
                })?;
                let next = requested.join(location).map_err(|_| {
                    ClientError::Extraction("Crawl redirect destination was invalid".into())
                })?;
                if !Self::same_origin(origin, &next) {
                    return Err(ClientError::Configuration(
                        "Crawl redirect was outside the seed origin".into(),
                    ));
                }
                requested = next;
                continue;
            }
            if !response.status().is_success() {
                return Err(ClientError::HttpStatus(
                    response.status(),
                    "Crawl page request failed".into(),
                ));
            }
            let text = response
                .text()
                .await
                .map_err(|_| self.transport_failure())?;
            return Ok((text, requested.to_string()));
        }
        unreachable!("bounded crawl redirect loop")
    }

    /// Explicit offline transport; parsing and identity still use provider URLs.
    pub fn with_replay_origin(mut self, origin: &str) -> Result<Self, String> {
        let url = Url::parse(origin).map_err(|e| e.to_string())?;
        if url.scheme() != "http"
            || !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err("Replay origin must be a plain loopback HTTP origin".into());
        }
        self.replay_origin = Some(url);
        Ok(self)
    }

    fn transport_url(&self, original: &str) -> Result<Url, ClientError> {
        let parsed = Url::parse(original)?;
        if let Some(origin) = &self.replay_origin {
            let mut replay = origin.clone();
            replay.set_path(&format!(
                "/{}{}",
                parsed.host_str().unwrap_or(""),
                parsed.path()
            ));
            replay.set_query(parsed.query());
            Ok(replay)
        } else {
            Ok(parsed)
        }
    }

    fn same_origin(left: &Url, right: &Url) -> bool {
        left.scheme() == right.scheme()
            && left.host_str() == right.host_str()
            && left.port_or_known_default() == right.port_or_known_default()
    }

    pub async fn prepare_navigation(
        &self,
        url: &str,
        check_robots: bool,
    ) -> Result<Url, ClientError> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url.host_str().unwrap_or("").to_string();
        if check_robots
            && (self.replay_origin.is_some() || std::env::var_os("SCRAPE_CDP_WS_URL").is_none())
        {
            self.check_robots(&parsed_url).await?;
        }
        let _guard = self.rate_limit(&host).await;
        self.transport_url(url)
    }

    pub fn canonical_url(&self, requested: &str, actual: &str) -> String {
        if actual.trim().is_empty() {
            return requested.to_owned();
        }
        if let Some(origin) = &self.replay_origin {
            let origin_prefix = origin.as_str().trim_end_matches('/');
            if let Some(rest) = actual.strip_prefix(origin_prefix) {
                let rest = rest.trim_start_matches('/');
                if !rest.is_empty() {
                    let scheme = Url::parse(requested)
                        .map(|u| u.scheme().to_string())
                        .unwrap_or_else(|_| "https".to_string());
                    return format!("{scheme}://{rest}");
                }
                return requested.to_owned();
            }
        }
        actual.to_owned()
    }

    async fn get_host_lock(&self, host: &str) -> Arc<Mutex<Instant>> {
        let mut locks = self.host_locks.lock().await;
        locks
            .entry(host.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(Instant::now() - self.per_host_delay)))
            .clone()
    }

    async fn rate_limit(&self, host: &str) -> tokio::sync::OwnedMutexGuard<Instant> {
        let host_lock = self.get_host_lock(host).await;
        let mut last_req = host_lock.lock_owned().await;
        let elapsed = last_req.elapsed();
        let jitter_millis = self
            .jitter
            .map(|j| {
                let max_j = j.as_millis() as u64;
                if max_j > 0 {
                    use std::time::SystemTime;
                    let nanos = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.subsec_nanos() as u64)
                        .unwrap_or(0);
                    nanos % (max_j + 1)
                } else {
                    0
                }
            })
            .unwrap_or(0);
        let target_delay = self.per_host_delay + Duration::from_millis(jitter_millis);
        if elapsed < target_delay {
            sleep(target_delay - elapsed).await;
        }
        *last_req = Instant::now();
        last_req
    }

    pub async fn check_robots(&self, parsed_url: &Url) -> Result<(), ClientError> {
        if !self.respect_robots {
            return Ok(());
        }
        let host = match parsed_url.host_str() {
            Some(h) => h,
            None => return Ok(()),
        };
        let origin_key = parsed_url.origin().ascii_serialization();
        let cached_robot = {
            let cache = self.robots_cache.lock().await;
            cache.get(&origin_key).cloned()
        };

        let robot = match cached_robot {
            Some(r) => r,
            None => {
                let _guard = self.rate_limit(host).await;
                let mut robots_url = parsed_url.clone();
                robots_url.set_path("/robots.txt");
                robots_url.set_query(None);
                robots_url.set_fragment(None);
                let resp = self
                    .crawl_client
                    .get(self.transport_url(robots_url.as_str())?)
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await;
                let parsed_robot = match resp {
                    Ok(r) if r.status().is_success() => {
                        let text = r.text().await.unwrap_or_default();
                        Robot::new("Scrape", text.as_bytes()).ok().map(Arc::new)
                    }
                    _ => None,
                };
                let mut cache = self.robots_cache.lock().await;
                cache.insert(origin_key, parsed_robot.clone());
                parsed_robot
            }
        };

        if let Some(r) = robot {
            let path = parsed_url.path();
            if !r.allowed(path) {
                return Err(ClientError::RobotsDisallowed(parsed_url.to_string()));
            }
        }

        Ok(())
    }

    async fn execute_with_retries<F, Fut>(
        &self,
        make_request: F,
    ) -> Result<reqwest::Response, ClientError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let mut attempts = 0;
        let mut backoff = Duration::from_millis(1000);

        loop {
            attempts += 1;
            match make_request().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }
                    if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                        && attempts < 3
                    {
                        let wait_duration = if status == StatusCode::TOO_MANY_REQUESTS {
                            resp.headers()
                                .get(reqwest::header::RETRY_AFTER)
                                .and_then(|v| v.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .map(Duration::from_secs)
                                .unwrap_or(backoff)
                        } else {
                            backoff
                        };
                        tracing::warn!(
                            "Received status {}, retrying in {:?} (attempt {})",
                            status,
                            wait_duration,
                            attempts
                        );
                        sleep(wait_duration).await;
                        backoff *= 2;
                        continue;
                    }
                    let body = resp.text().await.unwrap_or_default();
                    return Err(ClientError::HttpStatus(status, body));
                }
                Err(err) => {
                    if attempts < 3 && (err.is_timeout() || err.is_connect()) {
                        tracing::warn!(
                            "Request error {:?}, retrying in {:?} (attempt {})",
                            err,
                            backoff,
                            attempts
                        );
                        sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }
                    return Err(ClientError::Reqwest(err));
                }
            }
        }
    }

    pub async fn fetch_text_at(
        &self,
        url: &str,
        check_robots: bool,
    ) -> Result<(String, String), ClientError> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url.host_str().unwrap_or("").to_string();

        if check_robots {
            self.check_robots(&parsed_url).await?;
        }

        let _guard = self.rate_limit(&host).await;

        let transport = self.transport_url(url)?;
        let resp = self
            .execute_with_retries(|| self.client.get(transport.clone()).send())
            .await
            .map_err(|error| self.request_failure(error))?;

        let actual = if self.replay_origin.is_some() {
            url.to_owned()
        } else {
            resp.url().to_string()
        };
        let text = resp.text().await.map_err(|_| self.transport_failure())?;
        Ok((text, actual))
    }

    pub async fn fetch_text(&self, url: &str, check_robots: bool) -> Result<String, ClientError> {
        self.fetch_text_at(url, check_robots)
            .await
            .map(|(text, _)| text)
    }

    /// Cache detail responses for this worker lifetime, with a bounded size and TTL.
    pub async fn fetch_json_cached<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, ClientError> {
        let mut cache = self.json_cache.lock().await;
        if let Some((fetched, value)) = cache.get(url) {
            if fetched.elapsed() < Duration::from_secs(300) {
                return Ok(serde_json::from_value(value.clone())?);
            }
        }
        cache.remove(url);
        drop(cache);
        let value: serde_json::Value = self.fetch_json(url).await?;
        let result = serde_json::from_value(value.clone())?;
        let mut cache = self.json_cache.lock().await;
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(url.into(), (Instant::now(), value));
        Ok(result)
    }

    /// Adzuna credentials may only leave this process for its exact HTTPS collection.
    /// Do not use execute_with_retries: reqwest diagnostics and response bodies can
    /// contain the complete credential-bearing URL. Every error here is fixed text.
    pub async fn fetch_adzuna_json(&self, url: &Url) -> Result<serde_json::Value, ClientError> {
        let page = url.path().strip_prefix("/v1/api/jobs/us/search/");
        if url.scheme() != "https"
            || url.host_str() != Some("api.adzuna.com")
            || url.port().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || !page.is_some_and(|page| {
                !page.is_empty()
                    && page.bytes().all(|byte| byte.is_ascii_digit())
                    && page.parse::<u32>().is_ok_and(|page| page > 0)
            })
        {
            return Err(ClientError::Configuration(
                "Adzuna requires its fixed HTTPS search collection".into(),
            ));
        }
        let transport = self
            .transport_url(url.as_str())
            .map_err(|_| ClientError::Extraction("Adzuna transport configuration failed".into()))?;
        let _guard = self.rate_limit("api.adzuna.com").await;
        let mut backoff = Duration::from_secs(1);
        for attempt in 1..=3 {
            let response = match self.credential_client.get(transport.clone()).send().await {
                Ok(response) => response,
                Err(error) => {
                    let class = if error.is_timeout() {
                        "timeout"
                    } else if error.is_connect() {
                        "connection"
                    } else {
                        "transport"
                    };
                    if attempt < 3 && (error.is_timeout() || error.is_connect()) {
                        sleep(backoff).await;
                        backoff *= 2;
                        continue;
                    }
                    return Err(ClientError::Extraction(format!("Adzuna {class} error")));
                }
            };
            let status = response.status();
            if !status.is_success() {
                if attempt < 3
                    && (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                {
                    sleep(backoff).await;
                    backoff *= 2;
                    continue;
                }
                return Err(ClientError::Extraction(format!(
                    "Adzuna HTTP status {}",
                    status.as_u16()
                )));
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|_| ClientError::Extraction("Adzuna response transport error".into()))?;
            return serde_json::from_slice(&bytes)
                .map_err(|_| ClientError::Extraction("Adzuna invalid JSON response".into()));
        }
        unreachable!()
    }

    pub async fn fetch_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, ClientError> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url.host_str().unwrap_or("").to_string();

        let _guard = self.rate_limit(&host).await;

        let transport = self.transport_url(url)?;
        let resp = self
            .execute_with_retries(|| self.client.get(transport.clone()).send())
            .await
            .map_err(|error| self.request_failure(error))?;
        let bytes = resp.bytes().await.map_err(|_| self.transport_failure())?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn post_json<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url.host_str().unwrap_or("").to_string();

        let _guard = self.rate_limit(&host).await;

        let transport = self.transport_url(url)?;
        let resp = self
            .execute_with_retries(|| {
                self.client
                    .post(transport.clone())
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json")
                    .json(body)
                    .send()
            })
            .await
            .map_err(|error| self.request_failure(error))?;
        let bytes = resp.bytes().await.map_err(|_| self.transport_failure())?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

impl HttpClient {
    /// Decoding never relies on Content-Type. Bound decompressed bytes while reading.
    pub async fn fetch_logo(&self, raw: &str) -> Result<Vec<u8>, String> {
        if !scrape_core::branding::http_url(raw) {
            return Err("Unsafe logo URL".into());
        }
        let url = Url::parse(raw).map_err(|e| e.to_string())?;
        self.check_robots(&url).await.map_err(|e| e.to_string())?;
        let _guard = self.rate_limit(url.host_str().unwrap_or("")).await;
        let mut builder = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(4))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5
                    || !scrape_core::branding::http_url(attempt.url().as_str())
                {
                    attempt.error("Unsafe or excessive logo redirect")
                } else {
                    attempt.follow()
                }
            }));
        if let Some(proxy) = &self.proxy {
            let mut configured = reqwest::Proxy::all(&proxy.server)
                .map_err(|_| "HTTP proxy configuration failed".to_string())?
                .no_proxy(None);
            if let (Some(username), Some(password)) = (&proxy.username, &proxy.password) {
                configured = configured.basic_auth(username, password);
            }
            builder = builder.proxy(configured);
        }
        let client = builder
            .build()
            .map_err(|_| "HTTP transport initialization failed".to_string())?;
        let mut response = client
            .get(self.transport_url(raw).map_err(|e| e.to_string())?)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        if response
            .content_length()
            .is_some_and(|n| n > scrape_core::branding::MAX_LOGO_BYTES as u64)
        {
            return Err("Logo exceeds byte limit".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if bytes.len() + chunk.len() > scrape_core::branding::MAX_LOGO_BYTES {
                return Err("Logo exceeds byte limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

impl HttpClient {
    pub async fn fetch_branding_page(&self, raw: &str) -> Result<(String, String), String> {
        if !scrape_core::branding::http_url(raw) {
            return Err("Unsafe branding page URL".into());
        }
        let url = Url::parse(raw).map_err(|e| e.to_string())?;
        self.check_robots(&url).await.map_err(|e| e.to_string())?;
        let _guard = self.rate_limit(url.host_str().unwrap_or("")).await;
        let mut response = self
            .client
            .get(self.transport_url(raw).map_err(|e| e.to_string())?)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let actual = if self.replay_origin.is_some() {
            raw.to_owned()
        } else {
            response.url().to_string()
        };
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
                return Err("Branding page exceeds 2 MiB".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok((String::from_utf8_lossy(&bytes).into_owned(), actual))
    }
}

#[cfg(test)]
mod proxy_tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        thread,
    };

    #[tokio::test]
    async fn authenticated_proxy_receives_absolute_request_and_basic_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            {
                let mut reader = BufReader::new(&mut stream);
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    request.push_str(&line);
                }
            }
            let body = "fixture";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            request
        });
        let proxy = scrape_core::protocol::HttpProxyConfig {
            server: endpoint,
            username: Some("fixture-user".into()),
            password: Some("fixture-password".into()),
        };
        let client = HttpClient::new(2, 0).with_proxy(Some(&proxy)).unwrap();
        let origin = Url::parse("http://jobs.example.test/").unwrap();
        let (body, actual) = client
            .fetch_text_same_origin_at("http://jobs.example.test/list", &origin, false)
            .await
            .unwrap();
        let request = server.join().unwrap();
        assert_eq!(body, "fixture");
        assert_eq!(actual, "http://jobs.example.test/list");
        assert!(request.starts_with("GET http://jobs.example.test/list HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("proxy-authorization: basic "));
        assert!(!request.contains("fixture-password"));
    }

    #[tokio::test]
    async fn crawl_redirect_is_rejected_before_cross_origin_request() {
        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let target_endpoint = format!("http://{}", target.local_addr().unwrap());
        let target = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_millis(200);
            loop {
                match target.accept() {
                    Ok(_) => return true,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            return false;
                        }
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => return true,
                }
            }
        });
        let seed = TcpListener::bind("127.0.0.1:0").unwrap();
        let seed_endpoint = format!("http://{}", seed.local_addr().unwrap());
        let seed_server = thread::spawn(move || {
            let (mut stream, _) = seed.accept().unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
                line.clear();
            }
            drop(reader);
            write!(
                stream,
                "HTTP/1.1 302 Found\r\nLocation: {target_endpoint}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });
        let origin = Url::parse(&format!("{seed_endpoint}/")).unwrap();
        let result = HttpClient::new(2, 0)
            .fetch_text_same_origin_at(&format!("{seed_endpoint}/"), &origin, false)
            .await;
        assert!(result.is_err());
        seed_server.join().unwrap();
        assert!(
            !target.join().unwrap(),
            "cross-origin redirect was followed"
        );
    }
    #[tokio::test]
    async fn unavailable_proxy_fails_without_connecting_to_loopback_origin() {
        let origin_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        origin_listener.set_nonblocking(true).unwrap();
        let origin = Url::parse(&format!(
            "http://{}/",
            origin_listener.local_addr().unwrap()
        ))
        .unwrap();
        let proxy_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let proxy_endpoint = format!("http://{}", proxy_listener.local_addr().unwrap());
        drop(proxy_listener);
        let proxy = scrape_core::protocol::HttpProxyConfig {
            server: proxy_endpoint,
            username: Some("fixture-user".into()),
            password: Some("fixture-password".into()),
        };
        let error = HttpClient::new(2, 0)
            .with_proxy(Some(&proxy))
            .unwrap()
            .fetch_text_same_origin_at(origin.as_str(), &origin, false)
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(error, "Source extraction failed: HTTP proxy request failed");
        assert!(!error.contains("fixture-user"));
        assert!(!error.contains("fixture-password"));
        assert!(
            origin_listener.accept().is_err(),
            "configured proxy failure fell back to the direct origin"
        );
    }
    #[tokio::test]
    async fn fetch_status_errors_keep_status_without_response_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            while reader.read_line(&mut line).unwrap() > 0 && line != "\r\n" {
                line.clear();
            }
            drop(reader);
            write!(
                stream,
                "HTTP/1.1 404 Not Found\r\nContent-Length: 13\r\nConnection: close\r\n\r\nprivate body!"
            )
            .unwrap();
        });
        let error = HttpClient::new(2, 0)
            .fetch_text_at(&endpoint, false)
            .await
            .unwrap_err()
            .to_string();
        server.join().unwrap();
        assert!(error.contains("404"), "{error}");
        assert!(!error.contains("private body"));
    }

    #[tokio::test]
    async fn disabled_robots_bypasses_network_check_completely() {
        let client = HttpClient::new(2, 0).with_respect_robots(false);
        let url = Url::parse("http://127.0.0.1:1/disallowed-path").unwrap();
        assert!(client.check_robots(&url).await.is_ok());
    }

    #[tokio::test]
    async fn rate_limit_with_jitter_paces_consecutive_requests() {
        let client = HttpClient::new(2, 5).with_jitter_ms(Some(10));
        let start = std::time::Instant::now();
        {
            let _g1 = client.rate_limit("test-host").await;
        }
        {
            let _g2 = client.rate_limit("test-host").await;
        }
        assert!(start.elapsed() >= Duration::from_millis(5));
    }
}
