Let's plan together a website to promote our artcraft cloud:
the website should be able to show the users how cloudcraft works, what are the purposes of cloudart, running your own cloud which is able to store your files, manage your own email server with your domain, and at the same time open files with the right artcraft tools.
Ideally we will also offer a way to pick your VPS host, and host your aircraft cloud as close as a “one-click” so we're gonna need a front and a backend for that.
We want to capture savvy and not-so-savvy tech people who want to run their own office / drive personal cloud on a VPS they trust for as cheap as possible.
We will first focus on European market, starting with cheapest full European hosts https://contabo.com/en/vps/, https://alphavps.com/clients/store/cheap-cloud-vps, https://euronodes.com, https://hostkey.com (I accept more suggestions, but we can start with the ones who would let us do stuff through api key, or any other integration type).
For now there are no fees, we just give you instructions on how to configure it, and for less savvy people we try to do as much as we can for them.
There should be just a buy me a beer / coffee button somewhere.
Stack will be also RUST on the frontend and backend with sqlite (I think it should be enough for now, correct me if you think not)
Would you also add the source code here? Or better in another repo.

I inspected the existing artcraftCloud workspace first so the website plan fits what you have already built (single Axum + SQLite binary at ~8–12 MB idle RSS, Leptos Wasm drive shell, SnowUI purple branding in style.css, the 10 signed apps in apps.json, and optional Stalwart mail in docs/mail.md).

Here is a concrete plan covering product flow, EU VPS providers, the Rust + SQLite stack, and repo layout.

1. Website Story & User Flows (Savvy + Non-Savvy)

Your biggest advantage is simplicity: instead of renting an opaque SaaS suite, users get their own folder-backed drive, custom-domain mail/calendar, and 10 native Rust/Wasm apps (WordCraft, GridCraft, DeckCraft, PdfCraft, CADCraft, FilmCraft, FigCraft, LightCraft, CalendarCraft, MailCraft) running on a cheap EU VPS they control.

A crisp structure that converts both audiences:


  
Hero & Live Architecture Visual: Clear split — "Your files, your domain email, your EU server — opened straight in native-speed Craft apps." Include an interactive toggle showing Drive-only (~€3–5/mo, 1 GB RAM / 20+ GB disk) vs Drive + Own Mail Server (~€5–7/mo, 2–4 GB RAM / 40+ GB disk), because docs/mail.md notes Stalwart adds ~55 MB+ RAM (and needs cache limits on small hosts) while the 10 apps unpack to ~600 MB (+ ~200–400 MB cache per docs/build.md).

  
How It Works (3 visual steps):
  
  

  
Pick a European host (cost/privacy/mail-readiness comparison).

  
Launch via 1-Click API or Guided Recipe (provisions Docker + Caddy auto-HTTPS + artcraft-cloud + optional stalwart). Auto-HTTPS is essential for non-savvy users because ArtCraft apps require a secure context (https://) for WebGPU and clipboard access.

  
Point your domain & verify DNS (copy-paste table + a live DNS checker in your Rust backend that turns green when A, MX, SPF, DMARC, and DKIM propagate).

  

  
Interactive App Showcase: Auto-synced from apps.json + screenshots from docs/screenshots/, showing file types (.docx, .xlsx, .pptx, .pdf, .dxf, .fig, .dng, .ics) and zero vendor lock-in (plain files on disk + WebDAV /dav/ + standard IMAP/SMTP).

  
Support / Tip CTA ("Buy me a beer / coffee"): Place a subtle pill in the navbar/footer, and a warmer "Your cloud is up! No fees — if this saved you a weekend, buy me a coffee/beer" card right on the post-deploy success screen (consider Liberapay or Ko-fi alongside Buy Me a Coffee — European open-source users heavily trust Liberapay).

  
2. European VPS Hosts & "One-Click" Feasibility

I checked the APIs, cloud-init support, pricing, and Port 25 (outbound email) rules of your 4 hosts plus other cheap full-EU candidates. The critical constraint for non-savvy users is that almost every cheap VPS blocks Port 25 initially and some lack a self-serve "create & bill VM via API key" endpoint.

Your 4 initial hosts
Host	HQ / EU DCs	Cheapest ~tier (excl. VAT)	API + cloud-init Reality	Mail / Port 25 Gotcha	Best Integration Mode
Contabo	Germany (Munich, Nuremberg + EU)	~€5/mo	REST API (api.contabo.com/v1, OAuth2) + userData (cloud-init). Note: API uses 4 OAuth credentials and starts 1/12/24-mo contracts; PTR usually set in Customer Control Panel, not API.	Port 25 frequently restricted/monitored; shared IP ranges often struggle with blacklists.	API 1-Click (advanced token form) + SMTP relay recommended for mail
Hostkey	Netherlands (HOSTKEY B.V., Amsterdam; NL, FI, DE, IS)	Budget instant VPS	Full invapi.hostkey.com API (eq/order_instance), cloud-init, and PTR/DNS via API (requires pre-funded account/billing).	Ports 25/465/587 blocked by default until domain A-record + support ticket. (Note: worth flagging their historical Russia ties transparently if users pick hosts for strict EU provenance.)	API 1-Click
AlphaVPS	Bulgaria (Sofia Telepoint + DE/EU)	~€2.54–€3.28/mo Cloud; ~€1.67/mo Storage	Modern cloud lines use VirtFusion/Proxmox + cloud-init + VirtFusion API, but storefront uses WHMCS (end-users normally order in portal first rather than spinning up new VMs via a single personal API key).	Port 25 blocked by default (support ticket to unlock).	Guided Order + cloud-init / 1-line SSH bootstrap
EuroNodes	EU / Cyprus (Lisbon, Prague; own AS199053)	~€1.99/mo (hourly!)	Supports cloud-init "Recipes" in client panel + auto initial PTR. However api.euronodes.com is reseller-oriented without public OpenAPI/self-serve docs yet.	Port 25 closed by default; ticket unlocks personal/low-volume mail.	Guided Order + paste generated Recipe
Strong full-EU additions for true non-savvy "One-Click"

If you want a true single-API-token, 90-second deploy for less savvy users, add these first:


  
Hetzner Cloud (Germany / Finland): Best developer & user UX in Europe. One project API token creates the server (CX23 ~€5.49/mo or Arm CAX11 ~€5.99/mo, 2 vCPU / 4 GB RAM / 40 GB NVMe, 20 TB traffic), passes cloud-init, configures firewall, and sets PTR via API. Mail note: Hetzner blocks Port 25 for the first 30 days + 1 paid invoice — easily solved by offering an optional EU SMTP relay (below) on day 1.

  
Scaleway (France / Netherlands / Poland / Italy): OpenAPI REST API + cloud-init, Dedibox VPS from ~€4.99/mo (and ultra-cheap Stardust instances), plus Scaleway Transactional Email (TEM) in Paris as an optional EU outbound mail relay!

  
UpCloud (Finland + DE, NL, PL, ES, SE, DK): Clean REST API + cloud-init, Developer plans from €3.00/mo (1 GB RAM, Drive-only) and higher tiers for Mail.

  
Netcup (Germany / Austria): Very cheap hardware + cloud-init on G11/G12 + instant mail-block toggle in their panel (no waiting 30 days), though their SOAP/SCP API is clunkier so it fits the Guided/Recipe tier best.

  
Recommended 2-Tier Deploy Architecture (so ALL hosts work)

  
Mode A — True 1-Click (Hetzner, Scaleway, UpCloud, Hostkey, Contabo): User creates an account at the host, pastes their API token into your site's Deploy Wizard, picks Drive-only or Drive + Mail, enters their domain, and clicks Deploy. Your Rust backend creates the VM with a generated cloud-init script (Docker + Caddy TLS + compose.yaml), streams live install progress over SSE, and shows the initial admin credentials + DNS records.

  
Mode B — Guided "Copy-Paste Recipe" (AlphaVPS, EuroNodes, Netcup, or privacy-conscious users who refuse to share API keys): User orders the recommended €2–€5 plan on the host's page (with screenshots showing which box to click), then pastes our generated cloud-init recipe (or a single curl ... | sh setup command on fresh Ubuntu/Debian) and clicks Verify My Server on your website to run live HTTPS + DNS checks.

  

And for Mail Deliverability: make the wizard honest and helpful! Warn non-savvy users upfront that home-grown Port 25 on new cheap VPSes often gets throttled or flagged by Microsoft/Gmail, and offer a switch in the setup: "Direct SMTP (Port 25)" vs "Use an EU Outbound Relay (e.g. Scaleway TEM / Brevo EU / SMTP2GO)" while keeping all incoming mail + MailCraft + Stalwart directly on their own VPS.

3. Stack Critique: Rust (Frontend & Backend) + SQLite

  

  

Backend (Axum + SQLite): Keep it — it's a great fit.


  

  
Matches artcraft-cloud directly (axum 0.8, rusqlite with WAL mode, reqwest with rustls-tls, tokio).

  
You can also add hickory-resolver in Rust for the live DNS/MX/SPF/DKIM propagation checker, and ed25519/ssh-key or rand to generate ephemeral SSH keys/admin passwords during provisioning.

  
Crucial security rule for SQLite: Never persist user VPS API keys, OAuth secrets, or SSH private keys to SQLite (or disk at all). Hold the API token strictly in memory for the ~60–120s provisioning task (zeroize/drop on completion or timeout), and store only non-sensitive deployment metadata (job_id, provider, status, created_at, anonymized aggregate stats) in SQLite. That keeps GDPR/breach risk near zero and avoids acting as a credential custodian.

  

  

  

  

Frontend (Rust / Leptos): One important correction for a promo site.


  

  
In artcraft-cloud, pure client-side Wasm (Leptos CSR via trunk) makes total sense because it's behind a login wall.

  
On a public marketing + wizard site, pure CSR Wasm hurts SEO, social previews (OpenGraph cards when shared on HN/Reddit/Mastodon), and mobile First Contentful Paint (downloading ~500 KB+ gzipped Wasm before rendering the hero).

  
Two 100%-Rust options that fix this:
  
  

  
Best balance (simplest build): Axum + maud or askama (compile-time Rust HTML templates) for all marketing, host comparisons, and docs pages (instant <15 KB HTML, 100/100 SEO) + mount a focused Leptos CSR Wasm island only on /deploy (the interactive VPS wizard & live DNS checker, using the exact same trunk pipeline you already have in crates/shell).

  
Full-stack Leptos (leptos_axum + cargo-leptos with islands): Everything in Leptos with SSR HTML + selective Wasm hydration only on interactive calculators/wizards. Slightly more build tooling complexity (cargo-leptos), great if you want one unified component model.

  

  

  

  
4. Should Source Code Live Here or in Another Repo?

Put it in a separate repository (e.g., artcraft-cloud-web or artcraft-cloud-site).

Why:


  
Clean separation of concerns & trust: Users auditing or building artcraft-cloud expect a lean, zero-telemetry self-hosted office server image (ghcr.io/gheiler/artcraft-cloud, ~100 MB, 4 crates). Mixing marketing pages, provider billing/API clients, and donation links into this repo bloats CI, releases, and Helm/Docker builds.

  
Independent release cadence: You will tweak copy, add VPS promos/pricing, or fix a Contabo/Hetzner API quirk much more often than you release artcraft-cloud binaries.

  
Credential surface isolation: The provisioner backend talks to third-party cloud APIs; keeping it out of the self-hosted binary guarantees zero accidental attack surface on users' instances.

  

What to keep in this repo (artcraft-cloud):


  
Add a clean deploy/cloud-init/ (or install.sh + Caddyfile template) in artcraft-cloud so both manual CLI users and the new website's 1-click wizard deploy the exact same canonical stack.

  
The website repo can pull registry/apps.json, icons, and screenshots directly (or mirror them at build time) so app versions stay in sync automatically.

  
5. Decisions to Align On Next

Before we write code or scaffold the new repo, let's nail down 4 quick choices:


  
Name & branding consistency: Notice the repo uses artcraftCloud (with SnowUI purple + beveled cloud mark). Do you want the site branded strictly as artcraftCloud (e.g., artcraftcloud.org / .eu), or are you considering renaming it?

  
First-launch VPS lineup: Shall we target Hetzner Cloud + Contabo + Hostkey for true API 1-Click, and AlphaVPS + EuroNodes for Guided Recipe on v1 (plus Scaleway/UpCloud if you want another €3/mo API option)?

  
Frontend flavor: Do you prefer Askama/Maud SSR pages + Leptos CSR Wasm on /deploy (simple, shares your trunk setup), or cargo-leptos SSR Islands?

  
Domain & HTTPS during 1-Click: Should the wizard require users to bring their own domain upfront (recommended if they want Mail), or also offer an optional IP / sslip.io / temporary domain mode just to test Drive + Craft apps in 2 minutes before buying a domain?

  
1. I already got artcraftcloud.com, so let's go with it. We should use the same logo as the app and I guess we should also keep the purple, but you can convince me of otherwise regarding color.
2. Let's go with all of them, plus's too. But let's skip Hostkey if they have ties to Russia
3. first one
4. If they pick mail with self domain, they should bring a domain or press the checkbox to skip the config for now and do manually later.

I checked logo-light.svg, logo-dark.svg, drive.png, Dockerfile, and ci.yml so the blueprint below aligns cleanly with your app and avoids deployment traps.

1. Color & Visual Identity (artcraftcloud.com)

Keep SnowUI Purple — used with restraint.

  Looking at drive.png, your brand already looks remarkably cohesive:


  
The 10 Crafting App icons (blue WordCraft, green GridCraft, orange DeckCraft, crimson PdfCraft, cyan CADCraft, emerald FigCraft, gold LightCraft, violet CalendarCraft, sky-blue MailCraft) carry plenty of color on their own.

  
A heavy all-purple marketing theme would fight those icons. Instead, mirror the exact SnowUI light/dark surfaces from style.css:
  
  

  
90% neutral canvas: #f9f9fa / #ffffff in light mode, #1c1c1c / #333333 in dark mode, 1px hairline borders (#0000001a), Inter typography.

  
Purple (#6e48b8 light / #b899eb dark) reserved strictly for the beveled cloud wordmark (artcraft in ink + Cloud in purple), primary CTAs, active wizard steps, focus rings, and a soft hero glow (#b899eb1f).

  
Status accents: green (#16a34a / #71dd8c) for Server Online / DNS Verified / Port 25 Open, and amber (#d97706 / #ffb55b) for SMTP relay recommended.

  

  

Visitors moving from artcraftcloud.com into their freshly deployed Drive will feel zero jarring transition.

2. Important Finding from Dockerfile & ci.yml

Your CI currently builds linux/amd64 only (the toolchain layer downloads x86_64 binaries for trunk, wasm-bindgen, wasm-opt, and cargo-chef, and docker/build-push-action does not set platforms: linux/amd64,linux/arm64).


  
In our website v1: Filter VPS recommendations to x86_64 plans only (e.g., Hetzner CX23 rather than Arm CAX11) so no user hits an exec format error.

  
Optional follow-up in artcraft-cloud: Adding multi-arch (linux/amd64,linux/arm64) later unlocks Arm instances too.

  
3. Sitemap & Content Architecture

All marketing/guide routes are Axum + Askama (or Maud) SSR HTML for instant load and SEO; /deploy mounts your Leptos CSR Wasm app via trunk:

Route	Renderer	Purpose
/	SSR HTML	Hero ("Your files. Your email. Your European server."), interactive mode preview (Drive-only vs Drive + Mail), 3-step visual flow, trust pillars (plain folders on disk, signed app registry, ~10 MB idle binary, EU-only hosts), screenshot showcase, "Buy me a beer/coffee" pill.
/apps	SSR HTML	Driven from apps.json — cards for all 10 Craft apps, supported extensions, WebDAV (/dav/), safe ETag saves, and MailCraft/CalendarCraft JMAP/CalDAV proxies.
/hosts	SSR HTML	Transparent EU VPS comparison table (country, jurisdiction, price, RAM/disk, API 1-click vs Guided Recipe, Port 25 policy) — 7 providers, Hostkey excluded.
/deploy	Leptos Wasm	The interactive Provisioner + Cloud-Init Generator + DNS/Health Verifier (detailed below).
/guide	SSR HTML	Savvy quick-reference (docker compose, Caddy reverse proxy, DNS records, Port 25 / EU SMTP relays, backups).
4. EU Host Lineup (v1 — Hostkey Excluded)

Tier A — API "One-Click" (user pastes API key/credentials -> backend provisions & boots server):


  
Hetzner Cloud (Germany / Finland) — Primary non-savvy recommendation. Single project API token, x86 CX23 (~€5.49/mo, 2 vCPU / 4 GB / 40 GB NVMe), native user_data, API PTR. Mail note: Port 25 blocked first 30 days + 1 paid invoice (pair with optional EU relay or wait 30 days).

  
UpCloud (Finland + DE, NL, PL, ES, SE, DK) — Cheapest Drive-only API option (€3.00/mo Developer 1 GB for Drive-only; €5+/mo tier for Drive + Mail), REST API + cloud-init.

  
Scaleway (France, Netherlands, Poland, Italy) — OpenAPI + cloud-init, Dedibox VPS (~€4.99/mo), Security Group port controls, plus Scaleway TEM (EU-hosted Paris transactional relay) as an easy outbound mail bridge.

  
Contabo (Germany — Munich/Nuremberg + EU) — OAuth2 REST API (api.contabo.com/v1) + userData, huge RAM/disk from ~€4.99/mo. Wizard warns clearly: 4-field OAuth setup, monthly/term billing commitment, PTR set in Contabo panel, and SMTP relay strongly advised due to shared IP reputation/Port 25 limits.

  

Tier B — Guided Order + Generated Recipe / One-Liner (for budget/panel-first EU hosts):
5. AlphaVPS (Bulgaria — Sofia + DE/EU) — Ultra-budget €2.54–€3.28/mo Cloud VPS (and €1.67/mo Storage VPS). Step-by-step screenshots to order Ubuntu/Debian on VirtFusion + copy-paste cloud-init or single curl | sh bootstrap + Port 25 ticket template.
6. EuroNodes (Cyprus EU, DCs in Lisbon & Prague) — From €1.99/mo hourly! Tailored for their native panel "Recipes" cloud-init box + auto initial PTR + Port 25 ticket template.
7. Netcup (Germany / Austria) — High-value G11/G12 VPS with cloud-init + instant Mail-block toggle in SCP (no 30-day wait!).

(Every Tier B host still gets the exact same automated server install, Caddy HTTPS, DNS table, and live health/DNS check once booted.)

5. The /deploy Wizard Flow (Addressing Requirement #4)

  
Step 1 — What do you want to run?
  
  

  
Drive + 10 Craft Apps (recommends ≥1 GB RAM, ≥20 GB SSD/NVMe)

  
Drive + 10 Craft Apps + Own Mail & Calendar (Stalwart) (recommends ≥2 GB RAM, ≥40 GB SSD/NVMe)

  

  
Step 2 — Pick your EU Host & Region
  
  

  
Filterable by budget, country (DE, FI, BG, PT, CZ, FR, NL, AT, PL), and 1-Click API vs Guided Setup.

  

  
Step 3 — Domain & Mail Setup
  
  

  
Domain field (e.g. cloud.example.eu / example.eu + mail host mail.example.eu).

  
Checkbox: "Skip domain / email configuration for now (I'll configure it manually later)":
  
  

  
Unchecked (Normal): Generates Caddy auto-HTTPS for their domain, sets ARTCRAFT_PUBLIC_URL=https://<domain>, and if Mail is enabled, outputs exact copy-paste A, AAAA, MX, SPF, DMARC records + optional EU SMTP relay settings + post-boot Stalwart DKIM guide.

  
Checked (Skip for now): Deploys artcraft-cloud (and optionally the Stalwart container stopped or on localhost profile) using http://<ip>:8080 or Caddy tls internal with a clear reminder banner explaining that WebGPU/clipboard in Craft apps and custom-domain mail require attaching a domain + HTTPS later (showing the 3-line Caddyfile edit).

  

  

  
Step 4 — Provision & Live Verify
  
  

  
Tier A (API): Ephemeral in-memory API credential (never written to SQLite), creates server + firewall (ports 22, 80, 443, plus 25/465/993 if Mail), injects cloud-init, streams setup progress via SSE, prints the one-time generated admin password + SSH private key download, and verifies /healthz + DNS propagation.

  
Tier B (Guided): Shows exact host order link + copy-paste cloud-init YAML (or single bootstrap command), then user pastes their new server IP/domain and clicks Verify Server & DNS.

  

  
Step 5 — "You're live!" + Buy Me a Beer/Coffee
  
  

  
Direct link to their new artcraftCloud, checklist for first login (/mail if mail enabled), and a friendly "Buy me a coffee / beer" button.

  

  
6. Technical Architecture of the New Repo (artcraft-cloud-web)

Separate Git repo (e.g., artcraft-cloud-web), MIT OR Apache-2.0, deployed as one Rust binary on artcraftcloud.com:

text
artcraft-cloud-web/
├── Cargo.toml                 # workspace: server, types, wizard (wasm)
├── crates/
│   ├── types/                 # shared HostCatalog, DeployRequest, DnsCheckResult, AppCatalog
│   ├── wizard/                # Leptos CSR (trunk) mounted ONLY at /deploy
│   └── server/
│       ├── templates/         # Askama HTML templates (/, /apps, /hosts, /guide)
│       ├── static/            # SnowUI CSS, beveled cloud SVGs, app icons, screenshots
│       └── src/
│           ├── providers/     # hetzner.rs, upcloud.rs, scaleway.rs, contabo.rs
│           ├── cloud_init.rs  # generates cloud-init YAML + Caddyfile + compose.yaml
│           ├── dns_check.rs   # hickory-resolver live A/AAAA/MX/SPF/DMARC/PTR verifier
│           ├── jobs.rs        # in-memory ephemeral token handling + SSE progress stream
│           └── db.rs          # SQLite (WAL): host pricing/metadata, anonymized job status/counts

And in artcraft-cloud (this repo), we add a single clean deploy/cloud-init/artcraft-cloud.yaml (or scripts/install-vps.sh) so manual CLI/Docker users and artcraftcloud.com share one tested production recipe.

Ready to Build — Where Shall We Start?

If this plan looks solid, tell me which piece you'd like to tackle first:


  
Create the canonical VPS install / cloud-init + Caddy HTTPS setup right here in artcraft-cloud (tested locally/dry-run so the 1-click script is rock-solid before wrapping a UI around it).

  
Scaffold the new artcraft-cloud-web Rust workspace (Axum + Askama + SQLite + Leptos /deploy wizard + SnowUI styling) in a sibling folder like ../artcraft-cloud-web (or inspect first which folder path you want).

  
Wire the first API provider (Hetzner + cloud-init generator + live DNS verifier) end-to-end as the reference implementation for UpCloud/Scaleway/Contabo.

  
Start  with number 2, folder with git already exists