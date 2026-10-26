//! Isolated provider acceptance through canonical workflow persistence, packaged
//! worker execution, host budgets, ingestion and the production Discovered query.
//! This is not a Tauri/WebView runner: native/UI journeys are reported separately.
use super::{
    execution_diagnostics, ingestion, provider_verification_profiles, script_trust, workflow,
};
use crate::db::{
    jobs::{JobFilter, JobsRepo, ScrapedJob},
    postings,
    runs::{RunCounts, RunStatus, RunsRepo, TriggerType},
    sources::SourcesRepo,
};
use scrape_core::{JobStatus, WorkerConfig, WorkerEvent};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

type VResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Deserialize)]
struct Route {
    path: String,
    role: String,
    status: u16,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    json_pointer: Option<String>,
    #[serde(default)]
    body: Option<String>,
    set: String,
}
#[derive(Clone, Deserialize)]
struct Case {
    id: String,
    template_id: String,
    companies: Vec<String>,
    control: String,
    fixture_class: String,
    route_set: String,
    #[serde(default)]
    source_company: Option<String>,
    #[serde(default)]
    live: bool,
    expected: Value,
    reason: Option<String>,
}
#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    routes: Vec<Route>,
    workflow_cases: Vec<Case>,
    explicit_gaps: Vec<Value>,
    extractors: Value,
}

struct Storage(PathBuf);
impl Storage {
    fn new() -> VResult<Self> {
        let path = std::env::temp_dir().join(format!(
            "scrape-provider-verification-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Storage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Replay {
    origin: String,
    running: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<String>>>,
    unexpected: Arc<Mutex<Vec<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Replay {
    fn start(root: &Path, routes: &[Route], selected: &str) -> VResult<Self> {
        let mut responses = BTreeMap::new();
        for route in routes
            .iter()
            .filter(|r| r.set == "shared" || r.set == selected)
        {
            let body = if let Some(file) = &route.file {
                let bytes = checked_bytes(
                    root,
                    file,
                    route.sha256.as_deref().ok_or("Missing route checksum")?,
                )?;
                if let Some(pointer) = &route.json_pointer {
                    let value: Value = serde_json::from_slice(&bytes)?;
                    serde_json::to_vec(
                        value
                            .pointer(pointer)
                            .ok_or("Invalid detail projection pointer")?,
                    )?
                } else {
                    bytes
                }
            } else {
                route
                    .body
                    .as_ref()
                    .ok_or("Route lacks an explicit response body")?
                    .as_bytes()
                    .to_vec()
            };
            if responses
                .insert(route.path.clone(), (route.status, body, route.role.clone()))
                .is_some()
            {
                return Err(format!("Duplicate exact fixture route {}", route.path).into());
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let origin = format!("http://{}", listener.local_addr()?);
        listener.set_nonblocking(true)?;
        let running = Arc::new(AtomicBool::new(true));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let unexpected = Arc::new(Mutex::new(Vec::new()));
        let active = running.clone();
        let observed = requests.clone();
        let errors = unexpected.clone();
        let thread = std::thread::spawn(move || {
            while active.load(Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                };
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let mut bytes = [0; 16384];
                let count = stream.read(&mut bytes).unwrap_or(0);
                let request = String::from_utf8_lossy(&bytes[..count]);
                let mut words = request.split_whitespace();
                let method = words.next().unwrap_or("");
                let path = words.next().unwrap_or("");
                observed.lock().unwrap().push(path.to_string());
                let fallback = (
                    404,
                    b"Unexpected fixture route; no listing fallback".to_vec(),
                    "unexpected".to_string(),
                );
                let response = if method == "GET" {
                    responses.get(path)
                } else {
                    None
                };
                if response.is_none() {
                    errors.lock().unwrap().push(format!("{method} {path}"));
                }
                let (status, body, _) = response.unwrap_or(&fallback);
                let status_text = if *status == 200 { "OK" } else { "Not Found" };
                let content_type = if body.first() == Some(&b'<') {
                    "text/html; charset=utf-8"
                } else {
                    "application/json"
                };
                let _ = write!(stream, "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(body);
            }
        });
        Ok(Self {
            origin,
            running,
            requests,
            unexpected,
            thread: Some(thread),
        })
    }
    fn evidence(&self) -> Value {
        json!({"requests":self.requests.lock().unwrap().clone(),"unexpected_routes":self.unexpected.lock().unwrap().clone()})
    }
}
impl Drop for Replay {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn checked_bytes(root: &Path, file: &str, expected: &str) -> VResult<Vec<u8>> {
    let relative = Path::new(file);
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("Fixture paths must stay inside the repository".into());
    }
    let bytes = std::fs::read(root.join(relative))?;
    let observed = format!("{:x}", Sha256::digest(&bytes));
    if observed != expected {
        return Err(format!(
            "Fixture checksum mismatch for {file}: expected {expected}, observed {observed}"
        )
        .into());
    }
    Ok(bytes)
}

fn launch(
    config: &WorkerConfig,
    cwd: &Path,
    origin: Option<&str>,
    live: bool,
) -> VResult<Vec<WorkerEvent>> {
    let mut command = Command::new(super::process::packaged_worker_path()?);
    command
        .arg("--stdin")
        .current_dir(cwd)
        .env("SCRAPE_SESSION_DIR", cwd.join("sessions"))
        .env_remove("SCRAPE_CDP_WS_URL")
        .env_remove("SCRAPE_REPLAY_ORIGIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(origin) = origin {
        command
            .args(["--replay-origin", origin])
            .env("SCRAPE_REPLAY_ORIGIN", origin);
    }
    let mut child = command.spawn()?;
    child
        .stdin
        .take()
        .ok_or("Missing worker stdin")?
        .write_all(&serde_json::to_vec(config)?)?;
    let mut stdout = child.stdout.take().ok_or("Missing worker stdout")?;
    let mut stderr = child.stderr.take().ok_or("Missing worker stderr")?;
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(if live { 660 } else { 90 });
    let timed_out = loop {
        if child.try_wait()?.is_some() {
            break false;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            break true;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let status = child.wait()?;
    let output = out.join().map_err(|_| "Worker stdout reader panicked")??;
    let diagnostic = err.join().map_err(|_| "Worker stderr reader panicked")??;
    if timed_out || !status.success() {
        return Err(format!(
            "Packaged worker {}: {}",
            if timed_out {
                "timed out"
            } else {
                "exited unsuccessfully"
            },
            scrape_core::extraction::redact_script_stderr(&diagnostic)
        )
        .into());
    }
    let events: Vec<WorkerEvent> = std::str::from_utf8(&output)?
        .lines()
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    if events
        .iter()
        .filter(|e| matches!(e, WorkerEvent::Done { .. }))
        .count()
        != 1
        || !matches!(events.last(), Some(WorkerEvent::Done { .. }))
    {
        return Err("Worker must report exactly one terminal Done frame at the end".into());
    }
    Ok(events)
}

fn validated_jobs(config: &WorkerConfig, events: &[WorkerEvent]) -> VResult<Vec<ScrapedJob>> {
    let Some(WorkerEvent::Done { stats }) = events.last() else {
        return Err("Missing terminal Done".into());
    };
    let owned: BTreeSet<_> = config.sources.iter().map(|s| s.id.as_str()).collect();
    let reported: BTreeSet<_> = stats.sources.iter().map(|s| s.source_id.as_str()).collect();
    if stats.sources.len() != owned.len() || reported != owned {
        return Err("Missing, duplicated or foreign child source summary".into());
    }
    let mut batch = Vec::new();
    let mut emitted_by_source = BTreeMap::<&str, u64>::new();
    for event in events {
        if let Some(id) = event.source_id() {
            if !owned.contains(id) {
                return Err("Worker emitted a foreign source ID".into());
            }
        }
        if let WorkerEvent::Job { source_id, .. } = event {
            *emitted_by_source.entry(source_id).or_default() += 1;
            batch.push(ingestion::scraped_job_from_event(event.clone())?);
        }
    }
    if batch.len() as u32
        > config
            .max_jobs_per_run
            .unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN)
    {
        return Err("Worker exceeded shared workflow accepted-job budget".into());
    }
    for scan in &stats.sources {
        scan.validate()?;
        if scan.commit_feedback
            || scan.accepted
                != emitted_by_source
                    .get(scan.source_id.as_str())
                    .copied()
                    .unwrap_or(0)
        {
            return Err("Source accepted counters disagree with emitted jobs".into());
        }
        if scan.outcome.is_none() {
            return Err("Source summary lacks its terminal acceptance outcome".into());
        }
    }
    if stats.jobs_emitted != batch.len() as u64
        || stats.sources.iter().map(|s| s.accepted).sum::<u64>() != batch.len() as u64
    {
        return Err("Terminal accepted counters disagree with emitted jobs".into());
    }
    Ok(batch)
}

fn discovered(workflow_id: &str) -> JobFilter {
    JobFilter {
        source_id: Some(workflow_id.into()),
        statuses: Some(vec![JobStatus::Scraped, JobStatus::Viewed]),
        limit: Some(100),
        ..Default::default()
    }
}

/// Reject discrepancies, rather than converting unexpected zeroes into passes.
/// Kept public so integration regressions can mutate independently authored observations.
pub fn validate_observation(expected: &Value, observed: &Value) -> std::result::Result<(), String> {
    let expected_jobs = expected["jobs"].as_array().ok_or("Expected jobs missing")?;
    let jobs = observed["jobs"].as_array().ok_or("Observed jobs missing")?;
    let keys = |rows: &[Value]| -> std::result::Result<BTreeSet<(String, String)>, String> {
        rows.iter()
            .map(|v| {
                Ok((
                    v["external_id"]
                        .as_str()
                        .ok_or("Missing external ID")?
                        .into(),
                    v["url"].as_str().ok_or("Missing destination")?.into(),
                ))
            })
            .collect()
    };
    if keys(expected_jobs)? != keys(jobs)? || jobs.len() != expected_jobs.len() {
        return Err(format!("Exact provider IDs/destinations mismatch: expected {expected_jobs:?}, observed {jobs:?}"));
    }
    for item in expected_jobs {
        let job = jobs
            .iter()
            .find(|j| j["external_id"] == item["external_id"])
            .ok_or("Missing expected posting")?;
        for field in [
            "location",
            "posting_identity",
            "role_type",
            "workplace_type",
            "date_posted",
        ] {
            if let Some(evidence) = item.get(field) {
                if job.get(field) != Some(evidence) {
                    return Err(format!("Required {field} evidence mismatch"));
                }
            }
        }
        if let Some(country) = item.get("required_country") {
            let tuples = job["locations"]
                .as_array()
                .ok_or("Missing geographic field evidence")?;
            if !tuples.iter().any(|tuple| {
                tuple["country_code"] == *country
                    && item
                        .get("required_region")
                        .is_none_or(|region| tuple["region_code"] == *region)
            }) {
                return Err("Required same-tuple geographic evidence missing".into());
            }
        }
    }
    let stats = observed["source_stats"]
        .as_array()
        .ok_or("Missing source scan evidence")?;
    if stats.is_empty() {
        return Err("Missing source scan evidence".into());
    }
    for scan in stats {
        if scan["termination"] != expected["terminal"] {
            return Err(format!(
                "Unexpected terminal coverage: expected {}, observed {}",
                expected["terminal"], scan["termination"]
            ));
        }
        if scan["unresolved"].as_u64().unwrap_or(u64::MAX) != 0 {
            return Err("Unresolved required field evidence prevents acceptance".into());
        }
        if let Some(outcomes) = expected.get("outcomes").and_then(Value::as_array) {
            if !outcomes.contains(&scan["outcome"]) {
                return Err("Unexpected source acceptance outcome".into());
            }
        }
    }
    if let Some(expected_sources) = expected.get("source_counts").and_then(Value::as_object) {
        if expected_sources.len() != stats.len() {
            return Err("Missing expected child-source counter rows".into());
        }
        for (company, counters) in expected_sources {
            let scan = stats
                .iter()
                .find(|scan| {
                    scan["source_id"].as_str().is_some_and(|id| {
                        observed["source_companies"][id].as_str() == Some(company.as_str())
                    })
                })
                .ok_or_else(|| format!("Missing source evidence for {company}"))?;
            for (counter, value) in counters.as_object().ok_or("Missing expected counters")? {
                if scan[counter] != *value {
                    return Err(format!(
                        "{company} {counter} mismatch: expected {value}, observed {}",
                        scan[counter]
                    ));
                }
            }
        }
    }
    let errors = observed["errors"]
        .as_array()
        .ok_or("Missing error evidence")?;
    let allowed = expected["errors"]
        .as_array()
        .ok_or("Missing expected errors")?;
    if errors.iter().any(|e| {
        !allowed.iter().any(|a| {
            e.as_str()
                .unwrap_or("")
                .contains(a.as_str().unwrap_or("\0"))
        })
    }) || (!allowed.is_empty() && errors.is_empty())
    {
        return Err(format!(
            "Unexpected worker errors: {errors:?}; expected {allowed:?}"
        ));
    }
    if let Some(count) = expected.get("error_count").and_then(Value::as_u64) {
        if errors.len() as u64 != count {
            return Err("Unexpected number of source errors".into());
        }
    }
    let requests = observed["routes"]["requests"]
        .as_array()
        .ok_or("Missing exact route evidence")?;
    for required in expected["required_routes"]
        .as_array()
        .ok_or("Missing required route assertions")?
    {
        if !requests.contains(required) {
            return Err(format!("Missing captured listing/page request {required}"));
        }
    }
    if observed["routes"]["unexpected_routes"]
        .as_array()
        .is_none_or(|r| !r.is_empty())
    {
        return Err("Unexpected fixture route requested".into());
    }
    if observed["inserted"].as_u64() != Some(jobs.len() as u64)
        || observed["discovered"].as_u64() != Some(jobs.len() as u64)
    {
        return Err("Accepted postings did not reach Discovered".into());
    }
    if observed["saved_filters_match"] != true {
        return Err("Restrictive/template controls changed before execution".into());
    }
    Ok(())
}

async fn run_case(
    root: &Path,
    manifest: &Manifest,
    case: &Case,
    live: bool,
    refresh_example: bool,
) -> VResult<Value> {
    let storage = Storage::new()?;
    let pool = crate::db::init_db(&storage.0.join("verification.db")).await?;
    if sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await?
        != 1
    {
        return Err("SQLite foreign keys must remain enabled".into());
    }
    let parent = provider_verification_profiles::save(&pool, &case.template_id).await?;
    let mut spec = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC")?;
    let node = std::env::var("SCRAPE_VERIFY_NODE").ok();
    for entry in &mut spec.sources {
        entry.enabled = case
            .source_company
            .as_ref()
            .is_none_or(|name| entry.employer_hint.as_ref() == Some(name));
        if let Some(script) = &mut entry.extractor.script {
            let interpreter=node.as_ref().ok_or("Set SCRAPE_VERIFY_NODE to the absolute local Node.js executable for explicit verification setup")?;
            if !Path::new(interpreter).is_absolute() || !Path::new(interpreter).is_file() {
                return Err("SCRAPE_VERIFY_NODE must name an absolute executable file".into());
            }
            let name = entry
                .employer_hint
                .as_deref()
                .ok_or("Script employer missing")?
                .to_lowercase();
            let expected = manifest.extractors[&name]["sha256"]
                .as_str()
                .ok_or("Manifest lacks extractor checksum")?;
            let code = script
                .code
                .as_deref()
                .ok_or("Canonical script is not embedded")?;
            if format!("{:x}", Sha256::digest(code.as_bytes())) != expected {
                return Err(format!("Manifest extractor version drift for {name}").into());
            }
            script.interpreter = Some(interpreter.clone());
            // Transport, not fixture-path injection: exact provider URLs/cursors
            // pass through the real extractor and the explicit replay origin.
            script.args = None;
        }
    }
    if case.control == "unrestricted" {
        spec.filters = None;
    } else if case.control == "strict" {
        spec.filters = Some(serde_json::from_value(
            json!({"matching":{"keywords":{"all":["SECTION7_NO_MATCH_CONTROL"],"fields":["title"]}}}),
        )?);
    } else if case.control != "template" {
        return Err("Unknown manifest matching control".into());
    }
    let executable = spec.sources.iter().any(|s| s.extractor.script.is_some());
    spec.metadata.enabled = Some(!executable);
    // The same typed save/import boundary used by the editor, with canonical
    // owned IDs allocated by persistence. Executables are saved disabled.
    let saved = workflow::persist(&pool, spec, Some(parent.id.clone()), true, "UTC").await?;
    let mut spec = workflow::validate(saved.config["__workflow_spec"].clone(), "UTC")?;
    let expected_filters = spec.filters.clone();
    if executable {
        for entry in &spec.sources {
            let row = SourcesRepo::get(&pool, &entry.id).await?;
            if row.enabled || row.script_trusted || !row.needs_review {
                return Err("Executable setup bypassed disabled save/review".into());
            }
            script_trust::trust(
                &pool,
                &entry.id,
                row.script_sha256
                    .as_deref()
                    .ok_or("Saved script fingerprint missing")?,
                true,
            )
            .await?;
            if SourcesRepo::get(&pool, &entry.id).await?.enabled {
                return Err("Trust must not implicitly enable scripts".into());
            }
        }
    }
    spec.metadata.enabled = Some(true);
    for entry in &mut spec.sources {
        entry.enabled = case
            .source_company
            .as_ref()
            .is_none_or(|name| entry.employer_hint.as_ref() == Some(name));
    }
    let parent = workflow::persist(&pool, spec, Some(parent.id.clone()), false, "UTC").await?;
    let reopened = workflow::validate(parent.config["__workflow_spec"].clone(), "UTC")?;
    let run =
        RunsRepo::start_with_source(&pool, TriggerType::SingleSource, Some(&parent.id)).await?;
    let mut config = execution_diagnostics::prepare(&parent, "UTC", &run.started_at)?;
    config.commit_feedback = false;
    config.blocked_postings = postings::suppression_snapshot(&pool).await?;
    // Only replay timing is changed; budgets, enrichment and matching stay real.
    if !live {
        config.limits.per_host_delay_ms = 0;
    }
    execution_diagnostics::preflight(&pool, &run.id, &parent.id, &config).await?;
    let replay = if live {
        None
    } else {
        Some(Replay::start(root, &manifest.routes, &case.route_set)?)
    };
    let events = launch(
        &config,
        &storage.0,
        replay.as_ref().map(|r| r.origin.as_str()),
        live,
    )?;
    let WorkerEvent::Done { stats } = events.last().ok_or("Missing terminal frame")? else {
        return Err("Missing Done".into());
    };
    let batch = validated_jobs(&config, &events)?;
    let mut counts = RunCounts {
        fetched: stats.sources.iter().map(|s| s.examined as i64).sum(),
        rejected: stats.sources.iter().map(|s| s.rejected as i64).sum(),
        sources: Some(stats.sources.clone()),
        ..Default::default()
    };
    let inserted =
        execution_diagnostics::persist_batch(&pool, &run.id, &batch, &mut counts).await?;
    let saved = JobsRepo::list(&pool, discovered(&parent.id)).await?;
    for job in &saved.items {
        let Some(identity) = &job.posting_identity else {
            return Err("Saved canonical posting identity missing".into());
        };
        let parsed: Value = serde_json::from_str(identity)?;
        if parsed.is_null()
            || !batch
                .iter()
                .any(|j| j.external_id == job.external_id && j.url == job.url)
        {
            return Err(
                "Saved identity/destination differs from accepted provider evidence".into(),
            );
        }
    }
    let errors: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            WorkerEvent::Error { message, .. } => Some(message.clone()),
            WorkerEvent::Warning {
                message,
                termination: Some(scrape_core::protocol::CoverageTermination::Failed),
                ..
            } => Some(message.clone()),
            _ => None,
        })
        .collect();
    let warnings: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let WorkerEvent::Warning {
                message,
                eligibility_incomplete,
                ..
            } = e
            {
                Some(json!({"message":message,"eligibility_incomplete":eligibility_incomplete}))
            } else {
                None
            }
        })
        .collect();
    let observed = json!({
        "workflow_id":parent.id,"owned_source_ids":config.sources.iter().map(|s|&s.id).collect::<Vec<_>>(),"effective_filters":config.filters,
        "source_companies":config.sources.iter().map(|s|(s.id.clone(),s.name.clone())).collect::<BTreeMap<_,_>>(),
        "saved_filters_match":reopened.filters==expected_filters,
        "worker_protocol":config.version,"workflow_cap":config.max_jobs_per_run,
        "source_stats":stats.sources,"errors":errors,"warnings":warnings,
        "jobs":batch.iter().map(|j|json!({"external_id":j.external_id,"url":j.url,"company":j.company,"title":j.title,"location":j.location,"role_type":j.role_type,"workplace_type":j.workplace_type,"date_posted":j.date_posted,"locations":j.locations,"description_source":j.description_source,"posting_identity":saved.items.iter().find(|s|s.external_id==j.external_id).and_then(|s|s.posting_identity.clone())})).collect::<Vec<_>>(),
        "inserted":inserted.inserted,"discovered":saved.total,
        "routes":replay.as_ref().map(Replay::evidence).unwrap_or_else(||json!({"mode":"production_network_no_fixtures"}))
    });
    let acceptance = if live {
        Ok(())
    } else {
        validate_observation(&case.expected, &observed)
    };
    let status = if acceptance.is_err() {
        "failed"
    } else if live {
        let access_blocked = !errors.is_empty()
            && errors.iter().all(|message| {
                let message = message.to_ascii_lowercase();
                [
                    "403",
                    "429",
                    "needs login",
                    "login required",
                    "access denied",
                    "challenge",
                    "robots",
                    "timed out",
                    "timeout",
                    "dns",
                    "connection",
                ]
                .iter()
                .any(|pattern| message.contains(pattern))
            });
        if (!errors.is_empty() && !access_blocked)
            || stats.sources.iter().any(|s| {
                matches!(s.outcome, Some(scrape_core::protocol::ScanOutcome::Failed))
                    && !access_blocked
            })
        {
            "failed"
        } else if access_blocked && batch.is_empty() {
            "blocked"
        } else if !errors.is_empty()
            || batch.is_empty()
            || stats.sources.iter().any(|s| {
                s.unresolved > 0
                    || matches!(s.outcome, Some(scrape_core::protocol::ScanOutcome::Partial))
            })
        {
            "partial"
        } else {
            "passed"
        }
    } else {
        case.expected["status"]
            .as_str()
            .ok_or("Missing expected acceptance status")?
    };
    let reason=acceptance.err().or_else(||if live && status!="passed" {Some("Live workflow acceptance is incomplete; inspect source evidence/errors. Zero jobs alone is not success.".into())}else{case.reason.clone()});
    let run_status = match status {
        "failed" | "blocked" => RunStatus::Failed,
        "partial" => RunStatus::Partial,
        _ => RunStatus::Success,
    };
    RunsRepo::finish(
        &pool,
        &run.id,
        run_status,
        batch.len() as i64,
        inserted.inserted,
        reason.as_deref(),
    )
    .await?;
    let mut check = json!({"id":if live{format!("{}-live",case.template_id)}else{case.id.clone()},"template_id":case.template_id,"companies":case.companies,"status":status,"fixture_class":if live{"live_production_workflow"}else{&case.fixture_class},"reason":reason,"control":case.control,"expected_coverage":if live{json!("bounded_live_workflow_acceptance_not_full_board")}else{case.expected["coverage"].clone()},"observed":observed});
    if !live {
        check["lifecycle"] = lifecycle(
            &pool,
            &parent.id,
            &batch,
            &config,
            &storage.0,
            replay.as_ref().map(|r| r.origin.as_str()),
        )
        .await?;
        if replay
            .as_ref()
            .is_some_and(|r| !r.unexpected.lock().unwrap().is_empty())
        {
            return Err("Unexpected fixture route during refresh/suppression execution".into());
        }
    }
    if refresh_example && case.id == "sep-unrestricted" {
        check["recorded_example"] = json!({"provenance":serde_json::from_slice::<Value>(&std::fs::read(root.join("fixtures/providers/manifest.json"))?)?["sep"],"source":config.sources.first(),"events":events});
    }
    pool.close().await;
    Ok(check)
}

async fn lifecycle(
    pool: &sqlx::SqlitePool,
    workflow_id: &str,
    batch: &[ScrapedJob],
    config: &WorkerConfig,
    cwd: &Path,
    origin: Option<&str>,
) -> VResult<Value> {
    let run =
        RunsRepo::start_with_source(pool, TriggerType::SingleSource, Some(workflow_id)).await?;
    let run_id = run.id.as_str();
    let mut counts = RunCounts::default();
    execution_diagnostics::preflight(pool, run_id, workflow_id, config).await?;
    let refresh_events = launch(config, cwd, origin, false)?;
    let refresh_batch = validated_jobs(config, &refresh_events)?;
    fn keys(jobs: &[ScrapedJob]) -> BTreeSet<(Option<&str>, &str)> {
        jobs.iter()
            .map(|j| (j.external_id.as_deref(), j.url.as_str()))
            .collect()
    }
    if keys(&refresh_batch) != keys(batch) {
        return Err("Refresh worker changed the exact recorded identities/destinations".into());
    }
    let refresh =
        execution_diagnostics::persist_batch(pool, run_id, &refresh_batch, &mut counts).await?;
    if refresh.inserted != 0 || refresh.updated != batch.len() as i64 {
        return Err("Refresh did not preserve canonical posting identity".into());
    }
    let mut jobs = JobsRepo::list(pool, discovered(workflow_id)).await?.items;
    let mut excluded = 0;
    let mut protected_ids = Vec::new();
    let mut suppressed_identity = None;
    if let Some(job) = jobs.pop() {
        JobsRepo::update_status(pool, &job.id, JobStatus::Applied).await?;
        protected_ids.push(job.id);
        excluded += 1;
    }
    if let Some(job) = jobs.pop() {
        postings::remove(pool, &job.id, true).await?;
        suppressed_identity = Some((job.external_id, job.url));
        protected_ids.push(job.id);
        excluded += 1;
    }
    execution_diagnostics::persist_batch(pool, run_id, batch, &mut counts).await?;
    let remaining = JobsRepo::list(pool, discovered(workflow_id)).await?.total;
    if remaining != batch.len() as i64 - excluded {
        return Err("Refresh reintroduced a tracked or suppressed job into Discovered".into());
    }
    let mut protected_config = config.clone();
    protected_config.blocked_postings = postings::suppression_snapshot(pool).await?;
    execution_diagnostics::preflight(pool, run_id, workflow_id, &protected_config).await?;
    let protected_events = launch(&protected_config, cwd, origin, false)?;
    let protected_batch = validated_jobs(&protected_config, &protected_events)?;
    if let Some((external_id, url)) = &suppressed_identity {
        if protected_batch
            .iter()
            .any(|j| j.external_id == *external_id && j.url == *url)
        {
            return Err("Worker re-emitted a suppressed posting".into());
        }
        let Some(WorkerEvent::Done { stats }) = protected_events.last() else {
            return Err("Missing protected rerun summary".into());
        };
        if stats.sources.iter().map(|s| s.blocked).sum::<u64>() == 0 {
            return Err("Worker suppression was not reflected in source counters".into());
        }
    }
    let protected_stats =
        execution_diagnostics::persist_batch(pool, run_id, &protected_batch, &mut counts).await?;
    let protected_view = JobsRepo::list(pool, discovered(workflow_id)).await?;
    if protected_view.total != remaining + protected_stats.inserted
        || protected_view
            .items
            .iter()
            .any(|j| protected_ids.contains(&j.id))
    {
        return Err(
            "Worker refresh did not preserve tracked/suppressed exclusion in Discovered".into(),
        );
    }
    // Fault injection belongs to this run too: retain its preceding commits and refreshes.
    if let Some(job) = batch.first() {
        let mut invalid = job.clone();
        invalid.source_id = Some("section7-missing-owned-child".into());
        invalid.external_id = Some("section7-persistence-error".into());
        invalid.url = "https://fixture.invalid/section7/persistence-error".into();
        invalid.dedupe_key = "section7-persistence-error".into();
        if execution_diagnostics::persist_batch(pool, run_id, &[invalid], &mut counts)
            .await
            .is_ok()
            || counts.failed != 1
        {
            return Err("Persistence failure was hidden or accepted with a missing child".into());
        }
        if !RunsRepo::get_logs(pool, run_id)
            .await?
            .iter()
            .any(|l| l.message.contains("Persistence failed"))
        {
            return Err("Persistence failure lacks durable diagnostics".into());
        }
    }
    let status = if counts.failed > 0 {
        RunStatus::Failed
    } else if protected_events.iter().any(|e| {
        matches!(
            e,
            WorkerEvent::Error { .. }
                | WorkerEvent::Warning {
                    eligibility_incomplete: true,
                    ..
                }
        )
    }) {
        RunStatus::Partial
    } else {
        RunStatus::Success
    };
    RunsRepo::finish(
        pool,
        run_id,
        status,
        protected_batch.len() as i64,
        protected_stats.inserted,
        if counts.failed > 0 {
            Some("Expected verification fault injection: missing child persistence failure")
        } else {
            None
        },
    )
    .await?;
    Ok(
        json!({"refresh_execution":"packaged_worker_rerun","refreshed":refresh.updated,"tracked_and_suppressed_excluded":excluded,"discovered_after_refresh":protected_view.total,"protected_rerun_inserted":protected_stats.inserted,"protected_run_terminal":protected_events.last(),"injected_persistence_failures":counts.failed,"fault_injection_run_status":status}),
    )
}

/// stdout is owned by the CLI; this service returns one machine-readable report.
pub async fn verify(
    root: &Path,
    live: bool,
    refresh_example: bool,
) -> std::result::Result<Value, String> {
    verify_inner(root, live, refresh_example)
        .await
        .map_err(|e| e.to_string())
}
async fn verify_inner(root: &Path, live: bool, refresh_example: bool) -> VResult<Value> {
    if live && refresh_example {
        return Err("--refresh-example is recorded-only".into());
    }
    let raw: Value = serde_json::from_slice(&std::fs::read(
        root.join("fixtures/providers/manifest.json"),
    )?)?;
    let manifest: Manifest = serde_json::from_value(raw.clone())?;
    if manifest.schema_version != 2 || manifest.workflow_cases.is_empty() {
        return Err("Unsupported or empty workflow manifest".into());
    }
    if manifest.extractors["worker_protocol"] != scrape_core::PROTOCOL_VERSION
        || manifest.extractors["script_protocol"]
            != scrape_core::extraction::SCRIPT_PROTOCOL_VERSION
        || manifest.extractors["verification_profile_revision"]
            != provider_verification_profiles::REVISION
    {
        return Err("Manifest worker/script/verification-profile extraction version drift".into());
    }
    // Raw capture provenance is validated even when a case uses an authored control.
    for key in [
        "sep",
        "linear",
        "bitwarden",
        "google",
        "apple",
        "netflix",
        "amazon",
        "meta",
    ] {
        let capture = &raw[key];
        checked_bytes(
            root,
            capture["file"].as_str().ok_or("Missing capture file")?,
            capture["sha256"]
                .as_str()
                .ok_or("Missing capture checksum")?,
        )?;
    }
    let mut checks = Vec::new();
    let mut ids = BTreeSet::new();
    for case in manifest.workflow_cases.iter().filter(|c| !live || c.live) {
        if !ids.insert(&case.id) {
            return Err("Duplicate manifest case ID".into());
        }
        match run_case(root,&manifest,case,live,refresh_example).await{
            Ok(check)=>checks.push(check),
            Err(error)=>checks.push(json!({"id":case.id,"companies":case.companies,"status":"failed","fixture_class":if live{"live_production_workflow"}else{&case.fixture_class},"reason":error.to_string()})),
        }
    }
    for gap in &manifest.explicit_gaps {
        checks.push(gap.clone());
    }
    let example = checks
        .iter_mut()
        .find_map(|c| c.as_object_mut().and_then(|o| o.remove("recorded_example")));
    let mut report = json!({"schema_version":1,"mode":if live{"live_workflows"}else{"recorded_workflows"},"checks":checks,"execution_boundary":"canonical seed/save/review/trust + shared host preparation/preflight/budget/ingestion + packaged worker + isolated SQLite/Discovered; not Tauri IPC or UI/native acceptance","live_performed":live});
    if let Some(example) = example {
        report["recorded_example"] = example;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_rejects_an_unlisted_route_instead_of_returning_a_board() {
        let storage = Storage::new().unwrap();
        let routes = vec![Route {
            path: "/provider.example/list?page=1".into(),
            role: "listing".into(),
            status: 200,
            file: None,
            sha256: None,
            json_pointer: None,
            body: Some("{\"jobs\":[]}".into()),
            set: "recorded".into(),
        }];
        let replay = Replay::start(&storage.0, &routes, "recorded").unwrap();
        for (path, status) in [
            ("/provider.example/list?page=1", "HTTP/1.1 200"),
            ("/provider.example/list?page=2", "HTTP/1.1 404"),
        ] {
            let mut socket =
                std::net::TcpStream::connect(replay.origin.trim_start_matches("http://")).unwrap();
            write!(
                socket,
                "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut body = String::new();
            socket.read_to_string(&mut body).unwrap();
            assert!(body.starts_with(status), "{body}");
            if path.ends_with("page=2") {
                assert!(!body.contains("{\"jobs\":[]}"));
            }
        }
        assert_eq!(
            replay.evidence()["unexpected_routes"],
            json!(["GET /provider.example/list?page=2"])
        );
    }

    #[test]
    fn replay_rejects_a_capture_with_a_wrong_checksum() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        assert!(checked_bytes(
            root,
            "fixtures/providers/lever/sep.json",
            "not-the-capture-checksum"
        )
        .unwrap_err()
        .to_string()
        .contains("checksum mismatch"));
    }
}
