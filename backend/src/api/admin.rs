use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Extension, Json, Router,
};
use bcrypt::{hash, DEFAULT_COST};

use crate::api::{bad_request, internal_error, not_found, ApiResult, AuthUser};
use crate::models::user::{CreateUserRequest, UpdateUserRequest, User, UserResponse};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route(
            "/users/{id}",
            delete(delete_user).put(update_user),
        )
}

async fn list_users(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
) -> ApiResult<Vec<UserResponse>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

async fn create_user(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthUser>,
    Json(req): Json<CreateUserRequest>,
) -> ApiResult<UserResponse> {
    // 检查邮箱是否已存在
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?;

    if exists.is_some() {
        return Err(bad_request("邮箱已存在"));
    }

    let password_hash = hash(&req.password, DEFAULT_COST).map_err(internal_error)?;
    let role = req.role.unwrap_or_else(|| "user".to_string());
    let user = User::new(req.email, req.name, password_hash, role);

    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, created_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.name)
    .bind(&user.password_hash)
    .bind(&user.role)
    .bind(&user.created_at)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(user.into()))
}

async fn update_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> ApiResult<UserResponse> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("用户不存在"))?;

    // 如果修改 email，检查唯一性
    if let Some(ref new_email) = req.email {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM users WHERE email = ? AND id != ?")
                .bind(new_email)
                .bind(&id)
                .fetch_optional(&state.pool)
                .await
                .map_err(internal_error)?;
        if exists.is_some() {
            return Err(bad_request("邮箱已被占用"));
        }
    }

    // 不允许降级自己的权限
    if auth.user_id == id {
        if let Some(ref new_role) = req.role {
            if new_role != "admin" {
                return Err(bad_request("不能修改自己的角色"));
            }
        }
    }

    let new_name = req.name.unwrap_or(user.name);
    let new_email = req.email.unwrap_or(user.email);
    let new_role = req.role.unwrap_or(user.role);

    sqlx::query("UPDATE users SET name = ?, email = ?, role = ? WHERE id = ?")
        .bind(&new_name)
        .bind(&new_email)
        .bind(&new_role)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    // 如果提供了新密码，更新密码
    if let Some(ref new_password) = req.password {
        if !new_password.is_empty() {
            let new_hash = hash(new_password, DEFAULT_COST).map_err(internal_error)?;
            sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
                .bind(&new_hash)
                .bind(&id)
                .execute(&state.pool)
                .await
                .map_err(internal_error)?;
        }
    }

    let updated = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(Json(updated.into()))
}

async fn delete_user(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    // 不允许删除自己
    if auth.user_id == id {
        return Err(bad_request("不能删除自己"));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err(not_found("用户不存在"));
    }

    Ok(Json(serde_json::json!({"success": true})))
}
