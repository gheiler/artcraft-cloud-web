use leptos::prelude::*;

#[component]
pub fn GuidePage() -> impl IntoView {
    view! {
        <section class="section">
            <div class="container">
                <div class="section-head">
                    <span class="kicker">"Guide"</span>
                    <h2>"The savvy quick reference"</h2>
                    <p>
                        "Everything the wizard does, written out. Use this if you prefer a terminal,
                        or want to understand the recipe before running it."
                    </p>
                </div>

                <div class="doc">
                    <nav class="doc-nav">
                        <a href="#prereqs">"1. What you need"</a>
                        <a href="#recipe">"2. The cloud-init recipe"</a>
                        <a href="#compose">"3. By hand: Compose + Caddy"</a>
                        <a href="#dns">"4. DNS records"</a>
                        <a href="#mail">"5. Mail & Port 25"</a>
                        <a href="#backups">"6. Backups"</a>
                    </nav>

                    <div class="prose">
                        <h2 id="prereqs">"1. What you need"</h2>
                        <ul>
                            <li>"A fresh Ubuntu 24.04 or Debian 12 x86_64 VPS with at least 1 GB RAM (2 GB for mail)."</li>
                            <li>"A domain, with access to its DNS records."</li>
                            <li>"Ports 22, 80 and 443 open. Add 25, 465, 587 and 993 for mail."</li>
                            <li>"About 20 GB of disk for Drive-only, 40 GB with mail."</li>
                        </ul>

                        <h2 id="recipe">"2. The cloud-init recipe"</h2>
                        <p>
                            "The wizard fills in "
                            <a href="https://github.com/gheiler/artcraft-cloud/blob/main/deploy/cloud-init/artcraft-cloud.yaml" target="_blank" rel="noopener">
                                <code>"deploy/cloud-init/artcraft-cloud.yaml"</code>
                            </a>
                            " and nothing else. Edit the four values at the top yourself and paste it into your host's
                            “user data” box when you create the server:"
                        </p>
                        <pre><code>{r#"ARTCRAFT_DOMAIN=cloud.example.eu   # empty: https://<ip>.sslip.io
ARTCRAFT_ACME_EMAIL=you@example.eu  # optional
ARTCRAFT_ADMIN_PASSWORD=            # empty: generated
ARTCRAFT_MAIL=0                     # 1 also starts Stalwart
"#}</code></pre>
                        <p>
                            "It installs Docker, opens 22/80/443 in ufw, runs artcraftCloud behind Caddy with
                            automatic HTTPS, and writes the admin login to "
                            <code>"/root/artcraft-admin-password.txt"</code>
                            ". Watch it with " <code>"tail -f /var/log/artcraft-install.log"</code>
                            ". Change a value later in " <code>"/opt/artcraft/.env"</code> " and run "
                            <code>"bash /opt/artcraft/install.sh"</code> " again."
                        </p>

                        <h2 id="compose">"3. By hand: Docker Compose + Caddy"</h2>
                        <p>
                            "Prefer to do it yourself? Use artcraftCloud's "
                            <a href="https://github.com/gheiler/artcraft-cloud/blob/main/compose.yaml" target="_blank" rel="noopener"><code>"compose.yaml"</code></a>
                            " and put Caddy in front:"
                        </p>
                        <pre><code>{r#"git clone https://github.com/gheiler/artcraft-cloud && cd artcraft-cloud
    ARTCRAFT_PUBLIC_URL=https://cloud.example.eu \
ARTCRAFT_ADMIN_PASSWORD='choose-a-long-one' docker compose up -d

# /etc/caddy/Caddyfile
cloud.example.eu {
    reverse_proxy localhost:8080
}
"#}</code></pre>
                        <p>
                            "HTTPS matters beyond looks: WebGPU and clipboard access in the Craft apps
                            require a secure context."
                        </p>

                        <h2 id="dns">"4. DNS records"</h2>
                        <p>"Point the drive and, if you use mail, the mail host at your server:"</p>
                        <div class="table-wrap">
                            <table class="dns-table">
                                <thead>
                                    <tr><th>"Type"</th><th>"Name"</th><th>"Value"</th></tr>
                                </thead>
                                <tbody>
                                    <tr><td><b>"A"</b></td><td><code>"@"</code></td><td><code>"203.0.113.10"</code></td></tr>
                                    <tr><td><b>"A"</b></td><td><code>"mail"</code></td><td><code>"203.0.113.10"</code></td></tr>
                                    <tr><td><b>"MX"</b></td><td><code>"@"</code></td><td><code>"10 mail.example.eu"</code></td></tr>
                                    <tr><td><b>"TXT"</b></td><td><code>"@"</code></td><td><code>"v=spf1 mx a:mail.example.eu ~all"</code></td></tr>
                                    <tr><td><b>"TXT"</b></td><td><code>"_dmarc"</code></td><td><code>"v=DMARC1; p=quarantine; rua=mailto:postmaster@example.eu"</code></td></tr>
                                    <tr><td><b>"TXT"</b></td><td><code>"stalwart._domainkey"</code></td><td><code>"v=DKIM1; ... (printed on first boot)"</code></td></tr>
                                </tbody>
                            </table>
                        </div>

                        <h2 id="mail">"5. Mail and Port 25"</h2>
                        <p>
                            "Incoming mail works as soon as MX and the firewall are right. Outbound is
                            the hard part: many providers block Port 25 for new accounts, and shared
                            IPs are often flagged."
                        </p>
                        <ul>
                            <li>"Hetzner: opens after 30 days and one paid invoice."</li>
                            <li>"AlphaVPS, EuroNodes: open a ticket after the domain resolves."</li>
                            <li>"Netcup: flip the mail toggle in the SCP panel."</li>
                            <li>"Contabo: expect shared-IP reputation work; a relay is safer."</li>
                        </ul>
                        <p>
                            "If you want mail today, keep Stalwart for inbound and send outbound
                            through an EU relay:"
                        </p>
                        <pre><code>{r#"# Stalwart outbound relay (example)
[queue.outbound]
hostname = "smtp.scaleway.example"
port = 587
tls = "starttls"
"#}</code></pre>

                        <h2 id="backups">"6. Backups"</h2>
                        <p>
                            "Everything lives under " <code>"/opt/artcraft/data"</code> ": files, the
                            SQLite database, and mail. Back it up like any directory:"
                        </p>
                        <pre><code>{r#"rsync -a --delete /opt/artcraft/data/ backup-host:/srv/artcraft/
# or, on the same box:
tar czf /var/backups/artcraft-$(date +%F).tgz -C /opt/artcraft data
"#}</code></pre>
                        <p>
                            "Because the drive stores plain folders, you can also pull individual
                            files over WebDAV or SFTP without a special tool."
                        </p>

                        <div class="cta-band mt-40">
                            <div>
                                <h2>"Prefer the wizard?"</h2>
                                <p>"It generates this exact stack, with your domain and mode, and runs the checks for you."</p>
                            </div>
                            <div class="cta-actions">
                                <a class="button primary large" href="/deploy">"Open the wizard"</a>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    }
}
