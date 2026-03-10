use crate::store::{pop_esc_layer, push_esc_layer};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn Modal(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] title: Signal<String>,
    on_close: Callback<()>,
    children: ChildrenFn,
) -> impl IntoView {
    let children = StoredValue::new(children);

    // 弹窗打开时：
    //   1. 向全局 ESC 栈注册（让 Layout 知道有弹窗存在）
    //   2. 在 window 上注册 keydown，ESC 时关闭弹窗
    // 弹窗关闭/销毁时：
    //   1. 从全局 ESC 栈注销
    //   2. 移除 keydown 监听
    Effect::new(move |_| {
        if !show.get() {
            return;
        }

        // 注册到 ESC 优先级栈
        push_esc_layer();

        let closure =
            Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    on_close.run(());
                }
            });

        let window = web_sys::window().unwrap();
        window
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();

        // Effect cleanup：弹窗隐藏/卸载时移除监听并出栈
        let cb = closure.into_js_value();
        on_cleanup(move || {
            pop_esc_layer();
            let window = web_sys::window().unwrap();
            let _ = window.remove_event_listener_with_callback("keydown", cb.unchecked_ref());
        });
    });

    view! {
        <Show when=move || show.get()>
            <div
                class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
                on:click=move |_| on_close.run(())
            >
                <div
                    class="card rounded-xl p-6 w-full max-w-lg shadow-2xl"
                    on:click=|e: leptos::ev::MouseEvent| e.stop_propagation()
                >
                    <div class="flex items-center justify-between mb-4">
                        <h2 class="text-lg font-semibold">{move || title.get()}</h2>
                        <button
                            class="text-gray-400 hover:text-gray-700 dark:hover:text-white text-xl leading-none"
                            on:click=move |_| on_close.run(())
                        >"✕"</button>
                    </div>
                    {children.with_value(|c| c())}
                </div>
            </div>
        </Show>
    }
}
