use clap::Parser;
use scrape_core::{
    SourceKind, WorkerConfig, WorkerEvent, WorkerFilters, WorkerLimits, WorkerSource,
    PROTOCOL_VERSION,
};
use scrape_worker::client::HttpClient;
use scrape_worker::coordinator::SourceCoordinator;
use scrape_worker::detect;
use scrape_worker::filter::FilterMatcher;

use scrape_core::protocol::WorkerCommitAck;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

type AcknowledgementReader = Arc<Mutex<mpsc::UnboundedReceiver<Result<String, String>>>>;

#[derive(Parser, Debug)]
#[command(author, version, about = "Scrape worker sidecar binary")]
struct Cli {
    /// Print the authoritative worker protocol version and exit.
    #[arg(long)]
    protocol_version: bool,
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
    /// Read an optional HttpProxyConfig JSON line from stdin for --detect.
    #[arg(long, requires = "detect")]
    detect_proxy_stdin: bool,
    /// On-demand employer branding lookup for <company> and optional <url>
    #[arg(long, num_args = 1..=2)]
    branding: Option<Vec<String>>,
    /// Read an optional HttpProxyConfig JSON line from stdin for --branding.
    #[arg(long, requires = "branding")]
    branding_proxy_stdin: bool,
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
    if args.protocol_version {
        println!("{PROTOCOL_VERSION}");
        return Ok(());
    }

    if let Some(url) = args.detect {
        let proxy = if args.detect_proxy_stdin {
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            serde_json::from_str::<Option<scrape_core::protocol::HttpProxyConfig>>(&line)?
        } else {
            None
        };
        let client = HttpClient::new(10, 500).with_proxy(proxy.as_ref())?;
        let result = detect::detect_source(&client, &url).await;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }
    if let Some(parts) = args.branding {
        let company = parts.first().cloned().unwrap_or_default();
        let page_url = parts.get(1).cloned().unwrap_or_default();
        let proxy = if args.branding_proxy_stdin {
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            serde_json::from_str::<Option<scrape_core::protocol::HttpProxyConfig>>(&line)?
        } else {
            None
        };
        let client = HttpClient::new(5, 500).with_proxy(proxy.as_ref())?;
        let source = WorkerSource {
            id: "on_demand".into(),
            kind: SourceKind::Selector,
            name: company.clone(),
            identifier: page_url.clone(),
            config: serde_json::json!({"url": page_url}),
        };
        let branding = scrape_worker::adapters::branding::CompanyBranding::fetch_source(
            &client, &page_url, &company, &source,
        )
        .await;
        let mut logo = scrape_core::branding::CompanyLogo {
            evidence: branding.evidence.clone(),
            checked_at: chrono::Utc::now().to_rfc3339(),
            status: scrape_core::branding::LogoStatus::MissingMetadata,
            asset: None,
            cached_asset_reference: None,
            diagnostic: branding.warning.clone(),
        };
        if let Some(evidence) = &branding.evidence {
            match client.fetch_logo(&evidence.url).await {
                Ok(bytes) => match scrape_core::branding::cache_asset(&bytes) {
                    Ok(asset) => {
                        logo.asset = Some(asset);
                        logo.status = scrape_core::branding::LogoStatus::Verified;
                    }
                    Err(e) => {
                        logo.status = scrape_core::branding::LogoStatus::DecodeFailed;
                        logo.diagnostic = Some(e);
                    }
                },
                Err(e) => {
                    logo.status = scrape_core::branding::LogoStatus::DownloadFailed;
                    logo.diagnostic = Some(e.to_string());
                }
            }
        }
        println!("{}", serde_json::to_string(&logo)?);
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
    let mut reader = BufReader::new(io::stdin());
    let mut input = String::new();
    reader.read_line(&mut input)?;

    let config: WorkerConfig = match serde_json::from_str(input.trim()) {
        Ok(cfg) => cfg,
        Err(_) => {
            // Fallback for multiline or EOF-terminated JSON payloads
            reader.read_to_string(&mut input)?;
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

    let acknowledgements = if config.commit_feedback && !dry_run {
        let (sender, receiver) = mpsc::unbounded_channel();
        std::thread::spawn(move || loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if sender.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error.to_string()));
                    break;
                }
            }
        });
        Some(Arc::new(Mutex::new(receiver)))
    } else {
        None
    };
    execute_worker(config, dry_run, replay_origin, acknowledgements).await
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
        "workday" => SourceKind::Workday,
        "search" => SourceKind::Search,
        "web_discovery" => SourceKind::WebDiscovery,
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
        commit_feedback: false,
        candidate_diagnostics: false,
        run_started_at: Some(scrape_core::now_iso()),
        filters_by_source: Default::default(),
        sources: vec![source],
        filters: WorkerFilters::default(),
        limits: WorkerLimits::default(),
        blocked_postings: vec![],
        max_jobs_per_run,
        starting_source_index: Some(0),
        http_proxy: None,
    };

    execute_worker(config, dry_run, replay_origin, None).await
}

async fn execute_worker(
    config: WorkerConfig,
    dry_run: bool,
    replay_origin: Option<&str>,
    reader: Option<AcknowledgementReader>,
) -> Result<(), Box<dyn std::error::Error>> {
    config.limits.validate()?;
    if config.limits.concurrency != 1 {
        return Err("Worker concurrency currently supports one source at a time".into());
    }
    let enrichment = config.limits.enrichment.clone().unwrap_or_default();
    enrichment.validate()?;
    let mut client = HttpClient::new(
        config.limits.request_timeout_s,
        config.limits.per_host_delay_ms,
    )
    .with_jitter_ms(config.limits.jitter_ms)
    .with_respect_robots(!config.limits.ignore_robots.unwrap_or(false))
    .with_proxy(config.http_proxy.as_ref())?;
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
    if let Some(origin) = replay_origin {
        client = client.with_replay_origin(origin)?;
        tracing::info!("Recorded provider replay via {origin}; no live-provider verification");
    }
    for source in &config.sources {
        source.max_jobs_per_run()?;
    }
    let mut coordinator = SourceCoordinator::new(client, &config, dry_run, started)?;
    if config.commit_feedback && !dry_run {
        let reader = reader.ok_or("Commit feedback requires stdin")?;
        coordinator
            .run_with_feedback(emit_event, move |source_id, external_id| {
                let reader = reader.clone();
                async move {
                    let line = reader
                        .lock()
                        .await
                        .recv()
                        .await
                        .ok_or("EOF while awaiting acknowledgement")??;
                    let ack: WorkerCommitAck =
                        serde_json::from_str(&line).map_err(|e| e.to_string())?;
                    if ack.source_id != source_id || ack.external_id != external_id {
                        return Err("Acknowledgement does not match outstanding job".into());
                    }
                    Ok(ack.outcome)
                }
            })
            .await?;
    } else {
        coordinator.run(emit_event).await?;
    }
    Ok(())
}

fn emit_event(event: &WorkerEvent) -> Result<(), serde_json::Error> {
    let line = serde_json::to_string(event)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{line}").map_err(serde_json::Error::io)?;
    stdout.flush().map_err(serde_json::Error::io)?;
    Ok(())
}
