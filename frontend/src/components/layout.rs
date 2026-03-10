use crate::store::{toggle_theme, use_auth, use_dark_mode, AuthState};
use leptos::prelude::*;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let auth = use_auth();
    let dark = use_dark_mode();
    let collapsed = RwSignal::new(false);

    // ESC 键控制侧边栏折叠
    let collapsed_clone = collapsed;
    let _ = window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            collapsed_clone.update(|c| *c = !*c);
        }
    });

    let toggle_sidebar = move |_| collapsed.update(|c| *c = !*c);

    view! {
        // 根容器：深/浅色背景由 dark class 控制
        <div class=move || {
            if dark.get() {
                "flex min-h-screen bg-gray-900 text-gray-100"
            } else {
                "flex min-h-screen bg-gray-50 text-gray-900"
            }
        }>
            // ── 侧边栏 ──────────────────────────────────────────
            <aside class=move || {
                let base = "min-h-screen flex flex-col transition-all duration-200 border-r";
                let width = if collapsed.get() { "w-16 overflow-hidden" } else { "w-56" };
                let colors = if dark.get() {
                    "bg-gray-800 border-gray-700"
                } else {
                    "bg-white border-gray-200 shadow-sm"
                };
                format!("{base} {width} {colors}")
            }>
                // ── Logo + 折叠按钮 ──────────────────────────────
                <div class=move || {
                    let base = "flex items-center justify-between px-3 py-4 border-b";
                    let colors = if dark.get() { "border-gray-700" } else { "border-gray-200" };
                    format!("{base} {colors}")
                }>
                    {move || if collapsed.get() {
                        view! {
                            <a href="/" class="text-blue-500 text-xl font-bold mx-auto">"🦈"</a>
                        }.into_any()
                    } else {
                        view! {
                            <a href="/" class="text-blue-500 font-bold flex items-center gap-2 text-sm">
                                <span>"🦈"</span>
                                <span>"RustShark"</span>
                            </a>
                        }.into_any()
                    }}
                    <button
                        class=move || {
                            let color = if dark.get() { "text-gray-400 hover:text-gray-200" } else { "text-gray-400 hover:text-gray-600" };
                            format!("{color} transition-colors ml-auto text-xs")
                        }
                        title="ESC 切换侧边栏"
                        on:click=toggle_sidebar
                    >
                        {move || if collapsed.get() { "▶" } else { "◀" }}
                    </button>
                </div>

                // ── 导航菜单 ─────────────────────────────────────
                <nav class="flex-1 py-3 space-y-0.5">
                    <NavItem href="/" icon="📊" label="仪表盘" collapsed=collapsed dark=dark/>
                    <NavItem href="/servers" icon="🖥" label="服务器" collapsed=collapsed dark=dark/>
                    <NavItem href="/keys" icon="🔑" label="SSH密钥" collapsed=collapsed dark=dark/>
                    <NavItem href="/capture-profiles" icon="📋" label="抓包配置" collapsed=collapsed dark=dark/>
                    <NavItem href="/captures" icon="📡" label="抓包任务" collapsed=collapsed dark=dark/>
                    <NavItem href="/analysis" icon="🔍" label="包分析" collapsed=collapsed dark=dark/>
                    <NavItem href="/capture-guide" icon="📖" label="过滤器指南" collapsed=collapsed dark=dark/>
                    {move || auth.get().is_admin().then(|| view! {
                        <NavItem href="/admin" icon="⚙" label="用户管理" collapsed=collapsed dark=dark/>
                    })}
                </nav>

                // ── 底部区域 ─────────────────────────────────────
                <div class=move || {
                    let base = "border-t";
                    let colors = if dark.get() { "border-gray-700" } else { "border-gray-200" };
                    format!("{base} {colors}")
                }>
                    // 主题切换行
                    <div class=move || {
                        let base = "flex items-center px-3 py-2.5";
                        let border = if dark.get() { "border-b border-gray-700/60" } else { "border-b border-gray-100" };
                        format!("{base} {border}")
                    }>
                        {move || if collapsed.get() {
                            view! {
                                <button
                                    class=move || {
                                        let color = if dark.get() {
                                            "text-yellow-400 hover:text-yellow-300"
                                        } else {
                                            "text-gray-500 hover:text-gray-700"
                                        };
                                        format!("mx-auto {color} transition-colors text-base")
                                    }
                                    title="切换主题"
                                    on:click=move |_| toggle_theme()
                                >
                                    {move || if dark.get() { "☀" } else { "🌙" }}
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <span class=move || {
                                    if dark.get() { "text-xs text-gray-400 flex-1" } else { "text-xs text-gray-500 flex-1" }
                                }>
                                    {move || if dark.get() { "深色模式" } else { "浅色模式" }}
                                </span>
                                // Toggle Switch
                                <button
                                    class=move || {
                                        let base = "relative inline-flex h-5 w-9 items-center rounded-full transition-colors focus:outline-none";
                                        let color = if dark.get() { "bg-blue-600" } else { "bg-gray-300" };
                                        format!("{base} {color}")
                                    }
                                    on:click=move |_| toggle_theme()
                                    title="切换主题"
                                >
                                    <span class=move || {
                                        let base = "inline-block h-3.5 w-3.5 rounded-full bg-white shadow transform transition-transform";
                                        let pos = if dark.get() { "translate-x-[1.125rem]" } else { "translate-x-[0.125rem]" };
                                        format!("{base} {pos}")
                                    }/>
                                </button>
                            }.into_any()
                        }}
                    </div>

                    // 个人中心入口
                    <a
                        href="/profile"
                        class=move || {
                            let base = "flex items-center gap-2.5 px-3 py-2.5 w-full";
                            let colors = if dark.get() {
                                "text-gray-300 hover:bg-gray-700/60 hover:text-white"
                            } else {
                                "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
                            };
                            format!("{base} {colors} transition-colors")
                        }
                    >
                        // 头像占位圆圈
                        <span class=move || {
                            let base = "w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold shrink-0";
                            let colors = if dark.get() {
                                "bg-blue-600/30 text-blue-400"
                            } else {
                                "bg-blue-100 text-blue-600"
                            };
                            format!("{base} {colors}")
                        }>
                            {move || auth.get().email
                                .as_deref()
                                .and_then(|e| e.chars().next())
                                .map(|c| c.to_uppercase().to_string())
                                .unwrap_or_else(|| "U".to_string())
                            }
                        </span>
                        {move || if !collapsed.get() {
                            Some(view! {
                                <span class="text-xs truncate min-w-0">
                                    {move || auth.get().email.clone().unwrap_or_default()}
                                </span>
                            })
                        } else {
                            None
                        }}
                    </a>

                    // 退出登录
                    <button
                        class=move || {
                            let base = "flex items-center gap-2.5 px-3 py-2.5 w-full text-xs transition-colors";
                            let colors = if dark.get() {
                                "text-gray-500 hover:bg-gray-700/60 hover:text-red-400"
                            } else {
                                "text-gray-400 hover:bg-gray-100 hover:text-red-500"
                            };
                            format!("{base} {colors}")
                        }
                        on:click=move |_| {
                            AuthState::clear();
                            auth.set(AuthState::default());
                            web_sys::window().unwrap().location().set_href("/login").ok();
                        }
                    >
                        <span class="w-6 h-6 flex items-center justify-center shrink-0 text-sm">"↩"</span>
                        {move || if !collapsed.get() {
                            Some(view! { <span>"退出登录"</span> })
                        } else {
                            None
                        }}
                    </button>
                </div>
            </aside>

            // ── 主内容区 ─────────────────────────────────────────
            <main class="flex-1 p-6 overflow-auto min-w-0">
                {children()}
            </main>
        </div>
    }
}

#[component]
fn NavItem(
    href: &'static str,
    icon: &'static str,
    label: &'static str,
    collapsed: RwSignal<bool>,
    dark: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <a
            href=href
            class=move || {
                let base = "flex items-center gap-3 px-3 py-2 mx-1.5 rounded-md text-sm transition-colors";
                let colors = if dark.get() {
                    "text-gray-300 hover:bg-gray-700 hover:text-white"
                } else {
                    "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
                };
                format!("{base} {colors}")
            }
        >
            <span class="text-base shrink-0 w-5 text-center">{icon}</span>
            {move || if !collapsed.get() {
                Some(view! { <span>{label}</span> })
            } else {
                None
            }}
        </a>
    }
}
