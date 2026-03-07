use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::store::{use_auth, AuthState};

#[derive(Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
    user: UserInfo,
}

#[derive(Deserialize)]
struct UserInfo {
    id: String,
    email: String,
    role: String,
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let auth = use_auth();

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        let req = LoginRequest {
            email: email.get(),
            password: password.get(),
        };

        leptos::task::spawn_local(async move {
            match crate::api::post::<_, LoginResponse>("/api/auth/login", &req).await {
                Ok(resp) => {
                    let state = AuthState {
                        token: Some(resp.token),
                        user_id: Some(resp.user.id),
                        email: Some(resp.user.email),
                        role: Some(resp.user.role),
                    };
                    state.save();
                    auth.set(state);
                    let window = web_sys::window().unwrap();
                    window.location().set_href("/").ok();
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="min-h-screen bg-gray-900 flex items-center justify-center">
            <div class="bg-gray-800 p-8 rounded-2xl shadow-2xl w-full max-w-md border border-gray-700">
                <div class="text-center mb-8">
                    <div class="text-5xl mb-3">"🦈"</div>
                    <h1 class="text-2xl font-bold text-white">"RustShark"</h1>
                    <p class="text-gray-400 text-sm mt-1">"网络抓包管理平台"</p>
                </div>

                <form on:submit=on_submit class="space-y-4">
                    <div>
                        <label class="block text-sm text-gray-400 mb-1">"邮箱"</label>
                        <input
                            type="email"
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-4 py-2.5 text-white focus:outline-none focus:border-blue-500"
                            placeholder="admin@example.com"
                            prop:value=email
                            on:input=move |ev| email.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    <div>
                        <label class="block text-sm text-gray-400 mb-1">"密码"</label>
                        <input
                            type="password"
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-4 py-2.5 text-white focus:outline-none focus:border-blue-500"
                            placeholder="••••••••"
                            prop:value=password
                            on:input=move |ev| password.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-4 py-2 rounded-lg text-sm">
                            {e}
                        </div>
                    })}

                    <button
                        type="submit"
                        class="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2.5 rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || loading.get()
                    >
                        {move || if loading.get() { "登录中..." } else { "登录" }}
                    </button>
                </form>
            </div>
        </div>
    }
}
