use crate::components::layout::Layout;
use crate::store::use_auth;
use leptos::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
struct Stats {
    servers_total: i64,
    servers_online: i64,
    captures_total: i64,
    captures_running: i64,
    // Optional extended fields — gracefully absent if API doesn't return them
    #[allow(dead_code)]
    total_bytes_captured: Option<i64>,
}

#[derive(Deserialize, Clone)]
struct RecentCapture {
    id: String,
    interface: String,
    status: String,
    file_size: Option<i64>,
    created_at: String,
    #[allow(dead_code)]
    finished_at: Option<String>,
}

/// Parse "YYYY-MM-DD HH:MM:SS" → approximate seconds since epoch
fn parse_datetime_secs(s: &str) -> Option<f64> {
    if s.len() < 19 {
        return None;
    }
    let year: f64 = s[0..4].parse().ok()?;
    let month: f64 = s[5..7].parse().ok()?;
    let day: f64 = s[8..10].parse().ok()?;
    let hour: f64 = s[11..13].parse().ok()?;
    let min: f64 = s[14..16].parse().ok()?;
    let sec: f64 = s[17..19].parse().ok()?;
    let approx_days = (year - 1970.0) * 365.25 + (month - 1.0) * 30.44 + day - 1.0;
    Some(approx_days * 86400.0 + hour * 3600.0 + min * 60.0 + sec)
}

fn relative_time(datetime_str: &str) -> String {
    let now = js_sys::Date::now() / 1000.0;
    match parse_datetime_secs(datetime_str) {
        Some(then) => {
            let diff = now - then;
            if diff < 60.0 {
                "刚刚".to_string()
            } else if diff < 3600.0 {
                format!("{}分钟前", (diff / 60.0) as i64)
            } else if diff < 86400.0 {
                format!("{}小时前", (diff / 3600.0) as i64)
            } else {
                format!("{}天前", (diff / 86400.0) as i64)
            }
        }
        None => "最近".to_string(),
    }
}

fn format_bytes(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = use_auth();

    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            let window = web_sys::window().unwrap();
            window.location().set_href("/login").ok();
        }
    });

    let stats = LocalResource::new(|| async { crate::api::get::<Stats>("/api/stats").await.ok() });

    let recent_captures = LocalResource::new(|| async {
        crate::api::get::<Vec<RecentCapture>>("/api/captures")
            .await
            .ok()
    });

    view! {
        <Layout>
            <div class="max-w-6xl mx-auto">
                <h1 class="text-2xl font-bold mb-6">"仪表盘"</h1>

                // ── Stat Cards ────────────────────────────────────────────
                <Suspense fallback=|| view! { <div class="text-gray-400">"加载中..."</div> }>
                    {move || {
                        let s = stats.get();
                        view! {
                            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-8">
                                <StatCard
                                    title="服务器总数"
                                    value=s.as_ref().and_then(|s| s.as_ref()).map(|s| s.servers_total).unwrap_or(0).to_string()
                                    icon="🖥️"
                                    color="blue"
                                />
                                <StatCard
                                    title="在线服务器"
                                    value=s.as_ref().and_then(|s| s.as_ref()).map(|s| s.servers_online).unwrap_or(0).to_string()
                                    icon="✅"
                                    color="green"
                                />
                                <StatCard
                                    title="抓包任务"
                                    value=s.as_ref().and_then(|s| s.as_ref()).map(|s| s.captures_total).unwrap_or(0).to_string()
                                    icon="📦"
                                    color="purple"
                                />
                                <StatCard
                                    title="正在抓包"
                                    value=s.as_ref().and_then(|s| s.as_ref()).map(|s| s.captures_running).unwrap_or(0).to_string()
                                    icon="🔴"
                                    color="red"
                                />
                            </div>
                        }
                    }}
                </Suspense>

                // ── Quick Actions ─────────────────────────────────────────
                <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-10">
                    <QuickAction
                        title="添加服务器"
                        desc="连接新的远端服务器"
                        icon="➕"
                        href="/servers"
                    />
                    <QuickAction
                        title="开始抓包"
                        desc="在已连接的服务器上抓取网络数据包"
                        icon="🎯"
                        href="/captures"
                    />
                    <QuickAction
                        title="管理SSH密钥"
                        desc="添加和管理服务器认证密钥"
                        icon="🔑"
                        href="/keys"
                    />
                    <QuickAction
                        title="分析数据包"
                        desc="在浏览器中分析已捕获的数据包"
                        icon="🔬"
                        href="/analysis"
                    />
                </div>

                // ══════════════════════════════════════════════════════════
                // Analytics Section
                // ══════════════════════════════════════════════════════════
                <div class="mb-4">
                    <h2 class="text-lg font-semibold text-white border-l-4 border-blue-500 pl-3">
                        "抓包分析"
                    </h2>
                </div>

                <Suspense fallback=|| view! { <div class="text-gray-500 text-sm py-4">"加载分析数据..."</div> }>
                    {move || {
                        let s = stats.get();
                        let stats_ref = s.as_ref().and_then(|s| s.as_ref());

                        let total_bytes = stats_ref.and_then(|s| s.total_bytes_captured).unwrap_or(0);

                        // Derive all status counts from the full capture list (more accurate than Stats API)
                        let caps_list = recent_captures.get()
                            .as_ref()
                            .and_then(|c| c.as_ref())
                            .cloned()
                            .unwrap_or_default();
                        let total     = if caps_list.is_empty() {
                            stats_ref.map(|s| s.captures_total).unwrap_or(0)
                        } else {
                            caps_list.len() as i64
                        };
                        let running   = caps_list.iter().filter(|c| c.status == "running").count() as i64;
                        let done      = caps_list.iter().filter(|c| c.status == "done").count() as i64;
                        let cancelled = caps_list.iter().filter(|c| c.status == "cancelled").count() as i64;
                        let failed    = caps_list.iter().filter(|c| c.status == "failed").count() as i64;

                        let denominator = if total > 0 { total } else { 1 };

                        view! {
                            <div>
                                // Row 1: Donut chart + Bar chart
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
                                    // ── Donut Chart ──────────────────────
                                    <div class="bg-gray-900/80 border border-gray-700/50 rounded-2xl p-6">
                                        <h3 class="text-sm font-semibold text-gray-400 mb-4">
                                            "🍩 抓包状态分布"
                                        </h3>
                                        <div class="flex items-center gap-6">
                                            <DonutChart
                                                total=total
                                                done=done
                                                running=running
                                                cancelled=cancelled
                                                failed=failed
                                            />
                                            <div class="flex flex-col gap-2 text-sm">
                                                <div class="flex items-center gap-2">
                                                    <span class="w-3 h-3 rounded-full bg-green-500 shrink-0"></span>
                                                    <span class="text-gray-300">"完成"</span>
                                                    <span class="text-green-400 font-mono font-bold ml-1">{done}</span>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <span class="w-3 h-3 rounded-full bg-blue-500 shrink-0"></span>
                                                    <span class="text-gray-300">"运行中"</span>
                                                    <span class="text-blue-400 font-mono font-bold ml-1">{running}</span>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <span class="w-3 h-3 rounded-full bg-gray-500 shrink-0"></span>
                                                    <span class="text-gray-300">"已取消"</span>
                                                    <span class="text-gray-400 font-mono font-bold ml-1">{cancelled}</span>
                                                </div>
                                                <div class="flex items-center gap-2">
                                                    <span class="w-3 h-3 rounded-full bg-red-500 shrink-0"></span>
                                                    <span class="text-gray-300">"失败"</span>
                                                    <span class="text-red-400 font-mono font-bold ml-1">{failed}</span>
                                                </div>
                                            </div>
                                        </div>
                                    </div>

                                    // ── Horizontal Bar Chart ─────────────
                                    <div class="bg-gray-900/80 border border-gray-700/50 rounded-2xl p-6">
                                        <h3 class="text-sm font-semibold text-gray-400 mb-4">
                                            "📊 各状态数量"
                                        </h3>
                                        <div class="space-y-4">
                                            <BarRow
                                                label="完成"
                                                count=done
                                                total=denominator
                                                color="#22c55e"
                                                text_color="text-green-400"
                                            />
                                            <BarRow
                                                label="运行中"
                                                count=running
                                                total=denominator
                                                color="#3b82f6"
                                                text_color="text-blue-400"
                                            />
                                            <BarRow
                                                label="已取消"
                                                count=cancelled
                                                total=denominator
                                                color="#6b7280"
                                                text_color="text-gray-400"
                                            />
                                            <BarRow
                                                label="失败"
                                                count=failed
                                                total=denominator
                                                color="#ef4444"
                                                text_color="text-red-400"
                                            />
                                        </div>
                                    </div>
                                </div>

                                // Row 2: Storage overview
                                <div class="bg-gray-900/80 border border-gray-700/50 rounded-2xl p-6 mb-6">
                                    <h3 class="text-sm font-semibold text-gray-400 mb-4">"💾 存储概览"</h3>
                                    <StorageOverview total_bytes=total_bytes />
                                </div>
                            </div>
                        }
                    }}
                </Suspense>

                // ── Recent Activity ───────────────────────────────────────
                <div class="mb-4 mt-2">
                    <h2 class="text-lg font-semibold text-white border-l-4 border-purple-500 pl-3">
                        "最近抓包活动"
                    </h2>
                </div>

                <Suspense fallback=|| view! { <div class="text-gray-500 text-sm py-4">"加载活动记录..."</div> }>
                    {move || {
                        let caps = recent_captures.get();
                        let list: Vec<RecentCapture> = caps
                            .as_ref()
                            .and_then(|c| c.as_ref())
                            .cloned()
                            .unwrap_or_default();
                        let display: Vec<RecentCapture> = list.into_iter().take(5).collect();

                        if display.is_empty() {
                            view! {
                                <div class="bg-gray-900/80 border border-gray-700/50 rounded-2xl p-8 text-center mb-8">
                                    <div class="text-3xl mb-2">"📭"</div>
                                    <p class="text-gray-500 text-sm">"暂无抓包活动记录"</p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="bg-gray-900/80 border border-gray-700/50 rounded-2xl overflow-hidden mb-8">
                                    <div class="divide-y divide-gray-700/50">
                                        {display.into_iter().map(|cap| {
                                            let rel = relative_time(&cap.created_at);
                                            let (icon, color) = match cap.status.as_str() {
                                                "done"      => ("✓", "text-green-400"),
                                                "running"   => ("⟳", "text-blue-400"),
                                                "cancelled" => ("⊘", "text-gray-400"),
                                                "failed"    => ("✗", "text-red-400"),
                                                _           => ("○", "text-yellow-400"),
                                            };
                                            let size_str = cap.file_size
                                                .map(format_bytes)
                                                .unwrap_or_else(|| "—".to_string());
                                            view! {
                                                <div class="flex items-center gap-4 px-6 py-4 hover:bg-gray-800/40 transition-colors">
                                                    <span class=format!("text-lg font-bold w-6 text-center shrink-0 {}", color)>
                                                        {icon}
                                                    </span>
                                                    <span class="font-mono text-sm text-gray-200 flex-1 truncate">
                                                        {cap.interface.clone()}
                                                    </span>
                                                    <span class="text-xs text-gray-400 shrink-0">{rel}</span>
                                                    <span class="text-xs font-mono text-gray-500 shrink-0 w-20 text-right">
                                                        {size_str}
                                                    </span>
                                                    <a
                                                        href=format!("/captures/{}", cap.id)
                                                        class="text-xs text-blue-500 hover:text-blue-400 shrink-0 transition-colors"
                                                    >
                                                        "详情 →"
                                                    </a>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }}
                </Suspense>

            </div>
        </Layout>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DonutChart — pure SVG stroke-dasharray donut
// ─────────────────────────────────────────────────────────────────────────────
#[component]
fn DonutChart(total: i64, done: i64, running: i64, cancelled: i64, failed: i64) -> impl IntoView {
    // r=45, circumference = 2π*45 ≈ 282.74
    let circ: f64 = 2.0 * std::f64::consts::PI * 45.0;

    let denom = if total > 0 { total as f64 } else { 1.0 };
    let p_done = done as f64 / denom;
    let p_running = running as f64 / denom;
    let p_cancelled = cancelled as f64 / denom;
    let p_failed = failed as f64 / denom;

    let arc_done = p_done * circ;
    let arc_running = p_running * circ;
    let arc_cancelled = p_cancelled * circ;
    let arc_failed = p_failed * circ;

    // Start from top (rotate-90 is applied via transform on each segment).
    // stroke-dashoffset moves the start of the dash.
    // Offset for each segment = circ/4 - cumulative prior arcs
    let q = circ / 4.0;
    let off_done = q;
    let off_running = q - arc_done;
    let off_cancelled = q - arc_done - arc_running;
    let off_failed = q - arc_done - arc_running - arc_cancelled;

    let da = |arc: f64| format!("{:.2} {:.2}", arc, circ - arc);
    let do_ = |off: f64| format!("{:.2}", off);

    let is_empty = total == 0;

    view! {
        <svg width="120" height="120" viewBox="0 0 120 120" class="shrink-0">
            // Dark track ring
            <circle cx="60" cy="60" r="45" fill="none" stroke="#1f2937" stroke-width="14"/>

            {if is_empty {
                view! {
                    <circle cx="60" cy="60" r="45" fill="none" stroke="#374151" stroke-width="14"/>
                }.into_any()
            } else {
                view! {
                    // done — green
                    <circle cx="60" cy="60" r="45" fill="none" stroke="#22c55e" stroke-width="14"
                        stroke-dasharray=da(arc_done)
                        stroke-dashoffset=do_(off_done)
                        transform="rotate(-90 60 60)"
                        style="transition: stroke-dasharray 0.6s ease"
                    />
                    // running — blue
                    <circle cx="60" cy="60" r="45" fill="none" stroke="#3b82f6" stroke-width="14"
                        stroke-dasharray=da(arc_running)
                        stroke-dashoffset=do_(off_running)
                        transform="rotate(-90 60 60)"
                        style="transition: stroke-dasharray 0.6s ease"
                    />
                    // cancelled — gray
                    <circle cx="60" cy="60" r="45" fill="none" stroke="#6b7280" stroke-width="14"
                        stroke-dasharray=da(arc_cancelled)
                        stroke-dashoffset=do_(off_cancelled)
                        transform="rotate(-90 60 60)"
                        style="transition: stroke-dasharray 0.6s ease"
                    />
                    // failed — red
                    <circle cx="60" cy="60" r="45" fill="none" stroke="#ef4444" stroke-width="14"
                        stroke-dasharray=da(arc_failed)
                        stroke-dashoffset=do_(off_failed)
                        transform="rotate(-90 60 60)"
                        style="transition: stroke-dasharray 0.6s ease"
                    />
                }.into_any()
            }}

            // Center: total count label
            <text x="60" y="56" text-anchor="middle" fill="white"
                  font-size="16" font-weight="bold" font-family="monospace">
                {total.to_string()}
            </text>
            <text x="60" y="70" text-anchor="middle" fill="#6b7280"
                  font-size="9" font-family="sans-serif">
                "总任务"
            </text>
        </svg>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BarRow — one labeled horizontal SVG bar
// ─────────────────────────────────────────────────────────────────────────────
#[component]
fn BarRow(
    label: &'static str,
    count: i64,
    total: i64,
    color: &'static str,
    text_color: &'static str,
) -> impl IntoView {
    let pct = if total > 0 {
        (count as f64 / total as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    let bar_max: f64 = 200.0;
    let bar_w = (pct / 100.0 * bar_max).max(if count > 0 { 4.0 } else { 0.0 });

    view! {
        <div class="flex items-center gap-3">
            <span class="text-xs text-gray-400 w-14 shrink-0">{label}</span>
            <div class="flex-1">
                <svg width="100%" height="18" viewBox="0 0 200 18"
                     preserveAspectRatio="none" class="overflow-visible">
                    // Track
                    <rect x="0" y="4" width="200" height="10" rx="5" fill="#1f2937"/>
                    // Filled bar
                    <rect x="0" y="4"
                        width=format!("{:.2}", bar_w)
                        height="10" rx="5" fill=color
                        style="transition: width 0.6s ease"
                    />
                    // Grid lines at 25 / 50 / 75 %
                    <line x1="50"  y1="2" x2="50"  y2="16" stroke="#374151" stroke-width="1"/>
                    <line x1="100" y1="2" x2="100" y2="16" stroke="#374151" stroke-width="1"/>
                    <line x1="150" y1="2" x2="150" y2="16" stroke="#374151" stroke-width="1"/>
                </svg>
            </div>
            <span class="text-xs text-gray-500 w-10 text-right shrink-0">
                {format!("{:.0}%", pct)}
            </span>
            <span class=format!("text-xs font-mono font-bold w-8 text-right shrink-0 {}", text_color)>
                {count.to_string()}
            </span>
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// StorageOverview — big number + SVG gradient fill bar
// ─────────────────────────────────────────────────────────────────────────────
#[component]
fn StorageOverview(total_bytes: i64) -> impl IntoView {
    let display = format_bytes(total_bytes);
    // Reference "capacity" for the bar: 1 GB
    let cap: f64 = 1024.0 * 1024.0 * 1024.0;
    let fill_pct = ((total_bytes as f64 / cap) * 100.0).min(100.0);
    let fill_w = fill_pct / 100.0 * 300.0;

    view! {
        <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:gap-10">
            <div class="shrink-0">
                <p class="text-xs text-gray-500 mb-1">"已捕获数据总量"</p>
                <p class="text-3xl font-bold text-blue-400 font-mono">{display}</p>
            </div>
            <div class="flex-1">
                <svg width="100%" height="28" viewBox="0 0 300 28" preserveAspectRatio="none">
                    <rect x="0" y="8" width="300" height="12" rx="6" fill="#1f2937"/>
                    <defs>
                        <linearGradient id="storage-grad" x1="0%" y1="0%" x2="100%" y2="0%">
                            <stop offset="0%"   stop-color="#3b82f6"/>
                            <stop offset="100%" stop-color="#06b6d4"/>
                        </linearGradient>
                    </defs>
                    <rect x="0" y="8"
                        width=format!("{:.2}", fill_w.max(if total_bytes > 0 { 6.0 } else { 0.0 }))
                        height="12" rx="6" fill="url(#storage-grad)"
                        style="transition: width 0.8s ease"
                    />
                    // Grid lines at 25 / 50 / 75 %
                    <line x1="75"  y1="5" x2="75"  y2="23" stroke="#374151" stroke-width="1"/>
                    <line x1="150" y1="5" x2="150" y2="23" stroke="#374151" stroke-width="1"/>
                    <line x1="225" y1="5" x2="225" y2="23" stroke="#374151" stroke-width="1"/>
                    <text x="295" y="20" text-anchor="end" fill="#4b5563"
                          font-size="8" font-family="monospace">
                        {format!("{:.1}% / 1GB", fill_pct)}
                    </text>
                </svg>
            </div>
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// StatCard
// ─────────────────────────────────────────────────────────────────────────────
#[component]
fn StatCard(
    title: &'static str,
    value: String,
    icon: &'static str,
    color: &'static str,
) -> impl IntoView {
    let bg = match color {
        "green" => "bg-green-900/30 border-green-700",
        "purple" => "bg-purple-900/30 border-purple-700",
        "red" => "bg-red-900/30 border-red-700",
        _ => "bg-blue-900/30 border-blue-700",
    };

    view! {
        <div class=format!("rounded-xl p-5 border {} ", bg)>
            <div class="flex items-center justify-between">
                <div>
                    <p class="text-gray-400 text-sm">{title}</p>
                    <p class="text-3xl font-bold mt-1">{value}</p>
                </div>
                <div class="text-3xl">{icon}</div>
            </div>
        </div>
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// QuickAction
// ─────────────────────────────────────────────────────────────────────────────
#[component]
fn QuickAction(
    title: &'static str,
    desc: &'static str,
    icon: &'static str,
    href: &'static str,
) -> impl IntoView {
    view! {
        <a
            href=href
            class="bg-gray-800 border border-gray-700 rounded-xl p-5 flex items-center gap-4 hover:border-blue-600 hover:bg-gray-750 transition-all group"
        >
            <div class="text-3xl">{icon}</div>
            <div>
                <p class="font-medium group-hover:text-blue-400 transition-colors">{title}</p>
                <p class="text-sm text-gray-400 mt-0.5">{desc}</p>
            </div>
        </a>
    }
}
