use crate::{config::Config, db::DbPool, services::scheduler::Scheduler};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub scheduler: Arc<Mutex<Option<Scheduler>>>,
}

impl AppState {
    pub fn new(pool: DbPool, config: Config) -> Self {
        Self {
            pool,
            config,
            scheduler: Arc::new(Mutex::new(None)),
        }
    }
}
