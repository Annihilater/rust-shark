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

    // 弹窗打开时在 window 上注册 keydown 监听，关闭时自动移除
    Effect::new(move |_| {
        if !show.get() {
            return;
        }

        let closure = Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Escape" {
                on_close.run(());
            }
        });

        let window = web_sys::window().unwrap();
        window
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .unwrap();

        // 将 closure 转为 JS function 保存，Effect cleanup 时移除监听
        let cb = closure.into_js_value();
        on_cleanup(move || {
            let window = web_sys::window().unwrap();
            let _ = window.remove_event_listener_with_callback(
                "keydown",
                cb.unchecked_ref(),
            );
        });
    });

    view! {
        <Show when=move || show.get()>
            <div
                class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
                on:click=move |_| on_close.run(())
            >
                <div
                    class="bg-gray-800 rounded-xl p-6 w-full max-w-lg shadow-2xl border border-gray-700"
                    on:click=|e: leptos::ev::MouseEvent| e.stop_propagation()
                >
                    <div class="flex items-center justify-between mb-4">
                        <h2 class="text-lg font-semibold">{move || title.get()}</h2>
                        <button
                            class="text-gray-400 hover:text-white text-xl leading-none"
                            on:click=move |_| on_close.run(())
                        >"✕"</button>
                    </div>
                    {children.with_value(|c| c())}
                </div>
            </div>
        </Show>
    }
}
