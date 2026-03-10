use gloo_storage::Storage;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::api;
use crate::components::layout::Layout;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserProfile {
    id: String,
    email: String,
    name: String,
    role: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginLog {
    id: String,
    user_id: String,
    ip: String,
    user_agent: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateProfileReq {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangePasswordReq {
    old_password: String,
    new_password: String,
}

#[component]
pub fn ProfilePage() -> impl IntoView {
    let profile = RwSignal::new(None::<UserProfile>);
    let login_logs = RwSignal::new(Vec::<LoginLog>::new());
    let error = RwSignal::new(String::new());
    let success = RwSignal::new(String::new());

    // 编辑字段
    let edit_name = RwSignal::new(String::new());
    let edit_email = RwSignal::new(String::new());
    let edit_mode = RwSignal::new(false);

    // 密码字段
    let old_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let show_password_form = RwSignal::new(false);

    // 加载数据
    let load_data = move || {
        spawn_local(async move {
            match api::get::<UserProfile>("/api/profile").await {
                Ok(p) => {
                    edit_name.set(p.name.clone());
                    edit_email.set(p.email.clone());
                    profile.set(Some(p));
                }
                Err(e) => error.set(e),
            }
            match api::get::<Vec<LoginLog>>("/api/profile/login-logs").await {
                Ok(logs) => login_logs.set(logs),
                Err(_) => {}
            }
        });
    };
    load_data();

    // 保存资料
    let save_profile = move |_| {
        let name = edit_name.get();
        let email = edit_email.get();
        error.set(String::new());
        success.set(String::new());
        spawn_local(async move {
            let req = UpdateProfileReq {
                name: if name.is_empty() { None } else { Some(name) },
                email: if email.is_empty() { None } else { Some(email) },
            };
            match api::put::<_, UserProfile>("/api/profile", &req).await {
                Ok(p) => {
                    profile.set(Some(p));
                    edit_mode.set(false);
                    success.set("资料已更新".to_string());
                    // 更新 LocalStorage 中的 email
                    let updated_email = edit_email.get();
                    gloo_storage::LocalStorage::set("email", updated_email).ok();
                }
                Err(e) => error.set(e),
            }
        });
    };

    // 修改密码
    let submit_password = move |_| {
        let old = old_password.get();
        let new = new_password.get();
        let confirm = confirm_password.get();
        error.set(String::new());
        success.set(String::new());

        if new != confirm {
            error.set("两次输入的新密码不一致".to_string());
            return;
        }
        if new.len() < 6 {
            error.set("新密码至少需要6位".to_string());
            return;
        }
        spawn_local(async move {
            let req = ChangePasswordReq {
                old_password: old,
                new_password: new,
            };
            match api::put::<_, serde_json::Value>("/api/profile/password", &req).await {
                Ok(_) => {
                    success.set("密码修改成功".to_string());
                    old_password.set(String::new());
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    show_password_form.set(false);
                }
                Err(e) => error.set(e),
            }
        });
    };

    view! {
        <Layout>
            <div class="max-w-3xl mx-auto space-y-6">
                <h1 class="text-2xl font-bold text-gray-100">"个人中心"</h1>

                // 消息提示
                {move || (!error.get().is_empty()).then(|| view! {
                    <div class="bg-red-900/50 border border-red-700 rounded-lg p-3 text-red-300 text-sm">
                        {error.get()}
                    </div>
                })}
                {move || (!success.get().is_empty()).then(|| view! {
                    <div class="bg-green-900/50 border border-green-700 rounded-lg p-3 text-green-300 text-sm">
                        {success.get()}
                    </div>
                })}

                // ── 基本资料卡片 ─────────────────────────────────
                <div class="bg-gray-800 rounded-xl border border-gray-700 p-5 space-y-4">
                    <div class="flex items-center justify-between">
                        <h2 class="text-lg font-semibold">"基本资料"</h2>
                        {move || if !edit_mode.get() {
                            view! {
                                <button
                                    class="text-sm bg-blue-600 hover:bg-blue-500 px-3 py-1 rounded-md transition-colors"
                                    on:click=move |_| edit_mode.set(true)
                                >
                                    "编辑"
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex gap-2">
                                    <button
                                        class="text-sm bg-green-600 hover:bg-green-500 px-3 py-1 rounded-md transition-colors"
                                        on:click=save_profile
                                    >
                                        "保存"
                                    </button>
                                    <button
                                        class="text-sm bg-gray-600 hover:bg-gray-500 px-3 py-1 rounded-md transition-colors"
                                        on:click=move |_| edit_mode.set(false)
                                    >
                                        "取消"
                                    </button>
                                </div>
                            }.into_any()
                        }}
                    </div>

                    {move || profile.get().map(|p| view! {
                        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"姓名"</label>
                                {if edit_mode.get() {
                                    view! {
                                        <input
                                            type="text"
                                            class="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:border-blue-500"
                                            prop:value=edit_name.get()
                                            on:input=move |ev| edit_name.set(event_target_value(&ev))
                                        />
                                    }.into_any()
                                } else {
                                    view! {
                                        <p class="text-sm text-gray-200">{p.name.clone()}</p>
                                    }.into_any()
                                }}
                            </div>
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"邮箱"</label>
                                {if edit_mode.get() {
                                    view! {
                                        <input
                                            type="email"
                                            class="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:border-blue-500"
                                            prop:value=edit_email.get()
                                            on:input=move |ev| edit_email.set(event_target_value(&ev))
                                        />
                                    }.into_any()
                                } else {
                                    view! {
                                        <p class="text-sm text-gray-200">{p.email.clone()}</p>
                                    }.into_any()
                                }}
                            </div>
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"角色"</label>
                                <p class="text-sm">
                                    {if p.role == "admin" {
                                        view! { <span class="text-yellow-400 font-medium">"管理员"</span> }.into_any()
                                    } else {
                                        view! { <span class="text-gray-300">"普通用户"</span> }.into_any()
                                    }}
                                </p>
                            </div>
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"注册时间"</label>
                                <p class="text-sm text-gray-300">{p.created_at.clone()}</p>
                            </div>
                        </div>
                    })}
                </div>

                // ── 修改密码卡片 ─────────────────────────────────
                <div class="bg-gray-800 rounded-xl border border-gray-700 p-5 space-y-4">
                    <div class="flex items-center justify-between">
                        <h2 class="text-lg font-semibold">"修改密码"</h2>
                        {move || if !show_password_form.get() {
                            view! {
                                <button
                                    class="text-sm bg-blue-600 hover:bg-blue-500 px-3 py-1 rounded-md transition-colors"
                                    on:click=move |_| show_password_form.set(true)
                                >
                                    "修改"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>

                    {move || show_password_form.get().then(|| view! {
                        <div class="space-y-3 max-w-sm">
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"原密码"</label>
                                <input
                                    type="password"
                                    class="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:border-blue-500"
                                    prop:value=old_password.get()
                                    on:input=move |ev| old_password.set(event_target_value(&ev))
                                />
                            </div>
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"新密码"</label>
                                <input
                                    type="password"
                                    class="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:border-blue-500"
                                    prop:value=new_password.get()
                                    on:input=move |ev| new_password.set(event_target_value(&ev))
                                />
                            </div>
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"确认新密码"</label>
                                <input
                                    type="password"
                                    class="w-full bg-gray-700 border border-gray-600 rounded-md px-3 py-1.5 text-sm focus:outline-none focus:border-blue-500"
                                    prop:value=confirm_password.get()
                                    on:input=move |ev| confirm_password.set(event_target_value(&ev))
                                />
                            </div>
                            <div class="flex gap-2 pt-1">
                                <button
                                    class="bg-green-600 hover:bg-green-500 px-4 py-1.5 rounded-md text-sm transition-colors"
                                    on:click=submit_password
                                >
                                    "确认修改"
                                </button>
                                <button
                                    class="bg-gray-600 hover:bg-gray-500 px-4 py-1.5 rounded-md text-sm transition-colors"
                                    on:click=move |_| show_password_form.set(false)
                                >
                                    "取消"
                                </button>
                            </div>
                        </div>
                    })}
                </div>

                // ── 登录记录卡片 ─────────────────────────────────
                <div class="bg-gray-800 rounded-xl border border-gray-700 p-5 space-y-3">
                    <h2 class="text-lg font-semibold">"最近登录记录"</h2>
                    {move || {
                        let logs = login_logs.get();
                        if logs.is_empty() {
                            view! {
                                <p class="text-sm text-gray-500">"暂无登录记录"</p>
                            }.into_any()
                        } else {
                            view! {
                                <div class="overflow-x-auto">
                                    <table class="w-full text-sm">
                                        <thead>
                                            <tr class="text-xs text-gray-400 border-b border-gray-700">
                                                <th class="text-left py-2 pr-4">"登录时间"</th>
                                                <th class="text-left py-2 pr-4">"IP 地址"</th>
                                                <th class="text-left py-2">"User-Agent"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {logs.into_iter().map(|log| view! {
                                                <tr class="border-b border-gray-700/50 hover:bg-gray-700/30">
                                                    <td class="py-2 pr-4 text-gray-300 whitespace-nowrap">{log.created_at}</td>
                                                    <td class="py-2 pr-4 text-blue-300 font-mono">{log.ip}</td>
                                                    <td class="py-2 text-gray-400 text-xs max-w-xs truncate">{log.user_agent}</td>
                                                </tr>
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </Layout>
    }
}
