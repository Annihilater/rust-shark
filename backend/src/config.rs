use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub secret_key: String,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub data_dir: String,
    /// 管理员邮箱，启动时自动创建/同步
    pub admin_email: String,
    /// 管理员密码（明文），启动时与数据库比对，不一致则重置
    pub admin_password: String,
    /// 管理员显示名称
    pub admin_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "17823".to_string())
                .parse()?,
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/rust-shark.db?mode=rwc".to_string()),
            secret_key: std::env::var("SECRET_KEY")
                .unwrap_or_else(|_| "change-me-in-production-32bytes!!".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-jwt-secret-in-production!!!".to_string()),
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()?,
            data_dir: std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string()),
            admin_email: std::env::var("ADMIN_EMAIL")
                .unwrap_or_else(|_| "admin@example.com".to_string()),
            admin_password: std::env::var("ADMIN_PASSWORD")
                .unwrap_or_else(|_| "Admin@123456".to_string()),
            admin_name: std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Administrator".to_string()),
        })
    }
}
