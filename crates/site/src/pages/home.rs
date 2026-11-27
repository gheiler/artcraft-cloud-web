use leptos::prelude::*;

use types::{DeployMode, Tier, HOSTS};

use crate::catalog::registry;

/// Small inline tick used in feature lists.
#[component]
fn Tick() -> impl IntoView {
    view! {
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path d="M3 8.5 6.2 11.5 13 4.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <section class="hero">
            <div class="container">
                <div class="hero-grid">
                    <div>
                        <span class="eyebrow">
                            <span class="badge primary">"Open source"</span>
                            "EU-hosted · Rust · no lock-in"
                        </span>
                        <h1>
                            "Your files. Your email. "
                            <span class="accent">"Your European server."</span>
                        </h1>
                        <p class="lead">
                            "A folder-backed drive, mail and calendar on your own domain, and ten
                            native-speed apps that open the files you already have. Rent a cheap
                            EU VPS, click deploy, keep everything."
                        </p>
                        <div class="cta-row">
                            <a class="button primary large" href="/deploy">"Deploy my cloud"</a>
                            <a class="button large" href="#how">"See how it works"</a>
                        </div>
                        <div class="hero-meta">
                            <span>"From " <b>"€3 / month"</b></span>
                            <span>"No subscription"</span>
                            <span>"~10 MB idle"</span>
                            <span>"10 signed apps"</span>
                        </div>
                    </div>
                    <div class="window">
                        <div class="window-bar">
                            <span class="traffic"><i></i><i></i><i></i></span>
                            <span class="window-url">"cloud.example.eu/drive"</span>
                        </div>
                        <img
                            src="/assets/screenshots/drive.png"
                            alt="The artcraftCloud drive showing folders and files"
                        />
                    </div>
                </div>

                <ModePreview />
            </div>
        </section>

        <section class="section alt" id="how">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"How it works"</span>
                    <h2>"Three steps, one small server"</h2>
                    <p>
                        "No Kubernetes, no monthly SaaS bill, no data leaving the EU. Pick a host,
                        boot the stack, point your domain."
                    </p>
                </div>
                <div class="steps">
                    <div class="step">
                        <div class="step-num">"1"</div>
                        <h3>"Pick a European host"</h3>
                        <p>
                            "Compare seven EU providers by price, jurisdiction and mail policy. Four
                            can be provisioned with one API token; three use a guided recipe."
                        </p>
                        <div class="step-tags">
                            <span class="badge">"Hetzner"</span>
                            <span class="badge">"UpCloud"</span>
                            <span class="badge">"Scaleway"</span>
                            <span class="badge">"Contabo"</span>
                            <span class="badge">"+3 guided"</span>
                        </div>
                    </div>
                    <div class="step">
                        <div class="step-num">"2"</div>
                        <h3>"Launch in one click"</h3>
                        <p>
                            "We generate a cloud-init recipe that installs Docker, Caddy with
                            automatic HTTPS, the drive and — if you want it — Stalwart mail. Watch it
                            boot live."
                        </p>
                        <div class="step-tags">
                            <span class="badge success">"Auto-HTTPS"</span>
                            <span class="badge">"cloud-init"</span>
                            <span class="badge">"SSE progress"</span>
                        </div>
                    </div>
                    <div class="step">
                        <div class="step-num">"3"</div>
                        <h3>"Point your domain"</h3>
                        <p>
                            "Copy the generated A, MX, SPF, DMARC and DKIM records into your DNS
                            provider. The wizard checks propagation and tells you when you are live."
                        </p>
                        <div class="step-tags">
                            <span class="badge">"A / MX"</span>
                            <span class="badge">"SPF / DMARC"</span>
                            <span class="badge">"DKIM"</span>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <AppsShowcase />

        <section class="section">
            <div class="container">
                <div class="shots">
                    <div class="shot">
                        <img src="/assets/screenshots/app-wordcraft.png" alt="WordCraft editing a document" />
                    </div>
                    <div class="col">
                        <div class="shot">
                            <img src="/assets/screenshots/mail.png" alt="MailCraft with a mailbox" />
                        </div>
                        <div class="shot">
                            <img src="/assets/screenshots/calendarcraft.png" alt="CalendarCraft" />
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <section class="section alt">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"Why self-host"</span>
                    <h2>"Boring on purpose"</h2>
                    <p>"Plain files, standard protocols, and a stack you can audit in an afternoon."</p>
                </div>
                <div class="pillars">
                    <div class="pillar">
                        <div class="pillar-icon">
                            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                                <path d="M3 7.5 12 3l9 4.5v9L12 21l-9-4.5v-9Z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" />
                                <path d="M3 7.5 12 12l9-4.5M12 12v9" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" />
                            </svg>
                        </div>
                        <h3>"Plain folders on disk"</h3>
                        <p>"Your files are ordinary files. rsync, back up, or walk away with them."</p>
                    </div>
                    <div class="pillar">
                        <div class="pillar-icon">
                            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                                <path d="M12 3 5 6v6c0 4.2 2.9 7.6 7 9 4.1-1.4 7-4.8 7-9V6l-7-3Z" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" />
                                <path d="m9 12 2 2 4-4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" />
                            </svg>
                        </div>
                        <h3>"Signed app registry"</h3>
                        <p>"Every Craft app is verified against OpenPGP or Ed25519 keys before install."</p>
                    </div>
                    <div class="pillar">
                        <div class="pillar-icon">
                            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                                <rect x="3" y="4" width="18" height="12" rx="2" stroke="currentColor" stroke-width="1.7" />
                                <path d="M8 20h8M12 16v4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
                            </svg>
                        </div>
                        <h3>"~10 MB idle"</h3>
                        <p>"A single Rust binary. The smallest tier on any host is enough for Drive."</p>
                    </div>
                    <div class="pillar">
                        <div class="pillar-icon">
                            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                                <circle cx="12" cy="12" r="8.5" stroke="currentColor" stroke-width="1.7" />
                                <path d="M3.5 12h17M12 3.5c2.5 2.4 3.8 5.4 3.8 8.5S14.5 18.1 12 20.5C9.5 18.1 8.2 15.1 8.2 12S9.5 5.9 12 3.5Z" stroke="currentColor" stroke-width="1.7" />
                            </svg>
                        </div>
                        <h3>"EU-only hosts"</h3>
                        <p>"Germany, Finland, France, Bulgaria, Cyprus, Austria. Your data stays in Europe."</p>
                    </div>
                </div>
            </div>
        </section>

        <HostsPreview />

        <section class="section">
            <div class="container">
                <div class="cta-band">
                    <div>
                        <h2>"No fees. If it saved you a weekend, buy me a coffee."</h2>
                        <p>
                            "artcraftCloud is free and open source. Hosting costs go straight to your
                            VPS provider — there is no markup, no account, and no telemetry."
                        </p>
                    </div>
                    <div class="cta-actions">
                        <a class="button primary large" href="/deploy">"Deploy my cloud"</a>
                        <a
                            class="button large"
                            href="https://liberapay.com/"
                            target="_blank"
                            rel="noopener"
                        >
                            "☕ Buy me a coffee"
                        </a>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// Drive-only vs Drive + Mail preview in the hero. Static HTML: two radio buttons switch the
/// visible body with CSS (`:has`), so the marketing pages ship no Wasm.
#[component]
fn ModePreview() -> impl IntoView {
    view! {
        <div class="mode-card">
            <div class="segments" role="radiogroup" aria-label="Deployment mode">
                <input class="sr-only" type="radio" name="mode-preview" id="mp-drive" checked />
                <label class="segment" for="mp-drive">"Drive only"</label>
                <input class="sr-only" type="radio" name="mode-preview" id="mp-mail" />
                <label class="segment" for="mp-mail">"Drive + Mail"</label>
            </div>
            <ModeBody mode=DeployMode::Drive />
            <ModeBody mode=DeployMode::DriveMail />
        </div>
    }
}

#[component]
fn ModeBody(mode: DeployMode) -> impl IntoView {
    let (class, blurb, cpu, third) = match mode {
        DeployMode::Drive => (
            "mode-body mode-drive",
            "The drive and all ten Craft apps.",
            "1 vCPU",
            "Safe ETag saves and automatic HTTPS",
        ),
        DeployMode::DriveMail => (
            "mode-body mode-mail",
            "Everything in Drive, plus Stalwart mail and calendar on your domain.",
            "2 vCPU",
            "MailCraft + CalendarCraft on your own domain (IMAP, SMTP, JMAP, CalDAV)",
        ),
    };
    view! {
        <div class=class>
            <div class="mode-head">
                <div>
                    <div class="mode-price">{mode.price_note()} <small>" to your host"</small></div>
                    <div class="tiny muted mt-8">{blurb}</div>
                </div>
                <span class="badge success"><span class="dot"></span>{mode.label()}</span>
            </div>
            <div class="spec-row">
                <div class="spec"><b>{mode.ram()}</b><span>"RAM"</span></div>
                <div class="spec"><b>{mode.disk()}</b><span>"Disk"</span></div>
                <div class="spec"><b>{cpu}</b><span>"Compute"</span></div>
            </div>
            <ul class="mode-list">
                <li><span class="tick"><Tick /></span>"Folder-backed drive with WebDAV at " <code>"/dav/"</code></li>
                <li><span class="tick"><Tick /></span>"WordCraft, GridCraft, DeckCraft, PdfCraft and six more"</li>
                <li><span class="tick"><Tick /></span>{third}</li>
            </ul>
            <a class="button primary block" href="/deploy">{format!("Deploy {}", mode.label())}</a>
        </div>
    }
}

#[component]
fn AppsShowcase() -> impl IntoView {
    let reg = registry();

    view! {
        <section class="section" id="apps">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"The Craft suite"</span>
                    <h2>"Ten apps. Your real files."</h2>
                    <p>
                        "Native-speed Rust and WebAssembly apps that open standard formats — no
                        import step, no proprietary cloud."
                    </p>
                </div>
                <div class="app-grid">
                    {reg
                        .apps
                        .iter()
                        .map(|app| {
                            view! {
                                <article class="app-card">
                                    <img class="app-icon" src=app.icon_path() alt=format!("{} icon", app.title) />
                                    <h3>{app.title.clone()}</h3>
                                    <p>{app.blurb.clone()}</p>
                                    <div class="exts" title=app.extension_list()>
                                        {app.extension_list()}
                                    </div>
                                </article>
                            }
                        })
                        .collect_view()}
                </div>
                <div class="row mt-28">
                    <a class="button" href="/apps">"See all apps and file types"</a>
                    <span class="tiny muted">
                        "Registry serial " {reg.serial.to_string()} " · every app signature-checked"
                    </span>
                </div>
            </div>
        </section>
    }
}

#[component]
fn HostsPreview() -> impl IntoView {
    view! {
        <section class="section alt">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"European hosts"</span>
                    <h2>"Pick a server you trust"</h2>
                    <p>
                        "Prices are the hosts' own, excluding VAT. We never mark them up — you pay the
                        provider directly."
                    </p>
                </div>
                <div class="table-wrap">
                    <table class="data">
                        <thead>
                            <tr>
                                <th>"Provider"</th>
                                <th>"Location"</th>
                                <th>"From"</th>
                                <th>"Setup"</th>
                                <th class="hide-mobile">"Port 25"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {HOSTS
                                .iter()
                                .map(|h| {
                                    let tier_class = match h.tier {
                                        Tier::Api => "badge primary",
                                        Tier::Guided => "badge",
                                    };
                                    let mail_class = if h.port25.bad() { "badge warning" } else { "badge success" };
                                    view! {
                                        <tr>
                                            <td class="name">{h.name}</td>
                                            <td class="muted">{h.country}</td>
                                            <td class="price">{h.price_from} " / mo"</td>
                                            <td>
                                                <span class=tier_class>
                                                    {match h.tier {
                                                        Tier::Api => "1-click API",
                                                        Tier::Guided => "Guided recipe",
                                                    }}
                                                </span>
                                            </td>
                                            <td class="hide-mobile">
                                                <span class=mail_class>{h.port25.label()}</span>
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
                <div class="row mt-20">
                    <a class="button" href="/hosts">"Compare all seven hosts"</a>
                    <span class="tiny muted">
                        "Mail needs Port 25; a cheap EU relay is a fine fallback."
                    </span>
                </div>
            </div>
        </section>
    }
}
