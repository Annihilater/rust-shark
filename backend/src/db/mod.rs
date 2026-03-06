use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::info;

pub type DbPool = SqlitePool;

pub async fn init(database_url: &str) -> Result<DbPool> {
    // 确保数据库目录存在
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    // 运行迁移
    sqlx::migrate!("src/db/migrations").run(&pool).await?;
    info!("数据库迁移完成");

    Ok(pool)
}
