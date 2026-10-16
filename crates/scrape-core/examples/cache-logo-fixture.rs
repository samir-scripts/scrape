//! Produce review artwork through the production logo validator, without a network request.
use base64::{engine::general_purpose::STANDARD, Engine};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "Usage: cache-logo-fixture INPUT OUTPUT.png");
    let asset =
        scrape_core::branding::cache_asset(&std::fs::read(&args[1]).expect("Read source asset"))
            .expect("Validate source artwork");
    let png = STANDARD
        .decode(asset.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    std::fs::write(&args[2], png).expect("Write review PNG");
}
