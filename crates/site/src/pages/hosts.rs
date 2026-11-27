use leptos::prelude::*;
use types::{Port25, Tier, HOSTS};

fn tier_class(tier: Tier) -> &'static str {
    match tier {
        Tier::Api => "badge primary",
        Tier::Guided => "badge",
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
pub fn HostsPage() -> impl IntoView {
    view! {
        <section class="section">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"European hosts"</span>
                    <h2>"Seven hosts, one recipe"</h2>
                    <p>
                        "We only list providers with EU datacentres. Prices are theirs, excluding
                        VAT. Hetzner is one click with an API token today, and UpCloud, Scaleway and
                        Contabo are next; every host works now with an order plus our cloud-init recipe."
                    </p>
                </div>

                <div class="table-wrap">
                    <table class="data">
                        <thead>
                            <tr>
                                <th>"Provider"</th>
                                <th>"HQ"</th>
                                <th>"Datacentres"</th>
                                <th>"From"</th>
                                <th>"Typical spec"</th>
                                <th>"Setup"</th>
                                <th>"Port 25"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {HOSTS
                                .iter()
                                .map(|h| {
                                    view! {
                                        <tr>
                                            <td class="name">
                                                <a href=h.url target="_blank" rel="noopener">{h.name}</a>
                                                <div class="tiny muted">{h.integration}</div>
                                            </td>
                                            <td>{h.country}</td>
                                            <td class="muted">{h.datacenters}</td>
                                            <td class="price">{h.price_from} " / mo"</td>
                                            <td class="muted">{h.specs}</td>
                                            <td><span class=tier_class(h.tier)>{h.setup_label()}</span></td>
                                            <td>
                                                <span class=port_class(h.port25)>{h.port25.label()}</span>
                                                <div class="tiny muted">{h.mail_note}</div>
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>

                <div class="stack-lg mt-40">
                    <div class="note-box info">
                        <span class="ico">"ⓘ"</span>
                        <span>
                            <b>"Mail deliverability, honestly."</b>
                            " Most cheap VPS ranges block outbound Port 25 at first, and shared IPs
                            often land in spam. If you want mail on day one, use an EU relay
                            (Scaleway TEM in Paris, Brevo, SMTP2GO) for outbound while incoming mail,
                            MailCraft and Stalwart stay on your server."
                        </span>
                    </div>

                    <div>
                        <h2 class="small">"Why we left Hostkey out"</h2>
                        <p class="muted small mt-8">
                            "Hostkey has a full API and would fit the one-click tier, but we are not
                            comfortable recommending a host with reported ties to Russia for a
                            privacy-first EU product. The other six cover every budget from ~€1.99."
                        </p>
                    </div>

                    <div>
                        <h2 class="small">"x86_64 only, for now"</h2>
                        <p class="muted small mt-8">
                            "The published artcraftCloud image is built for linux/amd64, so we
                            recommend x86 plans (Hetzner CX23, not the Arm CAX11). Multi-arch images
                            are a planned follow-up; once they land, Arm instances unlock."
                        </p>
                    </div>

                    <div class="cta-band">
                        <div>
                            <h2>"Not sure which one?"</h2>
                            <p>
                                "Hetzner is the easiest all-rounder. UpCloud is the cheapest API
                                option for Drive-only. Netcup is best if you want mail without a
                                30-day wait."
                            </p>
                        </div>
                        <div class="cta-actions">
                            <a class="button primary large" href="/deploy">"Open the wizard"</a>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
