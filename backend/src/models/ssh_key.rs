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
    pub key_type: String,
    pub fingerprint: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyResponse {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub public_key: String,
    pub key_type: String,
    pub fingerprint: String,
    pub created_at: String,
}

impl From<SshKey> for SshKeyResponse {
    fn from(k: SshKey) -> Self {
        Self {
            id: k.id,
            user_id: k.user_id,
            name: k.name,
            public_key: k.public_key,
            key_type: k.key_type,
            fingerprint: k.fingerprint,
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
        key_type: String,
        fingerprint: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            private_key_encrypted,
            public_key,
            key_type,
            fingerprint,
            created_at: chrono::Utc::now().naive_utc().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSshKeyRequest {
    pub name: String,
    pub private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateKeyRequest {
    pub name: String,
    /// "ed25519" | "rsa-2048" | "rsa-4096" | "ecdsa-p256" | "ecdsa-p384"
    pub key_type: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateKeyResponse {
    /// 已入库的记录（含公钥、指纹）
    pub key: SshKeyResponse,
}
