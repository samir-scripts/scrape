//! Explicit data repair only; never starts scheduling or extraction.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "Usage: repair_workflow_ownership EXISTING_DATABASE WORKFLOW_ID (quit Scrape first)"
                .into(),
        );
    }
    let path = std::path::Path::new(&args[0]);
    if !path.is_file() {
        return Err("Database does not exist; refusing to create it".into());
    }
    let pool = scrape_lib::db::init_db(path).await?;
    let backup = path.with_file_name(format!(
        "{}.pre-ownership-repair-{}.db",
        path.file_name().unwrap().to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    let timezone =
        scrape_lib::db::settings::SettingsRepo::get_value::<String>(&pool, "system.timezone")
            .await?
            .unwrap_or_else(|| "UTC".into());
    let source = scrape_lib::services::workflow_store::repair_ownership(
        &pool,
        &args[1].to_string_lossy(),
        &backup,
        &timezone,
    )
    .await?;
    pool.close().await;
    println!(
        "OWNERSHIP_REPAIR_OK workflow={} backup={}",
        source.id,
        if backup.exists() {
            backup.display().to_string()
        } else {
            "not needed (already consistent)".into()
        }
    );
    Ok(())
}
