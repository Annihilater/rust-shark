use gloo_net::http::Request;
use gloo_storage::Storage;
use serde::{Deserialize, Serialize};

fn get_token() -> Option<String> {
    gloo_storage::LocalStorage::get::<String>("token").ok()
}

pub fn auth_header() -> String {
    get_token()
        .map(|t| format!("Bearer {}", t))
        .unwrap_or_default()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiError {
    pub code: u16,
    pub message: String,
}

/// 收到 401 时自动清除本地 token 并跳转登录页
fn handle_401(status: u16) {
    if status == 401 {
        crate::store::AuthState::clear();
        web_sys::window()
            .unwrap()
            .location()
            .set_href("/login")
            .ok();
    }
}

pub async fn get<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let token = auth_header();
    let resp = Request::get(path)
        .header("Authorization", &token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<T>().await.map_err(|e| e.to_string())
    } else {
        let status = resp.status();
        let err = resp.json::<ApiError>().await.unwrap_or(ApiError {
            code: status,
            message: "请求失败".to_string(),
        });
        handle_401(status);
        Err(err.message)
    }
}

pub async fn post<B: Serialize, T: for<'de> Deserialize<'de>>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let token = auth_header();
    let resp = Request::post(path)
        .header("Authorization", &token)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).unwrap())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<T>().await.map_err(|e| e.to_string())
    } else {
        let status = resp.status();
        let err = resp.json::<ApiError>().await.unwrap_or(ApiError {
            code: status,
            message: "请求失败".to_string(),
        });
        handle_401(status);
        Err(err.message)
    }
}

pub async fn delete<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, String> {
    let token = auth_header();
    let resp = Request::delete(path)
        .header("Authorization", &token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<T>().await.map_err(|e| e.to_string())
    } else {
        let status = resp.status();
        let err = resp.json::<ApiError>().await.unwrap_or(ApiError {
            code: status,
            message: "请求失败".to_string(),
        });
        handle_401(status);
        Err(err.message)
    }
}

pub async fn put<B: Serialize, T: for<'de> Deserialize<'de>>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let token = auth_header();
    let resp = Request::put(path)
        .header("Authorization", &token)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(body).unwrap())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.ok() {
        resp.json::<T>().await.map_err(|e| e.to_string())
    } else {
        let status = resp.status();
        let err = resp.json::<ApiError>().await.unwrap_or(ApiError {
            code: status,
            message: "请求失败".to_string(),
        });
        handle_401(status);
        Err(err.message)
    }
}
