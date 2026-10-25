//! Public Jobicy response pages only. Credentials never participate in cache keys/data.
use crate::client::ClientError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

const TTL_SECONDS: u64 = 3600;
const MAX_PAGES: usize = 64;

#[derive(Clone)]
pub(crate) struct SearchCache {
    directory: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    fetched_at: u64,
    response: Value,
}

fn failure() -> ClientError {
    ClientError::Configuration(
        "Jobicy cache unavailable: check application cache directory permissions".into(),
    )
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl SearchCache {
    pub fn new(directory: &str) -> Result<Self, ClientError> {
        let path = Path::new(directory);
        if !path.is_absolute() {
            return Err(failure());
        }
        Ok(Self {
            directory: path.join("jobicy-pages-v1"),
        })
    }

    fn key(url: &Url) -> Result<String, ClientError> {
        // The cache is never a general-purpose credential-bearing response store.
        if url.scheme() != "https"
            || url.host_str() != Some("jobicy.com")
            || url.port().is_some()
            || url.path() != "/api/jobs"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || url
                .query_pairs()
                .any(|(key, _)| !matches!(key.as_ref(), "count" | "cursor"))
        {
            return Err(failure());
        }
        let mut pairs: Vec<_> = url.query_pairs().into_owned().collect();
        pairs.sort();
        let mut canonical = url.clone();
        canonical.set_query(None);
        canonical.query_pairs_mut().extend_pairs(pairs);
        Ok(format!(
            "{:x}",
            Sha256::digest(format!("jobicy\n{canonical}").as_bytes())
        ))
    }

    fn read_at(&self, url: &Url, at: u64) -> Result<Option<Value>, ClientError> {
        fs::create_dir_all(&self.directory).map_err(|_| failure())?;
        // Probe writability even on hits: unavailable storage must not bypass the policy.
        tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| failure())?;
        let path = self.directory.join(format!("{}.json", Self::key(url)?));
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(failure()),
        };
        let entry: Entry = match serde_json::from_slice(&bytes) {
            Ok(entry) => entry,
            Err(_) => return Ok(None),
        };
        if at < entry.fetched_at || at - entry.fetched_at >= TTL_SECONDS {
            return Ok(None);
        }
        Ok(Some(entry.response))
    }

    fn store_at(&self, url: &Url, response: Value, at: u64) -> Result<Value, ClientError> {
        fs::create_dir_all(&self.directory).map_err(|_| failure())?;
        let path = self.directory.join(format!("{}.json", Self::key(url)?));
        let mut temporary =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| failure())?;
        let entry = Entry {
            fetched_at: at,
            response,
        };
        serde_json::to_writer(&mut temporary, &entry).map_err(|_| failure())?;
        temporary.flush().map_err(|_| failure())?;
        temporary.as_file().sync_all().map_err(|_| failure())?;
        temporary.persist(&path).map_err(|_| failure())?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&self.directory).map_err(|_| failure())? {
            let entry = entry.map_err(|_| failure())?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(hash) = name.strip_suffix(".json") else {
                continue;
            };
            if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_err(|_| failure())?;
            entries.push((modified, entry.path()));
        }
        entries.sort();
        let excess = entries.len().saturating_sub(MAX_PAGES);
        for (_, path) in entries.into_iter().take(excess) {
            fs::remove_file(path).map_err(|_| failure())?;
        }
        Ok(entry.response)
    }

    pub async fn read(&self, url: &Url) -> Result<Option<Value>, ClientError> {
        let cache = self.clone();
        let url = url.clone();
        tokio::task::spawn_blocking(move || cache.read_at(&url, now()))
            .await
            .map_err(|_| failure())?
    }
    pub async fn store(&self, url: &Url, response: Value) -> Result<Value, ClientError> {
        let cache = self.clone();
        let url = url.clone();
        tokio::task::spawn_blocking(move || cache.store_at(&url, response, now()))
            .await
            .map_err(|_| failure())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_pages_expire_at_one_hour_and_corruption_is_a_miss() {
        let directory = tempfile::tempdir().unwrap();
        let cache = SearchCache::new(directory.path().to_str().unwrap()).unwrap();
        let url = Url::parse("https://jobicy.com/api/jobs?count=100").unwrap();
        cache.store_at(&url, json!({"jobs":[]}), 10000).unwrap();
        assert_eq!(
            cache.read_at(&url, 13599).unwrap(),
            Some(json!({"jobs":[]}))
        );
        assert!(cache.read_at(&url, 13600).unwrap().is_none());
        assert!(cache.read_at(&url, 9999).unwrap().is_none());
        fs::write(
            cache
                .directory
                .join(format!("{}.json", SearchCache::key(&url).unwrap())),
            b"broken",
        )
        .unwrap();
        assert!(cache.read_at(&url, 10001).unwrap().is_none());
    }

    #[test]
    fn cache_retains_only_64_pages_and_canonicalizes_query_order() {
        let directory = tempfile::tempdir().unwrap();
        let cache = SearchCache::new(directory.path().to_str().unwrap()).unwrap();
        for page in 0..70 {
            let url = Url::parse(&format!(
                "https://jobicy.com/api/jobs?count=100&cursor={page}"
            ))
            .unwrap();
            cache.store_at(&url, json!({"jobs":[]}), 10000).unwrap();
        }
        assert_eq!(fs::read_dir(&cache.directory).unwrap().count(), 64);
        assert_eq!(
            SearchCache::key(
                &Url::parse("https://jobicy.com/api/jobs?count=100&cursor=a").unwrap()
            )
            .unwrap(),
            SearchCache::key(
                &Url::parse("https://jobicy.com/api/jobs?cursor=a&count=100").unwrap()
            )
            .unwrap()
        );
        assert!(SearchCache::key(
            &Url::parse("https://jobicy.com/api/jobs?app_key=secret").unwrap()
        )
        .is_err());
    }

    #[test]
    fn unavailable_cache_is_an_error_not_a_rate_limit_bypass() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("file");
        fs::write(&file, b"not a directory").unwrap();
        let cache = SearchCache::new(file.to_str().unwrap()).unwrap();
        assert!(cache
            .read_at(
                &Url::parse("https://jobicy.com/api/jobs?count=100").unwrap(),
                10000
            )
            .is_err());
    }
}
