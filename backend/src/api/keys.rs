use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Extension, Json, Router,
};

use crate::api::{bad_request, internal_error, not_found, AuthUser, ApiResult};
use crate::models::ssh_key::{CreateSshKeyRequest, SshKey, SshKeyResponse};
use crate::services::crypto::CryptoService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_keys).post(create_key))
        .route("/:id", delete(delete_key))
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
    // 提取公钥
    let public_key = extract_public_key(&req.private_key)
        .map_err(|e| bad_request(format!("无效的私钥: {}", e)))?;

    // 加密私钥
    let crypto = CryptoService::new(&state.config.secret_key).map_err(internal_error)?;
    let encrypted = crypto.encrypt(&req.private_key).map_err(internal_error)?;

    let key = SshKey::new(auth.user_id, req.name, encrypted, public_key);

    sqlx::query(
        "INSERT INTO ssh_keys (id, user_id, name, private_key_encrypted, public_key, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&key.id)
    .bind(&key.user_id)
    .bind(&key.name)
    .bind(&key.private_key_encrypted)
    .bind(&key.public_key)
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

fn extract_public_key(private_key_pem: &str) -> anyhow::Result<String> {
    use ssh_key::PrivateKey;
    let key = PrivateKey::from_openssh(private_key_pem)?;
    let pub_key = key.public_key();
    Ok(pub_key.to_openssh()?)
}
