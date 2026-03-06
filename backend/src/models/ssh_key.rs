use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SshKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub private_key_encrypted: String,
    pub public_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub public_key: String,
    pub created_at: String,
}

impl From<SshKey> for SshKeyResponse {
    fn from(k: SshKey) -> Self {
        Self {
            id: k.id,
            user_id: k.user_id,
            name: k.name,
            public_key: k.public_key,
            created_at: k.created_at,
        }
    }
}

impl SshKey {
    pub fn new(
        user_id: String,
        name: String,
        private_key_encrypted: String,
        public_key: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            private_key_encrypted,
            public_key,
            created_at: chrono::Utc::now().naive_utc().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSshKeyRequest {
    pub name: String,
    pub private_key: String,
}
