use crate::components::{
    layout::Layout,
    modal::Modal,
    select::{Select, SelectOption},
};
use crate::store::use_auth;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsCast;

#[derive(Deserialize, Clone, Debug)]
struct CaptureTask {
    id: String,
    server_id: String,
    interface: String,
    filter: Option<String>,
    duration: Option<i64>,
    #[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Deserialize, Clone, Debug)]
struct LivePacket {
    number: u64,
    time: String,
    source: String,
    destination: String,
    protocol: String,
    length: u64,
    info: String,
}

#[component]
pub fn CapturesPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window()
                .unwrap()
                .location()
                .set_href("/login")
                .ok();
        }
    });

    let page = RwSignal::new(1usize);
    const PAGE_SIZE: usize = 10;
    let tasks = RwSignal::new(Vec::<CaptureTask>::new());
    let servers = RwSignal::new(Vec::<Server>::new());
    let interfaces = RwSignal::new(Vec::<Interface>::new());
    let ports = RwSignal::new(Vec::<u16>::new());
    let show_modal = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let loading_ifaces = RwSignal::new(false);
    let loading_ports = RwSignal::new(false);
    let stopping_id = RwSignal::new(Option::<String>::None);

    // 日志展开状态
    let expanded_log_id = RwSignal::new(Option::<String>::None);
    let task_logs = RwSignal::new(HashMap::<String, String>::new());
    // 实时抓包预览（仅 running 任务）
    let live_packets = RwSignal::new(HashMap::<String, Vec<LivePacket>>::new());

    // 表单字段（带默认值）
    let server_id = RwSignal::new(String::new());
    let iface = RwSignal::new(String::new());
    let filter = RwSignal::new(String::new());
    let duration = RwSignal::new("60".to_string());
    let packet_limit = RwSignal::new("1000".to_string());
    let scheduled_at = RwSignal::new(String::new());
    let selected_ports = RwSignal::new(Vec::<u16>::new());

    // ── 加载 ──────────────────────────────────────────────────────────────
    let load_tasks = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<CaptureTask>>("/api/captures").await {
                tasks.set(list);
                page.set(1);
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

    // 定时刷新：有 pending 或 running 任务时每 3 秒刷新一次
    Effect::new(move |_| {
        let has_active = tasks
            .get()
            .iter()
            .any(|t| t.status == "running" || t.status == "pending");
        if has_active {
            leptos::task::spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(3_000).await;
                load_tasks();
            });
        }
    });

    // 轮询日志：expanded_log_id 设置时，每 2 秒拉一次
    Effect::new(move |_| {
        let maybe_id = expanded_log_id.get();
        if let Some(id) = maybe_id {
            let should_poll = tasks
                .get()
                .iter()
                .any(|t| t.id == id && (t.status == "running" || t.status == "pending"));
            if should_poll {
                let id2 = id.clone();
                let id3 = id.clone();
                leptos::task::spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(2_000).await;
                    let log_path = format!("/api/captures/{}/log", id2);
                    let packets_path = format!("/api/captures/{}/packets?limit=50", id3);

                    let (log_res, pkts_res) = futures_join(
                        crate::api::get::<CaptureLogResponse>(&log_path),
                        crate::api::get::<Vec<LivePacket>>(&packets_path),
                    )
                    .await;

                    if let Ok(resp) = log_res {
                        task_logs.update(|m| {
                            m.insert(id2.clone(), resp.log);
                        });
                    }
                    if let Ok(pkts) = pkts_res {
                        live_packets.update(|m| {
                            m.insert(id2.clone(), pkts);
                        });
                    }

                    // 自动滚动日志到底部
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            if let Some(el) = doc.get_element_by_id(&format!("log-{}", id2)) {
                                let scroll_height = el.scroll_height();
                                el.set_scroll_top(scroll_height);
                            }
                        }
                    }

                    // 触发重新求值
                    if expanded_log_id.get_untracked() == Some(id2.clone()) {
                        expanded_log_id.set(expanded_log_id.get_untracked());
                    }
                });
            }
        }
    });

    // ESC 键关闭日志侧边栏
    Effect::new(move |_| {
        use wasm_bindgen::prelude::*;
        let closure =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    expanded_log_id.set(None);
                }
            });
        if let Some(win) = web_sys::window() {
            win.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
                .ok();
        }
        closure.forget();
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
            let ifaces_path = format!("/api/servers/{}/interfaces", sid);
            let ports_path = format!("/api/servers/{}/ports", sid2);

            let (iface_res, ports_res) = futures_join(
                crate::api::get::<Vec<Interface>>(&ifaces_path),
                crate::api::get::<Vec<u16>>(&ports_path),
            )
            .await;

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

    // ── 下载 ──────────────────────────────────────────────────────────────
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
                        let uint8 = js_sys::Uint8Array::from(bytes.as_slice());
                        let array = js_sys::Array::new();
                        array.push(&uint8.buffer());
                        let blob = web_sys::Blob::new_with_u8_array_sequence(&array).unwrap();
                        let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
                        let doc = web_sys::window().unwrap().document().unwrap();
                        let a = doc.create_element("a").unwrap();
                        a.set_attribute("href", &url).ok();
                        a.set_attribute("download", &format!("capture-{}.pcap", id))
                            .ok();
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

    // ── 停止 ──────────────────────────────────────────────────────────────
    let on_stop = move |id: String| {
        stopping_id.set(Some(id.clone()));
        leptos::task::spawn_local(async move {
            let path = format!("/api/captures/{}/stop", id);
            if crate::api::post::<_, serde_json::Value>(&path, &serde_json::json!({}))
                .await
                .is_ok()
            {
                load_tasks();
            }
            stopping_id.set(None);
        });
    };

    // ── 删除 ──────────────────────────────────────────────────────────────
    let on_delete = move |id: String| {
        leptos::task::spawn_local(async move {
            let path = format!("/api/captures/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                load_tasks();
            }
        });
    };

    // ── 重试（复用同一任务的配置重新创建） ────────────────────────────────
    let on_retry = move |task: CaptureTask| {
        leptos::task::spawn_local(async move {
            let req = CreateCaptureRequest {
                server_id: task.server_id.clone(),
                interface: task.interface.clone(),
                filter: task.filter.clone(),
                duration: task.duration,
                packet_limit: task.packet_limit,
                scheduled_at: None,
                repeat_type: None,
                repeat_until: None,
            };
            if crate::api::post::<_, CaptureTask>("/api/captures", &req)
                .await
                .is_ok()
            {
                load_tasks();
            }
        });
    };

    // ── 展开日志（点击任务行） ────────────────────────────────────────────
    let open_log = move |task: &CaptureTask| {
        let id = task.id.clone();
        // 初始化日志缓存
        if let Some(init_log) = &task.log_msg {
            if !init_log.is_empty() {
                task_logs.update(|m| {
                    m.entry(id.clone()).or_insert_with(|| init_log.clone());
                });
            }
        }
        let id2 = id.clone();
        let id3 = id.clone();
        leptos::task::spawn_local(async move {
            let log_path = format!("/api/captures/{}/log", id2);
            let pkts_path = format!("/api/captures/{}/packets?limit=50", id3);
            let (log_res, pkts_res) = futures_join(
                crate::api::get::<CaptureLogResponse>(&log_path),
                crate::api::get::<Vec<LivePacket>>(&pkts_path),
            )
            .await;
            if let Ok(resp) = log_res {
                task_logs.update(|m| {
                    m.insert(id2.clone(), resp.log);
                });
            }
            if let Ok(pkts) = pkts_res {
                live_packets.update(|m| {
                    m.insert(id2.clone(), pkts);
                });
            }
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(el) = doc.get_element_by_id(&format!("log-{}", id2)) {
                        el.set_scroll_top(el.scroll_height());
                    }
                }
            }
        });
        expanded_log_id.set(Some(id));
    };

    // ── 提交新建 ─────────────────────────────────────────────────────────
    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);

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
            (true, true) => None,
            (true, false) => Some(port_filter),
            (false, true) => Some(user_filter),
            (false, false) => Some(format!("({}) and {}", user_filter, port_filter)),
        };

        let req = CreateCaptureRequest {
            server_id: server_id.get(),
            interface: iface.get(),
            filter: combined_filter,
            duration: duration.get().parse().ok(),
            packet_limit: packet_limit.get().parse().ok(),
            scheduled_at: {
                let s = scheduled_at.get();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            },
            repeat_type: None,
            repeat_until: None,
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, CaptureTask>("/api/captures", &req).await {
                Ok(_) => {
                    show_modal.set(false);
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
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        }
    };

    view! {
        <Layout>
            // ── 整体布局：左主列表（固定55%）+ 右侧日志（固定44%），始终两栏 ──
            <div class="flex gap-4" style="min-height: calc(100vh - 80px);">

                // ── 左：任务列表（固定宽度55%） ──────────────────────────────
                <div class="w-[55%] min-w-0 shrink-0">
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

                    <div class="space-y-2">
                        {move || {
                            let all = tasks.get();
                            let start = (page.get() - 1) * PAGE_SIZE;
                            all.into_iter().skip(start).take(PAGE_SIZE).map(|task| {
                            let id_stop  = task.id.clone();
                            let id_dl    = task.id.clone();
                            let id_ana   = task.id.clone();
                            let id_del   = task.id.clone();
                            let id_sel   = task.id.clone(); // 点击行展开
                            let id_cls   = task.id.clone(); // class 响应
                            let task_retry = task.clone();
                            let task_open  = task.clone();

                            let is_running   = task.status == "running";
                            let is_pending   = task.status == "pending";
                            let is_done      = task.status == "done";
                            let is_failed    = task.status == "failed";
                            let is_cancelled = task.status == "cancelled";
                            let is_active    = is_running || is_pending;
                            // done/failed/cancelled 都可以重试
                            let can_retry    = is_done || is_failed || is_cancelled;

                            let status_class = match task.status.as_str() {
                                "done"      => "text-green-400",
                                "running"   => "text-blue-400",
                                "failed"    => "text-red-400",
                                "cancelled" => "text-gray-400",
                                _           => "text-yellow-400",
                            };
                            let status_label = match task.status.as_str() {
                                "done"      => "✓ 完成",
                                "running"   => "运行中",
                                "failed"    => "✗ 失败",
                                "cancelled" => "⊘ 已取消",
                                _           => "○ 等待",
                            };

                            // 配置摘要 tag
                            let dur_tag = task.duration.map(|d| if d == 0 { "不限时".to_string() } else { format!("{}s", d) });
                            let pkt_tag = task.packet_limit.map(|p| if p == 0 { "不限包".to_string() } else { format!("{}包", p) });

                            // 初始化日志缓存
                            {
                                let init_log = task.log_msg.clone().unwrap_or_default();
                                let tid = task.id.clone();
                                if !init_log.is_empty() {
                                    task_logs.update(|m| { m.entry(tid).or_insert(init_log); });
                                }
                            }

                            view! {
                                // 点击整个卡片展开日志
                                <div
                                    class=move || format!(
                                        "bg-gray-800 border rounded-xl p-3 transition-all cursor-pointer select-none {}",
                                        if expanded_log_id.get() == Some(id_cls.clone()) {
                                            "border-blue-500/70 shadow-lg shadow-blue-900/20"
                                        } else if is_running {
                                            "border-blue-700/40 hover:border-blue-600/60"
                                        } else {
                                            "border-gray-700 hover:border-gray-500"
                                        }
                                    )
                                    on:click=move |ev| {
                                        // 不拦截按钮的点击事件
                                        let target = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok());
                                        let is_btn = target.as_ref().map(|el| {
                                            el.closest("button").ok().flatten().is_some()
                                            || el.closest("a").ok().flatten().is_some()
                                        }).unwrap_or(false);
                                        if !is_btn {
                                            if expanded_log_id.get_untracked() == Some(id_sel.clone()) {
                                                expanded_log_id.set(None);
                                            } else {
                                                open_log(&task_open);
                                            }
                                        }
                                    }
                                >
                                    // ── 第一行：状态 + 接口 + 配置标签 + 操作按钮 ──
                                    <div class="flex items-center gap-2 flex-wrap">
                                        // 状态指示
                                        <div class="shrink-0">
                                            {if is_running {
                                                view! {
                                                    <span class="relative flex h-2.5 w-2.5">
                                                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
                                                        <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-blue-500"></span>
                                                    </span>
                                                }.into_any()
                                            } else if is_pending {
                                                view! {
                                                    <span class="relative flex h-2.5 w-2.5">
                                                        <span class="animate-pulse absolute inline-flex h-full w-full rounded-full bg-yellow-400 opacity-75"></span>
                                                        <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-yellow-500"></span>
                                                    </span>
                                                }.into_any()
                                            } else {
                                                view! { <span class=format!("text-xs font-bold {}", status_class)>"●"</span> }.into_any()
                                            }}
                                        </div>

                                        // 状态标签
                                        <span class=format!("text-xs font-semibold shrink-0 {}", status_class)>
                                            {status_label}
                                        </span>

                                        // 接口
                                        <span class="font-mono text-sm text-gray-200 shrink-0">{task.interface.clone()}</span>

                                        // 配置标签
                                        {task.filter.clone().map(|f| {
                                            let ft = f.clone();
                                            view! {
                                                <span class="text-xs bg-gray-700/80 border border-gray-600 px-2 py-0.5 rounded text-gray-300 font-mono truncate max-w-[180px]" title=ft>{f}</span>
                                            }
                                        })}
                                        {dur_tag.map(|d| view! {
                                            <span class="text-xs bg-gray-700/50 border border-gray-700 px-1.5 py-0.5 rounded text-gray-400 shrink-0">{d}</span>
                                        })}
                                        {pkt_tag.map(|p| view! {
                                            <span class="text-xs bg-gray-700/50 border border-gray-700 px-1.5 py-0.5 rounded text-gray-400 shrink-0">{p}</span>
                                        })}

                                        // 操作区（右对齐）
                                        <div class="ml-auto flex items-center gap-1.5 shrink-0">
                                            // 停止（pending/running）
                                            {if is_active {
                                                let id_s  = id_stop.clone();
                                                let id_s2 = id_stop.clone();
                                                view! {
                                                    <button
                                                        class="text-xs bg-yellow-600/80 hover:bg-yellow-600 text-white px-2.5 py-1.5 rounded-lg transition-colors disabled:opacity-40"
                                                        disabled=move || stopping_id.get() == Some(id_s.clone())
                                                        on:click=move |_| on_stop(id_s2.clone())
                                                    >
                                                        {move || if stopping_id.get() == Some(id_stop.clone()) { "停止中…" } else { "⏹ 停止" }}
                                                    </button>
                                                }.into_any()
                                            } else if is_done {
                                                // done: 下载 + 分析 + 重试
                                                let task_r = task_retry.clone();
                                                view! {
                                                    <div class="flex gap-1.5">
                                                        <button
                                                            class="text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 px-2.5 py-1.5 rounded-lg transition-colors"
                                                            on:click=move |_| on_download(id_dl.clone())
                                                        >"⬇ 下载"</button>
                                                        <a
                                                            href=format!("/captures/{}", id_ana)
                                                            class="text-xs bg-blue-700 hover:bg-blue-600 text-white px-2.5 py-1.5 rounded-lg transition-colors"
                                                        >"🔬 分析"</a>
                                                        <button
                                                            class="text-xs bg-green-700/80 hover:bg-green-700 text-white px-2.5 py-1.5 rounded-lg transition-colors"
                                                            on:click=move |_| on_retry(task_r.clone())
                                                        >"↺"</button>
                                                    </div>
                                                }.into_any()
                                            } else if can_retry {
                                                // failed / cancelled: 只有重试
                                                let task_r = task_retry.clone();
                                                view! {
                                                    <button
                                                        class="text-xs bg-green-700/80 hover:bg-green-700 text-white px-2.5 py-1.5 rounded-lg transition-colors"
                                                        on:click=move |_| on_retry(task_r.clone())
                                                    >"↺ 重试"</button>
                                                }.into_any()
                                            } else {
                                                view! { <span/> }.into_any()
                                            }}

                                            // 删除
                                            <button
                                                class="text-xs text-red-500 hover:text-red-400 px-2 py-1.5 transition-colors"
                                                on:click=move |_| on_delete(id_del.clone())
                                            >"✕"</button>
                                        </div>
                                    </div>

                                    // ── 第二行：时间 + 文件大小 ──────────────────
                                    <div class="flex gap-3 mt-1.5 pl-5 text-xs text-gray-600 flex-wrap">
                                        <span>"创建 "{task.created_at.clone()}</span>
                                        {task.finished_at.clone().map(|t| view! {
                                            <span>"结束 "{t}</span>
                                        })}
                                        {task.file_size.map(|s| view! {
                                            <span class="text-gray-500">{format_size(s)}</span>
                                        })}
                                        // 当前选中时提示 ESC
                                        {
                                            let id_esc = task.id.clone();
                                            move || if expanded_log_id.get() == Some(id_esc.clone()) {
                                                view! {
                                                    <span class="text-gray-700 ml-auto">"ESC 关闭日志"</span>
                                                }.into_any()
                                            } else {
                                                view! { <span/> }.into_any()
                                            }
                                        }
                                    </div>
                                </div>
                            }
                            }).collect::<Vec<_>>()
                        }}

                        {move || tasks.get().is_empty().then(|| view! {
                            <div class="text-center py-16 text-gray-500">
                                <div class="text-4xl mb-3">"📦"</div>
                                <p>"还没有抓包任务"</p>
                            </div>
                        })}

                        // 分页
                        {move || {
                            let total = (tasks.get().len() + PAGE_SIZE - 1) / PAGE_SIZE;
                            if total <= 1 {
                                view! { <div/> }.into_any()
                            } else {
                                let total2 = total;
                                view! {
                                    <div class="flex items-center justify-between mt-4 text-sm text-gray-400">
                                        <button
                                            class="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg hover:bg-gray-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                                            disabled=move || page.get() <= 1
                                            on:click=move |_| page.update(|p| *p = p.saturating_sub(1))
                                        >"← 上一页"</button>
                                        <span class="text-gray-500">"第 " {move || page.get()} " / " {total2} " 页"</span>
                                        <button
                                            class="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg hover:bg-gray-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                                            disabled=move || page.get() >= total2
                                            on:click=move |_| page.update(|p| if *p < total2 { *p += 1 })
                                        >"下一页 →"</button>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </div>

                // ── 右侧：日志侧边栏（固定44%，始终显示） ──────────────────
                <div class="flex-1 min-w-0">
                    {move || {
                        let log_id = expanded_log_id.get();

                        match log_id {
                            None => {
                                // 没有展开时显示空状态提示
                                view! {
                                    <div class="flex flex-col items-center justify-center h-full text-gray-600 rounded-xl border border-dashed border-gray-700/50"
                                         style="min-height: 300px;">
                                        <div class="text-4xl mb-3 opacity-30">"📋"</div>
                                        <p class="text-sm">"点击左侧任务查看日志"</p>
                                        <p class="text-xs mt-1 text-gray-700">"ESC 可关闭日志"</p>
                                    </div>
                                }.into_any()
                            }
                            Some(lid) => {
                                let task_opt = tasks.get().into_iter().find(|t| t.id == lid);
                                let (task_status, task_iface, task_filter, task_dur) = task_opt.as_ref()
                                    .map(|t| (
                                        t.status.clone(),
                                        t.interface.clone(),
                                        t.filter.clone(),
                                        t.duration,
                                    ))
                                    .unwrap_or_default();

                                let log_content = task_logs.get()
                                    .get(&lid)
                                    .cloned()
                                    .unwrap_or_else(|| "⏳ 等待日志...".to_string());

                                let pkts = live_packets.get()
                                    .get(&lid)
                                    .cloned()
                                    .unwrap_or_default();

                                let is_live = task_status == "running" || task_status == "pending";

                                let (status_color, status_text) = match task_status.as_str() {
                                    "running"   => ("text-blue-400",  "运行中"),
                                    "pending"   => ("text-yellow-400","等待中"),
                                    "done"      => ("text-green-400", "已完成"),
                                    "failed"    => ("text-red-400",   "失败"),
                                    "cancelled" => ("text-gray-400",  "已取消"),
                                    _           => ("text-gray-400",  "未知"),
                                };

                                let lid2 = lid.clone();

                                view! {
                                    <div class="flex flex-col bg-gray-900 border border-gray-700 rounded-xl overflow-hidden"
                                         style="height: calc(100vh - 120px); position: sticky; top: 0;">

                                        // 侧边栏标题栏
                                        <div class="flex items-center gap-2 px-4 py-3 bg-gray-800 border-b border-gray-700 shrink-0 flex-wrap">
                                            {if is_live {
                                                view! {
                                                    <span class="relative flex h-2 w-2 shrink-0">
                                                        <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
                                                        <span class="relative inline-flex rounded-full h-2 w-2 bg-blue-500"></span>
                                                    </span>
                                                }.into_any()
                                            } else { view! { <span/> }.into_any() }}
                                            <span class=format!("text-xs font-semibold shrink-0 {}", status_color)>{status_text}</span>
                                            <span class="font-mono text-sm text-gray-300 shrink-0">{task_iface.clone()}</span>
                                            {task_filter.map(|f| {
                                                let ft = f.clone();
                                                view! {
                                                    <span class="text-xs text-gray-500 font-mono truncate max-w-[120px]" title=ft>{f}</span>
                                                }
                                            })}
                                            {task_dur.map(|d| view! {
                                                <span class="text-xs text-gray-600 shrink-0">{d}"s"</span>
                                            })}
                                            <div class="ml-auto flex items-center gap-2 shrink-0">
                                                <span class="text-xs text-gray-700 font-mono hidden sm:block">"ESC"</span>
                                                <button
                                                    class="text-gray-500 hover:text-white transition-colors text-lg leading-none"
                                                    on:click=move |_| expanded_log_id.set(None)
                                                >"×"</button>
                                            </div>
                                        </div>

                                        // 日志区
                                        <div class="flex flex-col flex-1 overflow-hidden">
                                            <div class="px-3 py-1.5 bg-gray-800/50 border-b border-gray-700/50 flex items-center gap-2 shrink-0">
                                                <span class="text-xs font-semibold text-gray-400 uppercase tracking-wide">"执行日志"</span>
                                                {if is_live {
                                                    view! {
                                                        <span class="text-xs text-blue-400 animate-pulse">"● 实时"</span>
                                                    }.into_any()
                                                } else { view! { <span/> }.into_any() }}
                                            </div>
                                            <pre
                                                id=format!("log-{}", lid2)
                                                class="flex-1 overflow-auto p-4 text-xs font-mono text-green-300 leading-relaxed whitespace-pre-wrap break-words bg-transparent"
                                                style="scrollbar-width: thin; scrollbar-color: #374151 transparent; min-height: 0;"
                                            >
                                                {log_content}
                                            </pre>
                                        </div>

                                        // 实时包预览（仅 running 任务）
                                        {if task_status == "running" && !pkts.is_empty() {
                                            let pkt_count = pkts.len();
                                            view! {
                                                <div class="flex flex-col border-t border-gray-700/50 shrink-0" style="max-height: 220px;">
                                                    <div class="px-3 py-1.5 bg-gray-800/50 flex items-center gap-2 shrink-0">
                                                        <span class="text-xs font-semibold text-gray-400 uppercase tracking-wide">"实时捕获"</span>
                                                        <span class="text-xs font-mono text-green-400 bg-green-900/40 px-1.5 rounded">{pkt_count}" 包"</span>
                                                    </div>
                                                    <div class="overflow-auto">
                                                        <table class="w-full text-xs">
                                                            <thead class="sticky top-0 bg-gray-800/90">
                                                                <tr class="text-gray-500 border-b border-gray-700/50">
                                                                    <th class="text-left px-3 py-1 w-10">"No."</th>
                                                                    <th class="text-left px-3 py-1">"源地址"</th>
                                                                    <th class="text-left px-3 py-1 w-16">"协议"</th>
                                                                    <th class="text-left px-3 py-1">"信息"</th>
                                                                </tr>
                                                            </thead>
                                                            <tbody>
                                                                {pkts.into_iter().rev().map(|p| {
                                                                    let pc = match p.protocol.as_str() {
                                                                        "TCP"  => "text-blue-300",
                                                                        "UDP"  => "text-green-300",
                                                                        "HTTP" | "HTTPS" | "HTTP/2" => "text-orange-300",
                                                                        "DNS"  => "text-purple-300",
                                                                        "ICMP" | "ICMPv6" => "text-yellow-300",
                                                                        "TLS" | "SSL" => "text-teal-300",
                                                                        "ARP"  => "text-pink-300",
                                                                        _ => "text-gray-300",
                                                                    };
                                                                    view! {
                                                                        <tr class="border-b border-gray-700/20 hover:bg-gray-800/50">
                                                                            <td class="px-3 py-0.5 text-gray-600 tabular-nums">{p.number}</td>
                                                                            <td class="px-3 py-0.5 font-mono text-gray-400 truncate max-w-0 w-32">{p.source}</td>
                                                                            <td class=format!("px-3 py-0.5 font-mono font-medium {}", pc)>{p.protocol}</td>
                                                                            <td class="px-3 py-0.5 text-gray-400 truncate max-w-0">{p.info}</td>
                                                                        </tr>
                                                                    }
                                                                }).collect::<Vec<_>>()}
                                                            </tbody>
                                                        </table>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else { view! { <div/> }.into_any() }}
                                    </div>
                                }.into_any()
                            }
                        }
                    }}
                </div>
            </div>

            // ── 新建抓包弹窗 ─────────────────────────────────────────────
            <Modal
                show=show_modal.read_only()
                title="新建抓包任务"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                <form on:submit=on_create class="space-y-3">

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

                    {move || {
                        let ps = ports.get();
                        if ps.is_empty() {
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

                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"BPF 过滤器（可选，与端口选择叠加）"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500"
                            placeholder="tcp and host 1.2.3.4"
                            prop:value=filter
                            on:input=move |ev| filter.set(event_target_value(&ev))
                        />
                    </div>

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

// 简化的并行 future join
async fn futures_join<A, B>(
    a: impl std::future::Future<Output = A>,
    b: impl std::future::Future<Output = B>,
) -> (A, B) {
    let ra = a.await;
    let rb = b.await;
    (ra, rb)
}
