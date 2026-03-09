use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Server {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub ssh_key_id: Option<String>,
    #[serde(skip_serializing)]
    pub password_encrypted: Option<String>,
    pub status: String,
    pub last_checked_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub host: String,
    pub port: Option<i64>,
    pub username: String,
    pub auth_type: String,
    pub ssh_key_id: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub auth_type: Option<String>,
    pub ssh_key_id: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTestResult {
    pub success: bool,
    pub message: String,
    pub tcpdump_available: bool,
    pub tcpdump_version: Option<String>,
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub description: Option<String>,
}

impl Server {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: String,
        name: String,
        host: String,
        port: i64,
        username: String,
        auth_type: String,
        ssh_key_id: Option<String>,
        password_encrypted: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            host,
            port,
            username,
            auth_type,
            ssh_key_id,
            password_encrypted,
            status: "unknown".to_string(),
            last_checked_at: None,
            created_at: chrono::Utc::now().naive_utc().to_string(),
        }
    }
}
