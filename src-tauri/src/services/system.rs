use crate::db::settings::{Settings, SettingsRepo};
use crate::error::Result;
use scrape_core::protocol::{CandidateEvaluation, WorkerFilters};
use serde_json::Value;
use sqlx::SqlitePool;
use std::io::Write;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ControlInteractionKind {
    Activate,
    Change,
    Submit,
    Resize,
    Drag,
    Shortcut,
    Dismiss,
}

impl ControlInteractionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Activate => "activate",
            Self::Change => "change",
            Self::Submit => "submit",
            Self::Resize => "resize",
            Self::Drag => "drag",
            Self::Shortcut => "shortcut",
            Self::Dismiss => "dismiss",
        }
    }
}

fn parse_control_catalog(
    input: &str,
) -> std::result::Result<std::collections::BTreeMap<String, String>, String> {
    use serde::de::{MapAccess, Visitor};
    struct CatalogVisitor;
    impl<'de> Visitor<'de> for CatalogVisitor {
        type Value = std::collections::BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a map of unique control action identifiers and descriptions")
        }
        fn visit_map<M: MapAccess<'de>>(
            self,
            mut entries: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut result = std::collections::BTreeMap::new();
            let pattern = regex::Regex::new(r"^[a-z][a-z0-9]*(\.[a-z][a-z0-9_]*)+$")
                .expect("constant action pattern");
            while let Some((key, value)) = entries.next_entry::<String, String>()? {
                if !pattern.is_match(&key)
                    || value.trim().is_empty()
                    || result.insert(key, value).is_some()
                {
                    return Err(serde::de::Error::custom(
                        "invalid or duplicate control action",
                    ));
                }
            }
            Ok(result)
        }
    }
    let mut deserializer = serde_json::Deserializer::from_str(input);
    let catalog = serde::de::Deserializer::deserialize_map(&mut deserializer, CatalogVisitor)
        .map_err(|error| error.to_string())?;
    deserializer.end().map_err(|error| error.to_string())?;
    Ok(catalog)
}

static CONTROL_CATALOG: LazyLock<
    std::result::Result<std::collections::BTreeMap<String, String>, String>,
> = LazyLock::new(|| parse_control_catalog(include_str!("../../../src/lib/control-actions.json")));

fn render_control_interaction(
    now: &str,
    action: &str,
    kind: ControlInteractionKind,
) -> Result<String> {
    let catalog = CONTROL_CATALOG
        .as_ref()
        .map_err(|_| crate::error::AppError::Internal("Invalid control action catalog".into()))?;
    let description = catalog
        .get(action)
        .ok_or_else(|| crate::error::AppError::Validation("Unknown control action".into()))?;
    Ok(render_log(
        now,
        "UI",
        None,
        &format!("UI {} {}: {}", kind.as_str(), action, description),
    ))
}

// Every scrape.log record, including ordinary logs, is appended under this lock.
// A card is rendered first and then submitted in one write_all, never line by line.
static CONSOLE_WRITE: Mutex<()> = Mutex::new(());
const CONSOLE_COLUMNS: usize = 80;
const FIELD_LIMIT: usize = 512;
const DESCRIPTION_LIMIT: usize = 2_048;
const BORDER: &str =
    "+------------------------------------------------------------------------------+";

fn append_record(path: &Path, record: &str) -> std::io::Result<()> {
    let _guard = CONSOLE_WRITE
        .lock()
        .map_err(|_| std::io::Error::other("Console append lock was poisoned"))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(record.as_bytes())
}

fn redact_urls(text: &str) -> String {
    static URLS: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?i)[a-z][a-z0-9+.-]*://[^\s<>"\x00-\x1f\x7f-\x9f]+"#)
            .expect("constant console URL pattern")
    });
    URLS.replace_all(text, |capture: &regex::Captures<'_>| {
        let Ok(mut url) = url::Url::parse(&capture[0]) else {
            return "[invalid URL]".to_string();
        };
        let _ = url.set_username("");
        let _ = url.set_password(None);
        if url.query().is_some() {
            let keys: Vec<String> = url.query_pairs().map(|(key, _)| key.into_owned()).collect();
            url.query_pairs_mut()
                .clear()
                .extend_pairs(keys.iter().map(|key| (key.as_str(), "[REDACTED]")));
        }
        url.set_fragment(None);
        url.to_string()
    })
    .into_owned()
}

/// Sanitize only display copies; persisted candidate evidence remains untouched.
fn display_text(text: &str, limit: usize) -> String {
    let redacted = redact_urls(text);
    let redacted = scrape_core::extraction::redact_script_stderr(redacted.as_bytes());
    let mut output = String::new();
    let mut chars = redacted.chars();
    for character in chars.by_ref().take(limit) {
        match character {
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c.is_control()
                || matches!(
                    c,
                    '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}'
                        | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
                ) =>
            {
                use std::fmt::Write;
                let _ = write!(output, "\\u{{{:04X}}}", c as u32);
            }
            c => output.push(c),
        }
    }
    if chars.next().is_some() {
        output.push_str(" [truncated]");
    }
    if output.trim().is_empty() {
        "Unknown".into()
    } else {
        output
    }
}

// Counting a non-ASCII scalar as two columns is deliberately conservative:
// combining marks may occupy less, but wide CJK/emoji must not overrun a card.
fn wrap_line(output: &mut String, first_prefix: &str, text: &str) {
    output.push_str(first_prefix);
    let mut columns = first_prefix.len();
    for character in text.chars() {
        let width = if character.is_ascii() { 1 } else { 2 };
        if columns + width > CONSOLE_COLUMNS {
            output.push_str("\n|   ");
            columns = 4;
        }
        output.push(character);
        columns += width;
    }
    output.push('\n');
}

fn field(output: &mut String, label: &str, value: &str, limit: usize) {
    let line = format!(
        "{}: {}",
        display_text(label, FIELD_LIMIT),
        display_text(value, limit)
    );
    wrap_line(output, "| ", &line);
}

fn optional_field(output: &mut String, label: &str, value: Option<&str>) {
    field(output, label, value.unwrap_or("Unknown"), FIELD_LIMIT);
}

/// Walk typed evidence rather than truncating one JSON blob and losing later
/// requirements/reasons. Each scalar is independently bounded and escaped.
fn value_fields(output: &mut String, label: &str, value: &Value) {
    match value {
        Value::Object(values) if !values.is_empty() => {
            for (key, value) in values {
                value_fields(output, &format!("{label}.{key}"), value);
            }
        }
        Value::Array(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                value_fields(output, &format!("{label}[{}]", index + 1), value);
            }
        }
        Value::String(value) => field(output, label, value, FIELD_LIMIT),
        Value::Bool(value) => field(
            output,
            label,
            if *value { "true" } else { "false" },
            FIELD_LIMIT,
        ),
        Value::Number(value) => field(output, label, &value.to_string(), FIELD_LIMIT),
        _ => field(output, label, "Unknown", FIELD_LIMIT),
    }
}

fn evidence<T: serde::Serialize>(output: &mut String, label: &str, value: &T) -> Result<()> {
    value_fields(output, label, &serde_json::to_value(value)?);
    Ok(())
}

fn render_candidate_card(
    run_id: &str,
    source_id: &str,
    evaluation: &CandidateEvaluation,
    outcome: &str,
    detail: Option<&str>,
) -> Result<String> {
    let mut output = String::new();
    wrap_line(
        &mut output,
        "",
        &format!(
            "+-- CANDIDATE {} | {} --",
            evaluation.sequence,
            display_text(outcome, FIELD_LIMIT)
        ),
    );
    output.push_str(BORDER);
    output.push('\n');
    field(&mut output, "Run", run_id, FIELD_LIMIT);
    field(&mut output, "Source", source_id, FIELD_LIMIT);
    evidence(&mut output, "Decision", &evaluation.disposition)?;
    if let Some(candidate) = &evaluation.candidate {
        field(&mut output, "Title", &candidate.title, FIELD_LIMIT);
        field(&mut output, "Company", &candidate.company, FIELD_LIMIT);
        field(
            &mut output,
            "External ID",
            &candidate.external_id,
            FIELD_LIMIT,
        );
        field(&mut output, "URL", &candidate.url, FIELD_LIMIT);
        field(
            &mut output,
            "Location (raw)",
            &candidate.location,
            FIELD_LIMIT,
        );
        evidence(&mut output, "Location (normalized)", &candidate.locations)?;
        evidence(
            &mut output,
            "Location data complete",
            &candidate.location_data_complete,
        )?;
        optional_field(
            &mut output,
            "Original publication",
            candidate.date_posted.as_deref(),
        );
        optional_field(
            &mut output,
            "Provider update",
            candidate.provider_updated_at.as_deref(),
        );
        evidence(&mut output, "Role", &candidate.role_type)?;
        evidence(&mut output, "Workplace", &candidate.workplace_type)?;
        evidence(&mut output, "Compensation", &candidate.salary)?;
        evidence(&mut output, "Experience", &candidate.experience)?;
        evidence(&mut output, "New graduate", &candidate.is_new_grad)?;
        evidence(&mut output, "Sponsorship", &candidate.sponsorship)?;
        evidence(&mut output, "Work authorization", &candidate.work_auth)?;
        evidence(&mut output, "Clearance", &candidate.clearance)?;
        let mut excerpt = display_text(
            candidate
                .description_excerpt
                .as_deref()
                .unwrap_or("Unknown"),
            DESCRIPTION_LIMIT,
        );
        if candidate.description_truncated && !excerpt.ends_with(" [truncated]") {
            excerpt.push_str(" [truncated]");
        }
        wrap_line(
            &mut output,
            "| ",
            &format!("Description excerpt: {excerpt}"),
        );
    } else {
        for label in [
            "Title",
            "Company",
            "External ID",
            "URL",
            "Location (raw)",
            "Location (normalized)",
            "Location data complete",
            "Original publication",
            "Provider update",
            "Role",
            "Workplace",
            "Compensation",
            "Experience",
            "New graduate",
            "Sponsorship",
            "Work authorization",
            "Clearance",
            "Description excerpt",
        ] {
            field(&mut output, label, "Unknown", FIELD_LIMIT);
        }
    }
    if evaluation.rejected.is_empty() {
        field(&mut output, "Rejected categories", "None", FIELD_LIMIT);
    } else {
        evidence(&mut output, "Rejected categories", &evaluation.rejected)?;
    }
    if evaluation.unresolved.is_empty() {
        field(&mut output, "Unresolved categories", "None", FIELD_LIMIT);
    } else {
        evidence(&mut output, "Unresolved categories", &evaluation.unresolved)?;
    }
    optional_field(&mut output, "Diagnostic", evaluation.diagnostic.as_deref());
    optional_field(&mut output, "Persistence / error detail", detail);
    output.push_str(BORDER);
    output.push('\n');
    Ok(output)
}

fn render_requirements(
    run_id: &str,
    source_id: &str,
    filters: &WorkerFilters,
    started_at: &str,
    description: &str,
) -> Result<String> {
    let mut output = String::from(BORDER);
    output.push('\n');
    wrap_line(
        &mut output,
        "| ",
        "SOURCE REQUIREMENTS (effective for this run)",
    );
    field(&mut output, "Run", run_id, FIELD_LIMIT);
    field(&mut output, "Source", source_id, FIELD_LIMIT);
    field(&mut output, "Run started at", started_at, FIELD_LIMIT);
    field(&mut output, "Acquisition", description, FIELD_LIMIT);
    let mut requirements = serde_json::to_value(filters)?;
    super::execution_diagnostics::redact(&mut requirements);
    value_fields(&mut output, "Requirements", &requirements);
    output.push_str(BORDER);
    output.push('\n');
    Ok(output)
}

fn render_log(now: &str, level: &str, source_id: Option<&str>, message: &str) -> String {
    let source = source_id
        .map(|source| format!(" [{}]", display_text(source, FIELD_LIMIT)))
        .unwrap_or_default();
    format!(
        "[{}] [{}]{} {}\n",
        display_text(now, FIELD_LIMIT),
        display_text(&level.to_uppercase(), FIELD_LIMIT),
        source,
        display_text(message, FIELD_LIMIT),
    )
}

fn posix_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(any(target_os = "windows", test))]
fn powershell_command(path: &str) -> String {
    format!(
        "Get-Content -LiteralPath '{}' -Wait",
        path.replace('\'', "''")
    )
}

fn console_script(path: &str) -> String {
    format!(
        "#!/usr/bin/env bash\nprintf '%s\\n' 'SCRAPE // EXECUTION CONSOLE' 'PRESS CTRL+C TO CLOSE STREAM'\nprintf 'LOG PATH: %s\\n' {}\nexec tail -n +1 -f -- {}\n",
        posix_quote(&display_text(path, FIELD_LIMIT)),
        posix_quote(path),
    )
}

pub struct SystemService;

impl SystemService {
    pub fn detect_default_settings() -> Settings {
        let mut settings = Settings::default();

        if let Some(locale) = sys_locale::get_locale() {
            settings.system_locale = locale.clone();
            settings.system_week_start = Self::infer_week_start(&locale).to_string();
        }

        if let Ok(tz) = iana_time_zone::get_timezone() {
            settings.system_timezone = tz;
        }

        settings
    }

    pub fn infer_week_start(locale: &str) -> &'static str {
        let upper = locale.to_uppercase();
        if upper.ends_with("-US")
            || upper.ends_with("-CA")
            || upper.ends_with("-MX")
            || upper.ends_with("-JP")
            || upper.ends_with("-IL")
            || upper.ends_with("_US")
            || upper.ends_with("_CA")
            || upper.ends_with("_MX")
            || upper.ends_with("_JP")
            || upper.ends_with("_IL")
        {
            "sunday"
        } else {
            "monday"
        }
    }

    pub async fn initialize_system_defaults(pool: &SqlitePool) -> Result<Settings> {
        let defaults = Self::detect_default_settings();
        SettingsRepo::ensure_defaults(pool, &defaults).await?;
        SettingsRepo::get_all(pool).await
    }

    pub fn get_log_file_path(app: &tauri::AppHandle) -> std::path::PathBuf {
        use tauri::Manager;
        let app_dir = crate::services::native_smoke::directory().unwrap_or_else(|| {
            app.path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
        });
        let _ = std::fs::create_dir_all(&app_dir);
        app_dir.join("scrape.log")
    }

    pub fn append_log(app: &tauri::AppHandle, level: &str, source_id: Option<&str>, message: &str) {
        let path = Self::get_log_file_path(app);
        let record = render_log(&scrape_core::now_iso(), level, source_id, message);
        if let Err(error) = append_record(&path, &record) {
            tracing::error!(%error, path = %path.display(), "Failed to append console log");
        }
    }

    pub fn append_control_interaction(
        app: &tauri::AppHandle,
        action: &str,
        kind: ControlInteractionKind,
    ) -> Result<()> {
        let record = render_control_interaction(&scrape_core::now_iso(), action, kind)?;
        append_record(&Self::get_log_file_path(app), &record)?;
        Ok(())
    }

    /// Append one complete, safe scrolling card. The caller supplies the actual
    /// persistence outcome; an append failure must not imply a database rollback.
    pub fn append_candidate_card(
        app: &tauri::AppHandle,
        run_id: &str,
        source_id: &str,
        evaluation: &CandidateEvaluation,
        outcome: &str,
        detail: Option<&str>,
    ) -> crate::error::Result<()> {
        let record = render_candidate_card(run_id, source_id, evaluation, outcome, detail)?;
        append_record(&Self::get_log_file_path(app), &record).map_err(|error| {
            crate::error::AppError::Internal(format!("Could not write candidate card to the console log: {error}. Check the application data directory permissions and free disk space."))
        })?;
        Ok(())
    }

    /// Print the already-resolved source requirements against the fixed run clock.
    /// This never reads provider config or recomputes matching/date semantics.
    pub fn append_requirements(
        app: &tauri::AppHandle,
        run_id: &str,
        source_id: &str,
        filters: &WorkerFilters,
        started_at: &str,
        description: &str,
    ) -> Result<()> {
        let record = render_requirements(run_id, source_id, filters, started_at, description)?;
        append_record(&Self::get_log_file_path(app), &record).map_err(|error| {
            crate::error::AppError::Internal(format!("Could not write source requirements to the console log: {error}. Check the application data directory permissions and free disk space."))
        })?;
        Ok(())
    }

    pub fn launch_terminal_console(app: &tauri::AppHandle) -> Result<()> {
        use tauri::Manager;
        let log_path = Self::get_log_file_path(app);
        // Also use the append lock when creating an empty stream.
        append_record(&log_path, "")?;

        let app_dir = crate::services::native_smoke::directory().unwrap_or_else(|| {
            app.path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
        });
        let script_path = app_dir.join("open_console.command");

        let log_path_str = log_path.to_string_lossy();
        let script_content = console_script(&log_path_str);

        std::fs::write(&script_path, script_content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-a")
                .arg("Terminal")
                .arg(&script_path)
                .spawn()
                .map_err(|e| {
                    crate::error::AppError::Internal(format!(
                        "Failed to launch Terminal.app: {}",
                        e
                    ))
                })?;
        }

        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("x-terminal-emulator")
                .arg("-e")
                .arg(&script_path)
                .spawn()
                .or_else(|_| {
                    std::process::Command::new("xterm")
                        .arg("-e")
                        .arg(&script_path)
                        .spawn()
                });
        }

        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args([
                    "/C",
                    "start",
                    "powershell",
                    "-NoExit",
                    "-Command",
                    &powershell_command(&log_path_str),
                ])
                .spawn();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[test]
    fn control_catalog_rejects_invalid_duplicate_and_unknown_actions() {
        assert!(
            parse_control_catalog(r#"{"workflow.delete":"Delete","workflow.delete":"Other"}"#)
                .is_err()
        );
        assert!(parse_control_catalog(r#"{"Bad Key":"Delete"}"#).is_err());
        assert!(parse_control_catalog(r#"{"workflow.delete":" "}"#).is_err());
        assert!(serde_json::from_str::<ControlInteractionKind>(r#""unsafe""#).is_err());
        assert!(render_control_interaction(
            "fixed",
            "secret user input",
            ControlInteractionKind::Activate
        )
        .is_err());
        let record = render_control_interaction(
            "fixed",
            "workflow.delete.request",
            ControlInteractionKind::Activate,
        )
        .unwrap();
        assert_eq!(
            record,
            "[fixed] [UI] UI activate workflow.delete.request: Request workflow deletion\n"
        );
    }

    #[test]
    fn control_records_append_atomically_without_user_content() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("scrape.log");
        std::thread::scope(|scope| {
            for _ in 0..2 {
                let path = &path;
                scope.spawn(move || {
                    for _ in 0..100 {
                        let record = render_control_interaction(
                            "fixed",
                            "workflow.delete.cancel",
                            ControlInteractionKind::Activate,
                        )
                        .unwrap();
                        append_record(path, &record).unwrap();
                    }
                });
            }
        });
        let contents = std::fs::read_to_string(path).unwrap();
        assert_eq!(contents.lines().count(), 200);
        assert!(contents.lines().all(|line| line
            == "[fixed] [UI] UI activate workflow.delete.cancel: Cancel workflow deletion"));
        assert!(!contents.contains("secret user input"));
    }

    fn candidate(sequence: u64, disposition: &str) -> CandidateEvaluation {
        serde_json::from_value(serde_json::json!({
            "sequence": sequence,
            "disposition": disposition,
            "candidate": {
                "external_id": format!("posting-{sequence}"),
                "title": "Software Engineer",
                "company": "Fixture Co",
                "url": "https://user:password@fixture.example/jobs/1?tracking=private&key=secret#fragment",
                "location": "Phoenix, AZ",
                "date_posted": "2026-10-25T12:00:00Z",
                "provider_updated_at": "2026-10-26T10:00:00Z",
                "description_excerpt": "Build reliable software.",
                "description_truncated": false,
                "role_type": "full_time",
                "locations": [{
                    "raw": "Phoenix, AZ", "city": "Phoenix", "region_code": "US-AZ",
                    "country_code": "US", "is_remote": false
                }],
                "location_data_complete": true,
                "workplace_type": "onsite",
                "salary": null,
                "experience": null,
                "is_new_grad": "unknown",
                "sponsorship": null,
                "work_auth": null,
                "clearance": null
            },
            "rejected": if disposition == "rejected" { vec!["posting_date", "geography"] } else { vec![] },
            "unresolved": if disposition == "unresolved" { vec!["geography"] } else { vec![] },
            "diagnostic": null
        }))
        .unwrap()
    }

    fn assert_safe_lines(record: &str) {
        for line in record.lines() {
            assert!(
                line.chars()
                    .map(|c| if c.is_ascii() { 1 } else { 2 })
                    .sum::<usize>()
                    <= CONSOLE_COLUMNS,
                "overwide line: {line:?}"
            );
            assert!(!line.chars().any(char::is_control), "{line:?}");
        }
    }

    #[test]
    fn candidate_card_preserves_separate_evidence_and_final_outcome() {
        let evaluation = candidate(7, "eligible");
        let card = render_candidate_card(
            "fixed-run",
            "muse",
            &evaluation,
            "UPDATED",
            Some("Existing saved user state retained."),
        )
        .unwrap();
        assert_eq!(
            card,
            render_candidate_card(
                "fixed-run",
                "muse",
                &evaluation,
                "UPDATED",
                Some("Existing saved user state retained."),
            )
            .unwrap()
        );
        assert!(card.starts_with("+-- CANDIDATE 7 | UPDATED --\n"));
        assert!(card.ends_with(&format!("{BORDER}\n")));
        for expected in [
            "| Run: fixed-run",
            "| Source: muse",
            "| Title: Software Engineer",
            "| External ID: posting-7",
            "| Location (raw): Phoenix, AZ",
            "| Location (normalized)[1].country_code: US",
            "| Location (normalized)[1].region_code: US-AZ",
            "| Location data complete: true",
            "| Original publication: 2026-10-25T12:00:00Z",
            "| Provider update: 2026-10-26T10:00:00Z",
            "| Compensation: Unknown",
            "| Workplace: onsite",
            "| Description excerpt: Build reliable software.",
        ] {
            assert!(card.contains(expected), "missing {expected}");
        }
        for secret in ["user:", "password", "private", "secret", "#fragment"] {
            assert!(!card.contains(secret), "leaked {secret}");
        }
        assert!(card.contains("REDACTED"));
        assert_safe_lines(&card);
        // Rendering redacts only a copy, not persistence/matcher evidence.
        assert!(evaluation
            .candidate
            .as_ref()
            .unwrap()
            .url
            .contains("password"));
    }

    #[test]
    fn invalid_without_job_uses_unknown_and_keeps_every_reason() {
        let mut evaluation = candidate(3, "invalid");
        evaluation.candidate = None;
        evaluation.diagnostic = Some("Adapter returned no identity".into());
        let card = render_candidate_card("run", "source", &evaluation, "INVALID", None).unwrap();
        for label in [
            "Title",
            "External ID",
            "Location (raw)",
            "Provider update",
            "Description excerpt",
        ] {
            assert!(card.contains(&format!("| {label}: Unknown")));
        }
        assert!(!card.contains("Software Engineer"));
        assert!(card.contains("Adapter returned no identity"));

        let mut rejected = candidate(4, "rejected");
        rejected.unresolved = serde_json::from_value(serde_json::json!(["geography"])).unwrap();
        let card = render_candidate_card("run", "source", &rejected, "REJECTED", None).unwrap();
        assert!(card.contains("Rejected categories[1]: posting_date"));
        assert!(card.contains("Rejected categories[2]: geography"));
        assert!(card.contains("Unresolved categories[1]: geography"));
    }

    #[test]
    fn escapes_terminal_controls_bidi_and_bounds_unicode_without_splitting_utf8() {
        let mut evaluation = candidate(1, "rejected");
        let facts = evaluation.candidate.as_mut().unwrap();
        facts.title = format!(
            "é界\u{001b}]0;fake title\u{0007}\n\r\u{009b}\u{202e}{}",
            "界".repeat(600)
        );
        facts.description_excerpt = Some("🦀".repeat(2_100));
        facts.description_truncated = true;
        let card = render_candidate_card(
            "run\nforged",
            "source\u{001b}[2J",
            &evaluation,
            "REJECTED\rFORGED",
            Some("failed\u{2066}\u{2028}detail"),
        )
        .unwrap();
        assert!(card.contains("\\u{001B}]0;fake title\\u{0007}\\n\\r\\u{009B}\\u{202E}"));
        assert!(card.contains("\\u{2066}\\u{2028}"));
        assert!(card.contains("run\\nforged"));
        assert_eq!(card.matches('🦀').count(), DESCRIPTION_LIMIT);
        assert!(card.matches("[truncated]").count() >= 2);
        assert_safe_lines(&card);

        for code in (0..=31).chain(127..=159) {
            let rendered = display_text(&char::from_u32(code).unwrap().to_string(), FIELD_LIMIT);
            assert!(!rendered.chars().any(char::is_control));
        }
    }

    #[test]
    fn ordinary_log_tags_and_messages_are_escaped_and_redacted_too() {
        let line = render_log(
            "fixed", "warn\nFAKE", Some("source\u{001b}[2J"),
            "message\r\nhttps://user:pass@example.test/a?plain=hidden&token=credential password=secret\u{202e}",
        );
        assert_eq!(line.lines().count(), 1);
        assert!(!line.chars().any(|c| c.is_control() && c != '\n'));
        assert!(line.contains("WARN\\nFAKE"));
        assert!(line.contains("source\\u{001B}[2J"));
        for secret in ["user:", "pass@", "hidden", "credential", "secret"] {
            assert!(!line.contains(secret), "leaked {secret}");
        }
        assert!(line.contains("REDACTED"));
        assert!(
            display_text("https://[malformed?token=private", FIELD_LIMIT).contains("[invalid URL]")
        );
    }

    #[test]
    fn source_header_shows_fixed_effective_requirements_without_recomputing() {
        let filters = WorkerFilters {
            include_keywords: vec!["AI Engineer".into(), "ML Engineer".into()],
            locations: vec!["United States".into()],
            query: Some("title LIKE '%Engineer%'".into()),
            ..Default::default()
        };
        let original = filters.clone();
        let header = render_requirements(
            "run",
            "muse",
            &filters,
            "2026-10-26T00:00:00Z",
            "The Muse feed; matching is applied locally; max_pages=50; details=1000",
        )
        .unwrap();
        assert!(header.contains("SOURCE REQUIREMENTS"));
        assert!(header.contains("Run started at: 2026-10-26T00:00:00Z"));
        assert!(header.contains("The Muse feed; matching is applied locally"));
        assert!(header.contains("Requirements.include_keywords[1]: AI Engineer"));
        assert!(header.contains("Requirements.include_keywords[2]: ML Engineer"));
        assert!(header.contains("Requirements.locations[1]: United States"));
        assert!(header.contains("Requirements.query: title LIKE '%Engineer%'"));
        assert_eq!(filters, original);
        assert_safe_lines(&header);
    }

    #[test]
    fn quoted_console_scripts_replay_complete_stream_and_never_interpolate_paths() {
        let path = "/tmp/a path/it's $HOME;$(touch bad).log";
        let quoted = "'/tmp/a path/it'\\''s $HOME;$(touch bad).log'";
        assert_eq!(posix_quote(path), quoted);
        let script = console_script(path);
        assert!(script.contains(&format!("exec tail -n +1 -f -- {quoted}\n")));
        assert!(!script.contains("-n 100"));
        assert!(!script.contains("\u{001b}"));
        assert_eq!(
            powershell_command(path),
            "Get-Content -LiteralPath '/tmp/a path/it''s $HOME;$(touch bad).log' -Wait"
        );
        assert!(!powershell_command(path).contains("-Tail"));
    }

    #[test]
    fn console_script_replays_literal_metacharacter_path() {
        use std::io::BufRead;
        use std::process::{Command, Stdio};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("it's $HOME;$(touch bad).log");
        std::fs::write(&path, "first complete record\nsecond complete record\n").unwrap();
        let mut child = Command::new("bash")
            .args(["-c", &console_script(path.to_str().unwrap())])
            .current_dir(directory.path())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut output = std::io::BufReader::new(stdout);
        let mut lines = Vec::new();
        for _ in 0..5 {
            let mut line = String::new();
            output.read_line(&mut line).unwrap();
            lines.push(line);
        }
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(lines[3], "first complete record\n");
        assert_eq!(lines[4], "second complete record\n");
        assert!(!directory.path().join("bad").exists());
    }

    #[test]
    fn append_failure_propagates_instead_of_reporting_a_card_written() {
        let directory = tempfile::tempdir().unwrap();
        let missing_parent = directory.path().join("missing").join("scrape.log");
        let record =
            render_candidate_card("run", "source", &candidate(1, "eligible"), "INSERTED", None)
                .unwrap();
        assert!(append_record(&missing_parent, &record).is_err());
    }

    #[test]
    fn thousand_complete_cards_and_concurrent_logs_never_interleave_records() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("scrape.log");
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                barrier.wait();
                for sequence in 1..=1_000 {
                    let (disposition, outcome) = match sequence {
                        1..=700 => ("rejected", "REJECTED"),
                        701..=900 => ("unresolved", "UNRESOLVED"),
                        901..=940 => ("duplicate", "DUPLICATE"),
                        941..=960 => ("blocked", "BLOCKED"),
                        961..=980 => ("invalid", "INVALID"),
                        _ => ("eligible", "INSERTED"),
                    };
                    let record = render_candidate_card(
                        "run",
                        "source",
                        &candidate(sequence, disposition),
                        outcome,
                        None,
                    )
                    .unwrap();
                    append_record(&path, &record).unwrap();
                }
            });
            scope.spawn(|| {
                barrier.wait();
                for sequence in 1..=1_000 {
                    append_record(
                        &path,
                        &render_log(
                            "fixed",
                            "INFO",
                            Some("source"),
                            &format!("ordinary-{sequence}"),
                        ),
                    )
                    .unwrap();
                }
            });
        });
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches("+-- CANDIDATE ").count(), 1_000);
        assert_eq!(
            content.lines().filter(|line| *line == BORDER).count(),
            2_000
        );
        assert_eq!(
            content.matches("[fixed] [INFO] [source] ordinary-").count(),
            1_000
        );
        for (outcome, count) in [
            ("REJECTED", 700),
            ("UNRESOLVED", 200),
            ("DUPLICATE", 40),
            ("BLOCKED", 20),
            ("INVALID", 20),
            ("INSERTED", 20),
        ] {
            assert_eq!(content.matches(&format!("| {outcome} --\n")).count(), count);
        }
        let mut in_card = false;
        let mut borders = 0;
        let mut next_sequence = 1;
        for line in content.lines() {
            if line.starts_with("+-- CANDIDATE ") {
                assert!(!in_card);
                assert!(line.starts_with(&format!("+-- CANDIDATE {next_sequence} | ")));
                next_sequence += 1;
                in_card = true;
                borders = 0;
            } else if line == BORDER {
                assert!(in_card);
                borders += 1;
                if borders == 2 {
                    in_card = false;
                }
            } else if line.starts_with("[fixed]") {
                assert!(!in_card, "ordinary log appeared inside a card");
            } else {
                assert!(in_card, "card line outside a card: {line}");
            }
        }
        assert!(!in_card);
        assert_eq!(next_sequence, 1_001);
    }

    #[test]
    fn test_infer_week_start() {
        assert_eq!(SystemService::infer_week_start("en-US"), "sunday");
        assert_eq!(SystemService::infer_week_start("en_CA"), "sunday");
        assert_eq!(SystemService::infer_week_start("en-GB"), "monday");
        assert_eq!(SystemService::infer_week_start("fr-FR"), "monday");
        assert_eq!(SystemService::infer_week_start("de-DE"), "monday");
    }

    #[tokio::test]
    async fn test_initialize_system_defaults() {
        let pool = init_memory_db().await.unwrap();
        let settings = SystemService::initialize_system_defaults(&pool)
            .await
            .unwrap();
        assert!(!settings.system_locale.is_empty());
        assert!(!settings.system_timezone.is_empty());
    }
}
