use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::{layout::Layout, modal::Modal};
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

    let tasks = RwSignal::new(Vec::<CaptureTask>::new());
    let servers = RwSignal::new(Vec::<Server>::new());
    let interfaces = RwSignal::new(Vec::<Interface>::new());
    let show_modal = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // 表单
    let server_id = RwSignal::new(String::new());
    let iface = RwSignal::new(String::new());
    let filter = RwSignal::new(String::new());
    let duration = RwSignal::new(String::new());
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

    // 服务器选择后加载网卡
    Effect::new(move |_| {
        let sid = server_id.get();
        if !sid.is_empty() {
            leptos::task::spawn_local(async move {
                let path = format!("/api/servers/{}/interfaces", sid);
                if let Ok(list) = crate::api::get::<Vec<Interface>>(&path).await {
                    interfaces.set(list);
                    if let Some(first_name) = interfaces.with(|v| v.first().map(|i| i.name.clone())) {
                        iface.set(first_name);
                    }
                }
            });
        }
    });

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = CreateCaptureRequest {
            server_id: server_id.get(),
            interface: iface.get(),
            filter: if filter.get().is_empty() { None } else { Some(filter.get()) },
            duration: duration.get().parse().ok(),
            packet_limit: packet_limit.get().parse().ok(),
            scheduled_at: if scheduled_at.get().is_empty() { None } else { Some(scheduled_at.get()) },
            repeat_type: None,
            repeat_until: None,
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, CaptureTask>("/api/captures", &req).await {
                Ok(_) => {
                    show_modal.set(false);
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
                        on:click=move |_| show_modal.set(true)
                    >"+ 新建抓包"</button>
                </div>

                <div class="space-y-3">
                    {move || tasks.get().into_iter().map(|task| {
                        let id = task.id.clone();
                        let id2 = task.id.clone();
                        let id3 = task.id.clone();
                        let status_class = match task.status.as_str() {
                            "done" => "text-green-400",
                            "running" => "text-blue-400",
                            "failed" => "text-red-400",
                            "cancelled" => "text-gray-400",
                            _ => "text-yellow-400",
                        };
                        let status_label = match task.status.as_str() {
                            "done" => "✓ 完成",
                            "running" => "⟳ 运行中",
                            "failed" => "✗ 失败",
                            "cancelled" => "⊘ 已取消",
                            _ => "○ 等待",
                        };
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4">
                                <div class="flex items-start justify-between">
                                    <div>
                                        <div class="flex items-center gap-3">
                                            <span class=format!("text-sm font-medium {}", status_class)>{status_label}</span>
                                            <span class="font-mono text-sm text-gray-300">{task.interface.clone()}</span>
                                            {task.filter.clone().map(|f| view! {
                                                <span class="text-xs bg-gray-700 px-2 py-0.5 rounded text-gray-300">{f}</span>
                                            })}
                                        </div>
                                        <p class="text-xs text-gray-500 mt-1">"创建: "{task.created_at.clone()}</p>
                                        {task.file_size.map(|s| view! {
                                            <p class="text-xs text-gray-400 mt-0.5">"大小: "{format_size(s)}</p>
                                        })}
                                    </div>
                                    <div class="flex gap-2">
                                        {if task.status == "done" {
                                            view! {
                                                <div class="flex gap-2">
                                                    <a
                                                        href=format!("/api/captures/{}/download", id3)
                                                        class="bg-gray-700 hover:bg-gray-600 text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                    >"下载"</a>
                                                    <a
                                                        href=format!("/captures/{}", id)
                                                        class="bg-blue-700 hover:bg-blue-600 text-sm px-3 py-1.5 rounded-lg transition-colors"
                                                    >"分析"</a>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }}
                                        <button
                                            class="text-red-400 hover:text-red-300 text-sm px-3 py-1.5"
                                            on:click=move |_| on_delete(id2.clone())
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

            // 新建抓包弹窗
            <Modal
                show=show_modal.read_only()
                title="新建抓包任务"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                <form on:submit=on_create class="space-y-3">
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"选择服务器"</label>
                        <select class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            on:change=move |ev| server_id.set(event_target_value(&ev)) required>
                            <option value="">"-- 选择服务器 --"</option>
                            {move || servers.get().into_iter().map(|s| {
                                let sid = s.id.clone();
                                view! { <option value=sid>{s.name}" ("{s.host}")"</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"网卡"</label>
                        <select class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            on:change=move |ev| iface.set(event_target_value(&ev)) required>
                            {move || {
                                let ifaces = interfaces.get();
                                if ifaces.is_empty() {
                                    view! { <option value="">"-- 先选择服务器 --"</option> }.into_any()
                                } else {
                                    ifaces.into_iter().map(|i| {
                                        let n = i.name.clone();
                                        let n2 = n.clone();
                                        view! { <option value=n>{n2}</option> }
                                    }).collect::<Vec<_>>().into_any()
                                }
                            }}
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"BPF 过滤器（可选）"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white font-mono focus:outline-none focus:border-blue-500"
                            placeholder="tcp port 80" prop:value=filter
                            on:input=move |ev| filter.set(event_target_value(&ev))/>
                    </div>
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"时长（秒，空=不限）"</label>
                            <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" placeholder="60" prop:value=duration
                                on:input=move |ev| duration.set(event_target_value(&ev))/>
                        </div>
                        <div>
                            <label class="block text-xs text-gray-400 mb-1">"包数限制（空=不限）"</label>
                            <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                                type="number" placeholder="1000" prop:value=packet_limit
                                on:input=move |ev| packet_limit.set(event_target_value(&ev))/>
                        </div>
                    </div>
                    <div>
                        <label class="block text-xs text-gray-400 mb-1">"定时执行（空=立即）"</label>
                        <input class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
                            type="datetime-local" prop:value=scheduled_at
                            on:input=move |ev| scheduled_at.set(event_target_value(&ev))/>
                    </div>
                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}
                    <div class="flex gap-3 pt-1">
                        <button type="submit" class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm">"开始抓包"</button>
                        <button type="button" class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm"
                            on:click=move |_| show_modal.set(false)>"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
