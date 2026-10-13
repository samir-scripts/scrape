fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=historical-migrations");
    tauri_build::build()
}
