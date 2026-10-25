//! Write-only OS-keystore credentials, hydrated into execution copies only.
use crate::error::{AppError, Result};
use scrape_core::{SourceKind, WorkerConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;
use tokio::sync::Mutex;
use ts_rs::TS;

static ACCESS: Mutex<()> = Mutex::const_new(());
const SETUP_REQUIRED: &str =
    "Adzuna setup required: configure App ID and App key in Settings > System > Job providers.";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProviderCredentialStatus {
    pub configured: bool,
}

// Deliberately no Debug: values must never enter diagnostic output.
#[derive(Serialize, Deserialize)]
struct Credentials {
    app_id: String,
    app_key: String,
}

fn store_error() -> AppError {
    AppError::Validation("Cannot access Adzuna credentials in the operating system secure store. Unlock/allow the credential store, then retry. No plaintext fallback is used.".into())
}

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new("scrape.providers", "adzuna").map_err(|_| store_error())
}

fn load(isolated: Option<&Path>) -> Result<Option<Credentials>> {
    let value = if let Some(directory) = isolated {
        match std::fs::read_to_string(directory.join("adzuna-test-credentials.json")) {
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
    let credentials: Credentials = serde_json::from_str(&value).map_err(|_| store_error())?;
    if credentials.app_id.trim().is_empty() || credentials.app_key.trim().is_empty() {
        return Err(store_error());
    }
    Ok(Some(credentials))
}

fn save(isolated: Option<&Path>, credentials: Option<Credentials>) -> Result<()> {
    if let Some(directory) = isolated {
        // This backend is reachable only from debug native-smoke profiles/tests.
        let path = directory.join("adzuna-test-credentials.json");
        if let Some(credentials) = credentials {
            let value = serde_json::to_vec(&credentials).map_err(|_| store_error())?;
            let temporary = directory.join(format!(".credentials-{}", uuid::Uuid::now_v7()));
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
    match credentials {
        Some(credentials) => entry
            .set_password(&serde_json::to_string(&credentials).map_err(|_| store_error())?)
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

pub async fn status() -> Result<ProviderCredentialStatus> {
    blocking(|isolated| {
        Ok(ProviderCredentialStatus {
            configured: load(isolated.as_deref())?.is_some(),
        })
    })
    .await
}

pub async fn set(app_id: String, app_key: String) -> Result<ProviderCredentialStatus> {
    let app_id = app_id.trim().to_owned();
    let app_key = app_key.trim().to_owned();
    if app_id.is_empty() || app_key.is_empty() {
        return Err(AppError::Validation(
            "Adzuna App ID and App key must both be nonblank.".into(),
        ));
    }
    blocking(move |isolated| {
        save(isolated.as_deref(), Some(Credentials { app_id, app_key }))?;
        Ok(ProviderCredentialStatus { configured: true })
    })
    .await
}

pub async fn clear() -> Result<ProviderCredentialStatus> {
    blocking(|isolated| {
        save(isolated.as_deref(), None)?;
        Ok(ProviderCredentialStatus { configured: false })
    })
    .await
}

fn inject(config: &mut WorkerConfig, credentials: Option<Credentials>, cache: &Path) -> Result<()> {
    for source in &mut config.sources {
        if source.kind != SourceKind::Search {
            continue;
        }
        match scrape_core::search_source::identifier(&source.identifier)
            .map_err(AppError::Validation)?
        {
            "adzuna" => {
                let credentials = credentials
                    .as_ref()
                    .ok_or_else(|| AppError::Validation(SETUP_REQUIRED.into()))?;
                source.config["__provider_credentials"] = serde_json::json!({"app_id": credentials.app_id, "app_key": credentials.app_key});
            }
            "jobicy" => source.config["__provider_cache_dir"] = serde_json::json!(cache),
            _ => {}
        }
    }
    Ok(())
}

pub async fn hydrate(app: &tauri::AppHandle, config: &mut WorkerConfig) -> Result<()> {
    let adzuna = config
        .sources
        .iter()
        .any(|s| s.kind == SourceKind::Search && s.identifier == "adzuna");
    let jobicy = config
        .sources
        .iter()
        .any(|s| s.kind == SourceKind::Search && s.identifier == "jobicy");
    if !adzuna && !jobicy {
        return Ok(());
    }
    let cache = if let Some(directory) = super::native_smoke::directory() {
        directory.join("provider-cache")
    } else {
        app.path()
            .app_cache_dir()
            .map_err(|_| {
                AppError::Validation("Cannot resolve Jobicy application cache directory.".into())
            })?
            .join("providers")
    };
    if jobicy {
        tokio::fs::create_dir_all(&cache).await.map_err(|_| AppError::Validation("Jobicy cache is unavailable. Allow access to the application cache directory and retry.".into()))?;
    }
    let credentials = if adzuna {
        blocking(|isolated| load(isolated.as_deref())).await?
    } else {
        None
    };
    inject(config, credentials, &cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scrape_core::WorkerSource;
    use serde_json::json;

    #[tokio::test]
    async fn blank_credentials_are_rejected_before_accessing_any_store() {
        assert!(set(" ".into(), "fictional-key".into()).await.is_err());
        assert!(set("fictional-id".into(), "\n".into()).await.is_err());
    }

    #[test]
    fn isolated_pair_is_atomic_write_only_and_clearable() {
        let directory = tempfile::tempdir().unwrap();
        assert!(load(Some(directory.path())).unwrap().is_none());
        save(
            Some(directory.path()),
            Some(Credentials {
                app_id: "fixture-id".into(),
                app_key: "fixture-key".into(),
            }),
        )
        .unwrap();
        let pair = load(Some(directory.path())).unwrap().unwrap();
        assert_eq!(pair.app_id, "fixture-id");
        assert_eq!(pair.app_key, "fixture-key");
        save(Some(directory.path()), None).unwrap();
        assert!(load(Some(directory.path())).unwrap().is_none());
    }

    #[test]
    fn missing_setup_fails_without_secret_and_hydration_does_not_mutate_saved_copy() {
        let mut config = WorkerConfig {
            sources: vec![WorkerSource {
                id: "source".into(),
                kind: SourceKind::Search,
                name: "Adzuna".into(),
                identifier: "adzuna".into(),
                config: json!({}),
            }],
            ..Default::default()
        };
        let saved = config.clone();
        assert!(inject(&mut config, None, Path::new("/isolated/cache"))
            .unwrap_err()
            .to_string()
            .contains(SETUP_REQUIRED));
        inject(
            &mut config,
            Some(Credentials {
                app_id: "fixture-id".into(),
                app_key: "fixture-key".into(),
            }),
            Path::new("/isolated/cache"),
        )
        .unwrap();
        assert!(saved.sources[0]
            .config
            .get("__provider_credentials")
            .is_none());
        let mut diagnostic = serde_json::to_value(config).unwrap();
        super::super::execution_diagnostics::redact(&mut diagnostic);
        assert!(!diagnostic.to_string().contains("fixture-key"));
        assert!(!diagnostic.to_string().contains("fixture-id"));
    }
}
