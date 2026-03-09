use crate::components::layout::Layout;
use crate::components::select::{Select, SelectOption};
use crate::store::use_auth;
use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen::JsCast;

// ── Data structs ────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone, Debug)]
struct CaptureTask {
    id: String,
    #[allow(dead_code)]
    server_id: String,
    interface: String,
    #[allow(dead_code)]
    filter: Option<String>,
    #[allow(dead_code)]
    duration: Option<i64>,
    #[allow(dead_code)]
    packet_limit: Option<i64>,
    status: String,
    file_size: Option<i64>,
    #[allow(dead_code)]
    created_at: String,
    finished_at: Option<String>,
    #[allow(dead_code)]
    log_msg: Option<String>,
}

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

// ── Helper functions ─────────────────────────────────────────────────────────

fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Render a sharkd protocol tree node recursively.
/// Each node has: "l" (label), "t" (type/abbreviation), "n" (children array)
fn render_tree(node: &serde_json::Value, depth: usize) -> String {
    let mut out = String::new();
    let indent = "  ".repeat(depth);

    match node {
        serde_json::Value::Array(arr) => {
            for child in arr {
                out.push_str(&render_tree(child, depth));
            }
        }
        serde_json::Value::Object(map) => {
            let label = map.get("l").and_then(|v| v.as_str()).unwrap_or("");
            let abbr = map.get("t").and_then(|v| v.as_str()).unwrap_or("");

            if !label.is_empty() {
                let prefix = if depth == 0 { "▸ " } else { "  " };
                out.push_str(&format!("{}{}{}", indent, prefix, label));
                if !abbr.is_empty() && abbr != label {
                    out.push_str(&format!(" [{}]", abbr));
                }
                out.push('\n');
            }

            if let Some(children) = map.get("n").and_then(|v| v.as_array()) {
                for child in children {
                    out.push_str(&render_tree(child, depth + 1));
                }
            }
        }
        _ => {}
    }
    out
}

/// Format raw hex bytes (hex string from sharkd) into classic Wireshark hex dump
fn format_hex_dump(hex: &str) -> String {
    let hex = hex.trim();
    if hex.is_empty() {
        return String::new();
    }

    // Collect bytes from hex string (may be space-separated or continuous)
    let bytes: Vec<u8> = if hex.contains(' ') {
        hex.split_whitespace()
            .filter_map(|s| u8::from_str_radix(s, 16).ok())
            .collect()
    } else {
        (0..hex.len())
            .step_by(2)
            .filter_map(|i| {
                hex.get(i..i + 2)
                    .and_then(|s| u8::from_str_radix(s, 16).ok())
            })
            .collect()
    };

    if bytes.is_empty() {
        return hex.to_string();
    }

    let mut result = String::new();
    for (i, chunk) in bytes.chunks(16).enumerate() {
        // Offset
        result.push_str(&format!("{:04x}  ", i * 16));
        // Hex part
        for (j, b) in chunk.iter().enumerate() {
            result.push_str(&format!("{:02x} ", b));
            if j == 7 {
                result.push(' ');
            }
        }
        // Padding if last row is short
        if chunk.len() < 16 {
            let missing = 16 - chunk.len();
            for j in 0..missing {
                result.push_str("   ");
                if chunk.len() + j == 7 {
                    result.push(' ');
                }
            }
        }
        result.push(' ');
        // ASCII part
        for b in chunk {
            let ch = if *b >= 0x20 && *b < 0x7f {
                *b as char
            } else {
                '.'
            };
            result.push(ch);
        }
        result.push('\n');
    }
    result
}

// ── Page component ────────────────────────────────────────────────────────────

#[component]
pub fn AnalysisPage() -> impl IntoView {
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

    // Captures list (only "done" ones, sorted by finished_at desc)
    let captures = RwSignal::new(Vec::<CaptureTask>::new());
    let captures_loading = RwSignal::new(true);

    // Selected capture
    let selected_capture_id = RwSignal::new(Option::<String>::None);

    // Packet analysis state
    let packets = RwSignal::new(Vec::<PacketSummary>::new());
    let selected = RwSignal::new(Option::<PacketDetail>::None);
    let filter = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let total_shown = RwSignal::new(0u64);

    // ── Load captures on mount ────────────────────────────────────────────────
    leptos::task::spawn_local(async move {
        if let Ok(mut list) = crate::api::get::<Vec<CaptureTask>>("/api/captures").await {
            // Filter to only "done" captures
            list.retain(|c| c.status == "done");
            // Sort by finished_at descending (most recent first)
            list.sort_by(|a, b| {
                let fa = a.finished_at.as_deref().unwrap_or("");
                let fb = b.finished_at.as_deref().unwrap_or("");
                fb.cmp(fa)
            });
            captures.set(list);
        }
        captures_loading.set(false);
    });

    // ── Load packets for selected capture ────────────────────────────────────
    let load_packets = move || {
        let maybe_id = selected_capture_id.get();
        let id = match maybe_id {
            Some(id) => id,
            None => return,
        };
        loading.set(true);
        selected.set(None);
        let f = filter.get();
        leptos::task::spawn_local(async move {
            let mut path = format!("/api/captures/{}/packets?limit=1000", id);
            if !f.is_empty() {
                let encoded: String = f
                    .bytes()
                    .flat_map(|b| match b {
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                            vec![b as char]
                        }
                        _ => format!("%{:02X}", b).chars().collect::<Vec<_>>(),
                    })
                    .collect();
                path.push_str(&format!("&filter={}", encoded));
            }
            match crate::api::get::<Vec<PacketSummary>>(&path).await {
                Ok(list) => {
                    total_shown.set(list.len() as u64);
                    packets.set(list);
                    error.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    // ── React to capture selection changes ───────────────────────────────────
    Effect::new(move |_| {
        let maybe_id = selected_capture_id.get();
        if maybe_id.is_some() {
            // Reset state when capture changes
            packets.set(Vec::new());
            selected.set(None);
            filter.set(String::new());
            error.set(None);
            total_shown.set(0);
            load_packets();
        }
    });

    // ── Packet detail fetch ───────────────────────────────────────────────────
    let on_select = move |no: u64| {
        let maybe_id = selected_capture_id.get();
        if let Some(id) = maybe_id {
            leptos::task::spawn_local(async move {
                let path = format!("/api/captures/{}/packets/{}", id, no);
                if let Ok(detail) = crate::api::get::<PacketDetail>(&path).await {
                    selected.set(Some(detail));
                }
            });
        }
    };

    // ── Download PCAP ─────────────────────────────────────────────────────────
    let on_download = move || {
        let maybe_id = selected_capture_id.get();
        if let Some(id) = maybe_id {
            leptos::task::spawn_local(async move {
                let url = format!("/api/captures/{}/download", id);
                let token = crate::api::auth_header();
                let resp = gloo_net::http::Request::get(&url)
                    .header("Authorization", &token)
                    .send()
                    .await;
                if let Ok(r) = resp {
                    if r.ok() {
                        if let Ok(bytes) = r.binary().await {
                            let uint8 = js_sys::Uint8Array::from(bytes.as_slice());
                            let array = js_sys::Array::new();
                            array.push(&uint8.buffer());
                            let blob = web_sys::Blob::new_with_u8_array_sequence(&array).unwrap();
                            let obj_url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
                            let doc = web_sys::window().unwrap().document().unwrap();
                            let a = doc.create_element("a").unwrap();
                            a.set_attribute("href", &obj_url).ok();
                            a.set_attribute("download", &format!("capture-{}.pcap", id))
                                .ok();
                            let body = doc.body().unwrap();
                            body.append_child(&a).ok();
                            a.unchecked_ref::<web_sys::HtmlElement>().click();
                            body.remove_child(&a).ok();
                            web_sys::Url::revoke_object_url(&obj_url).ok();
                        }
                    }
                }
            });
        }
    };

    view! {
        <Layout>
            <div class="max-w-full">
                // ── 顶栏 ──────────────────────────────────────────────────────
                <div class="flex items-center gap-4 mb-4">
                    <h1 class="text-xl font-bold">"数据包分析"</h1>
                    {move || {
                        let id = selected_capture_id.get();
                        let pkts = packets.get();
                        if id.is_some() && !loading.get() && !pkts.is_empty() {
                            view! {
                                <span class="text-xs text-gray-500">
                                    "共 " {total_shown.get()} " 个包"
                                </span>
                            }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        }
                    }}
                    <a
                        href="/capture-guide"
                        class="ml-auto text-xs text-blue-400 hover:text-blue-300 transition-colors border border-blue-800 hover:border-blue-600 rounded px-2 py-1"
                    >
                        "📖 过滤器指南"
                    </a>
                </div>

                // ── 抓包文件选择器 ─────────────────────────────────────────────
                <div class="mb-4">
                    {move || {
                        if captures_loading.get() {
                            view! {
                                <div class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 text-sm text-gray-500">
                                    "正在加载抓包文件列表…"
                                </div>
                            }.into_any()
                        } else if captures.get().is_empty() {
                            view! {
                                <div class="bg-gray-800 border border-gray-700 rounded-lg px-4 py-6 text-center">
                                    <div class="text-3xl mb-2">"📭"</div>
                                    <p class="text-sm text-gray-400">"暂无已完成的抓包文件"</p>
                                    <a
                                        href="/captures"
                                        class="inline-block mt-2 text-xs text-blue-400 hover:text-blue-300"
                                    >"前往抓包任务页面创建抓包 →"</a>
                                </div>
                            }.into_any()
                        } else {
                            let options = captures.get().into_iter().map(|cap| {
                                let finished = cap.finished_at
                                    .as_deref()
                                    .unwrap_or("未知时间")
                                    .to_string();
                                let size = cap.file_size
                                    .map(format_size)
                                    .unwrap_or_else(|| "未知大小".to_string());
                                let label = format!("{} · {} · {}", cap.interface, finished, size);
                                SelectOption::new(cap.id.clone(), label)
                            }).collect::<Vec<_>>();
                            view! {
                                <Select
                                    options=options
                                    value=Signal::derive(move || selected_capture_id.get().unwrap_or_default())
                                    on_change=Callback::new(move |v: String| {
                                        if v.is_empty() {
                                            selected_capture_id.set(None);
                                            packets.set(Vec::new());
                                            selected.set(None);
                                            filter.set(String::new());
                                            error.set(None);
                                            total_shown.set(0);
                                        } else {
                                            selected_capture_id.set(Some(v));
                                        }
                                    })
                                    placeholder="— 选择抓包文件 —"
                                />
                            }.into_any()
                        }
                    }}
                </div>

                // ── Only show analysis UI when a capture is selected ───────────
                {move || {
                    let maybe_id = selected_capture_id.get();
                    if maybe_id.is_none() {
                        // No capture selected yet — show placeholder
                        view! {
                            <div class="flex flex-col items-center justify-center py-24 text-gray-600">
                                <div class="text-6xl mb-4">"🦈"</div>
                                <p class="text-base">"请在上方选择一个抓包文件以开始分析"</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div>
                                // 过滤器栏
                                <div class="flex gap-2 mb-4">
                                    <input
                                        class="flex-1 bg-gray-800 border border-gray-600 rounded-lg px-4 py-2 text-sm font-mono text-white focus:outline-none focus:border-blue-500 placeholder-gray-500"
                                        placeholder="Wireshark 过滤器: tcp, http, ip.addr==1.2.3.4, tcp.port==443..."
                                        prop:value=filter
                                        on:input=move |ev| filter.set(event_target_value(&ev))
                                        on:keydown=move |ev| {
                                            if ev.key() == "Enter" { load_packets(); }
                                        }
                                    />
                                    <button
                                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                                        on:click=move |_| load_packets()
                                    >"应用过滤"</button>
                                    <button
                                        class="bg-gray-700 hover:bg-gray-600 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                                        on:click=move |_| {
                                            filter.set(String::new());
                                            load_packets();
                                        }
                                    >"清除"</button>
                                    <button
                                        class="bg-gray-700 hover:bg-gray-600 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                                        on:click=move |_| on_download()
                                    >"⬇ 下载 PCAP"</button>
                                </div>

                                {move || error.get().map(|e| view! {
                                    <div class="bg-red-900/50 border border-red-700 text-red-300 px-4 py-2 rounded-lg text-sm mb-4">{e}</div>
                                })}

                                // 主内容区：左列表 + 右详情
                                <div class="flex gap-4" style="height: calc(100vh - 260px)">
                                    // ── 左侧：数据包列表 ──────────────────────
                                    <div class="flex-1 bg-gray-900 border border-gray-700 rounded-xl overflow-auto min-w-0">
                                        <table class="w-full text-xs">
                                            <thead class="sticky top-0 bg-gray-800 border-b border-gray-700 z-10">
                                                <tr>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-12">"No."</th>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-32">"时间 (s)"</th>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-36">"源地址"</th>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-36">"目标地址"</th>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-20">"协议"</th>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium w-16">"长度"</th>
                                                    <th class="text-left px-3 py-2 text-gray-400 font-medium">"信息"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {move || {
                                                    if loading.get() {
                                                        view! {
                                                            <tr><td colspan="7" class="text-center py-12 text-gray-500">
                                                                <div class="flex flex-col items-center gap-2">
                                                                    <div class="text-2xl">"⏳"</div>
                                                                    <span>"加载中..."</span>
                                                                </div>
                                                            </td></tr>
                                                        }.into_any()
                                                    } else if packets.get().is_empty() {
                                                        view! {
                                                            <tr><td colspan="7" class="text-center py-12 text-gray-500">
                                                                <div class="flex flex-col items-center gap-2">
                                                                    <div class="text-2xl">"📭"</div>
                                                                    <span>"无数据包"</span>
                                                                    <span class="text-xs text-gray-600">"请确认 sharkd 已安装（brew install wireshark）"</span>
                                                                </div>
                                                            </td></tr>
                                                        }.into_any()
                                                    } else {
                                                        packets.get().into_iter().map(|p| {
                                                            let no = p.number;
                                                            let is_sel = selected.get()
                                                                .as_ref()
                                                                .map(|s| s.number == no)
                                                                .unwrap_or(false);
                                                            let proto_color = match p.protocol.as_str() {
                                                                "TCP"  => "text-blue-300",
                                                                "UDP"  => "text-green-300",
                                                                "HTTP" | "HTTPS" | "HTTP/2" => "text-orange-300",
                                                                "DNS"  => "text-purple-300",
                                                                "ICMP" | "ICMPv6" => "text-yellow-300",
                                                                "TLS" | "SSL"  => "text-teal-300",
                                                                "ARP"  => "text-pink-300",
                                                                _ => "text-gray-300",
                                                            };
                                                            let row_bg = if is_sel {
                                                                "bg-blue-900/60 cursor-pointer border-l-2 border-blue-500"
                                                            } else {
                                                                "hover:bg-gray-800/80 cursor-pointer border-l-2 border-transparent"
                                                            };
                                                            view! {
                                                                <tr
                                                                    class=row_bg
                                                                    on:click=move |_| on_select(no)
                                                                >
                                                                    <td class="px-3 py-1.5 text-gray-500 tabular-nums">{p.number}</td>
                                                                    <td class="px-3 py-1.5 font-mono text-gray-400 tabular-nums">{p.time}</td>
                                                                    <td class="px-3 py-1.5 font-mono text-gray-300">{p.source}</td>
                                                                    <td class="px-3 py-1.5 font-mono text-gray-300">{p.destination}</td>
                                                                    <td class=format!("px-3 py-1.5 font-medium font-mono {}", proto_color)>{p.protocol}</td>
                                                                    <td class="px-3 py-1.5 text-gray-400 tabular-nums">{p.length}</td>
                                                                    <td class="px-3 py-1.5 text-gray-300 truncate max-w-xs">{p.info}</td>
                                                                </tr>
                                                            }
                                                        }).collect::<Vec<_>>().into_any()
                                                    }
                                                }}
                                            </tbody>
                                        </table>
                                    </div>

                                    // ── 右侧：详情面板 ────────────────────────
                                    <div class="w-[560px] shrink-0 flex flex-col gap-3 overflow-auto">
                                        {move || match selected.get() {
                                            None => view! {
                                                <div class="flex-1 bg-gray-900 border border-gray-700 rounded-xl flex flex-col items-center justify-center text-gray-500 h-full">
                                                    <div class="text-5xl mb-3">"🔍"</div>
                                                    <p class="text-sm font-medium">"点击左侧数据包查看详情"</p>
                                                    <p class="text-xs text-gray-600 mt-1">"可查看协议树和原始字节"</p>
                                                </div>
                                            }.into_any(),
                                            Some(detail) => view! {
                                                <div class="flex flex-col gap-3 h-full">
                                                    // 包头信息
                                                    <div class="bg-gray-900 border border-gray-700 rounded-xl px-4 py-3 flex items-center gap-3">
                                                        <span class="text-blue-400 font-bold text-lg">"#"{detail.number}</span>
                                                        <span class="text-xs text-gray-500">"数据包详情"</span>
                                                    </div>

                                                    // 协议树
                                                    <div class="bg-gray-900 border border-gray-700 rounded-xl overflow-hidden flex flex-col" style="flex: 1 1 0; min-height: 200px">
                                                        <div class="px-4 py-2 bg-gray-800 border-b border-gray-700 flex items-center gap-2">
                                                            <span class="text-xs font-semibold text-gray-300 uppercase tracking-wide">"协议树"</span>
                                                            <span class="text-xs text-gray-600">"(Protocol Tree)"</span>
                                                        </div>
                                                        <div class="overflow-auto flex-1 p-3">
                                                            {
                                                                let tree_text = render_tree(&detail.layers, 0);
                                                                if tree_text.is_empty() {
                                                                    view! {
                                                                        <pre class="text-xs font-mono text-gray-500">"（无协议树数据）"</pre>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <pre class="text-xs font-mono text-green-300 leading-relaxed whitespace-pre-wrap">{tree_text}</pre>
                                                                    }.into_any()
                                                                }
                                                            }
                                                        </div>
                                                    </div>

                                                    // 原始字节
                                                    <div class="bg-gray-900 border border-gray-700 rounded-xl overflow-hidden flex flex-col" style="flex: 0 0 auto; max-height: 280px">
                                                        <div class="px-4 py-2 bg-gray-800 border-b border-gray-700 flex items-center gap-2">
                                                            <span class="text-xs font-semibold text-gray-300 uppercase tracking-wide">"原始字节"</span>
                                                            <span class="text-xs text-gray-600">"(Raw Bytes / Hex Dump)"</span>
                                                        </div>
                                                        <div class="overflow-auto p-3">
                                                            {
                                                                let hex_dump = format_hex_dump(&detail.raw);
                                                                if hex_dump.is_empty() {
                                                                    view! {
                                                                        <pre class="text-xs font-mono text-gray-500">"（无字节数据）"</pre>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <pre class="text-xs font-mono text-blue-300 leading-relaxed">{hex_dump}</pre>
                                                                    }.into_any()
                                                                }
                                                            }
                                                        </div>
                                                    </div>
                                                </div>
                                            }.into_any(),
                                        }}
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </Layout>
    }
}
