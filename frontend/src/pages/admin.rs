use crate::components::select::{Select, SelectOption};
use crate::components::{layout::Layout, modal::Modal};
use crate::store::use_auth;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
struct UserRow {
    id: String,
    email: String,
    name: String,
    role: String,
    created_at: String,
}

#[derive(Serialize)]
struct CreateUserRequest {
    email: String,
    name: String,
    password: String,
    role: Option<String>,
}

#[derive(Serialize)]
struct UpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    role: Option<String>,
    password: Option<String>,
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() || !auth.get().is_admin() {
            web_sys::window().unwrap().location().set_href("/").ok();
        }
    });

    let users = RwSignal::new(Vec::<UserRow>::new());
    let error = RwSignal::new(Option::<String>::None);
    let success = RwSignal::new(Option::<String>::None);

    // ── 创建用户弹窗状态 ─────────────────────────────────────────
    let show_create = RwSignal::new(false);
    let new_email = RwSignal::new(String::new());
    let new_name = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let new_role = RwSignal::new("user".to_string());

    // ── 编辑用户弹窗状态 ─────────────────────────────────────────
    let show_edit = RwSignal::new(false);
    let edit_id = RwSignal::new(String::new());
    let edit_email_val = RwSignal::new(String::new());
    let edit_name_val = RwSignal::new(String::new());
    let edit_role_val = RwSignal::new("user".to_string());
    let edit_password_val = RwSignal::new(String::new());

    // 加载用户列表
    let load_users = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<UserRow>>("/api/admin/users").await {
                users.set(list);
            }
        });
    };
    load_users();

    // 创建用户
    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = CreateUserRequest {
            email: new_email.get(),
            name: new_name.get(),
            password: new_password.get(),
            role: Some(new_role.get()),
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, UserRow>("/api/admin/users", &req).await {
                Ok(_) => {
                    show_create.set(false);
                    new_email.set(String::new());
                    new_name.set(String::new());
                    new_password.set(String::new());
                    new_role.set("user".to_string());
                    error.set(None);
                    success.set(Some("用户创建成功".to_string()));
                    load_users();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // 打开编辑弹窗
    let open_edit = move |user: UserRow| {
        edit_id.set(user.id.clone());
        edit_name_val.set(user.name.clone());
        edit_email_val.set(user.email.clone());
        edit_role_val.set(user.role.clone());
        edit_password_val.set(String::new());
        error.set(None);
        show_edit.set(true);
    };

    // 保存编辑
    let on_save_edit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let id = edit_id.get();
        let pwd = edit_password_val.get();
        let req = UpdateUserRequest {
            name: Some(edit_name_val.get()),
            email: Some(edit_email_val.get()),
            role: Some(edit_role_val.get()),
            password: if pwd.is_empty() { None } else { Some(pwd) },
        };
        leptos::task::spawn_local(async move {
            let path = format!("/api/admin/users/{}", id);
            match crate::api::put::<_, UserRow>(&path, &req).await {
                Ok(_) => {
                    show_edit.set(false);
                    error.set(None);
                    success.set(Some("用户已更新".to_string()));
                    load_users();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // 删除用户
    let on_delete = move |id: String| {
        error.set(None);
        leptos::task::spawn_local(async move {
            let path = format!("/api/admin/users/{}", id);
            match crate::api::delete::<serde_json::Value>(&path).await {
                Ok(_) => {
                    success.set(Some("用户已删除".to_string()));
                    load_users();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    view! {
        <Layout>
            <div class="max-w-5xl mx-auto space-y-5">
                <div class="flex items-center justify-between">
                    <h1 class="text-2xl font-bold">"用户管理"</h1>
                    <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm"
                        on:click=move |_| { error.set(None); show_create.set(true); }
                    >"＋ 创建用户"</button>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="notice-error px-4 py-2 rounded-lg text-sm">{e}</div>
                })}
                {move || success.get().map(|s| view! {
                    <div class="bg-green-50 border border-green-200 text-green-700 dark:bg-green-900/50 dark:border-green-700 dark:text-green-300 px-4 py-2 rounded-lg text-sm">{s}</div>
                })}

                // ── 用户表格 ──────────────────────────────────────
                <div class="card rounded-xl overflow-hidden">
                    <table class="w-full text-sm">
                        <thead class="table-head">
                            <tr>
                                <th class="text-left px-4 py-3 text-gray-500 dark:text-gray-400 font-medium">"姓名"</th>
                                <th class="text-left px-4 py-3 text-gray-500 dark:text-gray-400 font-medium">"邮箱"</th>
                                <th class="text-left px-4 py-3 text-gray-500 dark:text-gray-400 font-medium">"角色"</th>
                                <th class="text-left px-4 py-3 text-gray-500 dark:text-gray-400 font-medium">"注册时间"</th>
                                <th class="text-right px-4 py-3 text-gray-500 dark:text-gray-400 font-medium">"操作"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || users.get().into_iter().map(|user| {
                                let user_for_edit = user.clone();
                                let user_id_del = user.id.clone();
                                let current_id = auth.get().user_id.clone().unwrap_or_default();
                                let is_self = user.id == current_id;
                                view! {
                                    <tr class="table-row last:border-0">
                                        <td class="px-4 py-3 font-medium">{user.name.clone()}</td>
                                        <td class="px-4 py-3 text-gray-500 dark:text-gray-400">{user.email.clone()}</td>
                                        <td class="px-4 py-3">
                                            <span class=if user.role == "admin" { "text-yellow-500 dark:text-yellow-400 text-xs" } else { "text-gray-500 dark:text-gray-400 text-xs" }>
                                                {if user.role == "admin" { "👑 管理员" } else { "👤 用户" }}
                                            </span>
                                        </td>
                                        <td class="px-4 py-3 text-gray-400 dark:text-gray-500 text-xs">{user.created_at.clone()}</td>
                                        <td class="px-4 py-3 text-right space-x-2">
                                            <button
                                                class="text-blue-500 dark:text-blue-400 hover:text-blue-400 dark:hover:text-blue-300 text-xs"
                                                on:click=move |_| open_edit(user_for_edit.clone())
                                            >"编辑"</button>
                                            {if !is_self {
                                                view! {
                                                    <button
                                                        class="text-red-500 dark:text-red-400 hover:text-red-400 dark:hover:text-red-300 text-xs ml-2"
                                                        on:click=move |_| on_delete(user_id_del.clone())
                                                    >"删除"</button>
                                                }.into_any()
                                            } else {
                                                view! { <span class="text-gray-400 dark:text-gray-600 text-xs ml-2">"（自己）"</span> }.into_any()
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()}
                        </tbody>
                    </table>
                    {move || users.get().is_empty().then(|| view! {
                        <p class="text-center text-gray-500 text-sm py-8">"暂无用户"</p>
                    })}
                </div>
            </div>

            // ── 创建用户弹窗 ──────────────────────────────────────
            <Modal
                show=show_create.read_only()
                title="创建用户"
                on_close=Callback::new(move |_| show_create.set(false))
            >
                <form on:submit=on_create class="space-y-3">
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"姓名 *"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            prop:value=new_name
                            on:input=move |ev| new_name.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"邮箱 *"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            type="email"
                            prop:value=new_email
                            on:input=move |ev| new_email.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"密码 *"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            type="password"
                            prop:value=new_password
                            on:input=move |ev| new_password.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"角色"</label>
                        <Select
                            options=vec![
                                SelectOption::new("user", "👤 普通用户"),
                                SelectOption::new("admin", "👑 管理员"),
                            ]
                            value=new_role.read_only()
                            on_change=Callback::new(move |v| new_role.set(v))
                        />
                    </div>
                    <div class="flex gap-3 pt-1">
                        <button type="submit" class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm">"创建"</button>
                        <button type="button"
                            class="flex-1 btn-secondary py-2 rounded-lg text-sm"
                            on:click=move |_| show_create.set(false)
                        >"取消"</button>
                    </div>
                </form>
            </Modal>

            // ── 编辑用户弹窗 ──────────────────────────────────────
            <Modal
                show=show_edit.read_only()
                title=Signal::derive(move || format!("编辑用户"))
                on_close=Callback::new(move |_| show_edit.set(false))
            >
                <form on:submit=on_save_edit class="space-y-3">
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"姓名"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            prop:value=edit_name_val
                            on:input=move |ev| edit_name_val.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"邮箱"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            type="email"
                            prop:value=edit_email_val
                            on:input=move |ev| edit_email_val.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"角色"</label>
                        <Select
                            options=vec![
                                SelectOption::new("user", "👤 普通用户"),
                                SelectOption::new("admin", "👑 管理员"),
                            ]
                            value=edit_role_val.read_only()
                            on_change=Callback::new(move |v| edit_role_val.set(v))
                        />
                    </div>
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"新密码（留空则不修改）"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            type="password"
                            placeholder="不修改请留空"
                            prop:value=edit_password_val
                            on:input=move |ev| edit_password_val.set(event_target_value(&ev))
                        />
                    </div>
                    {move || error.get().map(|e| view! {
                        <div class="notice-error px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}
                    <div class="flex gap-3 pt-1">
                        <button type="submit" class="flex-1 bg-green-600 hover:bg-green-700 text-white py-2 rounded-lg text-sm">"保存"</button>
                        <button type="button"
                            class="flex-1 btn-secondary py-2 rounded-lg text-sm"
                            on:click=move |_| show_edit.set(false)
                        >"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
