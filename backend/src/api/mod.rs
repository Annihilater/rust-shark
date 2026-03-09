use axum::body::Body;
use axum::{
    extract::State,
    http::{header, Request, StatusCode, Uri},
    middleware::{self, Next},
    response::Response,
    Json, Router,
};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

pub mod admin;
pub mod auth;
pub mod capture_profiles;
pub mod captures;
pub mod keys;
pub mod servers;
pub mod stats;

// ─── 错误类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
}

pub type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiError>)>;

pub fn api_error(status: StatusCode, msg: impl ToString) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            code: status.as_u16(),
            message: msg.to_string(),
        }),
    )
}

pub fn internal_error(e: impl ToString) -> (StatusCode, Json<ApiError>) {
    api_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn not_found(msg: impl ToString) -> (StatusCode, Json<ApiError>) {
    api_error(StatusCode::NOT_FOUND, msg)
}

pub fn forbidden(msg: impl ToString) -> (StatusCode, Json<ApiError>) {
    api_error(StatusCode::FORBIDDEN, msg)
}

pub fn bad_request(msg: impl ToString) -> (StatusCode, Json<ApiError>) {
    api_error(StatusCode::BAD_REQUEST, msg)
}

// ─── 认证扩展 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    #[allow(dead_code)]
    pub email: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

// ─── 中间件 ───────────────────────────────────────────────────────────────────

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiError>)> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(t) => t.to_string(),
        None => return Err(api_error(StatusCode::UNAUTHORIZED, "缺少认证 Token")),
    };

    let auth_service = crate::services::auth::AuthService::new(
        state.config.jwt_secret.clone(),
        state.config.jwt_expiry_hours,
    );

    let claims = match auth_service.verify_token(&token) {
        Ok(c) => c,
        Err(_) => return Err(api_error(StatusCode::UNAUTHORIZED, "Token 无效或已过期")),
    };

    req.extensions_mut().insert(AuthUser {
        user_id: claims.sub,
        email: claims.email,
        role: claims.role,
    });

    Ok(next.run(req).await)
}

pub async fn admin_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiError>)> {
    let user = req
        .extensions()
        .get::<AuthUser>()
        .cloned()
        .ok_or_else(|| api_error(StatusCode::UNAUTHORIZED, "未认证"))?;

    if !user.is_admin() {
        return Err(forbidden("需要管理员权限"));
    }

    Ok(next.run(req).await)
}

// ─── 前端静态资源 ──────────────────────────────────────────────────────────────

#[derive(rust_embed::RustEmbed)]
#[folder = "../frontend/dist/"]
struct FrontendAssets;

async fn serve_frontend(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    match FrontendAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => match FrontendAssets::get("index.html") {
            Some(content) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(content.data.into_owned()))
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap(),
        },
    }
}

// ─── 主路由构建 ────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let protected = Router::new()
        .nest("/api/keys", keys::router())
        .nest("/api/servers", servers::router())
        .nest("/api/captures", captures::router())
        .nest("/api/capture-profiles", capture_profiles::router())
        .merge(stats::stats_router())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let admin_routes = Router::new()
        .nest("/api/admin", admin::router())
        .layer(middleware::from_fn(admin_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let public = Router::new().nest("/api/auth", auth::router()).route(
        "/api/health",
        axum::routing::get(|| async { axum::Json(serde_json::json!({"status": "ok"})) }),
    );

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(admin_routes)
        .fallback(serve_frontend)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
