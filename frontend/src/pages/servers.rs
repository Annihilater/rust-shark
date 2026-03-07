use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::{layout::Layout, modal::Modal, select::{Select, SelectOption}};
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
struct ServerPayload {
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

fn auth_type_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("key",      "SSH 密钥（推荐）"),
        SelectOption::new("password", "密码"),
    ]
}

#[component]
pub fn ServersPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window().unwrap().location().set_href("/login").ok();
        }
    });

    let servers     = RwSignal::new(Vec::<Server>::new());
    let keys        = RwSignal::new(Vec::<SshKey>::new());
    let show_modal  = RwSignal::new(false);
    let testing_id  = RwSignal::new(Option::<String>::None);
    let test_result = RwSignal::new(Option::<TestResult>::None);

    // None = 新建模式；Some(id) = 编辑模式
    let edit_id = RwSignal::new(Option::<String>::None);

    // 表单字段
    let name           = RwSignal::new(String::new());
    let host           = RwSignal::new(String::new());
    let port           = RwSignal::new("22".to_string());
    let username       = RwSignal::new("root".to_string());
    let auth_type      = RwSignal::new("key".to_string());
    let ssh_key_id_str = RwSignal::new(String::new());
    let password       = RwSignal::new(String::new());
    let error          = RwSignal::new(Option::<String>::None);

    // ── 加载 ──────────────────────────────────────────────────────────────
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

    // ── 重置表单 ──────────────────────────────────────────────────────────
    let reset_form = move || {
        edit_id.set(None);
        name.set(String::new());
        host.set(String::new());
        port.set("22".to_string());
        username.set("root".to_string());
        auth_type.set("key".to_string());
        ssh_key_id_str.set(String::new());
        password.set(String::new());
        error.set(None);
    };

    // ── 打开编辑弹窗，回填表单 ────────────────────────────────────────────
    let open_edit = move |server: Server| {
        edit_id.set(Some(server.id.clone()));
        name.set(server.name.clone());
        host.set(server.host.clone());
        port.set(server.port.to_string());
        username.set(server.username.clone());
        auth_type.set(server.auth_type.clone());
        ssh_key_id_str.set(server.ssh_key_id.clone().unwrap_or_default());
        password.set(String::new()); // 密码不回填，留空表示不修改
        error.set(None);
        show_modal.set(true);
    };

    // ── 提交（新建 or 更新）────────────────────────────────────────────────
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        let key_id = ssh_key_id_str.get();
        let payload = ServerPayload {
            name: name.get(),
            host: host.get(),
            port: port.get().parse().ok(),
            username: username.get(),
            auth_type: auth_type.get(),
            ssh_key_id: if key_id.is_empty() { None } else { Some(key_id) },
            password: if auth_type.get() == "password" {
                let p = password.get();
                if p.is_empty() { None } else { Some(p) }
            } else {
                None
            },
        };

        let id = edit_id.get();
        leptos::task::spawn_local(async move {
            let result = if let Some(ref eid) = id {
                // 编辑：PUT
                crate::api::put::<_, Server>(&format!("/api/servers/{}", eid), &payload).await
            } else {
                // 新建：POST
                crate::api::post::<_, Server>("/api/servers", &payload).await
            };
            match result {
                Ok(_) => {
                    show_modal.set(false);
                    reset_form();
                    load_servers();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // ── 测试连接 ──────────────────────────────────────────────────────────
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

    // ── 删除 ──────────────────────────────────────────────────────────────
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
                        on:click=move |_| { reset_form(); show_modal.set(true); }
                    >"+ 添加服务器"</button>
                </div>

                // 测试结果横幅
                {move || test_result.get().map(|r| {
                    let banner_class = if r.success {
                        "bg-green-900/40 border border-green-700 rounded-xl p-4 mb-4"
                    } else {
                        "bg-red-900/40 border border-red-700 rounded-xl p-4 mb-4"
                    };
                    view! {
                        <div class=banner_class>
                            <div class="flex items-center justify-between">
                                <span class="font-medium">
                                    {if r.success { "✅ 连接成功" } else { "❌ 连接失败" }}
                                </span>
                                <button class="text-gray-400 hover:text-white text-sm"
                                    on:click=move |_| test_result.set(None)>"✕"</button>
                            </div>
                            <p class="text-sm text-gray-300 mt-1">{r.message.clone()}</p>
                            {r.tcpdump_version.map(|v| view! {
                                <p class="text-sm text-green-400 mt-1">"tcpdump: "{v}</p>
                            })}
                        </div>
                    }
                })}

                // 服务器列表
                <div class="space-y-3">
                    {move || servers.get().into_iter().map(|server| {
                        let id_dis    = server.id.clone();
                        let id_label  = server.id.clone();
                        let id_test   = server.id.clone();
                        let id_delete = server.id.clone();
                        let server_edit = server.clone();
                        let status_color = match server.status.as_str() {
                            "online"  => "text-green-400",
                            "offline" => "text-red-400",
                            _         => "text-gray-400",
                        };
                        let status_label = match server.status.as_str() {
                            "online"  => "● 在线",
                            "offline" => "● 离线",
                            _         => "● 未知",
                        };
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4">
                                <div class="flex items-center justify-between">
                                    <div>
                                        <div class="flex items-center gap-2 mb-0.5">
                                            <span class="font-medium">{server.name.clone()}</span>
                                            <span class=format!("text-xs {}", status_color)>
                                                {status_label}
                                            </span>
                                        </div>
                                        <p class="text-sm text-gray-400 font-mono">
                                            {server.username.clone()}"@"{server.host.clone()}":"{server.port}
                                        </p>
                                        <p class="text-xs text-gray-500 mt-0.5">
                                            "认证: "
                                            {if server.auth_type == "key" { "SSH 密钥" } else { "密码" }}
                                        </p>
                                    </div>
                                    <div class="flex items-center gap-2">
                                        <button
                                            class="bg-gray-700 hover:bg-gray-600 text-sm px-3 py-1.5 rounded-lg transition-colors disabled:opacity-50"
                                            disabled=move || testing_id.get() == Some(id_dis.clone())
                                            on:click=move |_| on_test(id_test.clone())
                                        >
                                            {move || if testing_id.get() == Some(id_label.clone()) { "测试中…" } else { "测试连接" }}
                                        </button>
                                        <button
                                            class="text-blue-400 hover:text-blue-300 text-sm px-3 py-1.5 transition-colors"
                                            on:click=move |_| open_edit(server_edit.clone())
                                        >"编辑"</button>
                                        <button
                                            class="text-red-400 hover:text-red-300 text-sm px-3 py-1.5 transition-colors"
                                            on:click=move |_| on_delete(id_delete.clone())
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

            // ── 新建 / 编辑 服务器弹窗（复用同一表单）────────────────────
            <Modal
                show=show_modal.read_only()
                title=Signal::derive(move || if edit_id.get().is_some() { "编辑服务器".to_string() } else { "添加服务器".to_string() })
                on_close=Callback::new(move |_| { show_modal.set(false); reset_form(); })
            >
                <form on:submit=on_submit class="space-y-3">

                    // 名称 + 端口
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"名称"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                placeholder="生产服务器"
                                prop:value=name
                                on:input=move |ev| name.set(event_target_value(&ev))
                                required
                            />
                        </div>
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"端口"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" placeholder="22"
                                prop:value=port
                                on:input=move |ev| port.set(event_target_value(&ev))
                            />
                        </div>
                    </div>

                    // 主机地址
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"主机地址"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            placeholder="192.168.1.100"
                            prop:value=host
                            on:input=move |ev| host.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    // 用户名
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"用户名"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            placeholder="root"
                            prop:value=username
                            on:input=move |ev| username.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    // 认证方式
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"认证方式"</label>
                        <Select
                            options=auth_type_options()
                            value=auth_type.read_only()
                            on_change=Callback::new(move |v| {
                                auth_type.set(v);
                                ssh_key_id_str.set(String::new());
                            })
                        />
                    </div>

                    // 密钥 or 密码
                    {move || if auth_type.get() == "key" {
                        let key_options: Vec<SelectOption> = keys.get()
                            .into_iter()
                            .map(|k| SelectOption::new(k.id, k.name))
                            .collect();
                        view! {
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"选择密钥"</label>
                                <Select
                                    options=key_options
                                    value=ssh_key_id_str.read_only()
                                    on_change=Callback::new(move |v| ssh_key_id_str.set(v))
                                    placeholder="-- 选择密钥 --"
                                />
                            </div>
                        }.into_any()
                    } else {
                        let placeholder = if edit_id.get().is_some() {
                            "留空则不修改密码"
                        } else {
                            "请输入密码"
                        };
                        view! {
                            <div>
                                <label class="block text-xs text-gray-400 mb-1">"密码"</label>
                                <input
                                    class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                    type="password"
                                    placeholder=placeholder
                                    prop:value=password
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                />
                            </div>
                        }.into_any()
                    }}

                    // 错误信息
                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}

                    // 按钮
                    <div class="flex gap-3 pt-1">
                        <button type="submit"
                            class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm transition-colors"
                        >
                            {move || if edit_id.get().is_some() { "保存修改" } else { "添加" }}
                        </button>
                        <button type="button"
                            class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm transition-colors"
                            on:click=move |_| { show_modal.set(false); reset_form(); }
                        >"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
