use axum::{extract::State, routing::get, Extension, Json, Router};
use serde::Serialize;

use crate::api::{internal_error, ApiResult, AuthUser};
use crate::state::AppState;

pub fn stats_router() -> Router<AppState> {
    Router::new().route("/api/stats", get(get_stats))
}

#[derive(Serialize)]
struct Stats {
    servers_total: i64,
    servers_online: i64,
    captures_total: i64,
    captures_running: i64,
}

async fn get_stats(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> ApiResult<Stats> {
    let servers_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM servers WHERE user_id = ?")
        .bind(&auth.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(internal_error)?;

    let servers_online: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM servers WHERE user_id = ? AND status = 'online'")
            .bind(&auth.user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(internal_error)?;

    let captures_total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM capture_tasks WHERE user_id = ?")
            .bind(&auth.user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(internal_error)?;

    let captures_running: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM capture_tasks WHERE user_id = ? AND status = 'running'",
    )
    .bind(&auth.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(Stats {
        servers_total,
        servers_online,
        captures_total,
        captures_running,
    }))
}
