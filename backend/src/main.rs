use anyhow::Result;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod db;
mod models;
mod services;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // 向上查找 .env 文件（兼容从 backend/ 或项目根目录运行）
    if dotenvy::dotenv().is_err() {
        dotenvy::from_path("../.env").ok();
    }

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_shark=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let cfg = config::Config::from_env()?;
    info!("配置加载完成: host={}, port={}", cfg.host, cfg.port);

    // 初始化数据库
    let pool = db::init(&cfg.database_url).await?;
    info!("数据库初始化完成");

    // 构建应用状态
    let state = AppState::new(pool, cfg.clone());

    // 初始化调度器
    services::scheduler::init(state.clone()).await?;

    // 构建路由
    let app = api::router(state);

    // 启动服务器
    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;
    info!("服务启动: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
