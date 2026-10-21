//! Produce review artwork through the production logo validator, without a network request.
use base64::{engine::general_purpose::STANDARD, Engine};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "Usage: cache-logo-fixture INPUT OUTPUT.webp");
    let input = std::fs::read(&args[1]).expect("Read source asset");
    let asset = scrape_core::branding::cache_asset(&input).expect("Validate source artwork");
    let webp = STANDARD
        .decode(asset.strip_prefix("data:image/webp;base64,").unwrap())
        .unwrap();
    std::fs::write(&args[2], &webp).expect("Write review WebP");
    println!(
        "{}: {} original bytes -> {} cached WebP bytes",
        args[1],
        input.len(),
        webp.len()
    );
}
