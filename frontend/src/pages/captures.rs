use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::{layout::Layout, modal::Modal, select::{Select, SelectOption}};
use crate::store::use_auth;

#[derive(Deserialize, Clone, Debug)]
struct CaptureTask {
    id: String,
    server_id: String,
    interface: String,
    filter: Option<String>,
    duration: Option<i64>,
    status: String,
    file_size: Option<i64>,
    created_at: String,
    finished_at: Option<String>,
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

#[component]
pub fn CapturesPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window().unwrap().location().set_href("/login").ok();
        }
    });

    let tasks      = RwSignal::new(Vec::<CaptureTask>::new());
    let servers    = RwSignal::new(Vec::<Server>::new());
    let interfaces = RwSignal::new(Vec::<Interface>::new());
    let show_modal = RwSignal::new(false);
    let error      = RwSignal::new(Option::<String>::None);
    let loading_ifaces = RwSignal::new(false);

    // 表单
    let server_id    = RwSignal::new(String::new());
    let iface        = RwSignal::new(String::new());
    let filter       = RwSignal::new(String::new());
    let duration     = RwSignal::new(String::new());
    let packet_limit = RwSignal::new(String::new());
    let scheduled_at = RwSignal::new(String::new());

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

    // 服务器选择后加载网卡列表
    Effect::new(move |_| {
        let sid = server_id.get();
        if sid.is_empty() {
            interfaces.set(vec![]);
            iface.set(String::new());
            return;
        }
        loading_ifaces.set(true);
        iface.set(String::new());
        leptos::task::spawn_local(async move {
            let path = format!("/api/servers/{}/interfaces", sid);
            if let Ok(list) = crate::api::get::<Vec<Interface>>(&path).await {
                // 自动选中第一个网卡
                let first = list.first().map(|i| i.name.clone()).unwrap_or_default();
                interfaces.set(list);
                iface.set(first);
            }
            loading_ifaces.set(false);
        });
    });

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        let req = CreateCaptureRequest {
            server_id: server_id.get(),
            interface: iface.get(),
            filter: { let f = filter.get(); if f.is_empty() { None } else { Some(f) } },
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
                    duration.set(String::new());
                    packet_limit.set(String::new());
                    scheduled_at.set(String::new());
                    load_tasks();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_delete = move |id: String| {
        leptos::task::spawn_local(async move {
            let path = format!("/api/captures/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                load_tasks();
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
                        let id_dl  = task.id.clone();
                        let id_ana = task.id.clone();
                        let id_del = task.id.clone();
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
                        let is_done = task.status == "done";
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4">
                                <div class="flex items-start justify-between">
                                    <div>
                                        <div class="flex items-center gap-3 flex-wrap">
                                            <span class=format!("text-sm font-medium {}", status_class)>{status_label}</span>
                                            <span class="font-mono text-sm text-gray-300">{task.interface.clone()}</span>
                                            {task.filter.clone().map(|f| view! {
                                                <span class="text-xs bg-gray-700 px-2 py-0.5 rounded text-gray-300 font-mono">{f}</span>
                                            })}
                                            {task.duration.map(|d| view! {
                                                <span class="text-xs text-gray-500">{d}"s"</span>
                                            })}
                                        </div>
                                        <p class="text-xs text-gray-500 mt-1">"创建: "{task.created_at.clone()}</p>
                                        {task.file_size.map(|s| view! {
                                            <p class="text-xs text-gray-400 mt-0.5">"大小: "{format_size(s)}</p>
                                        })}
                                    </div>
                                    <div class="flex items-center gap-2 shrink-0">
                                        {if is_done {
                                            view! {
                                                <div class="flex gap-2">
                                                    <a
                                                        href=format!("/api/captures/{}/download", id_dl)
                                                        class="bg-gray-700 hover:bg-gray-600 text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                    >"下载"</a>
                                                    <a
                                                        href=format!("/captures/{}/analyze", id_ana)
                                                        class="bg-blue-700 hover:bg-blue-600 text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                    >"分析"</a>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div/> }.into_any()
                                        }}
                                        <button
                                            class="text-red-400 hover:text-red-300 text-sm px-3 py-1.5"
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

                    // 网卡（依赖服务器选择）
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

                    // BPF 过滤器
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"BPF 过滤器（可选）"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500"
                            placeholder="tcp port 80"
                            prop:value=filter
                            on:input=move |ev| filter.set(event_target_value(&ev))
                        />
                    </div>

                    // 时长 + 包数
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"时长（秒，空=不限）"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" placeholder="60"
                                prop:value=duration
                                on:input=move |ev| duration.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"包数限制（空=不限）"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" placeholder="1000"
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
