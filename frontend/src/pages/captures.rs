use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use crate::components::{layout::Layout, modal::Modal, select::{Select, SelectOption}};
use crate::store::use_auth;

#[derive(Deserialize, Clone, Debug)]
struct CaptureTask {
    id: String,
    server_id: String,
    interface: String,
    filter: Option<String>,
    duration: Option<i64>,
    packet_limit: Option<i64>,
    status: String,
    file_size: Option<i64>,
    created_at: String,
    finished_at: Option<String>,
    log_msg: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct Server {
    id: String,
    name: String,
    host: String,
}

#[derive(Deserialize, Clone, Debug)]
struct Interface {
    name: String,
}

#[derive(Serialize, Default)]
struct CreateCaptureRequest {
    server_id: String,
    interface: String,
    filter: Option<String>,
    duration: Option<i64>,
    packet_limit: Option<i64>,
    scheduled_at: Option<String>,
    repeat_type: Option<String>,
    repeat_until: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct CaptureLogResponse {
    log: String,
}

#[component]
pub fn CapturesPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window().unwrap().location().set_href("/login").ok();
        }
    });

    let tasks          = RwSignal::new(Vec::<CaptureTask>::new());
    let servers        = RwSignal::new(Vec::<Server>::new());
    let interfaces     = RwSignal::new(Vec::<Interface>::new());
    let ports          = RwSignal::new(Vec::<u16>::new());
    let show_modal     = RwSignal::new(false);
    let error          = RwSignal::new(Option::<String>::None);
    let loading_ifaces = RwSignal::new(false);
    let loading_ports  = RwSignal::new(false);
    let stopping_id    = RwSignal::new(Option::<String>::None);

    // 日志展开状态
    let expanded_log_id = RwSignal::new(Option::<String>::None);
    let task_logs       = RwSignal::new(HashMap::<String, String>::new());

    // 表单字段（带默认值）
    let server_id     = RwSignal::new(String::new());
    let iface         = RwSignal::new(String::new());
    let filter        = RwSignal::new(String::new());
    let duration      = RwSignal::new("60".to_string());      // 默认 60 秒
    let packet_limit  = RwSignal::new("1000".to_string());    // 默认 1000 包
    let scheduled_at  = RwSignal::new(String::new());
    // 选中的端口（勾选框）
    let selected_ports = RwSignal::new(Vec::<u16>::new());

    // ── 加载 ──────────────────────────────────────────────────────────────
    let load_tasks = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<CaptureTask>>("/api/captures").await {
                tasks.set(list);
            }
        });
    };

    let load_servers = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<Server>>("/api/servers").await {
                servers.set(list);
            }
        });
    };

    load_tasks();
    load_servers();

    // 定时刷新：有 running 任务时每 3 秒刷新一次
    Effect::new(move |_| {
        let has_running = tasks.get().iter().any(|t| t.status == "running");
        if has_running {
            leptos::task::spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(3_000).await;
                load_tasks();
            });
        }
    });

    // 轮询日志：expanded_log_id 设置时，每 2 秒拉一次日志
    Effect::new(move |_| {
        let maybe_id = expanded_log_id.get();
        if let Some(id) = maybe_id {
            // 检查任务是否仍在运行
            let is_running = tasks.get().iter().any(|t| t.id == id && t.status == "running");
            if is_running {
                let id2 = id.clone();
                leptos::task::spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(2_000).await;
                    let path = format!("/api/captures/{}/log", id2);
                    if let Ok(resp) = crate::api::get::<CaptureLogResponse>(&path).await {
                        task_logs.update(|m| { m.insert(id2.clone(), resp.log); });
                    }
                    // 触发重新求值（通过读取 expanded_log_id 的写入）
                    if expanded_log_id.get_untracked() == Some(id2) {
                        // force re-trigger by doing a no-op write if still expanded
                        expanded_log_id.set(expanded_log_id.get_untracked());
                    }
                });
            }
        }
    });

    // 服务器选择后加载网卡 + 端口
    Effect::new(move |_| {
        let sid = server_id.get();
        if sid.is_empty() {
            interfaces.set(vec![]);
            ports.set(vec![]);
            iface.set(String::new());
            selected_ports.set(vec![]);
            return;
        }
        loading_ifaces.set(true);
        loading_ports.set(true);
        iface.set(String::new());
        selected_ports.set(vec![]);

        let sid2 = sid.clone();
        leptos::task::spawn_local(async move {
            // 并行加载网卡和端口
            let ifaces_path = format!("/api/servers/{}/interfaces", sid);
            let ports_path  = format!("/api/servers/{}/ports", sid2);

            let (iface_res, ports_res) = futures_join(
                crate::api::get::<Vec<Interface>>(&ifaces_path),
                crate::api::get::<Vec<u16>>(&ports_path),
            ).await;

            if let Ok(list) = iface_res {
                let first = list.first().map(|i| i.name.clone()).unwrap_or_default();
                interfaces.set(list);
                iface.set(first);
            }
            loading_ifaces.set(false);

            if let Ok(list) = ports_res {
                ports.set(list);
            }
            loading_ports.set(false);
        });
    });

    // ── 下载文件（带 token，用 gloo_net fetch + blob URL） ──────────────
    let on_download = move |id: String| {
        leptos::task::spawn_local(async move {
            let url = format!("/api/captures/{}/download", id);
            let token = crate::api::auth_header();
            let resp = gloo_net::http::Request::get(&url)
                .header("Authorization", &token)
                .send()
                .await;
            match resp {
                Ok(r) if r.ok() => {
                    if let Ok(bytes) = r.binary().await {
                        // 构造 Blob 并触发下载
                        let uint8 = js_sys::Uint8Array::from(bytes.as_slice());
                        let array = js_sys::Array::new();
                        array.push(&uint8.buffer());
                        let blob = web_sys::Blob::new_with_u8_array_sequence(&array).unwrap();
                        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
                        let doc = web_sys::window().unwrap().document().unwrap();
                        let a = doc.create_element("a").unwrap();
                        a.set_attribute("href", &url).ok();
                        a.set_attribute("download", &format!("capture-{}.pcap", id)).ok();
                        let body = doc.body().unwrap();
                        body.append_child(&a).ok();
                        a.unchecked_ref::<web_sys::HtmlElement>().click();
                        body.remove_child(&a).ok();
                        web_sys::Url::revoke_object_url(&url).ok();
                    }
                }
                _ => {}
            }
        });
    };
    let on_stop = move |id: String| {
        stopping_id.set(Some(id.clone()));
        leptos::task::spawn_local(async move {
            let path = format!("/api/captures/{}/stop", id);
            if crate::api::post::<_, serde_json::Value>(&path, &serde_json::json!({})).await.is_ok() {
                load_tasks();
            }
            stopping_id.set(None);
        });
    };

    // ── 删除任务 ─────────────────────────────────────────────────────────
    let on_delete = move |id: String| {
        leptos::task::spawn_local(async move {
            let path = format!("/api/captures/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                load_tasks();
            }
        });
    };

    // ── 提交新建 ─────────────────────────────────────────────────────────
    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        // 把勾选的端口追加到 BPF filter
        let port_filter = {
            let ps = selected_ports.get();
            if ps.is_empty() {
                String::new()
            } else {
                let parts: Vec<String> = ps.iter().map(|p| format!("port {}", p)).collect();
                format!("({})", parts.join(" or "))
            }
        };
        let user_filter = filter.get();
        let combined_filter = match (user_filter.is_empty(), port_filter.is_empty()) {
            (true,  true)  => None,
            (true,  false) => Some(port_filter),
            (false, true)  => Some(user_filter),
            (false, false) => Some(format!("({}) and {}", user_filter, port_filter)),
        };

        let req = CreateCaptureRequest {
            server_id: server_id.get(),
            interface: iface.get(),
            filter: combined_filter,
            duration: duration.get().parse().ok(),
            packet_limit: packet_limit.get().parse().ok(),
            scheduled_at: { let s = scheduled_at.get(); if s.is_empty() { None } else { Some(s) } },
            repeat_type: None,
            repeat_until: None,
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, CaptureTask>("/api/captures", &req).await {
                Ok(_) => {
                    show_modal.set(false);
                    // 重置表单
                    server_id.set(String::new());
                    iface.set(String::new());
                    filter.set(String::new());
                    duration.set("60".to_string());
                    packet_limit.set("1000".to_string());
                    scheduled_at.set(String::new());
                    selected_ports.set(vec![]);
                    load_tasks();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let format_size = |bytes: i64| -> String {
        if bytes < 1024 { format!("{} B", bytes) }
        else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
        else { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    };

    view! {
        <Layout>
            <div class="max-w-5xl mx-auto">
                <div class="flex items-center justify-between mb-6">
                    <h1 class="text-2xl font-bold">"抓包任务"</h1>
                    <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                        on:click=move |_| {
                            error.set(None);
                            show_modal.set(true);
                        }
                    >"+ 新建抓包"</button>
                </div>

                // 任务列表
                <div class="space-y-3">
                    {move || tasks.get().into_iter().map(|task| {
                        let id_stop = task.id.clone();
                        let id_dl   = task.id.clone();
                        let id_ana  = task.id.clone();
                        let id_del  = task.id.clone();
                        let id_log  = task.id.clone();
                        let id_log2 = task.id.clone();
                        let is_running  = task.status == "running";
                        let is_done     = task.status == "done";
                        let status_class = match task.status.as_str() {
                            "done"      => "text-green-400",
                            "running"   => "text-blue-400",
                            "failed"    => "text-red-400",
                            "cancelled" => "text-gray-400",
                            _           => "text-yellow-400",
                        };
                        let status_label = match task.status.as_str() {
                            "done"      => "✓ 完成",
                            "running"   => "⟳ 运行中",
                            "failed"    => "✗ 失败",
                            "cancelled" => "⊘ 已取消",
                            _           => "○ 等待",
                        };
                        // 初始化日志（从 log_msg 字段或已缓存的日志）
                        {
                            let init_log = task.log_msg.clone().unwrap_or_default();
                            let tid = task.id.clone();
                            if !init_log.is_empty() {
                                task_logs.update(|m| { m.entry(tid).or_insert(init_log); });
                            }
                        }
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4">
                                <div class="flex items-start justify-between">
                                    <div class="flex-1 min-w-0">
                                        <div class="flex items-center gap-3 flex-wrap">
                                            <span class=format!("text-sm font-medium {}", status_class)>{status_label}</span>
                                            <span class="font-mono text-sm text-gray-300">{task.interface.clone()}</span>
                                            {task.filter.clone().map(|f| view! {
                                                <span class="text-xs bg-gray-700 px-2 py-0.5 rounded text-gray-300 font-mono max-w-xs truncate">{f}</span>
                                            })}
                                            {task.duration.map(|d| view! {
                                                <span class="text-xs text-gray-500">{d}"s"</span>
                                            })}
                                            {task.packet_limit.map(|p| view! {
                                                <span class="text-xs text-gray-500">"max "{p}" 包"</span>
                                            })}
                                        </div>
                                        <p class="text-xs text-gray-500 mt-1">"创建: "{task.created_at.clone()}</p>
                                        {task.finished_at.map(|t| view! {
                                            <p class="text-xs text-gray-500 mt-0.5">"结束: "{t}</p>
                                        })}
                                        {task.file_size.map(|s| view! {
                                            <p class="text-xs text-gray-400 mt-0.5">"大小: "{format_size(s)}</p>
                                        })}
                                        // 日志展开面板
                                        {move || {
                                            let log_id = id_log2.clone();
                                            if expanded_log_id.get() == Some(log_id.clone()) {
                                                let log_content = task_logs.get()
                                                    .get(&log_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| "加载中...".to_string());
                                                view! {
                                                    <pre class="mt-2 bg-gray-900 border border-gray-700 rounded p-2 text-xs font-mono text-green-400 max-h-32 overflow-auto whitespace-pre-wrap">
                                                        {log_content}
                                                    </pre>
                                                }.into_any()
                                            } else {
                                                view! { <div/> }.into_any()
                                            }
                                        }}
                                    </div>
                                    <div class="flex items-center gap-2 shrink-0 ml-3">
                                        // 运行中 → 停止按钮 + 查看日志按钮
                                        {if is_running {
                                            let id_s  = id_stop.clone();
                                            let id_s2 = id_stop.clone();
                                            let id_lg = id_log.clone();
                                            view! {
                                                <div class="flex gap-2">
                                                    <button
                                                        class="bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                        on:click=move |_| {
                                                            let tid = id_lg.clone();
                                                            if expanded_log_id.get_untracked() == Some(tid.clone()) {
                                                                expanded_log_id.set(None);
                                                            } else {
                                                                // 立即拉取一次日志
                                                                let tid2 = tid.clone();
                                                                leptos::task::spawn_local(async move {
                                                                    let path = format!("/api/captures/{}/log", tid2);
                                                                    if let Ok(resp) = crate::api::get::<CaptureLogResponse>(&path).await {
                                                                        task_logs.update(|m| { m.insert(tid2.clone(), resp.log); });
                                                                    }
                                                                });
                                                                expanded_log_id.set(Some(tid));
                                                            }
                                                        }
                                                    >
                                                        {move || if expanded_log_id.get() == Some(id_log.clone()) { "隐藏日志" } else { "查看日志" }}
                                                    </button>
                                                    <button
                                                        class="bg-yellow-600 hover:bg-yellow-500 text-white text-sm px-3 py-1.5 rounded-lg transition-colors disabled:opacity-50"
                                                        disabled=move || stopping_id.get() == Some(id_s.clone())
                                                        on:click=move |_| on_stop(id_s2.clone())
                                                    >
                                                        {move || if stopping_id.get() == Some(id_stop.clone()) { "停止中…" } else { "⏹ 停止" }}
                                                    </button>
                                                </div>
                                            }.into_any()
                                        } else if is_done {
                                            let id_dl2 = id_dl.clone();
                                            view! {
                                                <div class="flex gap-2">
                                                    <button
                                                        class="bg-gray-700 hover:bg-gray-600 text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                        on:click=move |_| on_download(id_dl.clone())
                                                    >"下载"</button>
                                                    <a
                                                        href=format!("/captures/{}", id_ana)
                                                        class="bg-blue-700 hover:bg-blue-600 text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                    >"分析"</a>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div/> }.into_any()
                                        }}
                                        <button
                                            class="text-red-400 hover:text-red-300 text-sm px-3 py-1.5 transition-colors"
                                            on:click=move |_| on_delete(id_del.clone())
                                        >"删除"</button>
                                    </div>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}

                    {move || tasks.get().is_empty().then(|| view! {
                        <div class="text-center py-12 text-gray-500">
                            <div class="text-4xl mb-3">"📦"</div>
                            <p>"还没有抓包任务"</p>
                        </div>
                    })}
                </div>
            </div>

            // ── 新建抓包弹窗 ─────────────────────────────────────────────
            <Modal
                show=show_modal.read_only()
                title="新建抓包任务"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                <form on:submit=on_create class="space-y-3">

                    // 选择服务器
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"选择服务器"</label>
                        <Select
                            options=Signal::derive(move || {
                                servers.get().into_iter()
                                    .map(|s| SelectOption::new(s.id, format!("{} ({})", s.name, s.host)))
                                    .collect::<Vec<_>>()
                            }).get()
                            value=server_id.read_only()
                            on_change=Callback::new(move |v| server_id.set(v))
                            placeholder="-- 选择服务器 --"
                        />
                    </div>

                    // 网卡
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">
                            "网卡"
                            {move || loading_ifaces.get().then(|| view! {
                                <span class="text-gray-500 ml-1">"加载中…"</span>
                            })}
                        </label>
                        <Select
                            options=Signal::derive(move || {
                                interfaces.get().into_iter()
                                    .map(|i| SelectOption::new(i.name.clone(), i.name))
                                    .collect::<Vec<_>>()
                            }).get()
                            value=iface.read_only()
                            on_change=Callback::new(move |v| iface.set(v))
                            placeholder="-- 先选择服务器 --"
                            disabled=Signal::derive(move || server_id.get().is_empty() || loading_ifaces.get())
                        />
                    </div>

                    // 端口勾选（服务器上正在监听的端口）
                    {move || {
                        let ps = ports.get();
                        if ps.is_empty() {
                            // 还没加载或没选服务器，不显示
                            if loading_ports.get() {
                                view! {
                                    <div class="text-xs text-gray-500">"正在检测监听端口…"</div>
                                }.into_any()
                            } else {
                                view! { <div/> }.into_any()
                            }
                        } else {
                            view! {
                                <div>
                                    <label class="block text-xs text-gray-400 mb-1">
                                        "监听端口过滤（可多选，空=不过滤）"
                                    </label>
                                    <div class="bg-gray-700 border border-gray-600 rounded-lg p-2 max-h-32 overflow-y-auto">
                                        <div class="flex flex-wrap gap-2">
                                            {ps.into_iter().map(|p| {
                                                let p2 = p;
                                                view! {
                                                    <label class="flex items-center gap-1 cursor-pointer select-none">
                                                        <input
                                                            type="checkbox"
                                                            class="accent-blue-500"
                                                            prop:checked=move || selected_ports.get().contains(&p2)
                                                            on:change=move |ev| {
                                                                let checked = event_target_checked(&ev);
                                                                selected_ports.update(|v| {
                                                                    if checked { if !v.contains(&p2) { v.push(p2); v.sort_unstable(); } }
                                                                    else { v.retain(|&x| x != p2); }
                                                                });
                                                            }
                                                        />
                                                        <span class="text-xs font-mono text-gray-200">{p}</span>
                                                    </label>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }}

                    // BPF 过滤器
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"BPF 过滤器（可选，与端口选择叠加）"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500"
                            placeholder="tcp and host 1.2.3.4"
                            prop:value=filter
                            on:input=move |ev| filter.set(event_target_value(&ev))
                        />
                    </div>

                    // 时长 + 包数（默认值已填好）
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"时长（秒，0=不限）"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" min="0" placeholder="60"
                                prop:value=duration
                                on:input=move |ev| duration.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"包数限制（0=不限）"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" min="0" placeholder="1000"
                                prop:value=packet_limit
                                on:input=move |ev| packet_limit.set(event_target_value(&ev))
                            />
                        </div>
                    </div>

                    // 定时执行
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"定时执行（空=立即）"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            type="datetime-local"
                            prop:value=scheduled_at
                            on:input=move |ev| scheduled_at.set(event_target_value(&ev))
                        />
                    </div>

                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}

                    <div class="flex gap-3 pt-1">
                        <button type="submit"
                            class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm transition-colors"
                        >"开始抓包"</button>
                        <button type="button"
                            class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm transition-colors"
                            on:click=move |_| show_modal.set(false)
                        >"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}

// 简化的并行 future join（避免引入 futures crate）
async fn futures_join<A, B>(a: impl std::future::Future<Output = A>, b: impl std::future::Future<Output = B>) -> (A, B) {
    // 顺序执行（WASM 单线程，无需真并行）
    let ra = a.await;
    let rb = b.await;
    (ra, rb)
}
