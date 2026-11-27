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
                        <a href="#compose">"2. Docker Compose"</a>
                        <a href="#caddy">"3. Caddy + HTTPS"</a>
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

                        <h2 id="compose">"2. Docker Compose"</h2>
                        <p>"Install Docker, then write this to " <code>"/opt/artcraft/compose.yaml"</code> ":"</p>
                        <pre><code>{r#"services:
  cloud:
    image: ghcr.io/gheiler/artcraft-cloud:latest
    restart: always
    ports:
      - "8080:8080"
    volumes:
      - /opt/artcraft/data:/data
    environment:
      - ARTCRAFT_PUBLIC_URL=https://cloud.example.eu
"#}</code></pre>
                        <p>"Start it with " <code>"docker compose -f /opt/artcraft/compose.yaml up -d"</code> "."</p>

                        <h2 id="caddy">"3. Caddy + automatic HTTPS"</h2>
                        <p>
                            "Caddy gets you a certificate without thinking. Point it at the container:"
                        </p>
                        <pre><code>{r#"cloud.example.eu {
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
