//! The deploy wizard: what to run, which host, domain, then a one-click deploy (hosts with an
//! automated API client) or a filled-in cloud-init recipe to paste (every other host).

use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use types::{
    dns_records, CloudInitRecipe, DeployAccepted, DeployMode, DeployRequest, JobStatus, Port25,
    Tier, HOSTS,
};
use web_sys::RequestMode;

#[derive(Clone, PartialEq)]
enum Status {
    Idle,
    Submitting,
    /// Job status from the server while the provider creates and boots the server.
    Running(JobStatus),
    /// The server is up (cloud-init is installing); the browser now waits for HTTPS.
    Up(JobStatus),
    /// A recipe to paste, for hosts we do not automate.
    Recipe(CloudInitRecipe),
    Failed(String),
}

fn tier_class(tier: Tier) -> &'static str {
    match tier {
        Tier::Api => "badge primary",
        Tier::Guided => "badge",
    }
}

fn step_class(current: u8, mine: u8) -> &'static str {
    if current == mine {
        "stepper-item on"
    } else if current > mine {
        "stepper-item done"
    } else {
        "stepper-item"
    }
}

fn port_class(p: Port25) -> &'static str {
    if p.bad() {
        "badge warning"
    } else {
        "badge success"
    }
}

/// Reads `{"error": "..."}` from a failed API reply.
async fn api_error(resp: gloo_net::http::Response) -> String {
    let status = resp.status();
    match resp.json::<serde_json::Value>().await {
        Ok(v) => v["error"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("Server returned {status}.")),
        Err(_) => format!("Server returned {status}, please try again."),
    }
}

/// True once `https://<host>/healthz` answers. A `no-cors` fetch cannot read the reply, but it
/// only resolves when DNS, TLS and the server all work, which is exactly what we want to know.
async fn reachable(url: &str) -> bool {
    Request::get(&format!("{url}/healthz"))
        .mode(RequestMode::NoCors)
        .send()
        .await
        .is_ok()
}

fn copy_to_clipboard(text: String) {
    if let Some(w) = web_sys::window() {
        let _ = w.navigator().clipboard().write_text(&text);
    }
}

#[component]
pub fn DeployPage() -> impl IntoView {
    let (step, set_step) = signal(1u8);
    let (mode, set_mode) = signal(DeployMode::Drive);
    let (provider_id, set_provider_id) = signal("hetzner");
    let (tier_filter, set_tier_filter) = signal(None::<Tier>);
    let (domain, set_domain) = signal(String::new());
    let (skip_domain, set_skip_domain) = signal(false);
    let (email, set_email) = signal(String::new());
    let (token, set_token) = signal(String::new());
    let (status, set_status) = signal(Status::Idle);
    let (error, set_error) = signal(None::<String>);

    let selected = move || {
        HOSTS
            .iter()
            .find(|h| h.id == provider_id.get())
            .copied()
            .unwrap_or(HOSTS[0])
    };

    let (online, set_online) = signal(false);

    let request = move || DeployRequest {
        provider: selected().id.to_string(),
        mode: mode.get(),
        domain: domain.get(),
        email: email.get(),
        api_token: token.get(),
        region: None,
        skip_domain: skip_domain.get(),
    };

    // Watches https://<url>/healthz from the browser until the install finishes.
    let watch = move |url: String| {
        spawn_local(async move {
            for _ in 0..120 {
                if reachable(&url).await {
                    set_online.set(true);
                    return;
                }
                TimeoutFuture::new(5000).await;
            }
        });
    };

    let submit = move |_| {
        set_error.set(None);
        set_online.set(false);
        let host = selected();
        let req = request();
        if let Err(e) = req.validate() {
            set_error.set(Some(e));
            return;
        }
        if host.one_click() && req.api_token.trim().is_empty() {
            set_error.set(Some(format!(
                "{} needs an API token for one-click deploy.",
                host.name
            )));
            return;
        }
        set_status.set(Status::Submitting);

        if !host.one_click() {
            spawn_local(async move {
                let resp = match Request::post("/api/cloud-init").json(&req) {
                    Ok(r) => r.send().await,
                    Err(e) => {
                        set_status
                            .set(Status::Failed(format!("Could not encode the request: {e}")));
                        return;
                    }
                };
                match resp {
                    Ok(r) if r.ok() => match r.json::<CloudInitRecipe>().await {
                        Ok(recipe) => set_status.set(Status::Recipe(recipe)),
                        Err(e) => set_status.set(Status::Failed(format!("Bad response: {e}"))),
                    },
                    Ok(r) => set_status.set(Status::Failed(api_error(r).await)),
                    Err(e) => set_status.set(Status::Failed(format!("Request failed: {e}"))),
                }
            });
            return;
        }

        spawn_local(async move {
            let resp = match Request::post("/api/deploy").json(&req) {
                Ok(r) => r.send().await,
                Err(e) => {
                    set_status.set(Status::Failed(format!("Could not encode the request: {e}")));
                    return;
                }
            };
            let resp = match resp {
                Ok(r) if r.ok() => r,
                Ok(r) => {
                    set_status.set(Status::Failed(api_error(r).await));
                    return;
                }
                Err(e) => {
                    set_status.set(Status::Failed(format!("Request failed: {e}")));
                    return;
                }
            };
            let accepted: DeployAccepted = match resp.json().await {
                Ok(v) => v,
                Err(e) => {
                    set_status.set(Status::Failed(format!("Bad response: {e}")));
                    return;
                }
            };
            // The token is not needed any more; drop it from the page too.
            set_token.set(String::new());

            // Poll until the server is up (status "installing") or the job failed.
            let url = format!("/api/deploy/{}", accepted.job_id);
            let mut misses = 0;
            loop {
                TimeoutFuture::new(2000).await;
                let job = match Request::get(&url).send().await {
                    Ok(r) if r.ok() => r.json::<JobStatus>().await.ok(),
                    _ => None,
                };
                let Some(job) = job else {
                    misses += 1;
                    if misses > 30 {
                        set_status.set(Status::Failed(
                            "Lost track of the deploy. Check your host's console.".into(),
                        ));
                        return;
                    }
                    continue;
                };
                match job.status.as_str() {
                    "installing" | "done" => {
                        if let Some(u) = job.url.clone() {
                            watch(u);
                        }
                        set_status.set(Status::Up(job));
                        return;
                    }
                    "failed" => {
                        set_status.set(Status::Failed(
                            job.detail.unwrap_or_else(|| "Provisioning failed.".into()),
                        ));
                        return;
                    }
                    _ => set_status.set(Status::Running(job)),
                }
            }
        });
    };

    let mail = move || mode.get() == DeployMode::DriveMail;

    view! {
        <div class="container">
            <div class="wizard-head">
                <span class="kicker">"Deploy"</span>
                <h1>"Your own cloud, in about two minutes"</h1>
                <p>
                    "Pick what to run, choose an EU host, and we generate the exact cloud-init
                    recipe for it. No account, no fee — you pay the host directly."
                </p>
            </div>

            <div class="wizard">
                <aside class="stepper">
                    <button
                        class=move || step_class(step.get(), 1)
                        on:click=move |_| set_step.set(1)
                    >
                        <span class="n">{move || if step.get() > 1 { "✓" } else { "1" }}</span>
                        "What to run"
                    </button>
                    <button
                        class=move || step_class(step.get(), 2)
                        on:click=move |_| set_step.set(2)
                    >
                        <span class="n">{move || if step.get() > 2 { "✓" } else { "2" }}</span>
                        "Host & region"
                    </button>
                    <button
                        class=move || step_class(step.get(), 3)
                        on:click=move |_| set_step.set(3)
                    >
                        <span class="n">{move || if step.get() > 3 { "✓" } else { "3" }}</span>
                        "Domain"
                    </button>
                    <button
                        class=move || step_class(step.get(), 4)
                        on:click=move |_| set_step.set(4)
                    >
                        <span class="n">"4"</span>
                        "Review & deploy"
                    </button>
                    <div class="stepper-foot tiny muted">
                        "API tokens stay in memory for the few minutes the server takes to boot, then they are dropped. Nothing is stored."
                    </div>
                </aside>

                <div class="panel">
                    {move || match step.get() {
                        1 => view! {
                            <div class="panel-head">
                                <div>
                                    <h2>"What do you want to run?"</h2>
                                    <p>"Start with the drive, or add your own mail and calendar."</p>
                                </div>
                            </div>
                            <div class="choice-grid">
                                <button
                                    class=move || if mode.get() == DeployMode::Drive { "choice on" } else { "choice" }
                                    on:click=move |_| set_mode.set(DeployMode::Drive)
                                >
                                    <div class="choice-top">
                                        <h3>"Drive only"</h3>
                                        <span class="badge success">"Lightest"</span>
                                    </div>
                                    <div class="price">"€3–5" <small>"/ month"</small></div>
                                    <ul>
                                        <li>"Folder-backed drive + WebDAV"</li>
                                        <li>"All ten Craft apps"</li>
                                        <li>"Needs 1 GB RAM · 20 GB disk"</li>
                                    </ul>
                                </button>
                                <button
                                    class=move || if mode.get() == DeployMode::DriveMail { "choice on" } else { "choice" }
                                    on:click=move |_| set_mode.set(DeployMode::DriveMail)
                                >
                                    <div class="choice-top">
                                        <h3>"Drive + Mail"</h3>
                                        <span class="badge primary">"Most complete"</span>
                                    </div>
                                    <div class="price">"€5–7" <small>"/ month"</small></div>
                                    <ul>
                                        <li>"Everything in Drive"</li>
                                        <li>"Stalwart mail + calendar on your domain"</li>
                                        <li>"Needs 2 GB RAM · 40 GB disk"</li>
                                    </ul>
                                </button>
                            </div>
                            <div class="note-box info mt-20">
                                <span class="ico">"ⓘ"</span>
                                <span>
                                    "Both modes include automatic HTTPS. Mail needs outbound Port 25
                                    (or an EU relay) — we flag this per host."
                                </span>
                            </div>
                            <div class="panel-foot">
                                <span class="tiny muted">"Step 1 of 4"</span>
                                <button class="button primary" on:click=move |_| set_step.set(2)>
                                    "Choose a host →"
                                </button>
                            </div>
                        }.into_any(),

                        2 => view! {
                            <div class="panel-head">
                                <div>
                                    <h2>"Pick your European host"</h2>
                                    <p>
                                        {move || format!(
                                            "{} needs at least {} RAM and {} disk.",
                                            mode.get().label(),
                                            mode.get().ram(),
                                            mode.get().disk(),
                                        )}
                                    </p>
                                </div>
                            </div>
                            <div class="tier-tabs" role="tablist">
                                <button
                                    class=move || if tier_filter.get().is_none() { "tier-tab on" } else { "tier-tab" }
                                    on:click=move |_| set_tier_filter.set(None)
                                >
                                    "All"
                                </button>
                                <button
                                    class=move || if tier_filter.get() == Some(Tier::Api) { "tier-tab on" } else { "tier-tab" }
                                    on:click=move |_| set_tier_filter.set(Some(Tier::Api))
                                >
                                    "1-click API"
                                </button>
                                <button
                                    class=move || if tier_filter.get() == Some(Tier::Guided) { "tier-tab on" } else { "tier-tab" }
                                    on:click=move |_| set_tier_filter.set(Some(Tier::Guided))
                                >
                                    "Guided"
                                </button>
                            </div>
                            <div class="provider-grid">
                                {move || {
                                    let filter = tier_filter.get();
                                    HOSTS
                                        .iter()
                                        .filter(|h| filter.is_none_or(|t| t == h.tier))
                                        .map(|h| {
                                            let id = h.id;
                                            view! {
                                                <button
                                                    class=move || if provider_id.get() == id { "provider on" } else { "provider" }
                                                    on:click=move |_| set_provider_id.set(id)
                                                >
                                                    <div class="provider-top">
                                                        <h3>{h.name}</h3>
                                                        <span class=port_class(h.port25)>{h.port25.label()}</span>
                                                    </div>
                                                    <div class="meta">
                                                        {h.country} " · " {h.datacenters}
                                                    </div>
                                                    <div class="row">
                                                        <span class="badge">{h.price_from} " / mo"</span>
                                                        <span class=tier_class(h.tier)>{h.setup_label()}</span>
                                                    </div>
                                                    <div class="meta">{h.specs}</div>
                                                    <div class="tiny muted">{h.integration}</div>
                                                </button>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                            {move || {
                                let host = selected();
                                if host.port25.bad() && mail() {
                                    view! {
                                        <div class="note-box warn mt-20">
                                            <span class="ico">"⚠"</span>
                                            <span>
                                                <b>{host.name}</b> " — " {host.mail_note}
                                                " Plan on an EU outbound relay at first: "
                                                <a href="/guide#mail">"how"</a>"."
                                            </span>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}
                            <div class="panel-foot">
                                <button class="button" on:click=move |_| set_step.set(1)>"← Back"</button>
                                <button class="button primary" on:click=move |_| set_step.set(3)>
                                    "Continue →"
                                </button>
                            </div>
                        }.into_any(),

                        3 => view! {
                            <div class="panel-head">
                                <div>
                                    <h2>"Domain and access"</h2>
                                    <p>"Where your cloud will live."</p>
                                </div>
                            </div>
                            <div class="form-grid">
                                <label class="field-label">
                                    "Domain"
                                    <span class="hint">"e.g. cloud.example.eu. You add one DNS record once the server has its IP."</span>
                                    <input
                                        class="field"
                                        type="text"
                                        placeholder="cloud.example.eu"
                                        autocomplete="off"
                                        spellcheck="false"
                                        prop:disabled=move || skip_domain.get()
                                        prop:value=move || domain.get()
                                        on:input=move |ev| set_domain.set(event_target_value(&ev))
                                    />
                                </label>

                                <label class="check">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || skip_domain.get()
                                        on:change=move |ev| set_skip_domain.set(event_target_checked(&ev))
                                    />
                                    <span>
                                        "No domain yet, set it up later"
                                        <span class="tiny muted">
                                            " You get a free https://<server-ip>.sslip.io address with a real certificate; add your own domain whenever you like."
                                        </span>
                                    </span>
                                </label>

                                <label class="field-label">
                                    "Email for certificate notices (optional)"
                                    <span class="hint">"Given to Let's Encrypt only, so they can warn you if a certificate fails to renew."</span>
                                    <input
                                        class="field"
                                        type="email"
                                        placeholder="you@example.eu"
                                        prop:value=move || email.get()
                                        on:input=move |ev| set_email.set(event_target_value(&ev))
                                    />
                                </label>

                                {move || {
                                    let host = selected();
                                    if host.one_click() {
                                        view! {
                                            <label class="field-label">
                                                {format!("{} API token", host.name)}
                                                <span class="hint">
                                                    "Project › Security › API tokens › Generate, with Read & Write. Used once to create the server, never stored. Delete it afterwards if you like."
                                                </span>
                                                <input
                                                    class="field"
                                                    type="password"
                                                    autocomplete="off"
                                                    placeholder="paste your token"
                                                    prop:value=move || token.get()
                                                    on:input=move |ev| set_token.set(event_target_value(&ev))
                                                />
                                            </label>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="note-box">
                                                <span class="ico">"ⓘ"</span>
                                                <span>
                                                    {format!("{}: you order the server in their panel and paste the recipe we generate in the next step. No API token needed.", host.name)}
                                                </span>
                                            </div>
                                        }.into_any()
                                    }
                                }}

                                {move || {
                                    if skip_domain.get() || domain.get().trim().is_empty() {
                                        view! { <div></div> }.into_any()
                                    } else {
                                        view! {
                                            <div>
                                                <div class="spread mt-8">
                                                    <b class="small">"DNS records you will add"</b>
                                                    <span class="tiny muted">"At your registrar or DNS provider"</span>
                                                </div>
                                                <DnsTable domain=domain.get() mail=mail() ip=String::new() />
                                                <div class="tiny muted mt-8">
                                                    "The server IP is filled in once the server exists; DKIM appears after Stalwart's first start."
                                                </div>
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                            <div class="panel-foot">
                                <button class="button" on:click=move |_| set_step.set(2)>"← Back"</button>
                                <button class="button primary" on:click=move |_| set_step.set(4)>
                                    "Review →"
                                </button>
                            </div>
                        }.into_any(),

                        _ => view! {
                            <div class="panel-head">
                                <div>
                                    <h2>{move || if selected().one_click() { "Review and deploy" } else { "Review and get your recipe" }}</h2>
                                    <p>{move || if selected().one_click() {
                                        "One last look before we create your server."
                                    } else {
                                        "We fill in the cloud-init recipe; you paste it when you order the server."
                                    }}</p>
                                </div>
                            </div>
                            <div class="review">
                                <div class="review-row">
                                    <span class="k">"Mode"</span>
                                    <span class="v">{move || mode.get().label()}</span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Host"</span>
                                    <span class="v">
                                        {move || format!("{} · {}", selected().name, selected().country)}
                                    </span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Address"</span>
                                    <span class="v">
                                        {move || if skip_domain.get() || domain.get().trim().is_empty() {
                                            "https://<server-ip>.sslip.io".to_string()
                                        } else {
                                            format!("https://{}", domain.get().trim())
                                        }}
                                    </span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Server size"</span>
                                    <span class="v">
                                        {move || format!("{} RAM · {} disk or more", mode.get().ram(), mode.get().disk())}
                                    </span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Setup"</span>
                                    <span class="v">{move || selected().setup_label()}</span>
                                </div>
                            </div>

                            {move || match status.get() {
                                Status::Idle | Status::Submitting => view! { <div></div> }.into_any(),
                                Status::Running(job) => view! {
                                    <div class="status-box mt-20">
                                        <div class="status-line">
                                            <span class="spinner"></span>
                                            <b>"Creating your server…"</b>
                                            <span class="muted tiny">{job.detail.unwrap_or(job.status)}</span>
                                        </div>
                                        <div class="progress indeterminate"><div class="progress-bar"></div></div>
                                        <div class="tiny muted">"This takes about a minute. Keep this page open."</div>
                                    </div>
                                }.into_any(),
                                Status::Up(job) => {
                                    let url = job.url.clone().unwrap_or_default();
                                    let ip = job.ip.clone().unwrap_or_default();
                                    let user = job.admin_user.clone().unwrap_or_else(|| "admin".into());
                                    let password = job.admin_password.clone().unwrap_or_default();
                                    let custom = !skip_domain.get() && !domain.get().trim().is_empty();
                                    let link = url.clone();
                                    let pw_copy = password.clone();
                                    view! {
                                        <div class="status-box mt-20">
                                            <div class="status-line">
                                                {move || if online.get() {
                                                    view! { <span class="badge success"><span class="dot"></span>"Online"</span> }.into_any()
                                                } else {
                                                    view! { <span class="spinner"></span> }.into_any()
                                                }}
                                                <b>{move || if online.get() { "Your cloud is ready" } else { "Server created, installing artcraftCloud…" }}</b>
                                                <span class="tiny muted">{ip.clone()}</span>
                                            </div>
                                            {move || (!online.get()).then(|| view! {
                                                <div class="progress indeterminate"><div class="progress-bar"></div></div>
                                            })}
                                            <div class="small">
                                                {move || if online.get() { "Open " } else { "In 2–4 minutes it answers at " }}
                                                <a href=link.clone() target="_blank" rel="noopener"><b>{link.clone()}</b></a>
                                            </div>
                                            <div class="secret-box">
                                                <div class="spread">
                                                    <b class="small">"Your admin login, shown only here"</b>
                                                    <button class="button small" on:click=move |_| copy_to_clipboard(pw_copy.clone())>"Copy password"</button>
                                                </div>
                                                <div class="mono">"user: " {user} <br /> "password: " {password}</div>
                                                <div class="tiny muted">
                                                    "Save it in your password manager now. It is also on the server in /root/artcraft-admin-password.txt; we keep nothing."
                                                </div>
                                            </div>
                                            {custom.then(|| view! {
                                                <div>
                                                    <b class="small">"Add these DNS records now"</b>
                                                    <DnsTable domain=domain.get_untracked() mail=mail() ip=ip.clone() />
                                                    <div class="tiny muted mt-8">"HTTPS starts working a few minutes after the A record is live."</div>
                                                </div>
                                            })}
                                            <TipCard />
                                        </div>
                                    }.into_any()
                                }
                                Status::Recipe(recipe) => {
                                    let host = selected();
                                    let yaml = recipe.yaml.clone();
                                    let address = recipe.url.clone().unwrap_or_else(|| "https://<server-ip>.sslip.io".into());
                                    view! {
                                        <div class="status-box mt-20">
                                            <ol class="recipe-steps small">
                                                <li>
                                                    "Order an Ubuntu 24.04 (or Debian 12) x86 server at "
                                                    <a href=host.url target="_blank" rel="noopener"><b>{host.name}</b></a>
                                                    {format!(" with at least {} RAM.", mode.get_untracked().ram())}
                                                </li>
                                                <li>"Paste the recipe below into the “cloud-init”, “user data” or “recipe” box while ordering."</li>
                                                <li>{format!("After 3–5 minutes, open {address}. The admin login is in /root/artcraft-admin-password.txt (log in with SSH: cat /root/artcraft-admin-password.txt).")}</li>
                                            </ol>
                                            <div class="spread">
                                                <b class="small">"artcraft-cloud.yaml"</b>
                                                <button class="button small primary" on:click={
                                                    let yaml = yaml.clone();
                                                    move |_| copy_to_clipboard(yaml.clone())
                                                }>"Copy recipe"</button>
                                            </div>
                                            <pre class="recipe">{yaml}</pre>
                                            <div class="tiny muted">
                                                "No cloud-init box at your host? On the fresh server, save this as artcraft-cloud.yaml and run: sudo cloud-init single --name write_files --file artcraft-cloud.yaml && sudo bash /opt/artcraft/install.sh"
                                            </div>
                                            <TipCard />
                                        </div>
                                    }.into_any()
                                }
                                Status::Failed(msg) => view! {
                                    <div class="status-box mt-20">
                                        <div class="status-line">
                                            <span class="badge danger">"Failed"</span>
                                        </div>
                                        <div class="error-text">{msg}</div>
                                    </div>
                                }.into_any(),
                            }}

                            {move || error.get().map(|e| view! { <div class="error-text mt-16">{e}</div> })}

                            <div class="panel-foot">
                                <button class="button" on:click=move |_| { set_status.set(Status::Idle); set_step.set(3) }>"← Back"</button>
                                <button
                                    class="button primary"
                                    disabled=move || matches!(status.get(), Status::Submitting | Status::Running(_) | Status::Up(_))
                                    on:click=submit
                                >
                                    {move || match (status.get(), selected().one_click()) {
                                        (Status::Up(_), _) => "Deployed ✓",
                                        (Status::Recipe(_), _) => "Regenerate recipe",
                                        (_, true) => "Deploy my cloud",
                                        (_, false) => "Get my recipe",
                                    }}
                                </button>
                            </div>
                        }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn DnsTable(domain: String, mail: bool, ip: String) -> impl IntoView {
    view! {
        <div class="table-wrap mt-8">
            <table class="dns-table">
                <thead>
                    <tr><th>"Type"</th><th>"Name"</th><th>"Value"</th></tr>
                </thead>
                <tbody>
                    {dns_records(domain.trim(), mail, &ip)
                        .into_iter()
                        .map(|r| view! {
                            <tr>
                                <td><b>{r.kind}</b></td>
                                <td><code>{r.name}</code></td>
                                <td><code>{r.value}</code></td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn TipCard() -> impl IntoView {
    view! {
        <div class="tip-card">
            <span>"No fees, ever. If this saved you a weekend, a coffee or a beer keeps it going."</span>
            <a class="button small" href=site::TIP_URL target="_blank" rel="noopener">"☕ Buy me a coffee"</a>
        </div>
    }
}
