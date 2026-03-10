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
        Self {
            token,
            user_id,
            email,
            role,
        }
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

// 全局静态信号，不依赖 reactive owner tree
use std::sync::OnceLock;

static AUTH: OnceLock<RwSignal<AuthState>> = OnceLock::new();

pub fn init_auth() {
    AUTH.get_or_init(|| RwSignal::new(AuthState::load()));
}

pub fn use_auth() -> RwSignal<AuthState> {
    *AUTH.get().expect("call init_auth() first")
}

// ── 主题管理 ─────────────────────────────────────────────────────────────────

/// true = 深色模式（dark），false = 浅色模式（light）
static DARK_MODE: OnceLock<RwSignal<bool>> = OnceLock::new();

fn load_dark_mode() -> bool {
    // 优先读 LocalStorage
    if let Ok(v) = gloo_storage::LocalStorage::get::<String>("theme") {
        return v == "dark";
    }
    // 通过 js_sys 检测系统媒体查询偏好
    js_sys::eval("window.matchMedia('(prefers-color-scheme: dark)').matches")
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

fn apply_dark_mode(dark: bool) {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(root) = doc.document_element() {
                let list = root.class_list();
                if dark {
                    list.add_1("dark").ok();
                } else {
                    list.remove_1("dark").ok();
                }
            }
        }
    }
    gloo_storage::LocalStorage::set("theme", if dark { "dark" } else { "light" }).ok();
}

pub fn init_theme() {
    let dark = load_dark_mode();
    apply_dark_mode(dark);
    DARK_MODE.get_or_init(|| RwSignal::new(dark));
}

pub fn use_dark_mode() -> RwSignal<bool> {
    *DARK_MODE.get().expect("call init_theme() first")
}

pub fn toggle_theme() {
    let signal = use_dark_mode();
    let new_dark = !signal.get_untracked();
    apply_dark_mode(new_dark);
    signal.set(new_dark);
}

// ── ESC 优先级管理 ────────────────────────────────────────────────────────────
//
// 规则：弹窗打开时 modal_depth +1，关闭时 -1。
// Layout 的侧边栏 ESC 只在 modal_depth == 0 时生效，从而弹窗 ESC 优先级更高。

static ESC_DEPTH: OnceLock<RwSignal<u32>> = OnceLock::new();

pub fn init_esc_depth() {
    ESC_DEPTH.get_or_init(|| RwSignal::new(0));
}

/// 弹窗打开时调用，增加深度
pub fn push_esc_layer() {
    if let Some(sig) = ESC_DEPTH.get() {
        sig.update(|n| *n += 1);
    }
}

/// 弹窗关闭时调用，减少深度
pub fn pop_esc_layer() {
    if let Some(sig) = ESC_DEPTH.get() {
        sig.update(|n| *n = n.saturating_sub(1));
    }
}

/// 当前是否有弹窗占据 ESC（供 Layout 判断）
pub fn has_esc_focus() -> bool {
    ESC_DEPTH
        .get()
        .map(|s| s.get_untracked() > 0)
        .unwrap_or(false)
}
