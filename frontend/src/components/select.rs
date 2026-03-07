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
#[component]
pub fn Select(
    options: Vec<SelectOption>,
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let options = StoredValue::new(options);
    let extra_class = class.unwrap_or_default();

    // 当前选中项的 label
    let selected_label = move || {
        let v = value.get();
        options.with_value(|opts| {
            opts.iter()
                .find(|o| o.value == v)
                .map(|o| o.label.clone())
                .unwrap_or_default()
        })
    };

    view! {
        <div class=format!("relative {}", extra_class)>
            // 触发按钮
            <button
                type="button"
                class="w-full flex items-center justify-between bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm focus:outline-none focus:border-blue-500 hover:border-gray-500 transition-colors"
                on:click=move |e| {
                    e.stop_propagation();
                    open.update(|v| *v = !*v);
                }
            >
                <span>{move || selected_label()}</span>
                <span class=move || {
                    if open.get() {
                        "text-gray-400 transition-transform duration-150 rotate-180 inline-block"
                    } else {
                        "text-gray-400 transition-transform duration-150 inline-block"
                    }
                }>"▾"</span>
            </button>

            // 透明遮罩，点击关闭
            <Show when=move || open.get()>
                <div class="fixed inset-0 z-10" on:click=move |_| open.set(false) />
            </Show>

            // 下拉面板
            <Show when=move || open.get()>
                <div class="absolute z-20 mt-1 w-full bg-gray-800 border border-gray-600 rounded-lg shadow-xl overflow-hidden">
                    {options.with_value(|opts| {
                        opts.iter().map(|opt| {
                            let opt_value  = opt.value.clone();
                            let opt_label  = opt.label.clone();
                            let val_click  = opt.value.clone();
                            let val_check  = opt.value.clone();
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
