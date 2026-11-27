//! The `/deploy` wizard: the only part of artcraftcloud.com that runs Wasm. Header and footer
//! come from the `site` crate, so it matches the server-rendered pages.

use leptos::prelude::*;
use site::{SiteFooter, SiteHeader};

mod deploy;

#[component]
pub fn App() -> impl IntoView {
    let year = js_sys::Date::new_0().get_full_year() as i32;
    view! {
        <SiteHeader active="/deploy" />
        <main>
            <deploy::DeployPage />
        </main>
        <SiteFooter year=year />
    }
}
