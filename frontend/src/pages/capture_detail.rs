use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde::Deserialize;
use crate::components::layout::Layout;
use crate::store::use_auth;

#[derive(Deserialize, Clone, Debug)]
struct PacketSummary {
    number: u64,
    time: String,
    source: String,
    destination: String,
    protocol: String,
    length: u64,
    info: String,
}

#[derive(Deserialize, Clone, Debug)]
struct PacketDetail {
    number: u64,
    layers: serde_json::Value,
    raw: String,
}

#[component]
pub fn CaptureDetailPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window().unwrap().location().set_href("/login").ok();
        }
    });

    let params = use_params_map();
    let capture_id = move || params.get().get("id").unwrap_or_default();

    let packets = RwSignal::new(Vec::<PacketSummary>::new());
    let selected = RwSignal::new(Option::<PacketDetail>::None);
    let filter = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let load_packets = move || {
        loading.set(true);
        let id = capture_id();
        let f = filter.get();
        leptos::task::spawn_local(async move {
            let mut path = format!("/api/captures/{}/packets?limit=500", id);
            if !f.is_empty() {
                path.push_str(&format!("&filter={}", f));
            }
            match crate::api::get::<Vec<PacketSummary>>(&path).await {
                Ok(list) => {
                    packets.set(list);
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    load_packets();

    let on_select = move |no: u64| {
        let id = capture_id();
        leptos::task::spawn_local(async move {
            let path = format!("/api/captures/{}/packets/{}", id, no);
            if let Ok(detail) = crate::api::get::<PacketDetail>(&path).await {
                selected.set(Some(detail));
            }
        });
    };

    view! {
        <Layout>
            <div class="max-w-full">
                <div class="flex items-center gap-4 mb-4">
                    <a href="/captures" class="text-gray-400 hover:text-white">"← 返回"</a>
                    <h1 class="text-xl font-bold">"数据包分析"</h1>
                    <span class="text-xs text-gray-500 font-mono">{capture_id}</span>
                </div>

                // 过滤器
                <div class="flex gap-2 mb-4">
                    <input
                        class="flex-1 bg-gray-800 border border-gray-600 rounded-lg px-4 py-2 text-sm font-mono text-white focus:outline-none focus:border-blue-500"
                        placeholder="Wireshark display filter: tcp, http, ip.addr==1.2.3.4..."
                        prop:value=filter
                        on:input=move |ev| filter.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" { load_packets(); }
                        }
                    />
                    <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm"
                        on:click=move |_| load_packets()
                    >"应用过滤"</button>
                    <a
                        href=move || format!("/api/captures/{}/download", capture_id())
                        class="bg-gray-700 hover:bg-gray-600 text-white px-4 py-2 rounded-lg text-sm"
                    >"下载 PCAP"</a>
                </div>

                {move || error.get().map(|e| view! {
                    <div class="bg-red-900/50 border border-red-700 text-red-300 px-4 py-2 rounded-lg text-sm mb-4">{e}</div>
                })}

                <div class="flex gap-4 h-[calc(100vh-220px)]">
                    // 数据包列表
                    <div class="flex-1 bg-gray-900 border border-gray-700 rounded-xl overflow-auto">
                        <table class="w-full text-xs">
                            <thead class="sticky top-0 bg-gray-800 border-b border-gray-700">
                                <tr>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-12">"No."</th>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-28">"时间"</th>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-32">"源地址"</th>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-32">"目标地址"</th>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-20">"协议"</th>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-16">"长度"</th>
                                    <th class="text-left px-3 py-2 text-gray-400 font-medium">"信息"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    if loading.get() {
                                        view! {
                                            <tr><td colspan="7" class="text-center py-8 text-gray-500">"加载中..."</td></tr>
                                        }.into_any()
                                    } else {
                                        packets.get().into_iter().map(|p| {
                                            let no = p.number;
                                            let is_selected = selected.get().as_ref().map(|s| s.number == no).unwrap_or(false);
                                            let proto_color = match p.protocol.as_str() {
                                                "TCP" => "text-blue-300",
                                                "UDP" => "text-green-300",
                                                "HTTP" | "HTTPS" => "text-orange-300",
                                                "DNS" => "text-purple-300",
                                                "ICMP" => "text-yellow-300",
                                                _ => "text-gray-300",
                                            };
                                            view! {
                                                <tr
                                                    class=move || if is_selected {
                                                        "bg-blue-900/50 cursor-pointer"
                                                    } else {
                                                        "hover:bg-gray-800 cursor-pointer"
                                                    }
                                                    on:click=move |_| on_select(no)
                                                >
                                                    <td class="px-3 py-1.5 text-gray-500">{p.number}</td>
                                                    <td class="px-3 py-1.5 font-mono text-gray-400">{p.time}</td>
                                                    <td class="px-3 py-1.5 font-mono">{p.source}</td>
                                                    <td class="px-3 py-1.5 font-mono">{p.destination}</td>
                                                    <td class=format!("px-3 py-1.5 font-medium {}", proto_color)>{p.protocol}</td>
                                                    <td class="px-3 py-1.5 text-gray-400">{p.length}</td>
                                                    <td class="px-3 py-1.5 text-gray-300 truncate max-w-xs">{p.info}</td>
                                                </tr>
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </tbody>
                        </table>
                    </div>

                    // 数据包详情面板
                    <div class="w-96 bg-gray-900 border border-gray-700 rounded-xl overflow-auto p-4">
                        {move || match selected.get() {
                            None => view! {
                                <div class="text-center text-gray-500 mt-12">
                                    <div class="text-3xl mb-2">"🔍"</div>
                                    <p class="text-sm">"点击数据包查看详情"</p>
                                </div>
                            }.into_any(),
                            Some(detail) => view! {
                                <div>
                                    <h3 class="font-medium mb-3">"数据包 #"{detail.number}</h3>
                                    <div class="mb-4">
                                        <p class="text-xs text-gray-400 mb-2">"协议树"</p>
                                        <pre class="text-xs font-mono text-gray-300 bg-gray-800 rounded p-2 overflow-auto max-h-64">
                                            {serde_json::to_string_pretty(&detail.layers).unwrap_or_default()}
                                        </pre>
                                    </div>
                                    <div>
                                        <p class="text-xs text-gray-400 mb-2">"原始数据 (Hex)"</p>
                                        <pre class="text-xs font-mono text-green-400 bg-gray-800 rounded p-2 overflow-auto max-h-48">
                                            {detail.raw}
                                        </pre>
                                    </div>
                                </div>
                            }.into_any(),
                        }}
                    </div>
                </div>
            </div>
        </Layout>
    }
}
