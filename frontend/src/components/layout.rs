use crate::store::{use_auth, AuthState};
use leptos::prelude::*;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    let auth = use_auth();

    view! {
        <div class="min-h-screen bg-gray-900 text-gray-100">
            <nav class="bg-gray-800 border-b border-gray-700 px-6 py-3 flex items-center justify-between">
                <div class="flex items-center gap-8">
                    <a href="/" class="text-xl font-bold text-blue-400 flex items-center gap-2">
                        <span>"🦈"</span>
                        <span>"RustShark"</span>
                    </a>
                    <div class="flex gap-4 text-sm">
                        <a href="/" class="hover:text-blue-400 transition-colors">"仪表盘"</a>
                        <a href="/servers" class="hover:text-blue-400 transition-colors">"服务器"</a>
                        <a href="/keys" class="hover:text-blue-400 transition-colors">"SSH密钥"</a>
                        <a href="/capture-profiles" class="hover:text-blue-400 transition-colors">"抓包配置"</a>
                        <a href="/captures" class="hover:text-blue-400 transition-colors">"抓包任务"</a>
                        <a href="/analysis" class="hover:text-blue-400 transition-colors">"包分析"</a>
                        <a href="/capture-guide" class="hover:text-blue-400 transition-colors text-gray-500 text-xs">"过滤器指南"</a>
                        {move || auth.get().is_admin().then(|| view! {
                            <a href="/admin" class="hover:text-blue-400 transition-colors text-yellow-400">"管理"</a>
                        })}
                    </div>
                </div>
                <div class="flex items-center gap-4 text-sm">
                    <span class="text-gray-400">{move || auth.get().email.clone().unwrap_or_default()}</span>
                    <button
                        class="text-red-400 hover:text-red-300 transition-colors"
                        on:click=move |_| {
                            AuthState::clear();
                            auth.set(AuthState::default());
                            let window = web_sys::window().unwrap();
                            window.location().set_href("/login").ok();
                        }
                    >
                        "退出"
                    </button>
                </div>
            </nav>
            <main class="p-6">
                {children()}
            </main>
        </div>
    }
}
