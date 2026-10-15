use clap::Parser;
use scrape_core::{
    SourceKind, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, WorkerStats,
    PROTOCOL_VERSION,
};
use scrape_worker::client::{self, HttpClient};
use scrape_worker::filter::{FilterMatcher, PostingGate};
use scrape_worker::{adapters, detect};

use std::io::{self, Read};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about = "Scrape worker sidecar binary")]
struct Cli {
    /// Read WorkerConfig from stdin JSON
    #[arg(long)]
    stdin: bool,

    /// Dry run mode (cap output to first 10 jobs)
    #[arg(long)]
    dry_run: bool,

    /// Test a single source in format <kind>:<identifier> (e.g. greenhouse:stripe)
    #[arg(long)]
    source: Option<String>,

    /// Maximum matching postings for --source (default 20).
    #[arg(long, requires = "source")]
    max_jobs_per_run: Option<u32>,

    /// Replay provider HTTP through a loopback fixture server (never live).
    #[arg(long)]
    replay_origin: Option<String>,

    /// Auto-detect ATS or JSON-LD configuration for a given URL
    #[arg(long)]
    detect: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Direct all tracing output to stderr so stdout is strictly NDJSON
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args = Cli::parse();

    if let Some(url) = args.detect {
        let client = HttpClient::new(10, 500);
        let result = detect::detect_source(&client, &url).await;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    if let Some(source_spec) = args.source {
        run_single_source(
            &source_spec,
            args.dry_run,
            args.replay_origin.as_deref(),
            args.max_jobs_per_run,
        )
        .await?;
        return Ok(());
    }

    if args.stdin {
        run_stdin_config(args.dry_run, args.replay_origin.as_deref()).await?;
        return Ok(());
    }

    eprintln!("Error: specify --stdin, --source <kind:identifier>, or --detect <url>");
    std::process::exit(1);
}

async fn run_stdin_config(
    dry_run: bool,
    replay_origin: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    let config: WorkerConfig = match serde_json::from_str(input.trim()) {
        Ok(cfg) => cfg,
        Err(_) => {
            // Fallback for multiline or EOF-terminated JSON payloads
            io::stdin().read_to_string(&mut input)?;
            serde_json::from_str(&input)?
        }
    };

    if config.version != PROTOCOL_VERSION {
        eprintln!(
            "Unsupported protocol version: expected {}, got {}",
            PROTOCOL_VERSION, config.version
        );
        std::process::exit(1);
    }

    execute_worker(config, dry_run, replay_origin).await
}

async fn run_single_source(
    spec: &str,
    dry_run: bool,
    replay_origin: Option<&str>,
    max_jobs_per_run: Option<u32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (kind_str, identifier) = spec
        .split_once(':')
        .filter(|(_, identifier)| !identifier.trim().is_empty())
        .ok_or("Invalid --source format. Expected <kind>:<identifier>")?;
    let name = identifier;

    let kind = match kind_str.to_lowercase().as_str() {
        "greenhouse" => SourceKind::Greenhouse,
        "lever" => SourceKind::Lever,
        "ashby" => SourceKind::Ashby,
        "jsonld" => SourceKind::Jsonld,
        "selector" => SourceKind::Selector,
        other => {
            eprintln!("Unknown source kind: {}", other);
            std::process::exit(1);
        }
    };

    let source = WorkerSource {
        id: "cli-test".to_string(),
        kind,
        name: name.to_string(),
        identifier: identifier.to_string(),
        config: serde_json::json!({"__workflow_spec": {"max_jobs_per_run": max_jobs_per_run.unwrap_or(scrape_core::protocol::DEFAULT_MAX_JOBS_PER_RUN)}}),
    };

    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        run_started_at: Some(scrape_core::now_iso()),
        filters_by_source: Default::default(),
        sources: vec![source],
        filters: WorkerFilters::default(),
        limits: WorkerLimits::default(),
        blocked_postings: vec![],
    };

    execute_worker(config, dry_run, replay_origin).await
}

async fn execute_worker(
    config: WorkerConfig,
    dry_run: bool,
    replay_origin: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.limits.concurrency != 1 {
        return Err("Worker concurrency currently supports one source at a time".into());
    }
    let start_time = Instant::now();
    let mut client = HttpClient::new(
        config.limits.request_timeout_s,
        config.limits.per_host_delay_ms,
    );
    // Validate every effective rule before contacting any source.
    FilterMatcher::new(&config.filters).map_err(|e| format!("Invalid workflow filters: {e}"))?;
    for filters in config.filters_by_source.values() {
        FilterMatcher::new(filters).map_err(|e| format!("Invalid workflow filters: {e}"))?;
    }

    let started = config
        .run_started_at
        .as_deref()
        .map(chrono::DateTime::parse_from_rfc3339)
        .transpose()?
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);
    let mut stats = WorkerStats::default();
    let mut total_emitted = 0u64;
    if let Some(origin) = replay_origin {
        client = client.with_replay_origin(origin)?;
        tracing::info!("Recorded provider replay via {origin}; no live-provider verification");
    }
    let blocked = config.blocked_postings.iter().cloned().collect();
    // Validate every source before making any provider requests.
    for source in &config.sources {
        source.max_jobs_per_run()?;
    }
    for source in &config.sources {
        let filters = config
            .filters_by_source
            .get(&source.id)
            .unwrap_or(&config.filters);
        let mut gate = PostingGate::new(source, filters, &blocked, started, dry_run)?;
        let mut output_error = None;
        let mut source_failed = false;
        tracing::info!(
            "Scraping source: {} ({:?}), limit {}",
            source.name,
            source.kind,
            gate.summary.limit
        );
        let result = adapters::scrape_source_with(&client, source, &mut |event| {
            if let Some(event) = gate.accept(event) {
                source_failed |= matches!(event, WorkerEvent::Error { .. });
                if let Err(error) = emit_event(&event) {
                    output_error = Some(error);
                    return false;
                }
            }
            !gate.full()
        })
        .await;
        if let Some(error) = output_error {
            return Err(error.into());
        }
        if let Err(error) = result {
            source_failed = true;
            emit_event(&WorkerEvent::Error {
                invalid_posting: Some(false),
                source_id: source.id.clone(),
                message: error.to_string(),
                retryable: matches!(
                    error,
                    client::ClientError::Reqwest(_) | client::ClientError::HttpStatus(..)
                ),
            })?;
        }
        if source_failed {
            stats.sources_failed += 1;
        } else {
            stats.sources_ok += 1;
        }
        total_emitted += u64::from(gate.accepted);
        emit_event(&WorkerEvent::Progress {
            source_id: source.id.clone(),
            fetched: gate.summary.examined,
            matched: gate.summary.matched,
        })?;
        stats.sources.push(gate.summary);
    }

    stats.jobs_emitted = total_emitted;
    stats.duration_ms = start_time.elapsed().as_millis() as u64;

    emit_event(&WorkerEvent::Done { stats })?;
    Ok(())
}

fn emit_event(event: &WorkerEvent) -> Result<(), serde_json::Error> {
    let line = serde_json::to_string(event)?;
    println!("{}", line);
    Ok(())
}
