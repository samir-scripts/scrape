//! Isolated, public-only HTTP transport for independent discovery.
//!
//! Each request gets a fresh, DNS-pinned, proxy-free client. The budget counts
//! every attempted HTTP request, including robots requests and redirect hops.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::Duration;

use reqwest::{header, StatusCode};
use texting_robots::Robot;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::{sleep_until, Instant};
use url::Url;

const USER_AGENT: &str = "Scrape/0.1.0 (local job tracker)";
const BODY_LIMIT: usize = 2 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

#[derive(Debug, thiserror::Error)]
pub enum PublicWebError {
    #[error("invalid or non-public destination: {0}")]
    Destination(String),
    #[error("DNS lookup failed: {0}")]
    Dns(String),
    #[error("public request budget exhausted")]
    BudgetExhausted,
    #[error("global public request budget exhausted")]
    GlobalBudgetExhausted,
    #[error("public origin budget exhausted: {0}")]
    OriginLimit(String),
    #[error("robots policy unavailable: {0}")]
    RobotsUnavailable(String),
    #[error("robots policy denies: {0}")]
    RobotsDenied(String),
    #[error("request failed: {0}")]
    Transport(String),
    #[error("HTTP status {0}")]
    Status(StatusCode),
    #[error("invalid redirect or redirect limit exceeded")]
    Redirect,
    #[error("unsupported content type")]
    ContentType,
    #[error("response exceeds the size limit")]
    BodyTooLarge,
    #[error("response is not UTF-8")]
    InvalidText,
}

#[derive(Debug)]
pub struct PublicPage {
    pub url: Url,
    pub body: String,
    pub robots_sitemaps: Vec<String>,
}

#[derive(Debug)]
pub struct PublicSitemap {
    pub url: Url,
    pub body: Vec<u8>,
}

struct RobotsPolicy {
    robot: Robot,
    sitemaps: Vec<String>,
}

struct OriginState {
    next_request: Instant,
    robots: Option<Result<RobotsPolicy, String>>,
}

struct Shared {
    max_requests: usize,
    requests: AtomicUsize,
    timeout: Duration,
    delay: Duration,
    slots: Semaphore,
    origins: Mutex<HashMap<String, Arc<Mutex<OriginState>>>>,
    replay: Option<Url>,
}

/// Clones share the request budget, robots cache, pacing and origin locks.
#[derive(Clone)]
pub struct PublicWeb(Arc<Shared>, Arc<Budget>);

struct Budget {
    max_requests: usize,
    requests: AtomicUsize,
    max_origins: usize,
    origins: StdMutex<HashSet<String>>,
}

impl PublicWeb {
    pub fn new(max_requests: usize, timeout: Duration, per_origin_delay: Duration) -> Self {
        Self(
            Arc::new(Shared {
                max_requests,
                requests: AtomicUsize::new(0),
                timeout,
                delay: per_origin_delay,
                slots: Semaphore::new(4),
                origins: Mutex::new(HashMap::new()),
                replay: None,
            }),
            Arc::new(Budget {
                max_requests,
                requests: AtomicUsize::new(0),
                max_origins: usize::MAX,
                origins: StdMutex::new(HashSet::new()),
            }),
        )
    }

    /// Allocate a source budget within this run, sharing its scheduler and
    /// origin state. Clone a source handle to share the source budget too.
    pub fn for_source(&self, max_requests: usize) -> Self {
        self.for_source_with_origin_limit(max_requests, usize::MAX)
    }

    /// Allocate a source budget whose origin cap also covers redirect hops.
    pub fn for_source_with_origin_limit(&self, max_requests: usize, max_origins: usize) -> Self {
        Self(
            self.0.clone(),
            Arc::new(Budget {
                max_requests,
                requests: AtomicUsize::new(0),
                max_origins,
                origins: StdMutex::new(HashSet::new()),
            }),
        )
    }

    pub fn origins_used(&self) -> usize {
        self.1
            .origins
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    fn admit_origin(&self, url: &Url) -> Result<(), PublicWebError> {
        let key = url.origin().ascii_serialization();
        let mut origins = self.1.origins.lock().unwrap_or_else(|e| e.into_inner());
        if !origins.contains(&key) {
            if origins.len() >= self.1.max_origins {
                return Err(PublicWebError::OriginLimit(key));
            }
            origins.insert(key);
        }
        Ok(())
    }

    /// Explicit offline fixture transport. Only wire this to the worker's
    /// test replay CLI, never to workflow configuration or an IPC argument.
    pub fn with_test_replay_origin(mut self, origin: &str) -> Result<Self, PublicWebError> {
        let url = Url::parse(origin).map_err(|_| PublicWebError::Destination(origin.into()))?;
        if url.scheme() != "http"
            || !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(PublicWebError::Destination(origin.into()));
        }
        let shared = Arc::get_mut(&mut self.0).ok_or_else(|| {
            PublicWebError::Transport("replay must be configured before cloning".into())
        })?;
        shared.replay = Some(url);
        Ok(self)
    }

    pub fn requests_used(&self) -> usize {
        self.1.requests.load(Ordering::Relaxed)
    }

    pub fn global_requests_used(&self) -> usize {
        self.0.requests.load(Ordering::Relaxed)
    }

    fn charge(&self) -> Result<(), PublicWebError> {
        self.1
            .requests
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < self.1.max_requests).then(|| n + 1)
            })
            .map_err(|_| PublicWebError::BudgetExhausted)?;
        if self
            .0
            .requests
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
                (n < self.0.max_requests).then(|| n + 1)
            })
            .is_err()
        {
            self.1.requests.fetch_sub(1, Ordering::Relaxed);
            return Err(PublicWebError::GlobalBudgetExhausted);
        }
        Ok(())
    }

    async fn origin(&self, url: &Url) -> Arc<Mutex<OriginState>> {
        let key = url.origin().ascii_serialization();
        let mut origins = self.0.origins.lock().await;
        origins
            .entry(key)
            .or_insert_with(|| {
                Arc::new(Mutex::new(OriginState {
                    next_request: Instant::now(),
                    robots: None,
                }))
            })
            .clone()
    }

    /// Fetch an HTML page, checking robots and the destination anew at every hop.
    /// Redirected-to origins consume their own robots request and pacing budget.
    pub async fn fetch(&self, input: &str) -> Result<PublicPage, PublicWebError> {
        let (url, bytes, robots_sitemaps) = self.fetch_inner(input, false).await?;
        let body = String::from_utf8(bytes).map_err(|_| PublicWebError::InvalidText)?;
        Ok(PublicPage {
            url,
            body,
            robots_sitemaps,
        })
    }

    /// Fetch XML, text, or gzip sitemap bytes. XML parsing, including DTD and
    /// external-entity rejection, is the caller's responsibility.
    pub async fn fetch_sitemap(&self, input: &str) -> Result<PublicSitemap, PublicWebError> {
        let (url, mut body, _) = self.fetch_inner(input, true).await?;
        if body.starts_with(&[0x1f, 0x8b]) {
            let mut decoded = Vec::new();
            flate2::read::MultiGzDecoder::new(body.as_slice())
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut decoded)
                .map_err(|e| PublicWebError::Transport(e.to_string()))?;
            if decoded.len() > 8 * 1024 * 1024 {
                return Err(PublicWebError::BodyTooLarge);
            }
            body = decoded;
        }
        Ok(PublicSitemap { url, body })
    }

    async fn fetch_inner(
        &self,
        input: &str,
        sitemap: bool,
    ) -> Result<(Url, Vec<u8>, Vec<String>), PublicWebError> {
        let mut url = Url::parse(input).map_err(|_| PublicWebError::Destination(input.into()))?;
        for redirects in 0..=MAX_REDIRECTS {
            validate_url(&url)?;
            self.admit_origin(&url)?;
            url.set_fragment(None);
            let origin = self.origin(&url).await;
            let mut state = origin.lock().await;
            let _permit = self
                .0
                .slots
                .acquire()
                .await
                .map_err(|_| PublicWebError::Transport("request scheduler closed".into()))?;
            if state.robots.is_none() {
                let mut robots_url = url.clone();
                robots_url.set_path("/robots.txt");
                robots_url.set_query(None);
                // Fail closed on redirects too: they are not permission to bypass a
                // destination's own robots policy.
                let result = match self.request(&robots_url, &mut state).await {
                    Ok(response) if response.status().is_success() => {
                        match bounded_body(response, false).await {
                            Ok(bytes) => Robot::new("Scrape", &bytes)
                                .map(|robot| RobotsPolicy {
                                    robot,
                                    sitemaps: String::from_utf8_lossy(&bytes)
                                        .lines()
                                        .filter_map(|line| {
                                            let (key, value) = line.split_once(':')?;
                                            (key.trim().eq_ignore_ascii_case("sitemap")
                                                && !value.trim().is_empty())
                                            .then(|| value.trim().to_owned())
                                        })
                                        .take(1000)
                                        .collect(),
                                })
                                .map_err(|e| format!("{e:?}")),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    Ok(response) => Err(format!("HTTP {}", response.status())),
                    Err(e) => {
                        if matches!(
                            e,
                            PublicWebError::BudgetExhausted | PublicWebError::GlobalBudgetExhausted
                        ) {
                            return Err(e);
                        }
                        Err(e.to_string())
                    }
                };
                if let Err(reason) = &result {
                    tracing::warn!(origin = %url.origin().ascii_serialization(), %reason, "public robots unavailable; denying origin");
                }
                state.robots = Some(result);
            }
            let policy = match state.robots.as_ref().expect("robots populated") {
                Ok(policy) => policy,
                Err(reason) => return Err(PublicWebError::RobotsUnavailable(reason.clone())),
            };
            let mut target = url.path().to_owned();
            if let Some(query) = url.query() {
                target.push('?');
                target.push_str(query);
            }
            if !policy.robot.allowed(&target) {
                return Err(PublicWebError::RobotsDenied(url.to_string()));
            }
            let robots_sitemaps = policy.sitemaps.clone();
            let response = self.request(&url, &mut state).await?;
            if response.status().is_redirection() {
                if redirects == MAX_REDIRECTS {
                    return Err(PublicWebError::Redirect);
                }
                let location = response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or(PublicWebError::Redirect)?;
                let next = url.join(location).map_err(|_| PublicWebError::Redirect)?;
                validate_url(&next)?;
                drop(state);
                url = next;
                continue;
            }
            if !response.status().is_success() {
                return Err(PublicWebError::Status(response.status()));
            }
            let content_type = response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .split(';')
                .next()
                .unwrap_or("")
                .trim();
            let allowed = if sitemap {
                [
                    "application/xml",
                    "text/xml",
                    "text/plain",
                    "application/gzip",
                    "application/x-gzip",
                    "application/octet-stream",
                ]
                .iter()
                .any(|kind| content_type.eq_ignore_ascii_case(kind))
            } else {
                content_type.eq_ignore_ascii_case("text/html")
                    || content_type.eq_ignore_ascii_case("application/xhtml+xml")
            };
            if !allowed {
                return Err(PublicWebError::ContentType);
            }
            let bytes = bounded_body(response, sitemap).await?;
            return Ok((url, bytes, robots_sitemaps));
        }
        unreachable!()
    }

    // The origin lock is held from pacing through response body consumption,
    // ensuring that simultaneous callers cannot overlap on the same origin.
    async fn request(
        &self,
        url: &Url,
        state: &mut OriginState,
    ) -> Result<reqwest::Response, PublicWebError> {
        validate_url(url)?;
        sleep_until(state.next_request).await;
        let (client, target) = self.pinned_client(url).await?;
        self.charge()?;
        state.next_request = Instant::now() + self.0.delay;
        client
            .get(target)
            .send()
            .await
            .map_err(|e| PublicWebError::Transport(e.to_string()))
    }

    async fn pinned_client(&self, url: &Url) -> Result<(reqwest::Client, Url), PublicWebError> {
        if let Some(replay) = &self.0.replay {
            let mut target = replay.clone();
            target.set_path(&format!("/{}{}", url.host_str().unwrap_or(""), url.path()));
            target.set_query(url.query());
            let client = reqwest::Client::builder()
                .no_proxy()
                .no_gzip()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(self.0.timeout)
                .user_agent(USER_AGENT)
                .build()
                .map_err(|e| PublicWebError::Transport(e.to_string()))?;
            return Ok((client, target));
        }
        let host = url
            .host_str()
            .ok_or_else(|| PublicWebError::Destination(url.to_string()))?;
        let port = url
            .port_or_known_default()
            .ok_or_else(|| PublicWebError::Destination(url.to_string()))?;
        let addresses: Vec<SocketAddr> = match url.host() {
            Some(url::Host::Ipv4(ip)) => vec![SocketAddr::new(IpAddr::V4(ip), port)],
            Some(url::Host::Ipv6(ip)) => vec![SocketAddr::new(IpAddr::V6(ip), port)],
            Some(url::Host::Domain(_)) => {
                tokio::time::timeout(self.0.timeout, tokio::net::lookup_host((host, port)))
                    .await
                    .map_err(|_| PublicWebError::Dns("lookup timed out".into()))?
                    .map_err(|e| PublicWebError::Dns(e.to_string()))?
                    .collect()
            }
            None => return Err(PublicWebError::Destination(url.to_string())),
        };
        if !addresses_are_public(&addresses) {
            return Err(PublicWebError::Destination(url.to_string()));
        }
        // resolve_to_addrs overrides reqwest's resolver while preserving the URL's
        // hostname for TLS verification and the HTTP Host header.
        let client = reqwest::Client::builder()
            .no_proxy()
            .no_gzip()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(self.0.timeout)
            .user_agent(USER_AGENT)
            .resolve_to_addrs(host, &addresses)
            .build()
            .map_err(|e| PublicWebError::Transport(e.to_string()))?;
        Ok((client, url.clone()))
    }
}

async fn bounded_body(
    mut response: reqwest::Response,
    allow_gzip: bool,
) -> Result<Vec<u8>, PublicWebError> {
    let encoded_gzip = if let Some(encoding) = response.headers().get(header::CONTENT_ENCODING) {
        let encoding = encoding.to_str().map_err(|_| PublicWebError::ContentType)?;
        if !encoding.eq_ignore_ascii_case("identity")
            && !(allow_gzip && encoding.eq_ignore_ascii_case("gzip"))
        {
            return Err(PublicWebError::ContentType);
        }
        encoding.eq_ignore_ascii_case("gzip")
    } else {
        false
    };
    if response
        .content_length()
        .is_some_and(|n| n > BODY_LIMIT as u64)
    {
        return Err(PublicWebError::BodyTooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| PublicWebError::Transport(e.to_string()))?
    {
        if chunk.len() > BODY_LIMIT - body.len() {
            return Err(PublicWebError::BodyTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    if encoded_gzip && !body.starts_with(&[0x1f, 0x8b]) {
        return Err(PublicWebError::ContentType);
    }
    Ok(body)
}

fn validate_url(url: &Url) -> Result<(), PublicWebError> {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host().is_none()
    {
        return Err(PublicWebError::Destination(url.to_string()));
    }
    if let Some(host) = url.host() {
        match host {
            url::Host::Ipv4(ip) if !public_ip(IpAddr::V4(ip)) => {
                return Err(PublicWebError::Destination(url.to_string()))
            }
            url::Host::Ipv6(ip) if !public_ip(IpAddr::V6(ip)) => {
                return Err(PublicWebError::Destination(url.to_string()))
            }
            _ => {}
        }
    }
    Ok(())
}

fn addresses_are_public(addresses: &[SocketAddr]) -> bool {
    !addresses.is_empty() && addresses.iter().all(|addr| public_ip(addr.ip()))
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => public_v4(ip),
        IpAddr::V6(ip) => public_v6(ip),
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 192 && b == 0 && (c == 0 || c == 2))
        || (a == 192 && b == 88 && c == 99)
        || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113))
}

fn public_v6(ip: Ipv6Addr) -> bool {
    // Restrict to ordinary global unicast; this excludes mapped/compatible IPv4,
    // local, multicast, translation, and reserved address space by construction.
    let s = ip.segments();
    (s[0] & 0xe000) == 0x2000
        && !(s[0] == 0x2001 && s[1] < 0x0200)
        && !(s[0] == 0x2001 && s[1] == 0x0db8)
        && s[0] != 0x2002
        && s[0] != 0x3ffe
        && !(s[0] == 0x3fff && s[1] < 0x1000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn fixture() -> (String, Arc<Mutex<Vec<String>>>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let recorded = recorded.clone();
                tokio::spawn(async move {
                    let mut request = vec![0u8; 8192];
                    let n = socket.read(&mut request).await.unwrap();
                    let text = String::from_utf8_lossy(&request[..n]);
                    let path = text.split_whitespace().nth(1).unwrap_or("").to_owned();
                    recorded.lock().await.push(path.clone());
                    let (status, headers, body) = if path.ends_with("/robots.txt") {
                        if path.starts_with("/missing.example/") {
                            ("404 Not Found", "", Vec::new())
                        } else {
                            ("200 OK", "Content-Type: text/plain\r\n", b"User-agent: *\nDisallow: /blocked\nSitemap: https://one.example/sitemap.xml\n".to_vec())
                        }
                    } else if path.ends_with("/private") {
                        (
                            "302 Found",
                            "Location: http://127.0.0.1/secret\r\n",
                            Vec::new(),
                        )
                    } else if path.ends_with("/redirect") {
                        (
                            "302 Found",
                            "Location: https://two.example/job\r\n",
                            Vec::new(),
                        )
                    } else if path.ends_with("/loop") {
                        ("302 Found", "Location: /loop\r\n", Vec::new())
                    } else if path.ends_with("/wrong") {
                        (
                            "200 OK",
                            "Content-Type: application/json\r\n",
                            b"{}".to_vec(),
                        )
                    } else if path.ends_with("/large") {
                        (
                            "200 OK",
                            "Content-Type: text/html\r\n",
                            vec![b'x'; BODY_LIMIT + 1],
                        )
                    } else if path.ends_with("/bomb.gz") {
                        let mut gzip = flate2::write::GzEncoder::new(
                            Vec::new(),
                            flate2::Compression::default(),
                        );
                        gzip.write_all(&vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
                        (
                            "200 OK",
                            "Content-Type: application/gzip\r\n",
                            gzip.finish().unwrap(),
                        )
                    } else if path.ends_with("/sitemap.xml") {
                        let mut gzip = flate2::write::GzEncoder::new(
                            Vec::new(),
                            flate2::Compression::default(),
                        );
                        gzip.write_all(b"<urlset/>").unwrap();
                        (
                            "200 OK",
                            "Content-Type: application/gzip\r\n",
                            gzip.finish().unwrap(),
                        )
                    } else {
                        (
                            "200 OK",
                            "Content-Type: text/html\r\n",
                            b"<html>job</html>".to_vec(),
                        )
                    };
                    let header = format!("HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = socket.write_all(header.as_bytes()).await;
                    let _ = socket.write_all(&body).await;
                });
            }
        });
        (address, requests, task)
    }

    fn replay(origin: &str, budget: usize) -> PublicWeb {
        PublicWeb::new(budget, Duration::from_secs(2), Duration::ZERO)
            .with_test_replay_origin(origin)
            .unwrap()
    }

    #[test]
    fn rejects_special_addresses_and_credentials() {
        for ip in [
            "0.0.0.1",
            "10.2.3.4",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.2",
            "172.16.0.1",
            "192.0.0.1",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "::ffff:8.8.8.8",
            "fe80::1",
            "fc00::1",
            "ff00::1",
            "2001:db8::1",
            "2002::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("8.8.8.8".parse().unwrap()));
        assert!(public_ip("2606:4700:4700::1111".parse().unwrap()));
        let public: SocketAddr = "8.8.8.8:443".parse().unwrap();
        let rebound: SocketAddr = "127.0.0.1:443".parse().unwrap();
        assert!(addresses_are_public(&[public]));
        assert!(!addresses_are_public(&[public, rebound]));
        assert!(!addresses_are_public(&[]));
        assert!(validate_url(&Url::parse("https://user:pass@example.com/").unwrap()).is_err());
        assert!(validate_url(&Url::parse("file:///tmp/test").unwrap()).is_err());
    }

    #[tokio::test]
    async fn checks_robots_redirects_and_budgets_across_sources() {
        let (address, requests, server) = fixture().await;
        let run = replay(&address, 8);
        let a = run.for_source(5);
        let b = run.for_source(2);
        let page = a.fetch("https://one.example/redirect").await.unwrap();
        assert_eq!(page.url.as_str(), "https://two.example/job");
        assert_eq!(
            page.robots_sitemaps,
            vec!["https://one.example/sitemap.xml"]
        );
        assert_eq!(a.requests_used(), 4);
        assert!(matches!(
            a.fetch("https://one.example/blocked").await,
            Err(PublicWebError::RobotsDenied(_))
        ));
        assert!(matches!(
            a.fetch("https://one.example/private").await,
            Err(PublicWebError::Destination(_))
        ));
        assert!(matches!(
            a.fetch("https://one.example/job").await,
            Err(PublicWebError::BudgetExhausted)
        ));
        assert!(matches!(
            b.fetch("https://missing.example/job").await,
            Err(PublicWebError::RobotsUnavailable(_))
        ));
        assert_eq!(run.global_requests_used(), 6);
        let paths = requests.lock().await;
        assert!(!paths
            .iter()
            .any(|p| p.contains("127.0.0.1") || p.ends_with("/blocked")));
        server.abort();
    }

    #[tokio::test]
    async fn rejects_redirect_origin_before_request_when_limit_is_reached() {
        let (address, requests, server) = fixture().await;
        let root = replay(&address, 10);
        let source = root.for_source_with_origin_limit(10, 1);
        assert!(matches!(
            source.fetch("https://one.example/redirect").await,
            Err(PublicWebError::OriginLimit(_))
        ));
        assert_eq!(source.origins_used(), 1);
        assert_eq!(source.requests_used(), 2);
        assert!(!requests
            .lock()
            .await
            .iter()
            .any(|path| path.starts_with("/two.example/")));
        server.abort();
    }

    #[tokio::test]
    async fn bounds_redirects_and_global_budget() {
        let (address, requests, server) = fixture().await;
        let root = replay(&address, 7);
        let first = root.for_source(10);
        let second = root.for_source(10);
        assert!(matches!(
            first.fetch("https://one.example/loop").await,
            Err(PublicWebError::Redirect)
        ));
        assert_eq!(first.requests_used(), 7);
        assert!(matches!(
            second.fetch("https://one.example/job").await,
            Err(PublicWebError::GlobalBudgetExhausted)
        ));
        assert_eq!(second.requests_used(), 0);
        assert_eq!(requests.lock().await.len(), 7);
        server.abort();
    }

    #[tokio::test]
    async fn enforces_content_and_compressed_sitemap_bounds() {
        let (address, _, server) = fixture().await;
        let web = replay(&address, 8);
        assert!(matches!(
            web.fetch("https://one.example/wrong").await,
            Err(PublicWebError::ContentType)
        ));
        assert!(matches!(
            web.fetch("https://one.example/large").await,
            Err(PublicWebError::BodyTooLarge)
        ));
        assert_eq!(
            web.fetch_sitemap("https://one.example/sitemap.xml")
                .await
                .unwrap()
                .body,
            b"<urlset/>"
        );
        assert!(matches!(
            web.fetch_sitemap("https://one.example/bomb.gz").await,
            Err(PublicWebError::BodyTooLarge)
        ));
        server.abort();
    }
}
