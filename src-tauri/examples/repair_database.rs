//! Apply only reviewed migrations/backfills; do not start schedulers or change app defaults.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("Provide an existing database path")?;
    let path = std::path::Path::new(&path);
    if !path.is_file() {
        return Err("Database does not exist; refusing to create one".into());
    }
    let pool = scrape_lib::db::init_db(path).await?;
    let version: i64 = sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
        .fetch_one(&pool)
        .await?;
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await?;
    if integrity != "ok" {
        return Err(format!("Integrity check failed: {integrity}").into());
    }
    pool.close().await;
    println!("REPAIR_OK migration_version={version} integrity=ok");
    Ok(())
}
