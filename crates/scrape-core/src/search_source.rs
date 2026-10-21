//! Provider identity and bounded public search configuration shared by host and worker.
use serde_json::Value;
use url::Url;

pub const DEFAULT_ENDPOINT: &str = "https://www.themuse.com/api/public/jobs?page=0";
pub const ARBEITNOW_ENDPOINT: &str = "https://www.arbeitnow.com/api/job-board-api";
pub const DEFAULT_MAX_PAGES: u32 = 5;

pub fn identifier(raw: &str) -> Result<&'static str, String> {
    match raw.trim() {
        "" | "default" | "themuse" => Ok("themuse"),
        "arbeitnow" => Ok("arbeitnow"),
        _ => Err("extractor.url: expected search provider themuse or arbeitnow; configure a same-schema feed with source_config.endpoint".into()),
    }
}

pub fn endpoint(provider: &str, config: &Value) -> Result<Url, String> {
    let default = match identifier(provider)? {
        "themuse" => DEFAULT_ENDPOINT,
        _ => ARBEITNOW_ENDPOINT,
    };
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
    {
        return Err("extractor.source_config.endpoint: use an HTTP(S) URL without credentials or a fragment, at most 8192 bytes".into());
    }
    Ok(url)
}

pub fn validate_config(config: &Value) -> Result<(), String> {
    if config.is_null() {
        return Ok(());
    }
    let object = config
        .as_object()
        .ok_or("extractor.source_config: expected mapping")?;
    for (key, value) in object {
        if key.starts_with("__") {
            continue;
        }
        match key.as_str() {
            "url" => { identifier(value.as_str().ok_or("extractor.url: expected search provider")?)?; }
            "endpoint" => { endpoint("themuse", config)?; }
            "max_pages" | "max_source_duration_s" | "max_workflow_duration_s" => {}
            "query" | "location" if value.is_string() => {}
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
        ] {
            assert!(validate_config(&config).is_err(), "{config}");
        }
        validate_config(&json!({"url":"arbeitnow","endpoint":"https://fixture.test/feed?page=1","max_pages":2,"companies":["Google"],"target_roles":["Engineer"],"max_days_old":7})).unwrap();
    }
}
