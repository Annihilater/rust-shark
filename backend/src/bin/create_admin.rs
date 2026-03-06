/// 创建初始 admin 用户的命令行工具
/// 用法: cargo run --bin create-admin -- admin@example.com Admin@123456

use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: create-admin <email> <password>");
        std::process::exit(1);
    }

    let email = &args[1];
    let password = &args[2];
    let name = args.get(3).cloned().unwrap_or_else(|| "Admin".to_string());

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data/rust-shark.db".to_string());

    // 确保数据库目录存在
    if let Some(path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
    }

    let pool = sqlx::SqlitePool::connect(&database_url).await?;

    // 运行迁移
    sqlx::migrate!("src/db/migrations").run(&pool).await?;

    // 检查邮箱是否已存在
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&pool)
            .await?;

    if exists.is_some() {
        eprintln!("用户 {} 已存在", email);
        std::process::exit(1);
    }

    let password_hash = hash(password, DEFAULT_COST)?;
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().naive_utc().to_string();

    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, created_at) VALUES (?, ?, ?, ?, 'admin', ?)",
    )
    .bind(&id)
    .bind(email)
    .bind(&name)
    .bind(&password_hash)
    .bind(&created_at)
    .execute(&pool)
    .await?;

    println!("✓ Admin 用户创建成功!");
    println!("  邮箱: {}", email);
    println!("  角色: admin");
    println!("\n现在可以使用此账号登录系统。");

    Ok(())
}
