use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CaptureProfile {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub server_id: String,
    pub interface: String,
    pub filter: Option<String>,
    pub duration: Option<i64>,
    pub packet_limit: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCaptureProfileRequest {
    pub name: String,
    pub server_id: String,
    pub interface: String,
    pub filter: Option<String>,
    pub duration: Option<i64>,
    pub packet_limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCaptureProfileRequest {
    pub name: Option<String>,
    pub server_id: Option<String>,
    pub interface: Option<String>,
    pub filter: Option<String>,
    pub duration: Option<i64>,
    pub packet_limit: Option<i64>,
}

impl CaptureProfile {
    pub fn new(
        user_id: String,
        name: String,
        server_id: String,
        interface: String,
        filter: Option<String>,
        duration: Option<i64>,
        packet_limit: Option<i64>,
    ) -> Self {
        let now = chrono::Utc::now().naive_utc().to_string();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            server_id,
            interface,
            filter,
            duration,
            packet_limit,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
