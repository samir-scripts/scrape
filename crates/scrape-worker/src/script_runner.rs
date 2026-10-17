//! Tier 3: Versioned Stdin/NDJSON script extraction engine.
//! Speaks SCRIPT_PROTOCOL_VERSION (1) over stdin/stdout NDJSON with structured argv,
//! bounded redacted stderr (max 16 KiB), and process tree termination.
use crate::adapters::selector::{SelectorPosting, UrlPatterns};
use crate::client::ClientError;
use chrono::{DateTime, Utc};
use scrape_core::enrich::DescriptionFormat;
use scrape_core::extraction::{
    redact_script_stderr, ScriptInputFrame, ScriptOutputFrame, ScriptRuntime, WorkflowScriptConfig,
    SCRIPT_PROTOCOL_VERSION,
};
use scrape_core::locator::apply_regex_capture;
use scrape_core::matching::MatchingEvidence;
use scrape_core::protocol::CoverageTermination;
use scrape_core::{WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource};
use std::collections::VecDeque;
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::time::{timeout, Instant};
use url::Url;

pub(crate) enum ScriptStep {
    Event(Box<(WorkerEvent, Option<MatchingEvidence>)>),
    Yield,
    Exhausted,
}

pub(crate) async fn kill_script_child(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        let _ = std::process::Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

struct FieldRegexes {
    title: Option<regex::Regex>,
    link: Option<regex::Regex>,
    location: Option<regex::Regex>,
    salary: Option<regex::Regex>,
    date: Option<regex::Regex>,
    description: Option<regex::Regex>,
    id: Option<regex::Regex>,
}

impl FieldRegexes {
    fn parse(config: &serde_json::Value) -> Result<Self, ClientError> {
        let compile = |key: &str| -> Result<Option<regex::Regex>, ClientError> {
            match config.get(key).and_then(serde_json::Value::as_str) {
                Some(s) if !s.trim().is_empty() => regex::Regex::new(s)
                    .map(Some)
                    .map_err(|e| ClientError::Configuration(format!("{key}: {e}"))),
                _ => Ok(None),
            }
        };
        Ok(Self {
            title: compile("title_regex")?,
            link: compile("link_regex")?,
            location: compile("location_regex")?,
            salary: compile("salary_regex")?,
            date: compile("date_regex")?,
            description: compile("description_regex")?,
            id: compile("id_regex")?,
        })
    }
}

pub(crate) struct ScriptSession {
    source: WorkerSource,
    script_config: WorkflowScriptConfig,
    patterns: UrlPatterns,
    regexes: FieldRegexes,
    company_logo: Option<scrape_core::branding::LogoEvidence>,
    source_deadline: Instant,
    turn_timeout: Duration,
    continuation_cursor: Option<String>,
    exhausted: bool,
    turns_run: u32,
    pending: VecDeque<(WorkerEvent, Option<MatchingEvidence>)>,
}

impl ScriptSession {
    pub fn new(
        source: &WorkerSource,
        limits: &WorkerLimits,
        workflow_deadline: Instant,
    ) -> Result<Self, ClientError> {
        let script_config = scrape_core::extraction::script(&source.config)
            .map_err(ClientError::Configuration)?
            .ok_or_else(|| {
                ClientError::Configuration("Missing script configuration in script mode".into())
            })?;
        let source_limits =
            scrape_core::extraction::limits(&source.config).map_err(ClientError::Configuration)?;
        let source_deadline = workflow_deadline
            .min(Instant::now() + Duration::from_secs(source_limits.max_source_duration_s));
        let turn_timeout = Duration::from_secs(
            script_config.timeout_s.unwrap_or(
                source_limits
                    .navigation_timeout_s
                    .min(limits.navigation_timeout_s),
            ),
        );
        let patterns = UrlPatterns::parse(&source.config)?;
        let regexes = FieldRegexes::parse(&source.config)?;
        let company_logo = source
            .config
            .get("employer_logo")
            .and_then(serde_json::Value::as_str)
            .filter(|raw| scrape_core::branding::http_url(raw))
            .map(|url| scrape_core::branding::LogoEvidence {
                url: url.to_string(),
                page_url: source.identifier.clone(),
                method: scrape_core::branding::LogoMethod::ConfiguredSelector,
            });

        Ok(Self {
            source: source.clone(),
            script_config,
            patterns,
            regexes,
            company_logo,
            source_deadline,
            turn_timeout,
            continuation_cursor: None,
            exhausted: false,
            turns_run: 0,
            pending: VecDeque::new(),
        })
    }

    pub fn turns_run(&self) -> u32 {
        self.turns_run
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted && self.pending.is_empty()
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub async fn next(
        &mut self,
        _filters: &WorkerFilters,
        _started: DateTime<Utc>,
        workflow_cap: u32,
    ) -> Result<ScriptStep, ClientError> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(ScriptStep::Event(Box::new(event)));
        }

        if self.exhausted {
            return Ok(ScriptStep::Exhausted);
        }

        if Instant::now() >= self.source_deadline {
            self.exhausted = true;
            return Ok(ScriptStep::Event(Box::new((
                WorkerEvent::Warning {
                    source_id: self.source.id.clone(),
                    message:
                        "Script source duration limit reached; remaining candidates were not examined"
                            .into(),
                    eligibility_incomplete: true,
                    termination: Some(CoverageTermination::TimeLimit),
                },
                None,
            ))));
        }

        let max_candidates = workflow_cap.clamp(1, 10);
        let turn_result = run_script_turn(
            &self.source,
            &self.script_config,
            self.continuation_cursor.take(),
            max_candidates,
            self.turn_timeout,
            self.source_deadline,
        )
        .await;

        let frames = match turn_result {
            Ok(res) => res.frames,
            Err(e) => {
                self.exhausted = true;
                return Err(e);
            }
        };

        let mut saw_continuation = None;
        for frame in frames {
            match frame {
                ScriptOutputFrame::Diagnostic { level, message, .. } => {
                    self.pending.push_back((
                        WorkerEvent::Warning {
                            source_id: self.source.id.clone(),
                            message: format!("Script diagnostic [{level}]: {message}"),
                            eligibility_incomplete: false,
                            termination: None,
                        },
                        None,
                    ));
                }
                ScriptOutputFrame::Continuation { cursor, .. } => {
                    saw_continuation = Some(cursor);
                }
                ScriptOutputFrame::Complete { exhausted, .. } => {
                    if exhausted {
                        self.exhausted = true;
                    }
                }
                ScriptOutputFrame::Candidate {
                    external_id,
                    mut title,
                    mut url,
                    location,
                    salary,
                    description,
                    date_posted,
                    company,
                    employer_logo_url,
                    ..
                } => {
                    if title.trim().is_empty() {
                        self.pending.push_back((
                            WorkerEvent::Error {
                                source_id: self.source.id.clone(),
                                message: "Script candidate has missing or blank title".into(),
                                retryable: false,
                                invalid_posting: Some(true),
                            },
                            None,
                        ));
                        continue;
                    }

                    let parsed_url = match Url::parse(&url) {
                        Ok(u) if matches!(u.scheme(), "http" | "https") => u,
                        _ => {
                            self.pending.push_back((
                                WorkerEvent::Error {
                                    source_id: self.source.id.clone(),
                                    message: format!(
                                        "Script candidate '{title}' has invalid HTTP(S) URL: '{url}'"
                                    ),
                                    retryable: false,
                                    invalid_posting: Some(true),
                                },
                                None,
                            ));
                            continue;
                        }
                    };
                    url = parsed_url.to_string();

                    if !self.patterns.matches(&url) {
                        continue;
                    }

                    if let Some(ref re) = self.regexes.title {
                        match apply_regex_capture(re, &title) {
                            Some(matched) => title = matched,
                            None => {
                                self.pending.push_back((
                                    WorkerEvent::Error {
                                        source_id: self.source.id.clone(),
                                        message: format!(
                                            "Posting '{title}' failed required title regex transform"
                                        ),
                                        retryable: false,
                                        invalid_posting: Some(true),
                                    },
                                    None,
                                ));
                                continue;
                            }
                        }
                    }

                    if let Some(ref re) = self.regexes.link {
                        if let Some(matched) = apply_regex_capture(re, &url) {
                            url = matched;
                        }
                    }

                    let location = location
                        .and_then(|l| {
                            self.regexes
                                .location
                                .as_ref()
                                .and_then(|re| apply_regex_capture(re, &l))
                                .or(Some(l))
                        })
                        .unwrap_or_default();

                    let salary_text = salary.and_then(|s| {
                        self.regexes
                            .salary
                            .as_ref()
                            .and_then(|re| apply_regex_capture(re, &s))
                            .or(Some(s))
                    });

                    let date_posted = date_posted.and_then(|d| {
                        self.regexes
                            .date
                            .as_ref()
                            .and_then(|re| apply_regex_capture(re, &d))
                            .or(Some(d))
                    });

                    let description_source = description.and_then(|d| {
                        self.regexes
                            .description
                            .as_ref()
                            .and_then(|re| apply_regex_capture(re, &d))
                            .or(Some(d))
                    });

                    let ext_id = match external_id {
                        Some(id) => self
                            .regexes
                            .id
                            .as_ref()
                            .and_then(|re| apply_regex_capture(re, &id))
                            .unwrap_or(id),
                        None => url.clone(),
                    };

                    let logo = self.company_logo.clone().or_else(|| {
                        employer_logo_url.as_deref().and_then(|raw_logo| {
                            let resolved = crate::adapters::branding::logo_url(
                                &serde_json::Value::String(raw_logo.to_string()),
                                &url,
                            )?;
                            Some(scrape_core::branding::LogoEvidence {
                                url: resolved,
                                page_url: url.clone(),
                                method: scrape_core::branding::LogoMethod::JobPosting,
                            })
                        })
                    });

                    let posting = SelectorPosting {
                        title: title.clone(),
                        url: url.clone(),
                        external_id: ext_id,
                        location,
                        salary_text: salary_text.clone(),
                        description_source,
                        description_format: Some(DescriptionFormat::Html),
                        date_posted: date_posted.clone(),
                        company_logo: logo,
                        detail_url: None,
                    };

                    let mut event = posting.event(&self.source);
                    if let Some(comp) = company {
                        if !comp.trim().is_empty() {
                            if let WorkerEvent::Job { company: c, .. } = &mut event {
                                *c = comp;
                            }
                        }
                    }

                    let evidence = candidate_evidence(&event);

                    self.pending.push_back((event, Some(evidence)));
                }
            }
        }

        self.continuation_cursor = saw_continuation;
        if self.continuation_cursor.is_none() {
            self.exhausted = true;
        }
        self.turns_run += 1;

        if let Some(event) = self.pending.pop_front() {
            return Ok(ScriptStep::Event(Box::new(event)));
        }

        if self.exhausted {
            Ok(ScriptStep::Exhausted)
        } else {
            Ok(ScriptStep::Yield)
        }
    }
}

struct ScriptTurnResult {
    frames: Vec<ScriptOutputFrame>,
}

async fn run_script_turn(
    source: &WorkerSource,
    script_cfg: &WorkflowScriptConfig,
    continuation: Option<String>,
    max_candidates: u32,
    turn_timeout: Duration,
    workflow_deadline: Instant,
) -> Result<ScriptTurnResult, ClientError> {
    let (program, args, _temp_file) = match &script_cfg.runtime {
        ScriptRuntime::Js => {
            let interp = script_cfg
                .interpreter
                .as_deref()
                .unwrap_or("node")
                .to_string();
            if let Some(code) = &script_cfg.code {
                let mut temp = tempfile::Builder::new()
                    .prefix("scrape-script-")
                    .suffix(".js")
                    .tempfile()
                    .map_err(|e| {
                        ClientError::Extraction(format!(
                            "Failed to create temporary script file: {e}"
                        ))
                    })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        temp.path(),
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
                temp.write_all(code.as_bytes()).map_err(|e| {
                    ClientError::Extraction(format!("Failed to write script code: {e}"))
                })?;
                temp.flush().map_err(|e| {
                    ClientError::Extraction(format!("Failed to flush script code: {e}"))
                })?;
                let path_str = temp.path().to_string_lossy().to_string();
                let mut all_args = vec![path_str];
                if let Some(custom_args) = &script_cfg.args {
                    all_args.extend(custom_args.iter().cloned());
                }
                (interp, all_args, Some(temp))
            } else if let Some(path) = &script_cfg.path {
                let mut all_args = vec![path.clone()];
                if let Some(custom_args) = &script_cfg.args {
                    all_args.extend(custom_args.iter().cloned());
                }
                (interp, all_args, None)
            } else {
                return Err(ClientError::Configuration(
                    "Script specifies neither code nor path".into(),
                ));
            }
        }
        ScriptRuntime::Python => {
            let interp = script_cfg.interpreter.clone().ok_or_else(|| {
                ClientError::Configuration(
                    "Python runtime requires an explicit interpreter path".into(),
                )
            })?;
            if let Some(code) = &script_cfg.code {
                let mut temp = tempfile::Builder::new()
                    .prefix("scrape-script-")
                    .suffix(".py")
                    .tempfile()
                    .map_err(|e| {
                        ClientError::Extraction(format!(
                            "Failed to create temporary script file: {e}"
                        ))
                    })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        temp.path(),
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
                temp.write_all(code.as_bytes()).map_err(|e| {
                    ClientError::Extraction(format!("Failed to write script code: {e}"))
                })?;
                temp.flush().map_err(|e| {
                    ClientError::Extraction(format!("Failed to flush script code: {e}"))
                })?;
                let path_str = temp.path().to_string_lossy().to_string();
                let mut all_args = vec![path_str];
                if let Some(custom_args) = &script_cfg.args {
                    all_args.extend(custom_args.iter().cloned());
                }
                (interp, all_args, Some(temp))
            } else if let Some(path) = &script_cfg.path {
                let mut all_args = vec![path.clone()];
                if let Some(custom_args) = &script_cfg.args {
                    all_args.extend(custom_args.iter().cloned());
                }
                (interp, all_args, None)
            } else {
                return Err(ClientError::Configuration(
                    "Script specifies neither code nor path".into(),
                ));
            }
        }
        ScriptRuntime::Custom => {
            let interp = script_cfg.interpreter.clone().ok_or_else(|| {
                ClientError::Configuration(
                    "Custom runtime requires an explicit executable path".into(),
                )
            })?;
            if let Some(code) = &script_cfg.code {
                let mut temp = tempfile::Builder::new()
                    .prefix("scrape-script-")
                    .suffix(".script")
                    .tempfile()
                    .map_err(|e| {
                        ClientError::Extraction(format!(
                            "Failed to create temporary script file: {e}"
                        ))
                    })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        temp.path(),
                        std::fs::Permissions::from_mode(0o700),
                    );
                }
                temp.write_all(code.as_bytes()).map_err(|e| {
                    ClientError::Extraction(format!("Failed to write script code: {e}"))
                })?;
                temp.flush().map_err(|e| {
                    ClientError::Extraction(format!("Failed to flush script code: {e}"))
                })?;
                let path_str = temp.path().to_string_lossy().to_string();
                let mut all_args = vec![path_str];
                if let Some(custom_args) = &script_cfg.args {
                    all_args.extend(custom_args.iter().cloned());
                }
                (interp, all_args, Some(temp))
            } else {
                let mut all_args = Vec::new();
                if let Some(path) = &script_cfg.path {
                    all_args.push(path.clone());
                }
                if let Some(custom_args) = &script_cfg.args {
                    all_args.extend(custom_args.iter().cloned());
                }
                (interp, all_args, None)
            }
        }
    };

    let mut cmd = tokio::process::Command::new(&program);
    cmd.args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = cmd.spawn().map_err(|e| {
        ClientError::Extraction(format!("Failed to execute script '{program}': {e}"))
    })?;

    let timeout_ms = turn_timeout.as_millis().min(600_000) as u64;
    let input_frame = ScriptInputFrame {
        version: SCRIPT_PROTOCOL_VERSION,
        source_id: source.id.clone(),
        url: source.identifier.clone(),
        continuation,
        max_candidates,
        timeout_ms,
        config: source.config.clone(),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let mut payload = serde_json::to_vec(&input_frame).map_err(|e| {
            ClientError::Extraction(format!("Failed to serialize script input frame: {e}"))
        })?;
        payload.push(b'\n');
        stdin.write_all(&payload).await.map_err(|e| {
            ClientError::Extraction(format!("Failed to write to script stdin: {e}"))
        })?;
        let _ = stdin.flush().await;
    }

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");

    let read_stdout = async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut frames = Vec::new();
        while let Some(line) = lines.next_line().await.map_err(|e| {
            ClientError::Extraction(format!("Failed reading line from script stdout: {e}"))
        })? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let frame: ScriptOutputFrame = serde_json::from_str(trimmed).map_err(|e| {
                ClientError::Extraction(format!(
                    "Script emitted invalid NDJSON on stdout: {trimmed:?} - {e}"
                ))
            })?;
            if frame.version() != SCRIPT_PROTOCOL_VERSION {
                return Err(ClientError::Extraction(format!(
                    "Script emitted unsupported protocol version {}; expected {SCRIPT_PROTOCOL_VERSION}",
                    frame.version()
                )));
            }
            frames.push(frame);
        }
        Ok(frames)
    };

    let read_stderr = async move {
        let mut buf = Vec::new();
        let mut reader = stderr.take(16_385);
        let _ = reader.read_to_end(&mut buf).await;
        buf
    };

    let remaining_workflow = workflow_deadline.saturating_duration_since(Instant::now());
    let run_timeout = turn_timeout.min(remaining_workflow);

    let (stdout_res, stderr_bytes, wait_res) = match timeout(run_timeout, async {
        let (out_res, err_bytes) = tokio::join!(read_stdout, read_stderr);
        let status = child.wait().await;
        (out_res, err_bytes, status)
    })
    .await
    {
        Ok(tuple) => tuple,
        Err(_) => {
            kill_script_child(&mut child).await;
            return Err(ClientError::TimeLimit(
                "Script execution reached its time limit".into(),
            ));
        }
    };

    let redacted_stderr = redact_script_stderr(&stderr_bytes);
    let exit_status = wait_res
        .map_err(|e| ClientError::Extraction(format!("Failed waiting for script child: {e}")))?;

    if !exit_status.success() {
        let code_str = exit_status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "terminated by signal".into());
        let detail = if redacted_stderr.trim().is_empty() {
            String::new()
        } else {
            format!("; stderr: {redacted_stderr}")
        };
        return Err(ClientError::Extraction(format!(
            "Script exited with non-zero exit status ({code_str}){detail}"
        )));
    }

    let frames = stdout_res?;
    let has_complete = frames
        .iter()
        .any(|f| matches!(f, ScriptOutputFrame::Complete { .. }));
    if !has_complete {
        let detail = if redacted_stderr.trim().is_empty() {
            String::new()
        } else {
            format!("; stderr: {redacted_stderr}")
        };
        return Err(ClientError::Extraction(format!(
            "Script exited successfully without emitting a complete frame{detail}"
        )));
    }

    Ok(ScriptTurnResult { frames })
}

fn candidate_evidence(event: &WorkerEvent) -> MatchingEvidence {
    let mut fields = vec![
        scrape_core::FilterField::Title,
        scrape_core::FilterField::Company,
        scrape_core::FilterField::RoleType,
    ];
    let WorkerEvent::Job {
        location,
        description_text,
        date_posted,
        salary,
        ..
    } = event
    else {
        return MatchingEvidence::default();
    };
    if !location.is_empty() {
        fields.push(scrape_core::FilterField::Location);
    }
    if description_text.is_some() {
        fields.push(scrape_core::FilterField::Description);
    }
    MatchingEvidence {
        fields,
        posting_date: date_posted.is_some(),
        compensation: salary.is_some(),
        opportunity_exhausted: true,
    }
}
