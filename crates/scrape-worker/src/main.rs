use clap::Parser;
use scrape_core::{
    SourceKind, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource, WorkerStats,
    PROTOCOL_VERSION,
};
use scrape_worker::client::{self, HttpClient};
use scrape_worker::filter::FilterMatcher;
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
        run_single_source(&source_spec, args.dry_run).await?;
        return Ok(());
    }

    if args.stdin {
        run_stdin_config(args.dry_run).await?;
        return Ok(());
    }

    eprintln!("Error: specify --stdin, --source <kind:identifier>, or --detect <url>");
    std::process::exit(1);
}

async fn run_stdin_config(dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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

    execute_worker(config, dry_run).await
}

async fn run_single_source(spec: &str, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        config: serde_json::Value::Null,
    };

    let config = WorkerConfig {
        version: PROTOCOL_VERSION,
        run_started_at: Some(scrape_core::now_iso()),
        filters_by_source: Default::default(),
        sources: vec![source],
        filters: WorkerFilters::default(),
        limits: WorkerLimits::default(),
    };

    execute_worker(config, dry_run).await
}

async fn execute_worker(
    config: WorkerConfig,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.limits.concurrency != 1 {
        return Err("Worker concurrency currently supports one source at a time".into());
    }
    let start_time = Instant::now();
    let client = HttpClient::new(
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
    let max_dry_run_jobs = 10u64;

    for source in &config.sources {
        let filters = config
            .filters_by_source
            .get(&source.id)
            .unwrap_or(&config.filters);
        let matcher = FilterMatcher::new(filters)?;
        if dry_run && total_emitted >= max_dry_run_jobs {
            break;
        }

        tracing::info!("Scraping source: {} ({:?})", source.name, source.kind);

        match adapters::scrape_source(&client, source).await {
            Ok(events) => {
                let has_invalid = events
                    .iter()
                    .any(|event| matches!(event, WorkerEvent::Error { .. }));
                if has_invalid {
                    stats.sources_failed += 1;
                } else {
                    stats.sources_ok += 1;
                }
                let fetched_count = events.len() as u64;
                let mut matched_count = 0u64;

                for event in events {
                    if let WorkerEvent::Error { .. } = &event {
                        emit_event(&event)?;
                        continue;
                    }
                    if let WorkerEvent::Job {
                        ref title,
                        ref location,
                        ref company,
                        ref description_text,
                        ref salary,
                        role_type,
                        ..
                    } = event
                    {
                        let ctx = scrape_core::JobFilterContext {
                            title,
                            location,
                            description: description_text.as_deref().unwrap_or(""),
                            company,
                            role_type,
                        };
                        if matcher.matches_context(&ctx)
                            && filters
                                .matching
                                .as_ref()
                                .is_none_or(|rules| rules.matches_event_at(&event, started))
                            && filters.compensation.as_ref().is_none_or(|pay| {
                                pay.matches(
                                    salary.as_ref().map(|s| s.ranges.as_slice()).unwrap_or(&[]),
                                )
                            })
                        {
                            matched_count += 1;
                            total_emitted += 1;
                            emit_event(&event)?;

                            if dry_run && total_emitted >= max_dry_run_jobs {
                                break;
                            }
                        }
                    }
                }

                let progress = WorkerEvent::Progress {
                    source_id: source.id.clone(),
                    fetched: fetched_count,
                    matched: matched_count,
                };
                emit_event(&progress)?;
            }
            Err(err) => {
                stats.sources_failed += 1;
                let error_event = WorkerEvent::Error {
                    invalid_posting: Some(false),
                    source_id: source.id.clone(),
                    message: err.to_string(),
                    retryable: matches!(
                        err,
                        client::ClientError::Reqwest(_) | client::ClientError::HttpStatus(..)
                    ),
                };
                emit_event(&error_event)?;
            }
        }
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
