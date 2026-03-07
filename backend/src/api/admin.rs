use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Extension, Json, Router,
};
use bcrypt::{hash, DEFAULT_COST};

use crate::api::{bad_request, internal_error, not_found, AuthUser, ApiResult};
use crate::models::user::{CreateUserRequest, User, UserResponse};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", delete(delete_user))
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
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
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
