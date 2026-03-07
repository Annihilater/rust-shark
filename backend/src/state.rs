use crate::{config::Config, db::DbPool, services::scheduler::Scheduler};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// task_id → (server_id, remote_tcpdump_pid)
pub type CaptureRegistry = Arc<Mutex<HashMap<String, (String, u32)>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub scheduler: Arc<Mutex<Option<Scheduler>>>,
    /// 正在运行的抓包任务：task_id → (server_id, remote_tcpdump_pid)
    pub capture_pids: CaptureRegistry,
}

impl AppState {
    pub fn new(pool: DbPool, config: Config) -> Self {
        Self {
            pool,
            config,
            scheduler: Arc::new(Mutex::new(None)),
            capture_pids: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
