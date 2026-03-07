use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CaptureTask {
    pub id: String,
    pub user_id: String,
    pub server_id: String,
    pub interface: String,
    pub filter: Option<String>,
    pub duration: Option<i64>,
    pub packet_limit: Option<i64>,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub error_msg: Option<String>,
    pub scheduled_at: Option<String>,
    pub repeat_type: String,
    pub repeat_until: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
    pub log_msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCaptureRequest {
    pub server_id: String,
    pub interface: String,
    pub filter: Option<String>,
    pub duration: Option<i64>,
    pub packet_limit: Option<i64>,
    pub scheduled_at: Option<String>,
    pub repeat_type: Option<String>,
    pub repeat_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatus {
    pub status: String,
    pub file_size: Option<i64>,
    pub error_msg: Option<String>,
    pub finished_at: Option<String>,
}

impl CaptureTask {
    pub fn new(
        user_id: String,
        server_id: String,
        interface: String,
        filter: Option<String>,
        duration: Option<i64>,
        packet_limit: Option<i64>,
        scheduled_at: Option<String>,
        repeat_type: Option<String>,
        repeat_until: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            server_id,
            interface,
            filter,
            duration,
            packet_limit,
            status: "pending".to_string(),
            file_path: None,
            file_size: None,
            error_msg: None,
            scheduled_at,
            repeat_type: repeat_type.unwrap_or_else(|| "none".to_string()),
            repeat_until,
            created_at: chrono::Utc::now().naive_utc().to_string(),
            finished_at: None,
            log_msg: None,
        }
    }
}
