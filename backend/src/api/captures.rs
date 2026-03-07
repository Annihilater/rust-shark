use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use tokio::fs;

use crate::api::{bad_request, internal_error, not_found, AuthUser, ApiResult};
use crate::models::capture::{CaptureTask, CreateCaptureRequest};
use crate::services::sharkd::{PacketDetail, PacketSummary, SharkdSession};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_captures).post(create_capture))
        .route("/{id}", get(get_capture).delete(delete_capture))
        .route("/{id}/stop", post(stop_capture))
        .route("/{id}/download", get(download_capture))
        .route("/{id}/log", get(get_capture_log))
        .route("/{id}/packets", get(get_packets))
        .route("/{id}/packets/{no}", get(get_packet_detail))
}

async fn list_captures(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Vec<CaptureTask>> {
    let tasks = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(&auth.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(tasks))
}

async fn get_capture(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<CaptureTask> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("抓包任务不存在"))?;

    Ok(Json(task))
}

async fn create_capture(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateCaptureRequest>,
) -> ApiResult<CaptureTask> {
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

    let task = CaptureTask::new(
        auth.user_id.clone(),
        req.server_id.clone(),
        req.interface,
        req.filter,
        req.duration,
        req.packet_limit,
        req.scheduled_at,
        req.repeat_type,
        req.repeat_until,
    );

    sqlx::query(
        "INSERT INTO capture_tasks (id, user_id, server_id, interface, filter, duration, packet_limit, status, scheduled_at, repeat_type, repeat_until, created_at, log_msg) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&task.id)
    .bind(&task.user_id)
    .bind(&task.server_id)
    .bind(&task.interface)
    .bind(&task.filter)
    .bind(&task.duration)
    .bind(&task.packet_limit)
    .bind(&task.status)
    .bind(&task.scheduled_at)
    .bind(&task.repeat_type)
    .bind(&task.repeat_until)
    .bind(&task.created_at)
    .bind(&task.log_msg)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    // 如果是立即执行（无 scheduled_at），启动后台任务
    if task.scheduled_at.is_none() {
        let task_clone = task.clone();
        let state_clone = state.clone();
        let user_id = auth.user_id.clone();
        tokio::spawn(async move {
            if let Err(e) = run_capture_task(&state_clone, &task_clone, &user_id).await {
                tracing::error!("抓包任务失败 {}: {}", task_clone.id, e);
            }
        });
    }

    Ok(Json(task))
}

async fn stop_capture(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("任务不存在"))?;

    if task.status != "running" {
        return Err(bad_request("任务不在运行状态"));
    }

    sqlx::query(
        "UPDATE capture_tasks SET status = 'cancelled', finished_at = datetime('now') WHERE id = ?",
    )
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_capture_log(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("任务不存在"))?;

    Ok(Json(serde_json::json!({"log": task.log_msg.unwrap_or_default()})))
}

async fn delete_capture(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("任务不存在"))?;

    // 删除本地文件
    if let Some(file_path) = &task.file_path {
        fs::remove_file(file_path).await.ok();
    }

    sqlx::query("DELETE FROM capture_tasks WHERE id = ? AND user_id = ?")
        .bind(&id)
        .bind(&auth.user_id)
        .execute(&state.pool)
        .await
        .map_err(internal_error)?;

    Ok(Json(serde_json::json!({"success": true})))
}

async fn download_capture(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<crate::api::ApiError>)> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("任务不存在"))?;

    let file_path = task.file_path.ok_or_else(|| not_found("文件尚未生成"))?;

    let data = fs::read(&file_path)
        .await
        .map_err(|e| internal_error(format!("读取文件失败: {}", e)))?;

    let filename = format!("capture-{}.pcap", id);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(data))
        .unwrap())
}

async fn get_packets(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Vec<PacketSummary>> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("任务不存在"))?;

    let file_path = task.file_path.ok_or_else(|| bad_request("文件尚未生成"))?;

    let skip: u64 = params.get("skip").and_then(|v| v.parse().ok()).unwrap_or(0);
    let limit: u64 = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let filter = params.get("filter").map(|s| s.as_str());

    let session = SharkdSession::new(&file_path, &state.config.data_dir)
        .await
        .map_err(|e| internal_error(format!("启动 sharkd 失败: {}", e)))?;

    let packets = session
        .get_packets(skip, limit, filter)
        .await
        .map_err(internal_error)?;

    Ok(Json(packets))
}

async fn get_packet_detail(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path((id, no)): Path<(String, u64)>,
) -> ApiResult<PacketDetail> {
    let task = sqlx::query_as::<_, CaptureTask>(
        "SELECT * FROM capture_tasks WHERE id = ? AND user_id = ?",
    )
    .bind(&id)
    .bind(&auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?
    .ok_or_else(|| not_found("任务不存在"))?;

    let file_path = task.file_path.ok_or_else(|| bad_request("文件尚未生成"))?;

    let session = SharkdSession::new(&file_path, &state.config.data_dir)
        .await
        .map_err(|e| internal_error(format!("启动 sharkd 失败: {}", e)))?;

    let detail = session
        .get_packet_detail(no)
        .await
        .map_err(internal_error)?;

    Ok(Json(detail))
}

/// 后台执行抓包任务
async fn run_capture_task(
    state: &AppState,
    task: &CaptureTask,
    user_id: &str,
) -> anyhow::Result<()> {
    use crate::api::servers::get_server_credentials;
    use crate::models::server::Server;
    use crate::services::capture;

    let server = sqlx::query_as::<_, Server>(
        "SELECT * FROM servers WHERE id = ? AND user_id = ?",
    )
    .bind(&task.server_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    let (private_key_pem, password) =
        get_server_credentials(state, &task.server_id, user_id)
            .await
            .map_err(|(_, e)| anyhow::anyhow!("{}", e.0.message))?;

    capture::run_capture(
        task,
        &server,
        private_key_pem.as_deref(),
        password.as_deref(),
        &state.config.data_dir,
        state.pool.clone(),
        state.capture_pids.clone(),
    )
    .await?;

    Ok(())
}
