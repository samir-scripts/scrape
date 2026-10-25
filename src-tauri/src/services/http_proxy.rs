//! Write-only proxy configuration. Secrets enter execution copies, never saved workflows.
use crate::error::{AppError, Result};
use scrape_core::{protocol::HttpProxyConfig, WorkerConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
use ts_rs::TS;

static ACCESS: Mutex<()> = Mutex::const_new(());
const FILE: &str = "http-proxy-test-credentials.json";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HttpProxyStatus {
    pub configured: bool,
    pub server: Option<String>,
    pub authenticated: bool,
}

fn store_error() -> AppError {
    AppError::Validation("Cannot access the HTTP proxy in the operating system secure store. Unlock/allow the credential store, then retry. No plaintext fallback is used.".into())
}

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new("scrape.network", "http-proxy").map_err(|_| store_error())
}

fn load(isolated: Option<&Path>) -> Result<Option<HttpProxyConfig>> {
    let value = if let Some(directory) = isolated {
        match std::fs::read_to_string(directory.join(FILE)) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(store_error()),
        }
    } else {
        match entry()?.get_password() {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(_) => return Err(store_error()),
        }
    };
    let config: HttpProxyConfig = serde_json::from_str(&value).map_err(|_| store_error())?;
    Ok(Some(config.validate().map_err(|_| store_error())?))
}

fn save(isolated: Option<&Path>, config: Option<&HttpProxyConfig>) -> Result<()> {
    if let Some(directory) = isolated {
        let path = directory.join(FILE);
        if let Some(config) = config {
            let value = serde_json::to_vec(config).map_err(|_| store_error())?;
            let temporary = directory.join(format!(".proxy-{}", uuid::Uuid::now_v7()));
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            use std::io::Write;
            let result = (|| -> std::io::Result<()> {
                let mut file = options.open(&temporary)?;
                file.write_all(&value)?;
                file.sync_all()?;
                std::fs::rename(&temporary, path)
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(temporary);
                return Err(store_error());
            }
        } else if let Err(error) = std::fs::remove_file(path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(store_error());
            }
        }
        return Ok(());
    }
    let entry = entry()?;
    match config {
        Some(config) => entry
            .set_password(&serde_json::to_string(config).map_err(|_| store_error())?)
            .map_err(|_| store_error()),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(store_error()),
        },
    }
}

async fn blocking<T: Send + 'static>(
    operation: impl FnOnce(Option<PathBuf>) -> Result<T> + Send + 'static,
) -> Result<T> {
    let _guard = ACCESS.lock().await;
    let isolated = super::native_smoke::directory();
    tokio::task::spawn_blocking(move || operation(isolated))
        .await
        .map_err(|_| store_error())?
}

fn describe(config: Option<&HttpProxyConfig>) -> HttpProxyStatus {
    HttpProxyStatus {
        configured: config.is_some(),
        server: config.map(|config| config.server.clone()),
        authenticated: config.is_some_and(|config| config.username.is_some()),
    }
}

pub async fn configuration() -> Result<Option<HttpProxyConfig>> {
    blocking(|isolated| load(isolated.as_deref())).await
}

pub async fn status() -> Result<HttpProxyStatus> {
    Ok(describe(configuration().await?.as_ref()))
}

pub async fn set(
    server: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<HttpProxyStatus> {
    let config = HttpProxyConfig {
        server,
        username,
        password,
    }
    .validate()
    .map_err(AppError::Validation)?;
    blocking(move |isolated| {
        save(isolated.as_deref(), Some(&config))?;
        Ok(describe(Some(&config)))
    })
    .await
}

pub async fn clear() -> Result<HttpProxyStatus> {
    blocking(|isolated| {
        save(isolated.as_deref(), None)?;
        Ok(describe(None))
    })
    .await
}

fn inject(
    config: &mut WorkerConfig,
    proxy: Option<HttpProxyConfig>,
    external_cdp: bool,
) -> Result<()> {
    if proxy.is_some() {
        for source in &config.sources {
            let mode =
                scrape_core::extraction::mode(&source.config).map_err(AppError::Validation)?;
            if mode == scrape_core::extraction::ExtractionMode::Script {
                return Err(AppError::Validation("HTTP proxy routing cannot be guaranteed for custom scripts. Disable script sources or clear the proxy.".into()));
            }
            if external_cdp && mode == scrape_core::extraction::ExtractionMode::Browser {
                return Err(AppError::Validation("HTTP proxy routing cannot be guaranteed for externally managed CDP browsers. Remove SCRAPE_CDP_WS_URL or clear the proxy.".into()));
            }
        }
    }
    config.http_proxy = proxy;
    Ok(())
}

pub async fn hydrate(config: &mut WorkerConfig) -> Result<()> {
    let external_cdp =
        std::env::var("SCRAPE_CDP_WS_URL").is_ok_and(|value| !value.trim().is_empty());
    inject(config, configuration().await?, external_cdp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> HttpProxyConfig {
        HttpProxyConfig {
            server: "http://localhost:8080".into(),
            username: Some("fixture-user".into()),
            password: Some("fixture-password".into()),
        }
        .validate()
        .unwrap()
    }

    #[test]
    fn isolated_storage_is_write_only_atomic_and_clear_is_idempotent() {
        let directory = tempfile::tempdir().unwrap();
        let path = Some(directory.path());
        assert!(load(path).unwrap().is_none());
        let proxy = fixture();
        save(path, Some(&proxy)).unwrap();
        let loaded = load(path).unwrap().unwrap();
        assert_eq!(loaded.password, proxy.password);
        let status = serde_json::to_string(&describe(Some(&loaded))).unwrap();
        assert!(!status.contains("fixture-user"));
        assert!(!status.contains("fixture-password"));
        assert!(status.contains("localhost:8080"));
        let invalid = HttpProxyConfig {
            server: "http://fixture-user:fixture-password@localhost:8080".into(),
            username: None,
            password: None,
        };
        assert!(invalid.validate().is_err());
        assert_eq!(load(path).unwrap().unwrap().password, proxy.password);
        // Storage failure returns a sanitized error rather than a plaintext fallback.
        let blocker = directory.path().join("not-a-directory");
        std::fs::write(&blocker, b"blocker").unwrap();
        let error = save(Some(&blocker), Some(&proxy)).unwrap_err().to_string();
        assert!(!error.contains("fixture-password"));
        assert!(!error.contains("fixture-user"));
        assert_eq!(load(path).unwrap().unwrap().password, proxy.password);
        save(path, None).unwrap();
        save(path, None).unwrap();
        assert!(load(path).unwrap().is_none());
    }

    #[tokio::test]
    async fn invalid_update_is_rejected_before_store_access() {
        for server in [
            "socks5://localhost:8080",
            "http://user:secret@localhost:8080",
            "http://localhost:8080/path",
            "http://localhost:8080/?secret=x",
            "http://localhost:8080/#secret",
        ] {
            assert!(set(server.into(), None, None).await.is_err());
        }
        assert!(
            set("http://localhost:8080".into(), Some("user".into()), None)
                .await
                .is_err()
        );
    }

    #[test]
    fn execution_copy_redacts_entire_proxy_and_rejects_unroutable_sources() {
        let saved = WorkerConfig::default();
        let mut execution = saved.clone();
        inject(&mut execution, Some(fixture()), false).unwrap();
        assert!(saved.http_proxy.is_none());
        assert!(super::super::execution_diagnostics::safe_config(&execution)
            .unwrap()
            .http_proxy
            .is_none());
        let mut diagnostic = serde_json::to_value(&execution).unwrap();
        super::super::execution_diagnostics::redact(&mut diagnostic);
        assert!(!diagnostic.to_string().contains("fixture-"));
        assert!(!diagnostic.to_string().contains("localhost:8080"));
        execution.sources.push(scrape_core::WorkerSource {
            id: "fixture".into(),
            kind: scrape_core::SourceKind::Selector,
            name: "fixture".into(),
            identifier: "https://fixture.test".into(),
            config: json!({"mode":"script"}),
        });
        assert!(inject(&mut execution, Some(fixture()), false).is_err());
        execution.sources[0].config = json!({"mode":"browser"});
        assert!(inject(&mut execution, Some(fixture()), true).is_err());
        inject(&mut execution, None, true).unwrap();
    }
}
