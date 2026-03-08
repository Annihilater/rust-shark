use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};

use crate::api::{bad_request, internal_error, not_found, AuthUser, ApiResult};
use crate::models::capture_profile::{
    CaptureProfile, CreateCaptureProfileRequest, UpdateCaptureProfileRequest,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_profiles).post(create_profile))
        .route("/{id}", get(get_profile).put(update_profile).delete(delete_profile))
}

async fn list_profiles(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<CaptureProfile>> {
    let profiles = sqlx::query_as::<_, CaptureProfile>(
        "SELECT * FROM capture_profiles WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(profiles))
}

async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<CaptureProfile> {
    let profile = sqlx::query_as::<_, CaptureProfile>(
        "SELECT * FROM capture_profiles WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("配置不存在"))?;

    Ok(Json(profile))
}

async fn create_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateCaptureProfileRequest>,
) -> ApiResult<CaptureProfile> {
    if req.name.trim().is_empty() {
        return Err(bad_request("配置名称不能为空"));
    }
    if req.server_id.trim().is_empty() {
        return Err(bad_request("请选择服务器"));
    }
    if req.interface.trim().is_empty() {
        return Err(bad_request("请选择网卡"));
    }

    // 验证服务器属于当前用户
    let server_exists: Option<String> = sqlx::query_scalar(
        "SELECT id FROM servers WHERE id = ? AND user_id = ?",
    )
    .bind(&req.server_id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    if server_exists.is_none() {
        return Err(not_found("服务器不存在"));
    }

    let profile = CaptureProfile::new(
        auth.user_id.clone(),
        req.name,
        req.server_id,
        req.interface,
        req.filter.filter(|s| !s.trim().is_empty()),
        req.duration,
        req.packet_limit,
    );

    sqlx::query(
        "INSERT INTO capture_profiles (id, user_id, name, server_id, interface, filter, duration, packet_limit, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&profile.id)
    .bind(&profile.user_id)
    .bind(&profile.name)
    .bind(&profile.server_id)
    .bind(&profile.interface)
    .bind(&profile.filter)
    .bind(&profile.duration)
    .bind(&profile.packet_limit)
    .bind(&profile.created_at)
    .bind(&profile.updated_at)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(profile))
}

async fn update_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCaptureProfileRequest>,
) -> ApiResult<CaptureProfile> {
    let profile = sqlx::query_as::<_, CaptureProfile>(
        "SELECT * FROM capture_profiles WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("配置不存在"))?;

    // 如果更新了 server_id，验证归属
    if let Some(ref sid) = req.server_id {
        let ok: Option<String> = sqlx::query_scalar(
            "SELECT id FROM servers WHERE id = ? AND user_id = ?",
        )
        .bind(sid)
        .bind(&auth.user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?;

        if ok.is_none() {
            return Err(not_found("服务器不存在"));
        }
    }

    let name      = req.name.unwrap_or(profile.name);
    let server_id = req.server_id.unwrap_or(profile.server_id);
    let interface = req.interface.unwrap_or(profile.interface);
    let filter    = req.filter.or(profile.filter);
    let duration  = req.duration.or(profile.duration);
    let packet_limit = req.packet_limit.or(profile.packet_limit);
    let updated_at = chrono::Utc::now().naive_utc().to_string();

    sqlx::query(
        "UPDATE capture_profiles SET name=?, server_id=?, interface=?, filter=?, duration=?, packet_limit=?, updated_at=? WHERE id=?",
    )
    .bind(&name)
    .bind(&server_id)
    .bind(&interface)
    .bind(&filter)
    .bind(duration)
    .bind(packet_limit)
    .bind(&updated_at)
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    let updated = sqlx::query_as::<_, CaptureProfile>(
        "SELECT * FROM capture_profiles WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(updated))
}

async fn delete_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let exists: Option<String> = sqlx::query_scalar(
        "SELECT id FROM capture_profiles WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    if exists.is_none() {
        return Err(not_found("配置不存在"));
    }

    sqlx::query("DELETE FROM capture_profiles WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(Json(serde_json::json!({"success": true})))
}
