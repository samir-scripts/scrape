//! Used by scripts/generate-template-catalog.mjs; no database or runtime execution.
fn main() {
    let catalog = scrape_lib::services::template_catalog::catalog()
        .expect("built-in catalog validation failed");
    println!("{}", serde_json::to_string_pretty(&catalog).unwrap());
}
