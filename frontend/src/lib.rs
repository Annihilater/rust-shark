use leptos::prelude::*;
use leptos_router::{components::*, path};
use wasm_bindgen::prelude::*;

mod api;
mod components;
mod pages;
mod store;

use pages::{
    admin::AdminPage, analysis::AnalysisPage, capture_detail::CaptureDetailPage,
    capture_guide::CaptureGuidePage, capture_profiles::CaptureProfilesPage, captures::CapturesPage,
    dashboard::DashboardPage, keys::KeysPage, login::LoginPage, servers::ServersPage,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <p style="color:white">"页面未找到"</p> }>
                <Route path=path!("/login") view=LoginPage/>
                <Route path=path!("/") view=DashboardPage/>
                <Route path=path!("/servers") view=ServersPage/>
                <Route path=path!("/keys") view=KeysPage/>
                <Route path=path!("/captures") view=CapturesPage/>
                <Route path=path!("/captures/:id") view=CaptureDetailPage/>
                <Route path=path!("/capture-profiles") view=CaptureProfilesPage/>
                <Route path=path!("/analysis") view=AnalysisPage/>
                <Route path=path!("/capture-guide") view=CaptureGuidePage/>
                <Route path=path!("/admin") view=AdminPage/>
            </Routes>
        </Router>
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    store::init_auth();
    mount_to_body(App);
}
