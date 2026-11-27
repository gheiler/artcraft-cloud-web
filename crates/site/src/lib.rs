#![recursion_limit = "512"]
//! The public pages of artcraftcloud.com.
//!
//! Every page here is plain, non-interactive Leptos markup. The server renders it to HTML
//! (`render_page`, behind the `ssr` feature), so search engines and link previews get real HTML and
//! visitors download no Wasm. Only `/deploy` runs Wasm (the `wizard` crate), and it reuses the
//! header and footer from this crate so both halves look the same.

use leptos::prelude::*;

pub mod catalog;
pub mod pages;

/// Where the "buy me a coffee" links point. One place, so it is easy to change.
pub const TIP_URL: &str = "https://liberapay.com/";
pub const SOURCE_URL: &str = "https://github.com/gheiler/artcraft-cloud";
pub const SITE_URL: &str = "https://artcraftcloud.com";

/// Sets `data-theme` before first paint (saved choice, else the system setting) and toggles it.
/// The only script on the static pages.
pub const THEME_SCRIPT: &str = r##"(function(){var k="artcraft-theme",d=document.documentElement;function s(t){d.setAttribute("data-theme",t);var m=document.querySelector('meta[name=theme-color]');if(m)m.content=t==="dark"?"#1c1c1c":"#f9f9fa";}var t=null;try{t=localStorage.getItem(k)}catch(e){}if(t!=="light"&&t!=="dark")t=matchMedia("(prefers-color-scheme: dark)").matches?"dark":"light";s(t);window.artcraftToggleTheme=function(){var n=d.getAttribute("data-theme")==="dark"?"light":"dark";s(n);try{localStorage.setItem(k,n)}catch(e){}};})();"##;

#[component]
pub fn LogoMark() -> impl IntoView {
    view! {
        <svg class="logo-mark" viewBox="0 0 64 64" aria-hidden="true">
            <path class="facet-a" d="M11 26 L13.24 26 L18.17 36.61 L4 37.5 L4 33 Z" />
            <path class="facet-b" d="M22 15 L35 15 L39 19.67 L35.03 35.56 L23.53 36.27 L17.49 23.27 Z" />
            <path class="facet-c" d="M43.58 22 L49 22 L60 33 L60 34 L40.27 35.23 Z" />
            <path class="facet-d" d="M4 44 L4 42.5 L60 39 L60 44 L54 50 L10 50 Z" />
        </svg>
    }
}

fn nav_class(active: &str, me: &str) -> &'static str {
    if active == me {
        "active"
    } else {
        ""
    }
}

/// Site header. `active` is the current path ("/apps", ...), used to mark the nav link.
#[component]
pub fn SiteHeader(#[prop(into)] active: String) -> impl IntoView {
    view! {
        <header class="site-header">
            <div class="container">
                <a href="/" class="brand" aria-label="artcraftCloud home">
                    <LogoMark />
                    <span>"artcraft" <b>"Cloud"</b></span>
                </a>
                <nav class="nav-links" aria-label="Primary">
                    <a href="/apps" class=nav_class(&active, "/apps")>"Apps"</a>
                    <a href="/hosts" class=nav_class(&active, "/hosts")>"Hosts"</a>
                    <a href="/guide" class=nav_class(&active, "/guide")>"Guide"</a>
                </nav>
                <div class="nav-actions">
                    <a class="tip-pill" href=TIP_URL target="_blank" rel="noopener" title="No fees. Tips keep it going.">
                        "☕ Tip"
                    </a>
                    <button
                        class="icon-button theme-toggle"
                        type="button"
                        title="Toggle dark mode"
                        aria-label="Toggle dark mode"
                        onclick="artcraftToggleTheme()"
                    >
                        <span class="when-light">"☾"</span>
                        <span class="when-dark">"☀"</span>
                    </button>
                    <a class="button primary small" href="/deploy">"Deploy"</a>
                </div>
            </div>
        </header>
    }
}

#[component]
pub fn SiteFooter(year: i32) -> impl IntoView {
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
                            "Your files, your email, your European server. Open source and free of fees."
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
                            <a href=TIP_URL target="_blank" rel="noopener">"Buy me a coffee"</a>
                            <a href=SOURCE_URL target="_blank" rel="noopener">"Source code"</a>
                        </div>
                    </div>
                </div>
                <div class="footer-bottom">
                    <span>"© " {year} " artcraftCloud · MIT OR Apache-2.0"</span>
                    <span>"Made in Europe · no telemetry"</span>
                </div>
            </div>
        </footer>
    }
}

#[component]
pub fn NotFound() -> impl IntoView {
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

/// The server-rendered pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Apps,
    Hosts,
    Guide,
    NotFound,
}

impl Page {
    pub const ALL: [Page; 4] = [Page::Home, Page::Apps, Page::Hosts, Page::Guide];

    pub fn from_path(path: &str) -> Page {
        match path.trim_end_matches('/') {
            "" => Page::Home,
            "/apps" => Page::Apps,
            "/hosts" => Page::Hosts,
            "/guide" => Page::Guide,
            _ => Page::NotFound,
        }
    }

    pub fn path(self) -> &'static str {
        match self {
            Page::Home => "/",
            Page::Apps => "/apps",
            Page::Hosts => "/hosts",
            Page::Guide => "/guide",
            Page::NotFound => "/404",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Page::Home => "artcraftCloud: your files, your email, your European server",
            Page::Apps => "The ten Craft apps · artcraftCloud",
            Page::Hosts => "European VPS hosts compared · artcraftCloud",
            Page::Guide => "Self-hosting guide · artcraftCloud",
            Page::NotFound => "Not found · artcraftCloud",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Page::Home => "Run your own office cloud on a cheap European VPS: a folder-backed drive, your own domain mail and calendar, and ten native-speed Craft apps. No subscriptions, no lock-in.",
            Page::Apps => "WordCraft, GridCraft, DeckCraft, PdfCraft and six more native Rust apps that open the files on your own server.",
            Page::Hosts => "Seven European VPS providers compared by price, jurisdiction, one-click support and mail (Port 25) policy.",
            Page::Guide => "Run artcraftCloud by hand: cloud-init, Docker Compose, Caddy HTTPS, DNS records, mail and backups.",
            Page::NotFound => "This page does not exist.",
        }
    }
}

#[cfg(feature = "ssr")]
mod render {
    use super::*;
    use crate::pages::{apps::AppsPage, guide::GuidePage, home::HomePage, hosts::HostsPage};

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    /// The `<head>` shared by the static pages and the wizard's index.html.
    pub fn head(title: &str, description: &str, canonical_path: &str) -> String {
        let title = escape(title);
        let description = escape(description);
        format!(
            r##"<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title}</title><meta name="description" content="{description}"><meta name="theme-color" content="#f9f9fa"><link rel="canonical" href="{SITE_URL}{canonical_path}"><meta property="og:type" content="website"><meta property="og:site_name" content="artcraftCloud"><meta property="og:title" content="{title}"><meta property="og:description" content="{description}"><meta property="og:url" content="{SITE_URL}{canonical_path}"><meta property="og:image" content="{SITE_URL}/assets/screenshots/drive.png"><meta name="twitter:card" content="summary_large_image"><link rel="icon" type="image/svg+xml" href="/favicon.svg"><link rel="preload" href="/assets/fonts/inter-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin><link rel="stylesheet" href="/assets/site.css"><script>{THEME_SCRIPT}</script>"##
        )
    }

    /// A whole HTML document for `page`. `year` goes in the footer.
    pub fn render_page(page: Page, year: i32) -> String {
        let active = page.path().to_string();
        let body = Owner::new().with(|| {
            let main = match page {
                Page::Home => view! { <HomePage /> }.into_any(),
                Page::Apps => view! { <AppsPage /> }.into_any(),
                Page::Hosts => view! { <HostsPage /> }.into_any(),
                Page::Guide => view! { <GuidePage /> }.into_any(),
                Page::NotFound => view! { <NotFound /> }.into_any(),
            };
            view! {
                <SiteHeader active=active />
                <main>{main}</main>
                <SiteFooter year=year />
            }
            .to_html()
        });
        format!(
            "<!DOCTYPE html><html lang=\"en\" data-theme=\"light\"><head>{}</head><body>{}</body></html>",
            head(page.title(), page.description(), page.path()),
            body
        )
    }
}

#[cfg(feature = "ssr")]
pub use render::{head, render_page};
