use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::{layout::Layout, modal::Modal};
use crate::store::use_auth;

#[derive(Deserialize, Clone, Debug)]
struct Server {
    id: String,
    name: String,
    host: String,
    port: i64,
    username: String,
    auth_type: String,
    status: String,
    ssh_key_id: Option<String>,
    created_at: String,
}

#[derive(Deserialize, Clone, Debug)]
struct SshKey {
    id: String,
    name: String,
}

#[derive(Serialize, Default)]
struct CreateServerRequest {
    name: String,
    host: String,
    port: Option<i64>,
    username: String,
    auth_type: String,
    ssh_key_id: Option<String>,
    password: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct TestResult {
    success: bool,
    message: String,
    tcpdump_available: bool,
    tcpdump_version: Option<String>,
    interfaces: Vec<Interface>,
}

#[derive(Deserialize, Clone, Debug)]
struct Interface {
    name: String,
}

#[component]
pub fn ServersPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window().unwrap().location().set_href("/login").ok();
        }
    });

    let servers = RwSignal::new(Vec::<Server>::new());
    let keys = RwSignal::new(Vec::<SshKey>::new());
    let show_modal = RwSignal::new(false);
    let testing_id = RwSignal::new(Option::<String>::None);
    let test_result = RwSignal::new(Option::<TestResult>::None);

    // 表单字段
    let name = RwSignal::new(String::new());
    let host = RwSignal::new(String::new());
    let port = RwSignal::new("22".to_string());
    let username = RwSignal::new("root".to_string());
    let auth_type = RwSignal::new("key".to_string());
    let ssh_key_id = RwSignal::new(Option::<String>::None);
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);

    let load_servers = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<Server>>("/api/servers").await {
                servers.set(list);
            }
        });
    };

    let load_keys = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<SshKey>>("/api/keys").await {
                keys.set(list);
            }
        });
    };

    load_servers();
    load_keys();

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = CreateServerRequest {
            name: name.get(),
            host: host.get(),
            port: port.get().parse().ok(),
            username: username.get(),
            auth_type: auth_type.get(),
            ssh_key_id: ssh_key_id.get(),
            password: if auth_type.get() == "password" { Some(password.get()) } else { None },
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, Server>("/api/servers", &req).await {
                Ok(_) => {
                    show_modal.set(false);
                    load_servers();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_test = move |id: String| {
        testing_id.set(Some(id.clone()));
        test_result.set(None);
        leptos::task::spawn_local(async move {
            let path = format!("/api/servers/{}/test", id);
            match crate::api::post::<_, TestResult>(&path, &serde_json::json!({})).await {
                Ok(result) => {
                    test_result.set(Some(result));
                    testing_id.set(None);
                    load_servers();
                }
                Err(_) => testing_id.set(None),
            }
        });
    };

    let on_delete = move |id: String| {
        leptos::task::spawn_local(async move {
            let path = format!("/api/servers/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                load_servers();
            }
        });
    };

    view! {
        <Layout>
            <div class="max-w-5xl mx-auto">
                <div class="flex items-center justify-between mb-6">
                    <h1 class="text-2xl font-bold">"服务器管理"</h1>
                    <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                        on:click=move |_| show_modal.set(true)
                    >"+ 添加服务器"</button>
                </div>

                // 测试结果展示
                {move || test_result.get().map(|r| view! {
                    <div class=if r.success { "bg-green-900/40 border border-green-700 rounded-xl p-4 mb-4" } else { "bg-red-900/40 border border-red-700 rounded-xl p-4 mb-4" }>
                        <div class="flex items-center gap-2 font-medium">
                            {if r.success { "✅ 连接成功" } else { "❌ 连接失败" }}
                        </div>
                        <p class="text-sm text-gray-300 mt-1">{r.message}</p>
                        {r.tcpdump_version.map(|v| view! {
                            <p class="text-sm text-green-400 mt-1">"tcpdump: "{v}</p>
                        })}
                    </div>
                })}

                <div class="space-y-3">
                    {move || servers.get().into_iter().map(|server| {
                        let id = server.id.clone();
                        let id2 = server.id.clone();
                        let is_testing = testing_id.get() == Some(id.clone());
                        let status_color = match server.status.as_str() {
                            "online" => "text-green-400",
                            "offline" => "text-red-400",
                            _ => "text-gray-400",
                        };
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4">
                                <div class="flex items-center justify-between">
                                    <div>
                                        <div class="flex items-center gap-2">
                                            <span class="font-medium">{server.name}</span>
                                            <span class=format!("text-xs {}", status_color)>
                                                {match server.status.as_str() {
                                                    "online" => "● 在线",
                                                    "offline" => "● 离线",
                                                    _ => "● 未知",
                                                }}
                                            </span>
                                        </div>
                                        <p class="text-sm text-gray-400 mt-0.5 font-mono">
                                            {server.username}"@"{server.host}":"{server.port}
                                        </p>
                                        <p class="text-xs text-gray-500 mt-0.5">
                                            "认证: "
                                            {if server.auth_type == "key" { "SSH密钥" } else { "密码" }}
                                        </p>
                                    </div>
                                    <div class="flex gap-2">
                                        <button
                                            class="bg-gray-700 hover:bg-gray-600 text-sm px-3 py-1.5 rounded-lg transition-colors disabled:opacity-50"
                                            disabled=move || is_testing
                                            on:click=move |_| on_test(id.clone())
                                        >
                                            {if is_testing { "测试中..." } else { "测试连接" }}
                                        </button>
                                        <button
                                            class="text-red-400 hover:text-red-300 text-sm px-3 py-1.5"
                                            on:click=move |_| on_delete(id2.clone())
                                        >"删除"</button>
                                    </div>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}

                    {move || servers.get().is_empty().then(|| view! {
                        <div class="text-center py-12 text-gray-500">
                            <div class="text-4xl mb-3">"🖥️"</div>
                            <p>"还没有服务器，点击右上角添加"</p>
                        </div>
                    })}
                </div>
            </div>

            // 添加服务器弹窗
            <Modal
                show=show_modal.read_only()
                title="添加服务器"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                <form on:submit=on_create class="space-y-3">
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"名称"</label>
                            <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                placeholder="生产服务器" prop:value=name
                                on:input=move |ev| name.set(event_target_value(&ev)) required/>
                        </div>
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"端口"</label>
                            <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" placeholder="22" prop:value=port
                                on:input=move |ev| port.set(event_target_value(&ev))/>
                        </div>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"主机地址"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            placeholder="192.168.1.100" prop:value=host
                            on:input=move |ev| host.set(event_target_value(&ev)) required/>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"用户名"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            placeholder="root" prop:value=username
                            on:input=move |ev| username.set(event_target_value(&ev)) required/>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"认证方式"</label>
                        <select class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            on:change=move |ev| auth_type.set(event_target_value(&ev))>
                            <option value="key">"SSH 密钥（推荐）"</option>
                            <option value="password">"密码"</option>
                        </select>
                    </div>

                    {move || if auth_type.get() == "key" {
                        let ks = keys.get();
                        view! {
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"选择密钥"</label>
                                <select class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                    on:change=move |ev| {
                                        let v = event_target_value(&ev);
                                        ssh_key_id.set(if v.is_empty() { None } else { Some(v) });
                                    }>
                                    <option value="">"-- 选择密钥 --"</option>
                                    {ks.into_iter().map(|k| {
                                        let kid = k.id.clone();
                                        view! { <option value=kid>{k.name}</option> }
                                    }).collect::<Vec<_>>()}
                                </select>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"密码"</label>
                                <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                    type="password" prop:value=password
                                    on:input=move |ev| password.set(event_target_value(&ev))/>
                            </div>
                        }.into_any()
                    }}

                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}
                    <div class="flex gap-3 pt-1">
                        <button type="submit" class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm">"添加"</button>
                        <button type="button" class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm"
                            on:click=move |_| show_modal.set(false)>"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
