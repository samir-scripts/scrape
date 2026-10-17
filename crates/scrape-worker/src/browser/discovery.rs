//! Browser runtime resolution and isolated session loader for `scrape-worker`.
use crate::client::ClientError;
use scrape_core::browser::{
    resolve_browser_runtime, BrowserCookie, ResolvedBrowserRuntime, StoredBrowserSession,
    NEEDS_LOGIN_MESSAGE,
};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

pub fn resolve_runtime() -> Result<ResolvedBrowserRuntime, ClientError> {
    resolve_browser_runtime().map_err(ClientError::BrowserRuntimeMissing)
}

pub fn resolve_source_session_cookies(
    config: &Value,
    target_url: &Url,
) -> Result<Vec<BrowserCookie>, ClientError> {
    if let Some(status) = config.get("__session_status").and_then(Value::as_str) {
        if status != "ready" {
            return Err(ClientError::NeedsLogin(NEEDS_LOGIN_MESSAGE.to_string()));
        }
    }

    let session_path = if let Some(explicit) = config
        .get("__session_file")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(PathBuf::from(explicit))
    } else if let Some(session_id) = config
        .get("session_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let validated_id = StoredBrowserSession::validate_session_id(session_id)
            .map_err(ClientError::Configuration)?;
        let dir = std::env::var_os("SCRAPE_SESSION_DIR")
            .map(PathBuf::from)
            .ok_or_else(|| ClientError::NeedsLogin(NEEDS_LOGIN_MESSAGE.to_string()))?;
        Some(dir.join(format!("{validated_id}.json")))
    } else {
        None
    };

    let Some(path) = session_path else {
        return Ok(Vec::new());
    };

    let bytes = std::fs::read(&path)
        .map_err(|_| ClientError::NeedsLogin(NEEDS_LOGIN_MESSAGE.to_string()))?;
    let stored: StoredBrowserSession = serde_json::from_slice(&bytes)
        .map_err(|_| ClientError::NeedsLogin(NEEDS_LOGIN_MESSAGE.to_string()))?;

    let now_epoch_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    stored
        .cookies_for_url(target_url, now_epoch_s)
        .map_err(|e| {
            if e.contains("scoped to domain") {
                ClientError::Configuration(e)
            } else {
                ClientError::NeedsLogin(e)
            }
        })
}
