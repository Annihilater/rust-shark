use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::{layout::Layout, modal::Modal};
use crate::store::use_auth;

#[derive(Deserialize, Clone, Debug)]
struct User {
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

#[component]
pub fn AdminPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() || !auth.get().is_admin() {
            web_sys::window().unwrap().location().set_href("/").ok();
        }
    });

    let users = RwSignal::new(Vec::<User>::new());
    let show_modal = RwSignal::new(false);
    let email = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let role = RwSignal::new("user".to_string());
    let error = RwSignal::new(Option::<String>::None);
    let success = RwSignal::new(Option::<String>::None);

    let load_users = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<User>>("/api/admin/users").await {
                users.set(list);
            }
        });
    };
    load_users();

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = CreateUserRequest {
            email: email.get(),
            name: name.get(),
            password: password.get(),
            role: Some(role.get()),
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, User>("/api/admin/users", &req).await {
                Ok(_) => {
                    show_modal.set(false);
                    success.set(Some("用户创建成功".to_string()));
                    load_users();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_delete = move |id: String| {
        leptos::task::spawn_local(async move {
            let path = format!("/api/admin/users/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                success.set(Some("用户已删除".to_string()));
                load_users();
            }
        });
    };

    view! {
        <Layout>
            <div class="max-w-4xl mx-auto">
                <div class="flex items-center justify-between mb-6">
                    <h1 class="text-2xl font-bold">"用户管理"</h1>
                    <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm"
                        on:click=move |_| show_modal.set(true)
                    >"+ 创建用户"</button>
                </div>

                {move || success.get().map(|s| view! {
                    <div class="bg-green-900/50 border border-green-700 text-green-300 px-4 py-2 rounded-lg text-sm mb-4">{s}</div>
                })}

                <div class="bg-gray-800 border border-gray-700 rounded-xl overflow-hidden">
                    <table class="w-full text-sm">
                        <thead class="bg-gray-700/50 border-b border-gray-700">
                            <tr>
                                <th class="text-left px-4 py-3 text-gray-400 font-medium">"姓名"</th>
                                <th class="text-left px-4 py-3 text-gray-400 font-medium">"邮箱"</th>
                                <th class="text-left px-4 py-3 text-gray-400 font-medium">"角色"</th>
                                <th class="text-left px-4 py-3 text-gray-400 font-medium">"创建时间"</th>
                                <th class="text-left px-4 py-3 text-gray-400 font-medium">"操作"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || users.get().into_iter().map(|user| {
                                let id = user.id.clone();
                                let current_id = auth.get().user_id.clone().unwrap_or_default();
                                let is_self = id == current_id;
                                view! {
                                    <tr class="border-b border-gray-700/50 last:border-0">
                                        <td class="px-4 py-3">{user.name}</td>
                                        <td class="px-4 py-3 text-gray-400">{user.email}</td>
                                        <td class="px-4 py-3">
                                            <span class=if user.role == "admin" { "text-yellow-400 text-xs" } else { "text-gray-400 text-xs" }>
                                                {if user.role == "admin" { "👑 管理员" } else { "👤 用户" }}
                                            </span>
                                        </td>
                                        <td class="px-4 py-3 text-gray-500 text-xs">{user.created_at}</td>
                                        <td class="px-4 py-3">
                                            {if !is_self {
                                                view! {
                                                    <button
                                                        class="text-red-400 hover:text-red-300 text-xs"
                                                        on:click=move |_| on_delete(id.clone())
                                                    >"删除"</button>
                                                }.into_any()
                                            } else {
                                                view! { <span class="text-gray-600 text-xs">"（自己）"</span> }.into_any()
                                            }}
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()}
                        </tbody>
                    </table>
                </div>
            </div>

            <Modal
                show=show_modal.read_only()
                title="创建用户"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                <form on:submit=on_create class="space-y-3">
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"姓名"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            prop:value=name on:input=move |ev| name.set(event_target_value(&ev)) required/>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"邮箱"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            type="email" prop:value=email on:input=move |ev| email.set(event_target_value(&ev)) required/>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"密码"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            type="password" prop:value=password on:input=move |ev| password.set(event_target_value(&ev)) required/>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"角色"</label>
                        <select class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            on:change=move |ev| role.set(event_target_value(&ev))>
                            <option value="user">"普通用户"</option>
                            <option value="admin">"管理员"</option>
                        </select>
                    </div>
                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}
                    <div class="flex gap-3 pt-1">
                        <button type="submit" class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm">"创建"</button>
                        <button type="button" class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm"
                            on:click=move |_| show_modal.set(false)>"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
