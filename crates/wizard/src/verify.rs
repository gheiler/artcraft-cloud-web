//! "Verify your server": asks `/api/verify` to check DNS and `https://<host>/healthz`, shows one
//! row per check with what we found and how to fix it, and re-checks every 15 seconds while
//! anything is still pending.

use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use types::{Check, CheckState, VerifyReport, VerifyRequest};

const POLL_MS: u32 = 15_000;
/// 15 s × 60 = 15 minutes of automatic re-checks, enough for a first install.
const MAX_ROUNDS: u32 = 60;

fn badge(state: CheckState) -> &'static str {
    match state {
        CheckState::Ok => "badge success",
        CheckState::Warn => "badge warning",
        CheckState::Fail => "badge danger",
        CheckState::Wait => "badge primary",
        CheckState::Skip => "badge",
    }
}

async fn run(req: VerifyRequest) -> Result<VerifyReport, String> {
    let resp = Request::post("/api/verify")
        .json(&req)
        .map_err(|e| format!("Could not encode the request: {e}"))?
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;
    if resp.ok() {
        resp.json::<VerifyReport>()
            .await
            .map_err(|e| format!("Bad response: {e}"))
    } else {
        let status = resp.status();
        Err(match resp.json::<serde_json::Value>().await {
            Ok(v) => v["error"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("Server returned {status}.")),
            Err(_) => format!("Server returned {status}."),
        })
    }
}

/// `ip`: prefilled after a one-click deploy (and then checking starts on its own).
/// `on_ready`: called once https answers, so the caller can flip its "online" badge.
#[component]
pub fn VerifyPanel(
    domain: String,
    skip_domain: bool,
    mail: bool,
    #[prop(optional)] ip: Option<String>,
    #[prop(optional)] on_online: Option<Callback<()>>,
) -> impl IntoView {
    let auto = ip.is_some();
    let (ipv4, set_ipv4) = signal(ip.unwrap_or_default());
    let (ipv6, set_ipv6) = signal(String::new());
    let (selector, set_selector) = signal(String::new());
    let (report, set_report) = signal(None::<VerifyReport>);
    let (error, set_error) = signal(None::<String>);
    let (busy, set_busy) = signal(false);
    // Bumped on every manual "Check now", so an older polling loop stops.
    let (generation, set_generation) = signal(0u32);
    let custom = !skip_domain && !domain.trim().is_empty();
    let domain_for_req = domain.clone();

    let start = move || {
        let req = VerifyRequest {
            domain: domain_for_req.clone(),
            skip_domain,
            ipv4: ipv4.get_untracked().trim().to_string(),
            ipv6: ipv6.get_untracked().trim().to_string(),
            mail,
            dkim_selector: selector.get_untracked().trim().to_string(),
        };
        if req.ipv4.is_empty() {
            set_error.set(Some(
                "Enter your server's IPv4 address from your host's panel.".into(),
            ));
            return;
        }
        set_generation.update(|g| *g += 1);
        let mine = generation.get_untracked();
        set_error.set(None);
        spawn_local(async move {
            for round in 0..MAX_ROUNDS {
                if generation.get_untracked() != mine {
                    return;
                }
                set_busy.set(true);
                let result = run(req.clone()).await;
                set_busy.set(false);
                if generation.get_untracked() != mine {
                    return;
                }
                match result {
                    Ok(r) => {
                        set_error.set(None);
                        let done = r.ready;
                        if r.https_ok() {
                            if let Some(cb) = on_online {
                                cb.run(());
                            }
                        }
                        set_report.set(Some(r));
                        if done {
                            return;
                        }
                    }
                    Err(e) => {
                        // Validation errors won't fix themselves; rate limits and hiccups will.
                        let retry = retryable(&e);
                        set_error.set(Some(e));
                        if !retry {
                            return;
                        }
                    }
                }
                if round + 1 < MAX_ROUNDS {
                    TimeoutFuture::new(POLL_MS).await;
                }
            }
        });
    };
    let start_click = start.clone();
    if auto {
        start();
    }

    let summary = move || {
        report.get().map(|r| {
            let ok = r.count(CheckState::Ok) + r.count(CheckState::Warn);
            let total = r.checks.len() - r.count(CheckState::Skip);
            let url = r.url.clone();
            if r.ready {
                view! {
                    <div class="status-line">
                        <span class="badge success"><span class="dot"></span>"Ready"</span>
                        <b>"Everything checks out."</b>
                        <a href=url.clone() target="_blank" rel="noopener"><b>{format!("Open {url}")}</b></a>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="status-line">
                        {move || if busy.get() {
                            view! { <span class="spinner"></span> }.into_any()
                        } else {
                            view! { <span class="badge primary">"Re-checking every 15 s"</span> }.into_any()
                        }}
                        <b>{format!("{ok} of {total} checks pass")}</b>
                        <span class="tiny muted">{r.host.clone()}</span>
                    </div>
                }
                .into_any()
            }
        })
    };

    view! {
        <section class="verify mt-20" aria-labelledby="verify-title">
            <div class="spread">
                <h3 id="verify-title" class="verify-title">"Verify your server"</h3>
                <span class="tiny muted">
                    {if custom { "DNS records and HTTPS" } else { "sslip.io address and HTTPS" }}
                </span>
            </div>
            <p class="small muted">
                {if auto {
                    "We check DNS and HTTPS from our server until your cloud answers. Nothing is stored."
                } else {
                    "Once the server is ordered, enter its IP address. We check DNS and HTTPS from our server, and only contact the address you enter."
                }}
            </p>
            <div class="verify-form">
                <label class="field-label">
                    "Server IPv4"
                    <input
                        class="field mono"
                        inputmode="decimal"
                        placeholder="203.0.113.10"
                        prop:value=move || ipv4.get()
                        on:input=move |ev| set_ipv4.set(event_target_value(&ev))
                    />
                </label>
                {custom.then(|| view! {
                    <label class="field-label">
                        <span>"Server IPv6 " <span class="hint">"optional"</span></span>
                        <input
                            class="field mono"
                            placeholder="2a01:4f8:…::1"
                            prop:value=move || ipv6.get()
                            on:input=move |ev| set_ipv6.set(event_target_value(&ev))
                        />
                    </label>
                })}
                {(custom && mail).then(|| view! {
                    <label class="field-label">
                        <span>"DKIM selector " <span class="hint">"from Stalwart, optional"</span></span>
                        <input
                            class="field mono"
                            placeholder="202611e"
                            prop:value=move || selector.get()
                            on:input=move |ev| set_selector.set(event_target_value(&ev))
                        />
                    </label>
                })}
                <button
                    class="button primary verify-go"
                    disabled=move || busy.get()
                    on:click=move |_| start_click()
                >
                    {move || if report.get().is_some() { "Check again" } else { "Check now" }}
                </button>
            </div>
            {move || error.get().map(|e| view! { <div class="error-text">{e}</div> })}
            {summary}
            {move || report.get().map(|r| view! {
                <ul class="checks">
                    {r.checks.into_iter().map(|c| view! { <CheckRow check=c /> }).collect_view()}
                </ul>
            })}
        </section>
    }
}

fn retryable(e: &str) -> bool {
    [
        "Too many",
        "Lots of",
        "This server was checked",
        "Request failed",
    ]
    .iter()
    .any(|p| e.starts_with(p))
}

#[component]
fn CheckRow(check: Check) -> impl IntoView {
    let state = check.state;
    let show_expected = matches!(state, CheckState::Fail | CheckState::Warn)
        && check.expected.as_deref().is_some_and(|e| e != check.found);
    view! {
        <li class=format!("check-row {}", format!("{state:?}").to_ascii_lowercase())>
            <span class=badge(state)>
                {(state == CheckState::Wait).then(|| view! { <span class="spinner tiny-spin"></span> })}
                {state.label()}
            </span>
            <div class="check-body">
                <div class="check-label">{check.label}</div>
                <div class="tiny"><span class="muted">"Found: "</span><code>{check.found}</code></div>
                {show_expected.then(|| view! {
                    <div class="tiny"><span class="muted">"Expected: "</span><code>{check.expected.clone().unwrap_or_default()}</code></div>
                })}
                {check.fix.map(|f| view! { <div class="check-fix small">{f}</div> })}
            </div>
        </li>
    }
}
