use crate::store::{use_auth, AuthState};
use leptos::prelude::*;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let auth = use_auth();
    let collapsed = RwSignal::new(false);

    // ESC 键控制侧边栏折叠
    let collapsed_clone = collapsed;
    let _ = window_event_listener(leptos::ev::keydown, move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            collapsed_clone.update(|c| *c = !*c);
        }
    });

    let toggle = move |_| collapsed.update(|c| *c = !*c);

    view! {
        <div class="flex min-h-screen bg-gray-900 text-gray-100">
            // ── 侧边栏 ──────────────────────────────────────────
            <aside class=move || {
                if collapsed.get() {
                    "w-16 min-h-screen bg-gray-800 border-r border-gray-700 flex flex-col transition-all duration-200 overflow-hidden"
                } else {
                    "w-56 min-h-screen bg-gray-800 border-r border-gray-700 flex flex-col transition-all duration-200"
                }
            }>
                // Logo + 折叠按钮
                <div class="flex items-center justify-between px-3 py-4 border-b border-gray-700">
                    {move || if collapsed.get() {
                        view! {
                            <a href="/" class="text-blue-400 text-xl font-bold mx-auto">"🦈"</a>
                        }.into_any()
                    } else {
                        view! {
                            <a href="/" class="text-blue-400 font-bold flex items-center gap-2 text-sm">
                                <span>"🦈"</span>
                                <span>"RustShark"</span>
                            </a>
                        }.into_any()
                    }}
                    <button
                        class="text-gray-400 hover:text-gray-200 transition-colors ml-auto"
                        title="ESC 切换侧边栏"
                        on:click=toggle
                    >
                        {move || if collapsed.get() {
                            view! { <span class="text-xs">"▶"</span> }.into_any()
                        } else {
                            view! { <span class="text-xs">"◀"</span> }.into_any()
                        }}
                    </button>
                </div>

                // 导航菜单
                <nav class="flex-1 py-3 space-y-1">
                    <NavItem href="/" icon="📊" label="仪表盘" collapsed=collapsed/>
                    <NavItem href="/servers" icon="🖥" label="服务器" collapsed=collapsed/>
                    <NavItem href="/keys" icon="🔑" label="SSH密钥" collapsed=collapsed/>
                    <NavItem href="/capture-profiles" icon="📋" label="抓包配置" collapsed=collapsed/>
                    <NavItem href="/captures" icon="📡" label="抓包任务" collapsed=collapsed/>
                    <NavItem href="/analysis" icon="🔍" label="包分析" collapsed=collapsed/>
                    <NavItem href="/capture-guide" icon="📖" label="过滤器指南" collapsed=collapsed/>
                    {move || auth.get().is_admin().then(|| view! {
                        <NavItem href="/admin" icon="⚙" label="用户管理" collapsed=collapsed/>
                    })}
                </nav>

                // 底部用户区域
                <div class="border-t border-gray-700 px-3 py-3 space-y-2">
                    {move || if collapsed.get() {
                        view! {
                            <div class="flex flex-col items-center gap-2">
                                <a href="/profile" class="text-gray-400 hover:text-blue-400 transition-colors" title="个人中心">
                                    <span class="text-base">"👤"</span>
                                </a>
                                <button
                                    class="text-red-400 hover:text-red-300 transition-colors"
                                    title="退出"
                                    on:click=move |_| {
                                        AuthState::clear();
                                        auth.set(AuthState::default());
                                        let window = web_sys::window().unwrap();
                                        window.location().set_href("/login").ok();
                                    }
                                >
                                    <span class="text-base">"🚪"</span>
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <a href="/profile" class="flex items-center gap-2 text-xs text-gray-400 hover:text-blue-400 transition-colors truncate">
                                <span>"👤"</span>
                                <span class="truncate">{move || auth.get().email.clone().unwrap_or_default()}</span>
                            </a>
                            <button
                                class="w-full flex items-center gap-2 text-xs text-red-400 hover:text-red-300 transition-colors"
                                on:click=move |_| {
                                    AuthState::clear();
                                    auth.set(AuthState::default());
                                    let window = web_sys::window().unwrap();
                                    window.location().set_href("/login").ok();
                                }
                            >
                                <span>"🚪"</span>
                                <span>"退出登录"</span>
                            </button>
                        }.into_any()
                    }}
                </div>
            </aside>

            // ── 主内容 ──────────────────────────────────────────
            <main class="flex-1 p-6 overflow-auto">
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
) -> impl IntoView {
    view! {
        <a
            href=href
            class="flex items-center gap-3 px-3 py-2 mx-1 rounded-md text-sm text-gray-300 hover:bg-gray-700 hover:text-white transition-colors"
        >
            <span class="text-base shrink-0">{icon}</span>
            {move || if !collapsed.get() {
                Some(view! { <span>{label}</span> })
            } else {
                None
            }}
        </a>
    }
}
