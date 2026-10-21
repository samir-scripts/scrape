//! Run after building the packaged sidecar. JSON only on stdout.
use serde_json::json;

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let live = args.iter().any(|a| a == "--live");
    let refresh = args.iter().any(|a| a == "--refresh-example");
    let invalid = args
        .iter()
        .any(|a| !matches!(a.as_str(), "--live" | "--refresh-example"))
        || (live && refresh);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let result = if invalid {
        Err("Usage: verify_provider_workflows [--live | --refresh-example]".into())
    } else {
        scrape_lib::services::provider_verification::verify(root, live, refresh).await
    };
    let report = match result {
        Ok(report) => report,
        Err(error) => json!({
            "schema_version":1,
            "mode":if live { "live_workflows" } else { "recorded_workflows" },
            "checks":[{
                "id":"verification-driver",
                "companies":["Cloudflare","Bitwarden","Linear","SEP","Google","Apple","Netflix","Amazon","Meta"],
                "status":"failed",
                "fixture_class":"verification_setup",
                "reason":error
            }]
        }),
    };
    let failed = report["checks"].as_array().is_none_or(|checks| {
        checks.is_empty() || checks.iter().any(|check| check["status"] == "failed")
    });
    println!("{}", serde_json::to_string(&report).expect("JSON report"));
    if failed {
        std::process::exit(1);
    }
}
