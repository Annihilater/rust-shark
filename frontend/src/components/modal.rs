use leptos::prelude::*;

#[component]
pub fn Modal(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] title: String,
    on_close: Callback<()>,
    children: ChildrenFn,
) -> impl IntoView {
    let title = StoredValue::new(title);
    let children = StoredValue::new(children);
    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
                on:click=move |_| on_close.run(())>
                <div class="bg-gray-800 rounded-xl p-6 w-full max-w-lg shadow-2xl border border-gray-700"
                    on:click=|e| e.stop_propagation()>
                    <div class="flex items-center justify-between mb-4">
                        <h2 class="text-lg font-semibold">{title.get_value()}</h2>
                        <button
                            class="text-gray-400 hover:text-white text-xl"
                            on:click=move |_| on_close.run(())
                        >"✕"</button>
                    </div>
                    {children.with_value(|c| c())}
                </div>
            </div>
        </Show>
    }
}
