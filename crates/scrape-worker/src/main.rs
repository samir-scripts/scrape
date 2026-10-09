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
    io::stdin().read_to_string(&mut input)?;

    let config: WorkerConfig = serde_json::from_str(&input)?;
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
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    if parts.len() < 2 {
        eprintln!("Invalid --source format. Expected <kind>:<identifier> (e.g. greenhouse:stripe)");
        std::process::exit(1);
    }

    let kind_str = parts[0];
    let identifier = parts[1];
    let name = if parts.len() >= 3 {
        parts[2]
    } else {
        identifier
    };

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
    let start_time = Instant::now();
    let client = HttpClient::new(
        config.limits.request_timeout_s,
        config.limits.per_host_delay_ms,
    );
    let matcher = FilterMatcher::new(&config.filters);

    let mut stats = WorkerStats::default();
    let mut total_emitted = 0u64;
    let max_dry_run_jobs = 10u64;

    for source in &config.sources {
        if dry_run && total_emitted >= max_dry_run_jobs {
            break;
        }

        tracing::info!("Scraping source: {} ({:?})", source.name, source.kind);

        match adapters::scrape_source(&client, source).await {
            Ok(events) => {
                stats.sources_ok += 1;
                let fetched_count = events.len() as u64;
                let mut matched_count = 0u64;

                for event in events {
                    if let WorkerEvent::Job {
                        source_id,
                        external_id,
                        title,
                        company,
                        role_type,
                        location,
                        url,
                        date_posted,
                    } = event
                    {
                        if matcher.matches(&title, &location, role_type) {
                            matched_count += 1;
                            total_emitted += 1;

                            let job_event = WorkerEvent::Job {
                                source_id,
                                external_id,
                                title,
                                company,
                                role_type,
                                location,
                                url,
                                date_posted,
                            };
                            emit_event(&job_event)?;

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
