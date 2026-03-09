use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Extension, Json, Router,
};

use crate::api::{bad_request, internal_error, not_found, ApiResult, AuthUser};
use crate::models::ssh_key::{
    CreateSshKeyRequest, GenerateKeyRequest, GenerateKeyResponse, SshKey, SshKeyResponse,
};
use crate::services::crypto::CryptoService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_keys).post(create_key))
        .route("/generate", post(generate_key))
        .route("/{id}", delete(delete_key))
}

async fn list_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<SshKeyResponse>> {
    let keys = sqlx::query_as::<_, SshKey>(
        "SELECT * FROM ssh_keys WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(keys.into_iter().map(SshKeyResponse::from).collect()))
}

async fn create_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateSshKeyRequest>,
) -> ApiResult<SshKeyResponse> {
    // 提取公钥、检测密钥类型、计算指纹
    let (public_key, key_type, fingerprint) = extract_key_info(&req.private_key)
        .map_err(|e| bad_request(format!("无效的私钥: {}", e)))?;

    // 加密私钥
    let crypto = CryptoService::new(&state.config.secret_key).map_err(internal_error)?;
    let encrypted = crypto.encrypt(&req.private_key).map_err(internal_error)?;

    let key = SshKey::new(
        auth.user_id,
        req.name,
        encrypted,
        public_key,
        key_type,
        fingerprint,
    );

    sqlx::query(
        "INSERT INTO ssh_keys (id, user_id, name, private_key_encrypted, public_key, key_type, fingerprint, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&key.id)
    .bind(&key.user_id)
    .bind(&key.name)
    .bind(&key.private_key_encrypted)
    .bind(&key.public_key)
    .bind(&key.key_type)
    .bind(&key.fingerprint)
    .bind(&key.created_at)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(key.into()))
}

async fn delete_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let result = sqlx::query("DELETE FROM ssh_keys WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err(not_found("密钥不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

/// 解析私钥，返回 (公钥 OpenSSH 字符串, 算法名称, SHA256 指纹)
fn extract_key_info(private_key_pem: &str) -> anyhow::Result<(String, String, String)> {
    use ssh_key::PrivateKey;
    let key = PrivateKey::from_openssh(private_key_pem)?;
    let pub_key = key.public_key();

    let key_type = classify_key_type(&key);
    let public_key_str = pub_key.to_openssh()?;
    let fingerprint = pub_key.fingerprint(ssh_key::HashAlg::Sha256).to_string();

    Ok((public_key_str, key_type, fingerprint))
}

/// 对密钥类型进行分类，RSA 区分 2048/4096
fn classify_key_type(key: &ssh_key::PrivateKey) -> String {
    use ssh_key::Algorithm;
    match key.public_key().algorithm() {
        Algorithm::Ed25519 => "ed25519".to_string(),
        Algorithm::Rsa { .. } => {
            // 通过 RSA 私钥数据获取模数位数
            if let ssh_key::private::KeypairData::Rsa(rsa) = key.key_data() {
                let bits = rsa
                    .public
                    .n
                    .as_positive_bytes()
                    .map(|b| b.len() * 8)
                    .unwrap_or(0);
                if bits >= 4096 {
                    "rsa-4096".to_string()
                } else {
                    "rsa-2048".to_string()
                }
            } else {
                "rsa-2048".to_string()
            }
        }
        Algorithm::Ecdsa { curve } => match curve {
            ssh_key::EcdsaCurve::NistP256 => "ecdsa-p256".to_string(),
            ssh_key::EcdsaCurve::NistP384 => "ecdsa-p384".to_string(),
            ssh_key::EcdsaCurve::NistP521 => "ecdsa-p521".to_string(),
        },
        other => other.as_str().to_string(),
    }
}

async fn generate_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<GenerateKeyRequest>,
) -> ApiResult<GenerateKeyResponse> {
    use ssh_key::{Algorithm, EcdsaCurve, PrivateKey};

    let algorithm = match req.key_type.as_str() {
        "rsa-2048" | "rsa-4096" => Algorithm::Rsa { hash: None },
        "ecdsa-p256" => Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        },
        "ecdsa-p384" => Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP384,
        },
        _ => Algorithm::Ed25519,
    };

    let mut private_key = if req.key_type == "rsa-4096" {
        generate_rsa_key(4096).map_err(|e| bad_request(e.to_string()))?
    } else if req.key_type == "rsa-2048" {
        generate_rsa_key(2048).map_err(|e| bad_request(e.to_string()))?
    } else {
        PrivateKey::random(&mut rand::rngs::OsRng, algorithm)
            .map_err(|e| bad_request(format!("密钥生成失败: {}", e)))?
    };

    let comment = req.comment.clone().unwrap_or_else(|| req.name.clone());
    private_key.set_comment(&comment);

    let private_pem = private_key
        .to_openssh(ssh_key::LineEnding::LF)
        .map_err(|e| internal_error(format!("私钥序列化失败: {}", e)))?
        .to_string();

    let public_key_str = private_key
        .public_key()
        .to_openssh()
        .map_err(|e| internal_error(format!("公钥序列化失败: {}", e)))?;

    let fingerprint = private_key
        .public_key()
        .fingerprint(ssh_key::HashAlg::Sha256)
        .to_string();

    let crypto = CryptoService::new(&state.config.secret_key).map_err(internal_error)?;
    let encrypted = crypto.encrypt(&private_pem).map_err(internal_error)?;

    let key = SshKey::new(
        auth.user_id,
        req.name,
        encrypted,
        public_key_str,
        req.key_type.clone(),
        fingerprint,
    );

    sqlx::query(
        "INSERT INTO ssh_keys (id, user_id, name, private_key_encrypted, public_key, key_type, fingerprint, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&key.id)
    .bind(&key.user_id)
    .bind(&key.name)
    .bind(&key.private_key_encrypted)
    .bind(&key.public_key)
    .bind(&key.key_type)
    .bind(&key.fingerprint)
    .bind(&key.created_at)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    let key_response: SshKeyResponse = key.into();

    Ok(Json(GenerateKeyResponse { key: key_response }))
}

/// 生成指定位数的 RSA 私钥
fn generate_rsa_key(bits: usize) -> anyhow::Result<ssh_key::PrivateKey> {
    use ssh_key::private::{KeypairData, RsaKeypair};
    let keypair = RsaKeypair::random(&mut rand::rngs::OsRng, bits)
        .map_err(|e| anyhow::anyhow!("RSA-{} 密钥生成失败: {}", bits, e))?;
    ssh_key::PrivateKey::new(KeypairData::Rsa(keypair), "")
        .map_err(|e| anyhow::anyhow!("私钥封装失败: {}", e))
}
