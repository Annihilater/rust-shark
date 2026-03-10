use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    routing::post,
    Json, Router,
};
use bcrypt::verify;
use std::net::SocketAddr;
use uuid::Uuid;

use crate::api::{bad_request, internal_error, ApiResult};
use crate::models::user::{LoginRequest, LoginResponse};
use crate::services::auth::AuthService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/login", post(login))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> ApiResult<LoginResponse> {
    let user =
        sqlx::query_as::<_, crate::models::user::User>("SELECT * FROM users WHERE email = ?")
            .bind(&req.email)
            .fetch_optional(&state.pool)
            .await
            .map_err(internal_error)?;

    let user = match user {
        Some(u) => u,
        None => return Err(bad_request("邮箱或密码错误")),
    };

    let valid = verify(&req.password, &user.password_hash).map_err(internal_error)?;

    if !valid {
        return Err(bad_request("邮箱或密码错误"));
    }

    let auth = AuthService::new(
        state.config.jwt_secret.clone(),
        state.config.jwt_expiry_hours,
    );
    let token = auth
        .generate_token(&user.id, &user.email, &user.role)
        .map_err(internal_error)?;

    // 记录登录日志
    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let user_agent = headers
        .get("User-Agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let _ = sqlx::query(
        "INSERT INTO login_logs (id, user_id, ip, user_agent, created_at) VALUES (?, ?, ?, ?, datetime('now'))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&user.id)
    .bind(&ip)
    .bind(&user_agent)
    .execute(&state.pool)
    .await;

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}
