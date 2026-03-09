use crate::components::{
    layout::Layout,
    modal::Modal,
    select::{Select, SelectOption},
};
use crate::store::use_auth;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Debug)]
struct SshKey {
    id: String,
    name: String,
    public_key: String,
    key_type: String,
    fingerprint: String,
    created_at: String,
}

#[derive(Serialize)]
struct CreateKeyRequest {
    name: String,
    private_key: String,
}

#[derive(Serialize)]
struct GenerateKeyRequest {
    name: String,
    key_type: String,
    comment: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct GenerateKeyResponse {
    key: SshKey,
}

#[derive(Clone, PartialEq)]
enum AddTab {
    Paste,
    Generate,
}

fn key_type_options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("ed25519", "Ed25519（推荐，最快最安全）"),
        SelectOption::new("ecdsa-p256", "ECDSA P-256"),
        SelectOption::new("ecdsa-p384", "ECDSA P-384"),
        SelectOption::new("rsa-4096", "RSA 4096 位"),
        SelectOption::new("rsa-2048", "RSA 2048 位"),
    ]
}

#[component]
pub fn KeysPage() -> impl IntoView {
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

    let keys = RwSignal::new(Vec::<SshKey>::new());
    let page = RwSignal::new(1usize);
    const PAGE_SIZE: usize = 10;
    let show_modal = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let success = RwSignal::new(Option::<String>::None);
    let view_key = RwSignal::new(Option::<SshKey>::None);
    let active_tab = RwSignal::new(AddTab::Paste);

    // ── 加载列表 ──────────────────────────────────────────────────────────
    let load_keys = move || {
        leptos::task::spawn_local(async move {
            if let Ok(list) = crate::api::get::<Vec<SshKey>>("/api/keys").await {
                keys.set(list);
                page.set(1);
            }
        });
    };
    load_keys();

    // ── 粘贴 Tab ──────────────────────────────────────────────────────────
    let key_name = RwSignal::new(String::new());
    let private_key = RwSignal::new(String::new());

    let on_create = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
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

    // ── 生成 Tab ──────────────────────────────────────────────────────────
    let gen_name = RwSignal::new(String::new());
    let gen_type = RwSignal::new("ed25519".to_string());
    let gen_comment = RwSignal::new(String::new());

    let on_generate = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        let req = GenerateKeyRequest {
            name: gen_name.get(),
            key_type: gen_type.get(),
            comment: {
                let c = gen_comment.get();
                if c.is_empty() {
                    None
                } else {
                    Some(c)
                }
            },
        };
        leptos::task::spawn_local(async move {
            match crate::api::post::<_, GenerateKeyResponse>("/api/keys/generate", &req).await {
                Ok(resp) => {
                    show_modal.set(false);
                    gen_name.set(String::new());
                    gen_comment.set(String::new());
                    success.set(Some(format!(
                        "密钥 \"{}\" 生成成功，私钥已加密保存",
                        resp.key.name
                    )));
                    load_keys();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    // ── 删除 ─────────────────────────────────────────────────────────────
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
                        on:click=move |_| {
                            error.set(None);
                            active_tab.set(AddTab::Paste);
                            show_modal.set(true);
                        }
                    >"+ 添加密钥"</button>
                </div>

                // 成功消息
                {move || success.get().map(|s| view! {
                    <div class="bg-green-900/50 border border-green-700 text-green-300 px-4 py-2 rounded-lg text-sm mb-4 flex items-center justify-between">
                        <span>{s}</span>
                        <button class="text-green-400 hover:text-green-200 ml-4"
                            on:click=move |_| success.set(None)>"✕"</button>
                    </div>
                })}

                // 密钥列表
                <div class="space-y-3">
                    {move || {
                        let all = keys.get();
                        let start = (page.get() - 1) * PAGE_SIZE;
                        all.into_iter().skip(start).take(PAGE_SIZE).map(|key| {
                        let id           = key.id.clone();
                        let on_del       = on_delete;
                        let key_for_view = key.clone();
                        view! {
                            <div class="bg-gray-800 border border-gray-700 rounded-xl p-4 flex items-start justify-between gap-4">
                                <div class="flex-1 min-w-0">
                                    <div class="flex items-center gap-2 mb-1">
                                        <span class="text-yellow-400">"🔑"</span>
                                        <span class="font-medium">{key.name.clone()}</span>
                                        <span class="text-xs bg-gray-700 text-gray-400 px-2 py-0.5 rounded-full font-mono">
                                            {key.key_type.clone()}
                                        </span>
                                    </div>
                                    <p class="text-xs text-gray-500 font-mono">"指纹: "{key.fingerprint.clone()}</p>
                                    <p class="text-xs text-gray-600 mt-0.5">"添加时间: "{key.created_at.clone()}</p>
                                </div>
                                <div class="flex items-center gap-3 shrink-0">
                                    <button
                                        class="text-blue-400 hover:text-blue-300 text-sm"
                                        on:click=move |_| view_key.set(Some(key_for_view.clone()))
                                    >"查看"</button>
                                    <button
                                        class="text-red-400 hover:text-red-300 text-sm"
                                        on:click=move |_| on_del(id.clone())
                                    >"删除"</button>
                                </div>
                            </div>
                        }
                        }).collect::<Vec<_>>()
                    }}

                    {move || keys.get().is_empty().then(|| view! {
                        <div class="text-center py-12 text-gray-500">
                            <div class="text-4xl mb-3">"🔑"</div>
                            <p>"还没有 SSH 密钥，点击右上角添加"</p>
                        </div>
                    })}

                    // 分页栏
                    {move || {
                        let total = keys.get().len().div_ceil(PAGE_SIZE);
                        if total <= 1 {
                            view! { <div/> }.into_any()
                        } else {
                            let total2 = total;
                            view! {
                                <div class="flex items-center justify-between mt-4 text-sm text-gray-400">
                                    <button
                                        class="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg hover:bg-gray-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                                        disabled=move || page.get() <= 1
                                        on:click=move |_| page.update(|p| *p = p.saturating_sub(1))
                                    >"← 上一页"</button>
                                    <span class="text-gray-500">
                                        "第 " {move || page.get()} " / " {total2} " 页"
                                    </span>
                                    <button
                                        class="px-3 py-1.5 bg-gray-800 border border-gray-700 rounded-lg hover:bg-gray-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                                        disabled=move || page.get() >= total2
                                        on:click=move |_| page.update(|p| if *p < total2 { *p += 1 })
                                    >"下一页 →"</button>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            // ── 添加/生成弹窗 ─────────────────────────────────────────────
            <Modal
                show=show_modal.read_only()
                title="添加 SSH 密钥"
                on_close=Callback::new(move |_| show_modal.set(false))
            >
                // Tab 切换
                <div class="flex border-b border-gray-700 mb-5 -mt-1">
                    <button
                        class=move || {
                            let base = "px-4 py-2 text-sm font-medium border-b-2 transition-colors";
                            if active_tab.get() == AddTab::Paste {
                                format!("{} border-blue-500 text-blue-400", base)
                            } else {
                                format!("{} border-transparent text-gray-400 hover:text-gray-300", base)
                            }
                        }
                        on:click=move |_| { active_tab.set(AddTab::Paste); error.set(None); }
                    >"粘贴私钥"</button>
                    <button
                        class=move || {
                            let base = "px-4 py-2 text-sm font-medium border-b-2 transition-colors";
                            if active_tab.get() == AddTab::Generate {
                                format!("{} border-blue-500 text-blue-400", base)
                            } else {
                                format!("{} border-transparent text-gray-400 hover:text-gray-300", base)
                            }
                        }
                        on:click=move |_| { active_tab.set(AddTab::Generate); error.set(None); }
                    >"自动生成"</button>
                </div>

                // 粘贴私钥 Tab
                {move || (active_tab.get() == AddTab::Paste).then(|| view! {
                    <form on:submit=on_create class="space-y-4">
                        <div>
                            <label class="block text-sm text-gray-400 mb-1">"密钥名称"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500"
                                placeholder="生产服务器密钥"
                                prop:value=key_name
                                on:input=move |ev| key_name.set(event_target_value(&ev))
                                required
                            />
                        </div>
                        <div>
                            <label class="block text-sm text-gray-400 mb-1">"私钥内容（OpenSSH PEM 格式）"</label>
                            <textarea
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white font-mono text-xs focus:outline-none focus:border-blue-500 h-36 resize-none"
                                placeholder="-----BEGIN OPENSSH PRIVATE KEY-----\n..."
                                prop:value=private_key
                                on:input=move |ev| private_key.set(event_target_value(&ev))
                                required
                            />
                        </div>
                        {move || error.get().map(|e| view! {
                            <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                        })}
                        <div class="flex gap-3 pt-1">
                            <button type="submit"
                                class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm transition-colors"
                            >"添加"</button>
                            <button type="button"
                                class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm transition-colors"
                                on:click=move |_| show_modal.set(false)
                            >"取消"</button>
                        </div>
                    </form>
                })}

                // 自动生成 Tab
                {move || (active_tab.get() == AddTab::Generate).then(|| view! {
                    <form on:submit=on_generate class="space-y-4">
                        <div>
                            <label class="block text-sm text-gray-400 mb-1">"密钥名称"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500"
                                placeholder="我的新密钥"
                                prop:value=gen_name
                                on:input=move |ev| gen_name.set(event_target_value(&ev))
                                required
                            />
                        </div>
                        <div>
                            <label class="block text-sm text-gray-400 mb-1">"密钥类型"</label>
                            <Select
                                options=key_type_options()
                                value=gen_type.read_only()
                                on_change=Callback::new(move |v| gen_type.set(v))
                            />
                        </div>
                        <div>
                            <label class="block text-sm text-gray-400 mb-1">"注释（可选）"</label>
                            <input
                                class="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500"
                                placeholder="user@hostname"
                                prop:value=gen_comment
                                on:input=move |ev| gen_comment.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="bg-blue-900/30 border border-blue-700/50 text-blue-300/80 px-3 py-2 rounded-lg text-xs flex items-start gap-2">
                            <span class="mt-0.5 shrink-0">"🔒"</span>
                            <span>"私钥将使用 AES-256-GCM 加密保存在数据库中，SSH 连接时从内存解密使用，不会明文传输"</span>
                        </div>
                        {move || error.get().map(|e| view! {
                            <div class="bg-red-900/50 border border-red-700 text-red-300 px-3 py-2 rounded-lg text-sm">{e}</div>
                        })}
                        <div class="flex gap-3 pt-1">
                            <button type="submit"
                                class="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg text-sm transition-colors"
                            >"生成并保存"</button>
                            <button type="button"
                                class="flex-1 bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm transition-colors"
                                on:click=move |_| show_modal.set(false)
                            >"取消"</button>
                        </div>
                    </form>
                })}
            </Modal>

            // ── 查看密钥详情弹窗 ─────────────────────────────────────────
            <Modal
                show=Signal::derive(move || view_key.get().is_some())
                title="密钥详情"
                on_close=Callback::new(move |_| view_key.set(None))
            >
                {move || view_key.get().map(|k| {
                    let cmd_copied   = RwSignal::new(false);
                    let pubkey_copied = RwSignal::new(false);

                    // 写入公钥的命令（先确保 .ssh 目录存在并设置正确权限）
                    let install_cmd = format!(
                        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && echo \"{}\" >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys",
                        k.public_key.trim()
                    );
                    let cmd_for_copy    = install_cmd.clone();
                    let pubkey_for_copy = k.public_key.clone();

                    let copy_to_clipboard = |text: String, copied: RwSignal<bool>| {
                        let win = web_sys::window().unwrap();
                        let _ = win.navigator().clipboard().write_text(&text);
                        copied.set(true);
                        // 2 秒后重置
                        leptos::task::spawn_local(async move {
                            gloo_timers::future::TimeoutFuture::new(2000).await;
                            copied.set(false);
                        });
                    };

                    view! {
                        <div class="space-y-4">
                            // 名称 + 类型
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-xs text-gray-500 mb-1">"名称"</label>
                                    <p class="text-white font-medium text-sm">{k.name.clone()}</p>
                                </div>
                                <div>
                                    <label class="block text-xs text-gray-500 mb-1">"类型"</label>
                                    <span class="text-xs bg-gray-700 text-gray-300 px-2 py-1 rounded font-mono">{k.key_type.clone()}</span>
                                </div>
                            </div>

                            // 指纹
                            <div>
                                <label class="block text-xs text-gray-500 mb-1">"SHA256 指纹"</label>
                                <p class="text-xs font-mono text-green-400 break-all bg-gray-900 rounded px-2 py-1.5">{k.fingerprint.clone()}</p>
                            </div>

                            // 公钥（带复制按钮）
                            <div>
                                <div class="flex items-center justify-between mb-1">
                                    <label class="text-xs text-gray-500">"公钥"</label>
                                    <button
                                        type="button"
                                        class=move || {
                                            if pubkey_copied.get() {
                                                "text-xs text-green-400 flex items-center gap-1"
                                            } else {
                                                "text-xs text-gray-400 hover:text-gray-200 flex items-center gap-1 transition-colors"
                                            }
                                        }
                                        on:click={
                                            let text = pubkey_for_copy.clone();
                                            move |_| copy_to_clipboard(text.clone(), pubkey_copied)
                                        }
                                    >
                                        {move || if pubkey_copied.get() { "✓ 已复制" } else { "复制" }}
                                    </button>
                                </div>
                                <textarea
                                    class="w-full bg-gray-900 border border-gray-700 rounded px-2 py-1.5 text-xs font-mono text-gray-300 h-16 resize-none focus:outline-none"
                                    readonly
                                    prop:value=k.public_key.clone()
                                />
                            </div>

                            // 写入 authorized_keys 命令
                            <div>
                                <div class="flex items-center justify-between mb-1">
                                    <label class="text-xs text-gray-500">"写入目标服务器"</label>
                                    <button
                                        type="button"
                                        class=move || {
                                            if cmd_copied.get() {
                                                "text-xs text-green-400 flex items-center gap-1"
                                            } else {
                                                "text-xs text-gray-400 hover:text-gray-200 flex items-center gap-1 transition-colors"
                                            }
                                        }
                                        on:click={
                                            let text = cmd_for_copy.clone();
                                            move |_| copy_to_clipboard(text.clone(), cmd_copied)
                                        }
                                    >
                                        {move || if cmd_copied.get() { "✓ 已复制" } else { "复制命令" }}
                                    </button>
                                </div>
                                <div class="bg-gray-900 border border-gray-700 rounded px-3 py-2 font-mono text-xs text-yellow-300 break-all select-all">
                                    {install_cmd.clone()}
                                </div>
                                <p class="text-xs text-gray-600 mt-1">"在目标服务器上执行此命令，将公钥追加到 authorized_keys"</p>
                            </div>

                            // 添加时间
                            <div>
                                <label class="block text-xs text-gray-500 mb-1">"添加时间"</label>
                                <p class="text-xs text-gray-400">{k.created_at.clone()}</p>
                            </div>

                            // 私钥安全提示
                            <div class="bg-gray-900/60 border border-gray-700 text-gray-400 px-3 py-2 rounded-lg text-xs flex items-center gap-2">
                                <span>"🔒"</span>
                                <span>"私钥已加密存储于数据库，SSH 连接时在内存中解密使用"</span>
                            </div>

                            <button
                                class="w-full bg-gray-700 hover:bg-gray-600 text-white py-2 rounded-lg text-sm transition-colors"
                                on:click=move |_| view_key.set(None)
                            >"关闭"</button>
                        </div>
                    }
                })}
            </Modal>
        </Layout>
    }
}
