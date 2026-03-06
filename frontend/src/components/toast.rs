use leptos::prelude::*;

#[component]
pub fn Toast(
    #[prop(into)] message: Signal<Option<String>>,
    #[prop(into, default = "success".to_string())] kind: String,
) -> impl IntoView {
    let (bg, icon) = match kind.as_str() {
        "error" => ("bg-red-600", "✗"),
        "warning" => ("bg-yellow-600", "⚠"),
        _ => ("bg-green-600", "✓"),
    };

    view! {
        <Show when=move || message.get().is_some()>
            <div class=format!("fixed top-4 right-4 {} text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-2 z-50 animate-in", bg)>
                <span>{icon}</span>
                <span>{move || message.get().unwrap_or_default()}</span>
            </div>
        </Show>
    }
}
