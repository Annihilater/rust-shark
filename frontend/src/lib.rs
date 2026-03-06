use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

mod api;
mod components;
mod pages;
mod store;

use pages::{
    admin::AdminPage,
    captures::CapturesPage,
    capture_detail::CaptureDetailPage,
    dashboard::DashboardPage,
    keys::KeysPage,
    login::LoginPage,
    servers::ServersPage,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <Title text="RustShark"/>
        <Router>
            <Routes fallback=|| view! { <p>"页面未找到"</p> }>
                <Route path=path!("/login") view=LoginPage/>
                <Route path=path!("/") view=DashboardPage/>
                <Route path=path!("/servers") view=ServersPage/>
                <Route path=path!("/keys") view=KeysPage/>
                <Route path=path!("/captures") view=CapturesPage/>
                <Route path=path!("/captures/:id") view=CaptureDetailPage/>
                <Route path=path!("/admin") view=AdminPage/>
            </Routes>
        </Router>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
