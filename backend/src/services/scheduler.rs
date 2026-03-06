use crate::state::AppState;
use anyhow::Result;
use tracing::info;

pub type Scheduler = tokio_cron_scheduler::JobScheduler;

pub async fn init(state: AppState) -> Result<()> {
    let scheduler = tokio_cron_scheduler::JobScheduler::new().await?;
    scheduler.start().await?;
    info!("定时任务调度器启动");

    let mut lock = state.scheduler.lock().await;
    *lock = Some(scheduler);

    Ok(())
}
