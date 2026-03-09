/// 启动时管理员账号同步
///
/// 策略：
///   - 数据库中不存在该邮箱的 admin → 创建
///   - 存在但密码与配置文件不一致 → 重置密码（以配置文件为准）
///   - 存在且密码一致 → 无操作
///
/// 这样可以将管理员账密写在 .env 里统一管理，重置密码只需改 .env 重启即可。
use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::db::DbPool;

pub async fn sync_admin(pool: &DbPool, cfg: &Config) -> Result<()> {
    let email = &cfg.admin_email;
    let password = &cfg.admin_password;
    let name = &cfg.admin_name;

    // 查询数据库中该邮箱的 admin 用户
    let existing: Option<(String, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = ? AND role = 'admin'")
            .bind(email)
            .fetch_optional(pool)
            .await?;

    match existing {
        None => {
            // 不存在，直接创建
            let password_hash = hash(password, DEFAULT_COST)?;
            let id = Uuid::new_v4().to_string();
            let created_at = chrono::Utc::now().naive_utc().to_string();

            sqlx::query(
                "INSERT INTO users (id, email, name, password_hash, role, created_at) \
                 VALUES (?, ?, ?, ?, 'admin', ?)",
            )
            .bind(&id)
            .bind(email)
            .bind(name)
            .bind(&password_hash)
            .bind(&created_at)
            .execute(pool)
            .await?;

            info!("✓ 管理员账号已创建: {}", email);
        }

        Some((id, stored_hash)) => {
            // 已存在，验证密码是否与配置一致
            let matches = verify(password, &stored_hash).unwrap_or(false);

            if matches {
                info!("✓ 管理员账号验证通过，无需更新: {}", email);
            } else {
                // 密码不一致，以配置文件为准，重置
                let new_hash = hash(password, DEFAULT_COST)?;

                sqlx::query("UPDATE users SET password_hash = ?, name = ? WHERE id = ?")
                    .bind(&new_hash)
                    .bind(name)
                    .bind(&id)
                    .execute(pool)
                    .await?;

                warn!(
                    "⚠ 管理员密码已按 .env 配置重置（邮箱: {}）。\
                     若非预期请检查 ADMIN_PASSWORD 配置。",
                    email
                );
            }
        }
    }

    Ok(())
}
