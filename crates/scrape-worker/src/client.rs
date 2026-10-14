use reqwest::{Client, StatusCode};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use texting_robots::Robot;
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use url::Url;

const USER_AGENT: &str = "Scrape/0.1.0 (local job tracker)";

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Network error: {0}")]
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

pub struct HttpClient {
    client: Client,
    replay_origin: Option<Url>,
    per_host_delay: Duration,
    host_locks: Mutex<HashMap<String, Arc<Mutex<Instant>>>>,
    robots_cache: Mutex<HashMap<String, Option<Arc<Robot>>>>,
    json_cache: Mutex<HashMap<String, (Instant, serde_json::Value)>>,
}

impl HttpClient {
    pub fn new(timeout_s: u64, per_host_delay_ms: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_s))
            .user_agent(USER_AGENT)
            .gzip(true)
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            replay_origin: None,
            per_host_delay: Duration::from_millis(per_host_delay_ms),
            host_locks: Mutex::new(HashMap::new()),
            robots_cache: Mutex::new(HashMap::new()),
            json_cache: Mutex::new(HashMap::new()),
        }
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
        if elapsed < self.per_host_delay {
            sleep(self.per_host_delay - elapsed).await;
        }
        *last_req = Instant::now();
        last_req
    }

    pub async fn check_robots(&self, parsed_url: &Url) -> Result<(), ClientError> {
        let host = match parsed_url.host_str() {
            Some(h) => h,
            None => return Ok(()),
        };

        let cached_robot = {
            let cache = self.robots_cache.lock().await;
            cache.get(host).cloned()
        };

        let robot = match cached_robot {
            Some(r) => r,
            None => {
                let robots_url = format!("{}://{}/robots.txt", parsed_url.scheme(), host);
                let resp = self
                    .client
                    .get(self.transport_url(&robots_url)?)
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
                cache.insert(host.to_string(), parsed_robot.clone());
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
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
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
                        tracing::warn!(
                            "Received status {}, retrying in {:?} (attempt {})",
                            status,
                            backoff,
                            attempts
                        );
                        sleep(backoff).await;
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

    pub async fn fetch_text(&self, url: &str, check_robots: bool) -> Result<String, ClientError> {
        let parsed_url = Url::parse(url)?;
        let host = parsed_url.host_str().unwrap_or("").to_string();

        if check_robots {
            self.check_robots(&parsed_url).await?;
        }

        let _guard = self.rate_limit(&host).await;

        let transport = self.transport_url(url)?;
        let resp = self
            .execute_with_retries(|| self.client.get(transport.clone()).send())
            .await?;

        let text = resp.text().await?;
        Ok(text)
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
            .await?;

        let json = resp.json::<T>().await?;
        Ok(json)
    }
}
