use axum::{extract::State, routing::post, Json, Router};
use bcrypt::verify;

use crate::api::{bad_request, internal_error, ApiResult};
use crate::models::user::{LoginRequest, LoginResponse};
use crate::services::auth::AuthService;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/login", post(login))
}

async fn login(
    State(state): State<AppState>,
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

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}
