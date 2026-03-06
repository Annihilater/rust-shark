use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Extension, Json, Router,
};

use crate::api::{bad_request, internal_error, not_found, AuthUser, ApiResult};
use crate::models::server::{
    CreateServerRequest, NetworkInterface, Server, ServerTestResult, UpdateServerRequest,
};
use crate::services::crypto::CryptoService;
use crate::services::ssh;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_servers).post(create_server))
        .route("/:id", put(update_server).delete(delete_server))
        .route("/:id/test", post(test_server))
        .route("/:id/interfaces", get(get_interfaces))
}

async fn list_servers(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<Server>> {
    let servers = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(servers))
}

async fn create_server(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateServerRequest>,
) -> ApiResult<Server> {
    let crypto = CryptoService::new(&state.config.secret_key).map_err(internal_error)?;

    let password_encrypted = if let Some(pwd) = &req.password {
        Some(crypto.encrypt(pwd).map_err(internal_error)?)
    } else {
        None
    };

    let server = Server::new(
        auth.user_id,
        req.name,
        req.host,
        req.port.unwrap_or(22),
        req.username,
        req.auth_type,
        req.ssh_key_id,
        password_encrypted,
    );

    sqlx::query(
        "INSERT INTO servers (id, user_id, name, host, port, username, auth_type, ssh_key_id, password_encrypted, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&server.id)
    .bind(&server.user_id)
    .bind(&server.name)
    .bind(&server.host)
    .bind(&server.port)
    .bind(&server.username)
    .bind(&server.auth_type)
    .bind(&server.ssh_key_id)
    .bind(&server.password_encrypted)
    .bind(&server.status)
    .bind(&server.created_at)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(server))
}

async fn update_server(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> ApiResult<Server> {
    let server = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("服务器不存在"))?;

    let crypto = CryptoService::new(&state.config.secret_key).map_err(internal_error)?;

    let password_encrypted = if let Some(pwd) = &req.password {
        Some(crypto.encrypt(pwd).map_err(internal_error)?)
    } else {
        server.password_encrypted.clone()
    };

    sqlx::query(
        "UPDATE servers SET name=COALESCE(?,name), host=COALESCE(?,host), port=COALESCE(?,port), username=COALESCE(?,username), auth_type=COALESCE(?,auth_type), ssh_key_id=COALESCE(?,ssh_key_id), password_encrypted=? WHERE id = ? AND user_id = ?",
    )
    .bind(&req.name)
    .bind(&req.host)
    .bind(&req.port)
    .bind(&req.username)
    .bind(&req.auth_type)
    .bind(&req.ssh_key_id)
    .bind(&password_encrypted)
    .bind(&id)
    .bind(&auth.user_id)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    let updated = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(updated))
}

async fn delete_server(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let result = sqlx::query("DELETE FROM servers WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err(not_found("服务器不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}

async fn test_server(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<ServerTestResult> {
    let (private_key_pem, password) = get_server_credentials(&state, &id, &auth.user_id).await?;

    let server = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("服务器不存在"))?;

    let result = ssh::test_server_connection(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
        private_key_pem.as_deref(),
        password.as_deref(),
    )
    .await
    .unwrap_or_else(|e| ServerTestResult {
        success: false,
        message: e.to_string(),
        tcpdump_available: false,
        tcpdump_version: None,
        interfaces: vec![],
    });

    // 更新服务器状态
    let new_status = if result.success { "online" } else { "offline" };
    sqlx::query(
        "UPDATE servers SET status = ?, last_checked_at = datetime('now') WHERE id = ?",
    )
    .bind(new_status)
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(result))
}

async fn get_interfaces(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<Vec<NetworkInterface>> {
    let (private_key_pem, password) = get_server_credentials(&state, &id, &auth.user_id).await?;

    let server = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("服务器不存在"))?;

    let interfaces = ssh::get_interfaces(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
        private_key_pem.as_deref(),
        password.as_deref(),
    )
    .await
    .map_err(|e| bad_request(format!("获取网卡失败: {}", e)))?;

    Ok(Json(interfaces))
}

/// 获取服务器凭证（解密）
pub async fn get_server_credentials(
    state: &AppState,
    server_id: &str,
    user_id: &str,
) -> Result<(Option<String>, Option<String>), (axum::http::StatusCode, Json<crate::api::ApiError>)>
{
    let server = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE id = ? AND user_id = ?",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("服务器不存在"))?;

    let crypto = CryptoService::new(&state.config.secret_key).map_err(internal_error)?;

    let private_key_pem = if server.auth_type == "key" {
        if let Some(key_id) = &server.ssh_key_id {
            let key = sqlx::query_as::<_, crate::models::ssh_key::SshKey>(
                "SELECT * FROM ssh_keys WHERE id = ? AND user_id = ?",
            )
            .bind(key_id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| not_found("SSH密钥不存在"))?;

            Some(
                crypto
                    .decrypt(&key.private_key_encrypted)
                    .map_err(internal_error)?,
            )
        } else {
            None
        }
    } else {
        None
    };

    let password = if server.auth_type == "password" {
        if let Some(enc) = &server.password_encrypted {
            Some(crypto.decrypt(enc).map_err(internal_error)?)
        } else {
            None
        }
    } else {
        None
    };

    Ok((private_key_pem, password))
}
