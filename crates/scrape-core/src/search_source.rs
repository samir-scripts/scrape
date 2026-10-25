//! Provider identity and bounded public search configuration shared by host and worker.
use serde_json::Value;
use url::Url;

pub const DEFAULT_ENDPOINT: &str = "https://www.themuse.com/api/public/jobs?page=0";
pub const ARBEITNOW_ENDPOINT: &str = "https://www.arbeitnow.com/api/job-board-api";
pub const ADZUNA_ENDPOINT: &str = "https://api.adzuna.com/v1/api/jobs/us/search/1";
pub const JOBICY_ENDPOINT: &str = "https://jobicy.com/api/jobs";
pub const DEFAULT_MAX_PAGES: u32 = 5;

pub fn identifier(raw: &str) -> Result<&'static str, String> {
    match raw.trim() {
        "" | "default" | "themuse" => Ok("themuse"),
        "arbeitnow" => Ok("arbeitnow"),
        "adzuna" => Ok("adzuna"),
        "jobicy" => Ok("jobicy"),
        _ => Err("extractor.url: expected search provider themuse, arbeitnow, adzuna or jobicy; configure a same-schema feed with source_config.endpoint".into()),
    }
}

pub fn endpoint(provider: &str, config: &Value) -> Result<Url, String> {
    let default = match identifier(provider)? {
        "themuse" => DEFAULT_ENDPOINT,
        "arbeitnow" => ARBEITNOW_ENDPOINT,
        "adzuna" => ADZUNA_ENDPOINT,
        "jobicy" => JOBICY_ENDPOINT,
        _ => unreachable!(),
    };
    if matches!(identifier(provider)?, "adzuna" | "jobicy") && config.get("endpoint").is_some() {
        return Err(
            "extractor.source_config.endpoint: this provider uses its fixed HTTPS collection"
                .into(),
        );
    }
    let raw = match config.get("endpoint") {
        None => default,
        Some(Value::String(value)) if !value.trim().is_empty() => value.trim(),
        _ => {
            return Err("extractor.source_config.endpoint: expected a nonblank HTTP(S) URL".into())
        }
    };
    let url = Url::parse(raw).map_err(|_| "extractor.source_config.endpoint: invalid URL")?;
    if raw.len() > 8192
        || !matches!(url.scheme(), "https" | "http")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query_pairs().any(|(key, _)| secret_key(&key))
    {
        return Err("extractor.source_config.endpoint: use an HTTP(S) URL without credentials or a fragment, at most 8192 bytes".into());
    }
    Ok(url)
}

pub fn validate_config(config: &Value) -> Result<(), String> {
    validate(config, false)
}

/// Only execution copies hydrated by the host may carry private runtime fields.
pub fn validate_runtime_config(config: &Value) -> Result<(), String> {
    validate(config, true)
}

fn secret_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().replace(['-', '_'], "").as_str(),
        "appid" | "appkey" | "apikey" | "token" | "accesstoken" | "password" | "secret"
    )
}

fn validate(config: &Value, runtime: bool) -> Result<(), String> {
    if config.is_null() {
        return Ok(());
    }
    let object = config
        .as_object()
        .ok_or("extractor.source_config: expected mapping")?;
    for (key, value) in object {
        if matches!(
            key.as_str(),
            "__provider_credentials" | "__provider_cache_dir"
        ) {
            if !runtime {
                return Err(format!(
                    "extractor.source_config.{key}: reserved execution-only field"
                ));
            }
            match key.as_str() {
                "__provider_credentials"
                    if value.as_object().is_some_and(|credentials| {
                        credentials.len() == 2
                            && ["app_id", "app_key"].iter().all(|key| {
                                credentials
                                    .get(*key)
                                    .and_then(Value::as_str)
                                    .is_some_and(|s| !s.trim().is_empty())
                            })
                    }) => {}
                "__provider_cache_dir"
                    if value
                        .as_str()
                        .is_some_and(|s| std::path::Path::new(s).is_absolute()) => {}
                _ => {
                    return Err(
                        "extractor.source_config: invalid private provider runtime configuration"
                            .into(),
                    )
                }
            }
            continue;
        }
        if secret_key(key) {
            return Err("extractor.source_config: credentials must be configured in Settings > System > Job providers".into());
        }
        if key.starts_with("__") {
            continue;
        }
        match key.as_str() {
            "url" => { identifier(value.as_str().ok_or("extractor.url: expected search provider")?)?; }
            "endpoint" => { endpoint(object.get("url").and_then(Value::as_str).unwrap_or("themuse"), config)?; }
            "max_pages" | "max_source_duration_s" | "max_workflow_duration_s" => {}
            "query" | "location" | "what_or" | "where" if value.is_string() => {}
            "companies" | "target_roles" if value.as_array().is_some_and(|items| items.iter().all(|item| item.as_str().is_some_and(|s| !s.trim().is_empty()))) => {}
            "max_days_old" if value.as_u64().is_some() => {}
            _ => return Err(format!("extractor.source_config.{key}: unsupported field or invalid search configuration value")),
        }
    }
    crate::extraction::limits(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn configured_feeds_reject_credentials_fragments_unknown_providers_and_unbounded_pages() {
        assert_eq!(identifier("").unwrap(), "themuse");
        for provider in ["https://unconfigured.test", "unknown"] {
            assert!(identifier(provider).is_err());
        }
        for config in [
            json!({"endpoint":"https://name:secret@fixture.test/jobs"}),
            json!({"endpoint":"file:///tmp/jobs"}),
            json!({"endpoint":"https://fixture.test/jobs#fragment"}),
            json!({"endpoint":""}),
            json!({"max_pages":0}),
            json!({"max_pages":101}),
            json!({"max_pages":1.5}),
            json!({"companies":[""]}),
            json!({"target_roles":false}),
            json!({"max_days_old":-1}),
            json!({"unknown_option":true}),
            json!({"app_id":"secret"}),
            json!({"app_key":"secret"}),
            json!({"__provider_credentials":{"app_id":"id","app_key":"key"}}),
            json!({"__provider_cache_dir":"/tmp/cache"}),
            json!({"endpoint":"https://fixture.test/jobs?app_key=secret"}),
        ] {
            assert!(validate_config(&config).is_err(), "{config}");
        }
        validate_config(&json!({"url":"arbeitnow","endpoint":"https://fixture.test/feed?page=1","max_pages":2,"companies":["Google"],"target_roles":["Engineer"],"max_days_old":7})).unwrap();
    }

    #[test]
    fn provider_runtime_fields_are_private_and_fixed_collections_cannot_be_overridden() {
        for (provider, expected) in [("adzuna", ADZUNA_ENDPOINT), ("jobicy", JOBICY_ENDPOINT)] {
            assert_eq!(identifier(provider).unwrap(), provider);
            assert_eq!(endpoint(provider, &json!({})).unwrap().as_str(), expected);
            assert!(endpoint(provider, &json!({"endpoint":expected})).is_err());
        }
        let runtime = json!({"what_or":"remote","where":"","max_days_old":1,
            "__provider_credentials":{"app_id":"fictional-id","app_key":"fictional-key"},
            "__provider_cache_dir":std::env::temp_dir().to_str().unwrap()});
        assert!(validate_config(&runtime).is_err());
        validate_runtime_config(&runtime).unwrap();
        for config in [
            json!({"__provider_credentials":{"app_id":"","app_key":"key"}}),
            json!({"__provider_credentials":{"app_id":"id","app_key":"key","other":"value"}}),
            json!({"__provider_credentials":{"app_id":12,"app_key":"key"}}),
            json!({"__provider_cache_dir":"relative/cache"}),
        ] {
            assert!(validate_runtime_config(&config).is_err());
        }
    }
}
