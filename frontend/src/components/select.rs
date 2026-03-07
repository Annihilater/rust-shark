use leptos::prelude::*;

#[derive(Clone)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into() }
    }
}

/// 自定义下拉选择框
///
/// - `options`     — 选项列表
/// - `value`       — 当前选中值的 Signal
/// - `on_change`   — 选中值变化回调，传入新值（空串表示选了 placeholder）
/// - `placeholder` — 可选占位文字，显示为首行灰色项，对应空值 `""`
/// - `class`       — 附加 CSS 类
#[component]
pub fn Select(
    options: Vec<SelectOption>,
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    #[prop(optional, into)] placeholder: Option<String>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let open        = RwSignal::new(false);
    let options     = StoredValue::new(options);
    let placeholder = StoredValue::new(placeholder);
    let extra_class = class.unwrap_or_default();

    // 当前显示文字（优先 options 里匹配，否则 placeholder）
    let selected_label = move || {
        let v = value.get();
        if v.is_empty() {
            return placeholder.with_value(|p| {
                p.clone().unwrap_or_default()
            });
        }
        options.with_value(|opts| {
            opts.iter()
                .find(|o| o.value == v)
                .map(|o| o.label.clone())
                .unwrap_or_default()
        })
    };

    // 按钮文字颜色：空值时显示灰色占位
    let label_class = move || {
        if value.get().is_empty() { "text-gray-400" } else { "text-white" }
    };

    view! {
        <div class=format!("relative {}", extra_class)>
            // ── 触发按钮 ────────────────────────────────────────────────
            <button
                type="button"
                class="w-full flex items-center justify-between bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 hover:border-gray-500 transition-colors"
                on:click=move |e| {
                    e.stop_propagation();
                    open.update(|v| *v = !*v);
                }
            >
                <span class=label_class>{move || selected_label()}</span>
                <span class=move || {
                    if open.get() {
                        "text-gray-400 transition-transform duration-150 rotate-180 inline-block ml-2 shrink-0"
                    } else {
                        "text-gray-400 transition-transform duration-150 inline-block ml-2 shrink-0"
                    }
                }>"▾"</span>
            </button>

            // ── 透明遮罩，点击关闭 ──────────────────────────────────────
            <Show when=move || open.get()>
                <div class="fixed inset-0 z-10" on:click=move |_| open.set(false) />
            </Show>

            // ── 下拉面板 ────────────────────────────────────────────────
            <Show when=move || open.get()>
                <div class="absolute z-20 mt-1 w-full bg-gray-800 border border-gray-600 rounded-lg shadow-xl overflow-hidden">

                    // placeholder 行（如果有）
                    {placeholder.with_value(|p| p.clone()).map(|ph| {
                        view! {
                            <button
                                type="button"
                                class=move || {
                                    let base = "w-full text-left px-3 py-2 text-sm flex items-center gap-2 transition-colors";
                                    if value.get().is_empty() {
                                        format!("{} bg-blue-600 text-white", base)
                                    } else {
                                        format!("{} text-gray-500 hover:bg-gray-700", base)
                                    }
                                }
                                on:click=move |_| {
                                    on_change.run(String::new());
                                    open.set(false);
                                }
                            >
                                <span class="w-4 shrink-0 text-center text-xs">
                                    {move || if value.get().is_empty() { "✓" } else { "" }}
                                </span>
                                {ph}
                            </button>
                        }
                    })}

                    // 普通选项行
                    {options.with_value(|opts| {
                        opts.iter().map(|opt| {
                            let opt_value = opt.value.clone();
                            let opt_label = opt.label.clone();
                            let val_click = opt.value.clone();
                            let val_check = opt.value.clone();
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        let base = "w-full text-left px-3 py-2 text-sm flex items-center gap-2 transition-colors";
                                        if value.get() == val_check {
                                            format!("{} bg-blue-600 text-white", base)
                                        } else {
                                            format!("{} text-gray-200 hover:bg-gray-700", base)
                                        }
                                    }
                                    on:click=move |_| {
                                        on_change.run(val_click.clone());
                                        open.set(false);
                                    }
                                >
                                    <span class="w-4 shrink-0 text-center text-xs">
                                        {move || if value.get() == opt_value { "✓" } else { "" }}
                                    </span>
                                    {opt_label.clone()}
                                </button>
                            }
                        }).collect::<Vec<_>>()
                    })}
                </div>
            </Show>
        </div>
    }
}
