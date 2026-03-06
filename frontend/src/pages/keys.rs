use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::components::{layout::Layout, modal::Modal};
use crate::store::use_auth;

#[derive(Deserialize, Clone, Debug)]
struct SshKey {
    id: String,
    name: String,
    public_key: String,
    created_at: String,
}

#[derive(Serialize)]
struct CreateKeyRequest {
    name: String,
    private_key: String,
}

#[component]
pub fn KeysPage() -> impl IntoView {
    let auth = use_auth();
    Effect::new(move |_| {
        if !auth.get().is_logged_in() {
            web_sys::window().unwrap().location().set_href("/login").ok();
        }
    });

    let keys = RwSignal::new(Vec::<SshKey>::new());
    let show_modal = RwSignal::new(false);
    let key_name = RwSignal::new(String::new());
    let private_key = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let success = RwSignal::new(Option::<String>::None);

    // 加载密钥列表
    let load_keys = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<SshKey>>("/api/keys").await {
                keys.set(list);
            }
        });
    };
    load_keys();

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let req = CreateKeyRequest {
            name: key_name.get(),
            private_key: private_key.get(),
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, SshKey>("/api/keys", &req).await {
                Ok(_) => {
                    show_modal.set(false);
                    key_name.set(String::new());
                    private_key.set(String::new());
                    success.set(Some("密钥添加成功".to_string()));
                    load_keys();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    let on_delete = move |id: String| {
        leptos::task::spawn_local(async move {
            let path = format!("/api/keys/{}", id);
            if crate::api::delete::<serde_json::Value>(&path).await.is_ok() {
                success.set(Some("密钥已删除".to_string()));
                load_keys();
            }
        });
    };

    view! {
        <Layout>
            <div class="max-w-4xl mx-auto">
                <div class="flex items-center justify-between mb-6">
                    <h1 class="text-2xl font-bold">"SSH 密钥管理"</h1>
                    <button
                        class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-sm transition-colors"
                        on:click=move |_| show_modal.set(true)
                    >"+ 添加密钥"</button>
                </div>

                {move || success.get().map(|s| view! {
                    <div class="bg-green-900/50 border border-green-700 text-green-300 px-4 py-2 rounded-lg text-sm mb-4">{s}</div>
                })}

                <div class="space-y-3">
                    {move || keys.get().into_iter().map(|key| {
                        let id = key.id.clone();
                        let on_del = on_delete.clone();
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4 flex items-start justify-between">
                                <div class="flex-1 min-w-0">
                                    <div class="flex items-center gap-2">
                                        <span class="text-yellow-400">"🔑"</span>
                                        <span class="font-medium">{key.name}</span>
                                    </div>
                                    <p class="text-xs text-gray-500 mt-1 font-mono truncate">{key.public_key}</p>
                                    <p class="text-xs text-gray-500 mt-1">"添加时间: "{key.created_at}</p>
                                </div>
                                <button
                                    class="text-red-400 hover:text-red-300 text-sm ml-4 shrink-0"
                                    on:click=move |_| on_del(id.clone())
                                >"删除"</button>
                            </div>
                        }
                    }).collect::<Vec<_>>()}

                    {move || keys.get().is_empty().then(|| view! {
                        <div class="text-center py-12 text-gray-500">
                            <div class="text-4xl mb-3">"🔑"</div>
                            <p>"还没有 SSH 密钥，点击右上角添加"</p>
                        </div>
                    })}
                </div>
            </div>

            // 添加密钥弹窗
            <Modal
                show=show_modal.read_only()
                title="添加 SSH 密钥"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                <form on:submit=on_create class="space-y-4">
                    <div>
                        <label class="block text-sm text-gray-400 mb-1">"密钥名称"</label>
                        <input
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white focus:outline-none focus:border-blue-500"
                            placeholder="生产服务器密钥"
                            prop:value=key_name
                            on:input=move |ev| key_name.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    <div>
                        <label class="block text-sm text-gray-400 mb-1">"私钥内容 (PEM格式)"</label>
                        <textarea
                            class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white font-mono text-xs focus:outline-none focus:border-blue-500 h-32 resize-none"
                            placeholder="-----BEGIN OPENSSH PRIVATE KEY-----\n..."
                            prop:value=private_key
                            on:input=move |ev| private_key.set(event_target_value(&ev))
                            required
                        />
                    </div>
                    {move || error.get().map(|e| view! {
                        <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                    })}
                    <div class="flex gap-3 pt-2">
                        <button type="submit" class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm transition-colors">"添加"</button>
                        <button type="button" class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm transition-colors"
                            on:click=move |_| show_modal.set(false)>"取消"</button>
                    </div>
                </form>
            </Modal>
        </Layout>
    }
}
