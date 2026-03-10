use axum::{
    extract::State,
    routing::{get, put},
    Extension, Json, Router,
};
use bcrypt::{hash, verify, DEFAULT_COST};

use crate::api::{bad_request, internal_error, ApiResult, AuthUser};
use crate::models::user::{
    ChangePasswordRequest, LoginLog, UpdateProfileRequest, User, UserResponse,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_profile).put(update_profile))
        .route("/password", put(change_password))
        .route("/login-logs", get(get_login_logs))
}

async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<UserResponse> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&auth.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("用户不存在"))?;

    Ok(Json(user.into()))
}

async fn update_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<UpdateProfileRequest>,
) -> ApiResult<UserResponse> {
    // 如果要更新 email，检查是否已被占用
    if let Some(ref new_email) = req.email {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM users WHERE email = ? AND id != ?")
                .bind(new_email)
                .bind(&auth.user_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(internal_error)?;
        if exists.is_some() {
            return Err(bad_request("邮箱已被占用"));
        }
    }

    // 构建动态 UPDATE
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&auth.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("用户不存在"))?;

    let new_name = req.name.unwrap_or(user.name);
    let new_email = req.email.unwrap_or(user.email);

    sqlx::query("UPDATE users SET name = ?, email = ? WHERE id = ?")
        .bind(&new_name)
        .bind(&new_email)
        .bind(&auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    let updated = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&auth.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(Json(updated.into()))
}

async fn change_password(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> ApiResult<serde_json::Value> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&auth.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| bad_request("用户不存在"))?;

    let valid = verify(&req.old_password, &user.password_hash).map_err(internal_error)?;
    if !valid {
        return Err(bad_request("原密码错误"));
    }

    if req.new_password.len() < 6 {
        return Err(bad_request("新密码至少需要6位"));
    }

    let new_hash = hash(&req.new_password, DEFAULT_COST).map_err(internal_error)?;

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(&auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_login_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<LoginLog>> {
    let logs = sqlx::query_as::<_, LoginLog>(
        "SELECT * FROM login_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT 50",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(logs))
}
