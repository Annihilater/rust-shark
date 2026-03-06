use leptos::prelude::*;
use serde::Deserialize;
use crate::components::layout::Layout;
use crate::store::use_auth;

#[derive(Deserialize, Clone)]
struct Stats {
    servers_total: i64,
    servers_online: i64,
    captures_total: i64,
    captures_running: i64,
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let auth = use_auth();

    // 重定向未登录用户
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            let window = web_sys::window().unwrap();
            window.location().set_href("/login").ok();
        }
    });

    let stats = LocalResource::new(|| async {
        crate::api::get::<Stats>("/api/stats").await.ok()
    });

    view! {
        <Layout>
            <div class="max-w-6xl mx-auto">
                <h1 class="text-2xl font-bold mb-6">"仪表盘"</h1>

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

                <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
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
                        href="/captures"
                    />
                </div>
            </div>
        </Layout>
    }
}

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
