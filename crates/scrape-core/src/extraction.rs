//! Shared extraction configuration. Saving a script never grants execution trust.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExtractionMode {
    #[default]
    Static,
    Browser,
    Script,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ScriptRuntime {
    Js,
    Python,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(deny_unknown_fields)]
#[ts(export)]
pub struct WorkflowScriptConfig {
    pub runtime: ScriptRuntime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub interpreter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable, type = "number")]
    pub timeout_s: Option<u64>,
}

impl WorkflowScriptConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.code.is_some() == self.path.is_some() {
            return Err("script: specify exactly one of code or path".into());
        }
        for (name, value) in [
            ("code", &self.code),
            ("path", &self.path),
            ("interpreter", &self.interpreter),
        ] {
            if value
                .as_ref()
                .is_some_and(|s| s.trim().is_empty() || s.contains('\0'))
            {
                return Err(format!("script.{name}: must not be blank or contain NUL"));
            }
        }
        if self.code.as_ref().is_some_and(|s| s.len() > 1_048_576) {
            return Err("script.code: maximum size is 1 MiB".into());
        }
        if matches!(self.runtime, ScriptRuntime::Python | ScriptRuntime::Custom)
            && self.interpreter.is_none()
        {
            return Err(
                "script.interpreter: explicitly select the Python interpreter or custom executable"
                    .into(),
            );
        }
        if self.args.as_ref().is_some_and(|args| {
            args.len() > 128 || args.iter().any(|s| s.contains('\0') || s.len() > 16_384)
        }) {
            return Err(
                "script.args: at most 128 arguments, each at most 16 KiB without NUL".into(),
            );
        }
        if self
            .timeout_s
            .is_some_and(|timeout| !(1..=600).contains(&timeout))
        {
            return Err("script.timeout_s: expected 1–600 seconds".into());
        }
        Ok(())
    }

    /// Stable tuple encoding; omitted arguments and an empty list mean the same.
    /// Timeout is a bound, not executable content, and does not grant authority.
    pub fn canonical_sha256(&self) -> Result<String, String> {
        self.validate()?;
        let encoded = serde_json::to_vec(&(
            self.runtime,
            &self.interpreter,
            &self.code,
            &self.path,
            self.args.as_deref().unwrap_or_default(),
        ))
        .map_err(|error| format!("script: {error}"))?;
        Ok(format!("{:x}", Sha256::digest(encoded)))
    }

    pub fn file_sha256(&self, bytes: &[u8]) -> Result<String, String> {
        let mut hash = Sha256::new();
        hash.update(b"scrape-file-script-v1\0");
        hash.update(self.canonical_sha256()?.as_bytes());
        hash.update(bytes);
        Ok(format!("{:x}", hash.finalize()))
    }
}

pub fn mode(config: &Value) -> Result<ExtractionMode, String> {
    let mode = config
        .get("mode")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|error| format!("mode: {error}"))?;
    let resolved = mode.unwrap_or(if config.get("script").is_some() {
        ExtractionMode::Script
    } else {
        ExtractionMode::Static
    });
    if config.get("script").is_some() && resolved != ExtractionMode::Script {
        return Err("script: executable configuration requires mode: script".into());
    }
    Ok(resolved)
}

pub fn script(config: &Value) -> Result<Option<WorkflowScriptConfig>, String> {
    if mode(config)? != ExtractionMode::Script {
        return Ok(None);
    }
    let script: WorkflowScriptConfig = serde_json::from_value(
        config
            .get("script")
            .cloned()
            .ok_or("script: required in script mode")?,
    )
    .map_err(|error| format!("script: {error}"))?;
    script.validate()?;
    Ok(Some(script))
}

/// Apply per-source overrides using the authoritative WorkerLimits validator.
pub fn limits(config: &Value) -> Result<crate::WorkerLimits, String> {
    let mut limits = crate::WorkerLimits::default();
    for (key, target) in [
        ("navigation_timeout_s", &mut limits.navigation_timeout_s),
        ("max_source_duration_s", &mut limits.max_source_duration_s),
        (
            "max_workflow_duration_s",
            &mut limits.max_workflow_duration_s,
        ),
    ] {
        if let Some(value) = config.get(key) {
            *target = value
                .as_u64()
                .ok_or_else(|| format!("{key}: expected whole seconds"))?;
        }
    }
    if let Some(value) = config.get("max_pages") {
        limits.max_pages_per_source = value
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or("max_pages: expected a whole number from 1 to 100")?;
    }
    limits.validate()?;
    Ok(limits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_script_hash_is_order_independent_and_covers_executable_inputs() {
        let script: WorkflowScriptConfig = serde_json::from_value(
            serde_json::json!({"runtime":"js","code":"console.log('fixture')"}),
        )
        .unwrap();
        let hash = script.canonical_sha256().unwrap();
        let reordered: WorkflowScriptConfig = serde_json::from_str(
            r#"{"args":[],"code":"console.log('fixture')","runtime":"js","timeout_s":30}"#,
        )
        .unwrap();
        assert_eq!(hash, reordered.canonical_sha256().unwrap());
        for key in ["runtime", "interpreter", "code", "args"] {
            let mut changed = serde_json::to_value(&script).unwrap();
            changed[key] = match key {
                "runtime" => {
                    changed["interpreter"] = Value::String("/usr/bin/python3".into());
                    Value::String("python".into())
                }
                "args" => serde_json::json!(["argument"]),
                _ => Value::String("changed".into()),
            };
            assert_ne!(
                hash,
                serde_json::from_value::<WorkflowScriptConfig>(changed)
                    .unwrap()
                    .canonical_sha256()
                    .unwrap()
            );
        }
        assert!(super::script(&serde_json::json!({"mode":"script"})).is_err());
    }
}
