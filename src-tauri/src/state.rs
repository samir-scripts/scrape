use crate::services::runner::ScrapeRunner;
use crate::services::scheduler::SchedulerService;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: SqlitePool,
    pub runner: ScrapeRunner,
    pub scheduler: Arc<Mutex<SchedulerService>>,
}
