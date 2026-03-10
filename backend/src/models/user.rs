use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            role: u.role,
            created_at: u.created_at,
        }
    }
}

impl User {
    pub fn new(email: String, name: String, password_hash: String, role: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            email,
            name,
            password_hash,
            role,
            created_at: chrono::Utc::now().naive_utc().to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LoginLog {
    pub id: String,
    pub user_id: String,
    pub ip: String,
    pub user_agent: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub password: Option<String>,
}
