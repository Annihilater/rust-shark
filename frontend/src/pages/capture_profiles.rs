use crate::components::{
    layout::Layout,
    modal::Modal,
    select::{Select, SelectOption},
};
use crate::store::use_auth;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug)]
struct CaptureProfile {
    id: String,
    name: String,
    server_id: String,
    interface: String,
    filter: Option<String>,
    duration: Option<i64>,
    packet_limit: Option<i64>,
    created_at: String,
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

#[derive(Serialize)]
struct ProfileRequest {
    name: String,
    server_id: String,
    interface: String,
    filter: Option<String>,
    duration: Option<i64>,
    packet_limit: Option<i64>,
}

#[derive(Serialize)]
struct RunCaptureRequest {
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
pub fn CaptureProfilesPage() -> impl IntoView {
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

    let profiles = RwSignal::new(Vec::<CaptureProfile>::new());
    let servers = RwSignal::new(Vec::<Server>::new());
    let interfaces = RwSignal::new(Vec::<Interface>::new());
    let show_modal = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let toast_msg = RwSignal::new(Option::<(bool, String)>::None); // (success, message)
    let loading_ifaces = RwSignal::new(false);
    let running_id = RwSignal::new(Option::<String>::None);

    // None = 新建模式；Some(id) = 编辑模式
    let edit_id = RwSignal::new(Option::<String>::None);

    // 表单字段
    let name = RwSignal::new(String::new());
    let server_id = RwSignal::new(String::new());
    let iface = RwSignal::new(String::new());
    let filter = RwSignal::new(String::new());
    let duration = RwSignal::new("60".to_string());
    let packet_limit = RwSignal::new("1000".to_string());

    // ── 加载 ──────────────────────────────────────────────────────────────
    let load_profiles = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<CaptureProfile>>("/api/capture-profiles").await
            {
                profiles.set(list);
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

    load_profiles();
    load_servers();

    // 服务器选择后自动加载网卡（仅新建模式触发，编辑模式手动控制）
    let skip_iface_effect = RwSignal::new(false);
    Effect::new(move |_| {
        let sid = server_id.get();
        if skip_iface_effect.get() {
            // 编辑模式回填时跳过一次，避免清空 iface
            skip_iface_effect.set(false);
            return;
        }
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
                let first = list.first().map(|i| i.name.clone()).unwrap_or_default();
                interfaces.set(list);
                iface.set(first);
            }
            loading_ifaces.set(false);
        });
    });

    // ── 重置表单 ──────────────────────────────────────────────────────────
    let reset_form = move || {
        edit_id.set(None);
        name.set(String::new());
        server_id.set(String::new());
        iface.set(String::new());
        filter.set(String::new());
        duration.set("60".to_string());
        packet_limit.set("1000".to_string());
        interfaces.set(vec![]);
        error.set(None);
    };

    // ── 打开编辑弹窗，回填表单 ────────────────────────────────────────────
    let open_edit = move |profile: CaptureProfile| {
        edit_id.set(Some(profile.id.clone()));
        name.set(profile.name.clone());
        filter.set(profile.filter.clone().unwrap_or_default());
        duration.set(
            profile
                .duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "60".to_string()),
        );
        packet_limit.set(
            profile
                .packet_limit
                .map(|p| p.to_string())
                .unwrap_or_else(|| "1000".to_string()),
        );
        error.set(None);

        let sid = profile.server_id.clone();
        let saved_iface = profile.interface.clone();

        // 标记跳过一次 Effect（server_id 变化时不要自动覆盖 iface）
        skip_iface_effect.set(true);
        server_id.set(sid.clone());

        // 异步加载该服务器的网卡列表，加载完再设置 iface
        loading_ifaces.set(true);
        leptos::task::spawn_local(async move {
            let path = format!("/api/servers/{}/interfaces", sid);
            if let Ok(list) = crate::api::get::<Vec<Interface>>(&path).await {
                interfaces.set(list);
            }
            loading_ifaces.set(false);
            iface.set(saved_iface);
        });

        show_modal.set(true);
    };

    // ── 提交（新建 or 更新）────────────────────────────────────────────────
    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        let req = ProfileRequest {
            name: name.get().trim().to_string(),
            server_id: server_id.get(),
            interface: iface.get(),
            filter: {
                let f = filter.get();
                if f.trim().is_empty() {
                    None
                } else {
                    Some(f)
                }
            },
            duration: duration.get().parse().ok(),
            packet_limit: packet_limit.get().parse().ok(),
        };
        let id = edit_id.get();
        leptos::task::spawn_local(async move {
            let result = if let Some(ref eid) = id {
                // 编辑：PUT
                crate::api::put::<_, CaptureProfile>(
                    &format!("/api/capture-profiles/{}", eid),
                    &req,
                )
                .await
            } else {
                // 新建：POST
                crate::api::post::<_, CaptureProfile>("/api/capture-profiles", &req).await
            };
            match result {
                Ok(_) => {
                    show_modal.set(false);
                    let msg = if id.is_some() {
                        "配置已更新".to_string()
                    } else {
                        "配置创建成功".to_string()
                    };
                    reset_form();
                    toast_msg.set(Some((true, msg)));
                    load_profiles();
                    leptos::task::spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(5_000).await;
                        toast_msg.set(None);
                    });
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // ── 删除配置 ─────────────────────────────────────────────────────────
    let on_delete = move |id: String| {
        toast_msg.set(None);
        leptos::task::spawn_local(async move {
            let path = format!("/api/capture-profiles/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                toast_msg.set(Some((true, "已删除".to_string())));
                load_profiles();
                leptos::task::spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(5_000).await;
                    toast_msg.set(None);
                });
            }
        });
    };

    // ── 立即执行（用此配置创建抓包任务） ─────────────────────────────────
    let on_run = move |profile: CaptureProfile| {
        running_id.set(Some(profile.id.clone()));
        toast_msg.set(None);
        leptos::task::spawn_local(async move {
            let req = RunCaptureRequest {
                server_id: profile.server_id.clone(),
                interface: profile.interface.clone(),
                filter: profile.filter.clone(),
                duration: profile.duration,
                packet_limit: profile.packet_limit,
                scheduled_at: None,
                repeat_type: None,
                repeat_until: None,
            };
            match crate::api::post::<_, serde_json::Value>("/api/captures", &req).await {
                Ok(_) => {
                    let msg = format!("已启动「{}」，前往抓包任务页面查看", profile.name);
                    toast_msg.set(Some((true, msg)));
                    leptos::task::spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(5_000).await;
                        toast_msg.set(None);
                    });
                }
                Err(e) => {
                    let msg = format!("启动失败: {}", e);
                    toast_msg.set(Some((false, msg)));
                    leptos::task::spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(5_000).await;
                        toast_msg.set(None);
                    });
                }
            }
            running_id.set(None);
        });
    };

    view! {
        <Layout>
            <div class="max-w-5xl mx-auto">
                // 标题栏
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-2xl font-bold">"抓包配置"</h1>
                        <p class="text-sm text-gray-500 mt-0.5">"保存常用配置，一键启动抓包任务"</p>
                    </div>
                    <div class="flex items-center gap-3">
                        <a href="/captures"
                           class="text-sm text-blue-400 hover:text-blue-300 transition-colors">
                            "查看任务列表 →"
                        </a>
                        <button
                            class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                            on:click=move |_| { reset_form(); show_modal.set(true); }
                        >"+ 新建配置"</button>
                    </div>
                </div>

                // 右上角 Toast 通知
                {move || toast_msg.get().map(|(success, msg)| {
                    let bg = if success { "bg-green-600" } else { "bg-red-600" };
                    let icon = if success { "✓" } else { "✗" };
                    view! {
                        <div class=format!("fixed top-4 right-4 {} text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3 z-50 max-w-sm", bg)>
                            <span class="text-base font-bold shrink-0">{icon}</span>
                            <span class="text-sm flex-1">{msg}</span>
                            <button
                                class="ml-1 text-white/70 hover:text-white transition-colors text-lg leading-none shrink-0"
                                on:click=move |_| toast_msg.set(None)
                            >"✕"</button>
                        </div>
                    }
                })}

                // 配置列表
                {move || {
                    let ps = profiles.get();
                    if ps.is_empty() {
                        view! {
                            <div class="text-center py-20 text-gray-500">
                                <div class="text-5xl mb-4 opacity-30">"⚙"</div>
                                <p class="text-base">"还没有抓包配置"</p>
                                <p class="text-sm text-gray-600 mt-1">"创建配置后可一键重复执行抓包任务"</p>
                                <button
                                    class="mt-6 bg-blue-600 hover:bg-blue-700 text-white px-5 py-2 rounded-lg text-sm transition-colors"
                                    on:click=move |_| { reset_form(); show_modal.set(true); }
                                >"+ 新建第一个配置"</button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="space-y-3">
                                {ps.into_iter().map(|profile| {
                                    let pid_del     = profile.id.clone();
                                    let pid_run     = profile.id.clone();
                                    let pid_cls     = profile.id.clone();
                                    let profile_run  = profile.clone();
                                    let profile_edit = profile.clone();

                                    let server_name = servers.get()
                                        .into_iter()
                                        .find(|s| s.id == profile.server_id)
                                        .map(|s| format!("{} ({})", s.name, s.host))
                                        .unwrap_or_else(|| profile.server_id.clone());

                                    let dur_tag = profile.duration.map(|d| {
                                        if d == 0 { "不限时".to_string() } else { format!("{}s", d) }
                                    });
                                    let pkt_tag = profile.packet_limit.map(|p| {
                                        if p == 0 { "不限包".to_string() } else { format!("{}包", p) }
                                    });

                                    view! {
                                        <div class="card rounded-xl p-4 hover:border-gray-300 dark:hover:border-gray-600 transition-all">
                                            <div class="flex items-center justify-between gap-4">
                                                // 左侧：名称 + 服务器 + 标签行
                                                <div class="min-w-0 flex-1">
                                                    <div class="flex items-center gap-2 mb-1 flex-wrap">
                                                        <h3 class="font-semibold">{profile.name.clone()}</h3>
                                                        <span class="text-xs text-gray-500 dark:text-gray-400 truncate"
                                                            title=server_name.clone()>
                                                            {server_name.clone()}
                                                        </span>
                                                    </div>
                                                    <div class="flex flex-wrap gap-1.5 items-center">
                                                        <span class="text-xs bg-blue-50 border border-blue-200 text-blue-700 dark:bg-blue-900/40 dark:border-blue-800/50 dark:text-blue-300 px-2 py-0.5 rounded font-mono shrink-0">
                                                            {profile.interface.clone()}
                                                        </span>
                                                        {profile.filter.clone().map(|f| {
                                                            let ft = f.clone();
                                                            view! {
                                                                <span class="text-xs badge-gray px-2 py-0.5 rounded font-mono truncate max-w-[200px]" title=ft>
                                                                    {f}
                                                                </span>
                                                            }
                                                        })}
                                                        {dur_tag.map(|d| view! {
                                                            <span class="text-xs badge-gray px-2 py-0.5 rounded">{d}</span>
                                                        })}
                                                        {pkt_tag.map(|p| view! {
                                                            <span class="text-xs badge-gray px-2 py-0.5 rounded">{p}</span>
                                                        })}
                                                        <span class="text-xs text-gray-400 font-mono ml-1">{profile.created_at.clone()}</span>
                                                    </div>
                                                </div>

                                                // 右侧：执行 + 编辑 + 删除
                                                <div class="flex items-center gap-2 shrink-0">
                                                    <button
                                                        class="text-sm bg-green-600 hover:bg-green-500 disabled:opacity-40 text-white px-3 py-1.5 rounded-lg transition-colors flex items-center gap-1.5"
                                                        disabled=move || running_id.get() == Some(pid_run.clone())
                                                        on:click=move |_| on_run(profile_run.clone())
                                                    >
                                                        {move || {
                                                            if running_id.get() == Some(pid_cls.clone()) {
                                                                "启动中…"
                                                            } else {
                                                                "▶ 立即执行"
                                                            }
                                                        }}
                                                    </button>
                                                    <button
                                                        class="text-blue-500 dark:text-blue-400 hover:text-blue-400 dark:hover:text-blue-300 text-sm px-3 py-1.5 transition-colors"
                                                        on:click=move |_| open_edit(profile_edit.clone())
                                                    >"编辑"</button>
                                                    <button
                                                        class="text-red-500 dark:text-red-400 hover:text-red-400 dark:hover:text-red-300 text-sm px-3 py-1.5 transition-colors"
                                                        on:click=move |_| on_delete(pid_del.clone())
                                                    >"删除"</button>
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // ── 新建 / 编辑 配置弹窗（复用同一表单）─────────────────────
            <Modal
                show=show_modal.read_only()
                title=Signal::derive(move || {
                    if edit_id.get().is_some() {
                        "编辑抓包配置".to_string()
                    } else {
                        "新建抓包配置".to_string()
                    }
                })
                on_close=Callback::new(move |_| { show_modal.set(false); reset_form(); })
            >
                <form on:submit=on_submit class="space-y-3">
                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"配置名称"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                            placeholder="例：生产服务器-全量抓包"
                            prop:value=name
                            on:input=move |ev| name.set(event_target_value(&ev))
                            required
                        />
                    </div>

                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"选择服务器"</label>
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
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">
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

                    <div>
                        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"BPF 过滤器（可选）"</label>
                        <input
                            class="input w-full rounded-lg px-3 py-2 text-sm font-mono text-gray-900 dark:text-white"
                            placeholder="tcp and host 1.2.3.4"
                            prop:value=filter
                            on:input=move |ev| filter.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"时长（秒，0=不限）"</label>
                            <input
                                class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                                type="number" min="0" placeholder="60"
                                prop:value=duration
                                on:input=move |ev| duration.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">"包数限制（0=不限）"</label>
                            <input
                                class="input w-full rounded-lg px-3 py-2 text-sm text-gray-900 dark:text-white"
                                type="number" min="0" placeholder="1000"
                                prop:value=packet_limit
                                on:input=move |ev| packet_limit.set(event_target_value(&ev))
                            />
                        </div>
                    </div>

                    {move || error.get().map(|e| view! {
                        <div class="notice-error px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}

                    <div class="flex gap-3 pt-1">
                        <button type="submit"
                            class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm transition-colors"
                        >
                            {move || if edit_id.get().is_some() { "保存修改" } else { "保存配置" }}
                        </button>
                        <button type="button"
                            class="flex-1 btn-secondary py-2 rounded-lg text-sm transition-colors"
                            on:click=move |_| { show_modal.set(false); reset_form(); }
                        >"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
