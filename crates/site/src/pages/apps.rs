use leptos::prelude::*;

use crate::catalog::registry;

#[component]
pub fn AppsPage() -> impl IntoView {
    let reg = registry();

    view! {
        <section class="section">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"The Craft suite"</span>
                    <h2>"Ten apps that open your real files"</h2>
                    <p>
                        "Every app runs in your browser on WebAssembly, reads and writes standard
                        formats, and saves straight back to your drive with safe ETag checks. No
                        import, no conversion, no lock-in."
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
                                    <div class="row mt-8">
                                        <span class="badge">{app.publisher.clone()}</span>
                                        <span class="badge primary">{format!("v{}", app.recommended)}</span>
                                    </div>
                                    <div class="exts" title=app.extension_list()>
                                        {app.extension_list()}
                                    </div>
                                </article>
                            }
                        })
                        .collect_view()}
                </div>

                <div class="stack-lg mt-40">
                    <div class="pillars">
                        <div class="pillar">
                            <h3>"Standard protocols"</h3>
                            <p>
                                "WebDAV at " <code>"/dav/"</code> ", IMAP and SMTP for mail, JMAP for
                                MailCraft, CalDAV for calendars. Any client works."
                            </p>
                        </div>
                        <div class="pillar">
                            <h3>"Signed and verified"</h3>
                            <p>
                                "The registry ships with OpenPGP and Ed25519 signatures. The drive
                                checks every app before install."
                            </p>
                        </div>
                        <div class="pillar">
                            <h3>"Files stay files"</h3>
                            <p>
                                "Documents live in ordinary folders. Copy them out with rsync at any
                                time — nothing is trapped."
                            </p>
                        </div>
                        <div class="pillar">
                            <h3>"Runs offline-ish"</h3>
                            <p>
                                "Apps are local WebAssembly, so editing is native-speed and does not
                                round-trip to a third party."
                            </p>
                        </div>
                    </div>

                    <div class="table-wrap">
                        <table class="data">
                            <thead>
                                <tr>
                                    <th>"App"</th>
                                    <th>"Purpose"</th>
                                    <th>"Opens"</th>
                                    <th>"Publisher"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {reg
                                    .apps
                                    .iter()
                                    .map(|app| {
                                        view! {
                                            <tr>
                                                <td class="name">
                                                    <span class="row">
                                                        <img
                                                            class="app-icon"
                                                            style="width:20px;height:20px;border-radius:6px"
                                                            src=app.icon_path()
                                                            alt=""
                                                        />
                                                        {app.title.clone()}
                                                    </span>
                                                </td>
                                                <td class="muted">{app.blurb.clone()}</td>
                                                <td><code>{app.extension_list()}</code></td>
                                                <td class="muted">{app.publisher.clone()}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    </div>

                    <div class="note-box info">
                        <span class="ico">"ⓘ"</span>
                        <span>
                            "Registry serial " <b>{reg.serial.to_string()}</b> " · app versions are
                            pinned per release and mirrored into the drive image."
                        </span>
                    </div>

                    <div class="cta-band">
                        <div>
                            <h2>"Open them on your own drive"</h2>
                            <p>"Deploy the stack and the ten apps arrive signed and ready."</p>
                        </div>
                        <div class="cta-actions">
                            <a class="button primary large" href="/deploy">"Deploy my cloud"</a>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
