use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes, A};
use leptos_router::path;

use pages::apps::AppsPage;
use pages::deploy::DeployPage;
use pages::guide::GuidePage;
use pages::home::HomePage;
use pages::hosts::HostsPage;

pub mod catalog;
pub mod pages;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <SiteHeader />
            <main>
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/apps") view=AppsPage />
                    <Route path=path!("/hosts") view=HostsPage />
                    <Route path=path!("/deploy") view=DeployPage />
                    <Route path=path!("/guide") view=GuidePage />
                </Routes>
            </main>
            <SiteFooter />
        </Router>
    }
}

#[component]
fn LogoMark() -> impl IntoView {
    view! {
        <svg class="logo-mark" viewBox="0 0 64 64" aria-hidden="true">
            <path class="facet-a" d="M11 26 L13.24 26 L18.17 36.61 L4 37.5 L4 33 Z" />
            <path class="facet-b" d="M22 15 L35 15 L39 19.67 L35.03 35.56 L23.53 36.27 L17.49 23.27 Z" />
            <path class="facet-c" d="M43.58 22 L49 22 L60 33 L60 34 L40.27 35.23 Z" />
            <path class="facet-d" d="M4 44 L4 42.5 L60 39 L60 44 L54 50 L10 50 Z" />
        </svg>
    }
}

fn stored_theme() -> String {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("artcraft-theme").ok().flatten())
        .filter(|t| t == "light" || t == "dark")
        .unwrap_or_else(|| "light".to_string())
}

fn apply_theme(theme: &str) {
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.set_attribute("data-theme", theme);
    }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let (theme, set_theme) = signal(stored_theme());

    Effect::new(move |_| {
        let current = theme.get();
        apply_theme(&current);
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("artcraft-theme", &current);
        }
    });

    view! {
        <button
            class="icon-button"
            title="Toggle dark mode"
            aria-label="Toggle dark mode"
            on:click=move |_| {
                set_theme
                    .update(|t| {
                        *t = if t == "light" { "dark".to_string() } else { "light".to_string() };
                    })
            }
        >
            {move || if theme.get() == "light" { "☾" } else { "☀" }}
        </button>
    }
}

#[component]
fn SiteHeader() -> impl IntoView {
    view! {
        <header class="site-header">
            <div class="container">
                <a href="/" class="brand" aria-label="artcraftCloud home">
                    <LogoMark />
                    <span>"artcraft" <b>"Cloud"</b></span>
                </a>
                <nav class="nav-links" aria-label="Primary">
                    <A href="/apps">"Apps"</A>
                    <A href="/hosts">"Hosts"</A>
                    <A href="/guide">"Guide"</A>
                </nav>
                <div class="nav-actions">
                    <ThemeToggle />
                    <a class="button primary small" href="/deploy">"Deploy"</a>
                </div>
            </div>
        </header>
    }
}

#[component]
fn SiteFooter() -> impl IntoView {
    view! {
        <footer class="site-footer">
            <div class="container">
                <div class="footer-grid">
                    <div>
                        <a href="/" class="brand">
                            <LogoMark />
                            <span>"artcraft" <b>"Cloud"</b></span>
                        </a>
                        <p class="muted small mt-8" style="max-width: 34ch">
                            "Your files, your email, your European server. Open source and free of
                            fees."
                        </p>
                    </div>
                    <div class="footer-cols">
                        <div class="footer-col">
                            <h4>"Product"</h4>
                            <a href="/apps">"Apps"</a>
                            <a href="/hosts">"Hosts"</a>
                            <a href="/deploy">"Deploy"</a>
                        </div>
                        <div class="footer-col">
                            <h4>"Learn"</h4>
                            <a href="/guide">"Guide"</a>
                            <a href="/guide#mail">"Mail & Port 25"</a>
                            <a href="/guide#backups">"Backups"</a>
                        </div>
                        <div class="footer-col">
                            <h4>"Support"</h4>
                            <a href="https://liberapay.com/" target="_blank" rel="noopener">"Buy me a coffee"</a>
                            <a href="https://github.com/gheiler/artcraft-cloud" target="_blank" rel="noopener">"Source code"</a>
                        </div>
                    </div>
                </div>
                <div class="footer-bottom">
                    <span>"© " {current_year()} " artcraftCloud · MIT OR Apache-2.0"</span>
                    <span>"Made in Europe · no telemetry"</span>
                </div>
            </div>
        </footer>
    }
}

fn current_year() -> i32 {
    (js_sys::Date::new_0().get_full_year()) as i32
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <section class="section">
            <div class="container center">
                <span class="kicker">"404"</span>
                <h2>"That page drifted off"</h2>
                <p class="muted mt-8">"The link may be old, or the page moved."</p>
                <div class="row mt-20" style="justify-content:center">
                    <a class="button primary" href="/">"Back home"</a>
                    <a class="button" href="/deploy">"Deploy"</a>
                </div>
            </div>
        </section>
    }
}
