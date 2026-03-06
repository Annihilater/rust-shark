use gloo_storage::Storage;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthState {
    pub token: Option<String>,
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

impl AuthState {
    pub fn load() -> Self {
        let token: Option<String> = gloo_storage::LocalStorage::get("token").ok();
        let user_id: Option<String> = gloo_storage::LocalStorage::get("user_id").ok();
        let email: Option<String> = gloo_storage::LocalStorage::get("email").ok();
        let role: Option<String> = gloo_storage::LocalStorage::get("role").ok();
        Self { token, user_id, email, role }
    }

    pub fn save(&self) {
        if let Some(t) = &self.token {
            gloo_storage::LocalStorage::set("token", t).ok();
        }
        if let Some(uid) = &self.user_id {
            gloo_storage::LocalStorage::set("user_id", uid).ok();
        }
        if let Some(e) = &self.email {
            gloo_storage::LocalStorage::set("email", e).ok();
        }
        if let Some(r) = &self.role {
            gloo_storage::LocalStorage::set("role", r).ok();
        }
    }

    pub fn clear() {
        gloo_storage::LocalStorage::delete("token");
        gloo_storage::LocalStorage::delete("user_id");
        gloo_storage::LocalStorage::delete("email");
        gloo_storage::LocalStorage::delete("role");
    }

    pub fn is_logged_in(&self) -> bool {
        self.token.is_some()
    }

    pub fn is_admin(&self) -> bool {
        self.role.as_deref() == Some("admin")
    }
}

pub fn provide_auth() -> (ReadSignal<AuthState>, WriteSignal<AuthState>) {
    let (auth, set_auth) = signal(AuthState::load());
    provide_context(auth);
    provide_context(set_auth);
    (auth, set_auth)
}

pub fn use_auth() -> ReadSignal<AuthState> {
    expect_context::<ReadSignal<AuthState>>()
}

pub fn use_set_auth() -> WriteSignal<AuthState> {
    expect_context::<WriteSignal<AuthState>>()
}
