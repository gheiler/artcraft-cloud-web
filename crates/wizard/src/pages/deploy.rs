use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use types::{
    dns_records, DeployAccepted, DeployMode, DeployRequest, JobStatus, Port25, Tier, HOSTS,
};

#[derive(Clone, PartialEq)]
enum Status {
    Idle,
    Submitting,
    Running(String),
    Done(String),
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

    let submit = move |_| {
        set_error.set(None);
        let host = selected();

        if email.get().trim().is_empty() || !email.get().contains('@') {
            set_error.set(Some("Add a notification email so we can send the login details.".into()));
            return;
        }
        if !skip_domain.get() && domain.get().trim().is_empty() {
            set_error.set(Some("Add your domain, or tick “configure it later”.".into()));
            return;
        }
        if host.tier == Tier::Api && token.get().trim().is_empty() {
            set_error.set(Some(format!(
                "{} needs an API token for one-click deploy. Or pick a guided host.",
                host.name
            )));
            return;
        }

        let req = DeployRequest {
            provider: host.id.to_string(),
            mode: mode.get(),
            domain: domain.get(),
            email: email.get(),
            api_token: token.get(),
            region: None,
            skip_domain: skip_domain.get(),
        };

        set_status.set(Status::Submitting);

        spawn_local(async move {
            let sent = Request::post("/api/deploy").json(&req);
            let response = match sent {
                Ok(request) => request.send().await,
                Err(e) => {
                    set_status.set(Status::Failed(format!("Could not encode request: {e}")));
                    return;
                }
            };

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    set_status.set(Status::Failed(format!("Request failed: {e}")));
                    return;
                }
            };

            if !response.ok() {
                set_status.set(Status::Failed(format!(
                    "Server returned {} — please try again.",
                    response.status()
                )));
                return;
            }

            let accepted: DeployAccepted = match response.json().await {
                Ok(v) => v,
                Err(e) => {
                    set_status.set(Status::Failed(format!("Bad response: {e}")));
                    return;
                }
            };

            let job_id = accepted.job_id.clone();
            set_status.set(Status::Running("queued".into()));

            // Poll until the backend reports the server is ready.
            loop {
                TimeoutFuture::new(1500).await;
                let url = format!("/api/deploy/{job_id}");
                let Ok(resp) = Request::get(&url).send().await else {
                    continue;
                };
                let Ok(job) = resp.json::<JobStatus>().await else {
                    continue;
                };
                match job.status.as_str() {
                    "done" => {
                        set_status.set(Status::Done(
                            job.ip.unwrap_or_else(|| "your server".into()),
                        ));
                        break;
                    }
                    "failed" => {
                        set_status.set(Status::Failed(
                            job.detail.unwrap_or_else(|| "Provisioning failed.".into()),
                        ));
                        break;
                    }
                    other => set_status.set(Status::Running(other.to_string())),
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
                        "Domain & mail"
                    </button>
                    <button
                        class=move || step_class(step.get(), 4)
                        on:click=move |_| set_step.set(4)
                    >
                        <span class="n">"4"</span>
                        "Review & deploy"
                    </button>
                    <div class="stepper-foot tiny muted">
                        "API tokens are held in memory only, for the duration of the install."
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
                                                        <span class=tier_class(h.tier)>{h.tier.label()}</span>
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
                                                " We can set up an EU outbound relay instead."
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
                                    <h2>"Domain, mail and notifications"</h2>
                                    <p>"Where your cloud will live, and where we send the keys."</p>
                                </div>
                            </div>
                            <div class="form-grid">
                                <label class="field-label">
                                    "Domain"
                                    <span class="hint">"e.g. cloud.example.eu — leave empty if you will attach it later"</span>
                                    <input
                                        class="field"
                                        type="text"
                                        placeholder="cloud.example.eu"
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
                                        "Skip domain / email configuration for now"
                                        <span class="tiny muted">
                                            " Boot on http://<ip>:8080 and attach the domain manually later."
                                        </span>
                                    </span>
                                </label>

                                <label class="field-label">
                                    "Notification email"
                                    <span class="hint">"We email you when the server is ready — and only then."</span>
                                    <input
                                        class="field"
                                        type="email"
                                        placeholder="you@example.eu"
                                        prop:value=move || email.get()
                                        on:input=move |ev| set_email.set(event_target_value(&ev))
                                    />
                                </label>

                                {move || {
                                    if selected().tier == Tier::Api {
                                        view! {
                                            <label class="field-label">
                                                {move || format!("{} API token", selected().name)}
                                                <span class="hint">
                                                    "Read-only provisioning scope. Held in memory only — never written to disk."
                                                </span>
                                                <input
                                                    class="field"
                                                    type="password"
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
                                                    "Guided host: after ordering, we give you a cloud-init recipe to paste.
                                                    No API token needed."
                                                </span>
                                            </div>
                                        }.into_any()
                                    }
                                }}

                                {move || {
                                    if skip_domain.get() {
                                        view! {
                                            <div class="note-box warn">
                                                <span class="ico">"⚠"</span>
                                                <span>
                                                    "WebGPU and clipboard in the Craft apps need a secure context (https://).
                                                    You can add the domain and a 3-line Caddyfile later."
                                                </span>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div>
                                                <div class="spread mt-8">
                                                    <b class="small">"DNS records to add"</b>
                                                    <span class="tiny muted">"Your registrar or DNS provider"</span>
                                                </div>
                                                <div class="table-wrap mt-8">
                                                    <table class="dns-table">
                                                        <thead>
                                                            <tr>
                                                                <th>"Type"</th>
                                                                <th>"Name"</th>
                                                                <th>"Value"</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            {dns_records(&domain.get(), mail(), "")
                                                                .into_iter()
                                                                .map(|r| {
                                                                    view! {
                                                                        <tr>
                                                                            <td><b>{r.kind}</b></td>
                                                                            <td><code>{r.name}</code></td>
                                                                            <td><code>{r.value}</code></td>
                                                                        </tr>
                                                                    }
                                                                })
                                                                .collect_view()}
                                                        </tbody>
                                                    </table>
                                                </div>
                                                <div class="tiny muted mt-8">
                                                    "The real server IP is filled in after provisioning; DKIM appears once Stalwart boots."
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
                                    <h2>"Review and deploy"</h2>
                                    <p>"One last look before we boot your server."</p>
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
                                    <span class="k">"Domain"</span>
                                    <span class="v">
                                        {move || if skip_domain.get() {
                                            "Attach later (http://<ip>:8080)".to_string()
                                        } else if domain.get().is_empty() {
                                            "—".to_string()
                                        } else {
                                            format!("https://{}", domain.get())
                                        }}
                                    </span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Notification email"</span>
                                    <span class="v">{move || email.get()}</span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Server size"</span>
                                    <span class="v">
                                        {move || format!("{} RAM · {} disk", mode.get().ram(), mode.get().disk())}
                                    </span>
                                </div>
                                <div class="review-row">
                                    <span class="k">"Setup"</span>
                                    <span class="v">{move || selected().tier.label()}</span>
                                </div>
                            </div>

                            {move || match status.get() {
                                Status::Idle | Status::Submitting => view! { <div></div> }.into_any(),
                                Status::Running(s) => view! {
                                    <div class="status-box mt-20">
                                        <div class="status-line">
                                            <span class="spinner"></span>
                                            <b>"Provisioning…"</b>
                                            <span class="muted tiny">{s}</span>
                                        </div>
                                        <div class="progress indeterminate"><div class="progress-bar"></div></div>
                                        <div class="tiny muted">
                                            "Installing Docker, Caddy and the drive. This usually takes 1–3 minutes."
                                        </div>
                                    </div>
                                }.into_any(),
                                Status::Done(ip) => view! {
                                    <div class="status-box mt-20">
                                        <div class="status-line">
                                            <span class="badge success"><span class="dot"></span>"Server online"</span>
                                            <span class="tiny muted">{ip}</span>
                                        </div>
                                        <div class="small">
                                            "We emailed your login details to "
                                            <b>{move || email.get()}</b>
                                            ". Open "
                                            <a href=move || if skip_domain.get() { "#".to_string() } else { format!("https://{}", domain.get()) }>
                                                {move || if skip_domain.get() { "your server".to_string() } else { domain.get() }}
                                            </a>
                                            " and sign in."
                                        </div>
                                    </div>
                                }.into_any(),
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
                                <button class="button" on:click=move |_| set_step.set(3)>"← Back"</button>
                                <button
                                    class="button primary"
                                    disabled=move || matches!(status.get(), Status::Submitting | Status::Running(_))
                                    on:click=submit
                                >
                                    {move || match status.get() {
                                        Status::Done(_) => "Deployed ✓",
                                        _ => "Deploy my cloud",
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
