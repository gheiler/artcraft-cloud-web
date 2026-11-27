//! Types shared by the marketing server and the WASM wizard.

use serde::{Deserialize, Serialize};

/// What the user wants to run on their server.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeployMode {
    /// Drive + the 10 Craft apps. Needs ~1 GB RAM / 20 GB disk.
    Drive,
    /// Drive + Craft apps + Stalwart mail/calendar. Needs ~2 GB RAM / 40 GB disk.
    DriveMail,
}

impl DeployMode {
    pub fn label(self) -> &'static str {
        match self {
            DeployMode::Drive => "Drive only",
            DeployMode::DriveMail => "Drive + Mail",
        }
    }

    pub fn ram(self) -> &'static str {
        match self {
            DeployMode::Drive => "1 GB",
            DeployMode::DriveMail => "2 GB",
        }
    }

    pub fn disk(self) -> &'static str {
        match self {
            DeployMode::Drive => "20 GB",
            DeployMode::DriveMail => "40 GB",
        }
    }

    pub fn price_note(self) -> &'static str {
        match self {
            DeployMode::Drive => "€3–5 / month",
            DeployMode::DriveMail => "€5–7 / month",
        }
    }
}

/// How much we can automate for a given provider.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// We call the provider's API with the user's token and boot the server.
    Api,
    /// The user orders in the provider's panel, then pastes our recipe.
    Guided,
}

impl Tier {
    pub fn label(self) -> &'static str {
        match self {
            Tier::Api => "1-click",
            Tier::Guided => "Guided",
        }
    }
}

/// Port 25 (outbound mail) policy, as documented by each provider.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Port25 {
    /// Open, or unblocked after a simple support ticket.
    Ticket,
    /// Blocked for a cooling-off period or until the account is paid up.
    Blocked,
    /// Frequently throttled on shared ranges; relay recommended.
    Restricted,
    /// A self-serve toggle in the provider's control panel.
    Toggle,
    /// Not stated in our notes — verify with the provider.
    Unknown,
}

impl Port25 {
    pub fn label(self) -> &'static str {
        match self {
            Port25::Ticket => "Ticket to open",
            Port25::Blocked => "Blocked at first",
            Port25::Restricted => "Restricted",
            Port25::Toggle => "Panel toggle",
            Port25::Unknown => "Check policy",
        }
    }

    pub fn bad(self) -> bool {
        matches!(self, Port25::Blocked | Port25::Restricted | Port25::Unknown)
    }
}

/// One European VPS provider we support.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Host {
    pub id: &'static str,
    pub name: &'static str,
    pub country: &'static str,
    pub datacenters: &'static str,
    pub price_from: &'static str,
    pub specs: &'static str,
    pub tier: Tier,
    pub port25: Port25,
    pub integration: &'static str,
    pub mail_note: &'static str,
    pub url: &'static str,
}

impl Host {
    /// True when `/api/deploy` can create the server with an API token today. The other API
    /// hosts get the same recipe to paste until their client lands.
    pub fn one_click(&self) -> bool {
        self.tier == Tier::Api && self.id == "hetzner"
    }

    /// "1-click", "Recipe · 1-click soon" or "Guided recipe".
    pub fn setup_label(&self) -> &'static str {
        if self.one_click() {
            "1-click"
        } else if self.tier == Tier::Api {
            "Recipe · 1-click soon"
        } else {
            "Guided recipe"
        }
    }
}

/// The v1 line-up. Hostkey is deliberately excluded (see plan.md).
pub const HOSTS: [Host; 7] = [
    Host {
        id: "hetzner",
        name: "Hetzner Cloud",
        country: "Germany",
        datacenters: "Nuremberg, Falkenstein · Helsinki",
        price_from: "€5.49",
        specs: "2 vCPU · 4 GB · 40 GB NVMe",
        tier: Tier::Api,
        port25: Port25::Blocked,
        integration: "One project token, cloud-init, PTR via API",
        mail_note: "Port 25 opens after 30 days and one paid invoice — pair with a relay or wait.",
        url: "https://www.hetzner.com/cloud",
    },
    Host {
        id: "upcloud",
        name: "UpCloud",
        country: "Finland",
        datacenters: "Helsinki · Frankfurt, Amsterdam, Warsaw, Madrid, Stockholm",
        price_from: "€3.00",
        specs: "1 vCPU · 1 GB · 25 GB",
        tier: Tier::Api,
        port25: Port25::Unknown,
        integration: "REST API, cloud-init, clean developer plans",
        mail_note: "Cheapest API option for Drive-only. Confirm outbound mail policy before enabling Mail.",
        url: "https://upcloud.com/pricing/",
    },
    Host {
        id: "scaleway",
        name: "Scaleway",
        country: "France",
        datacenters: "Paris · Amsterdam, Warsaw, Milan",
        price_from: "€4.99",
        specs: "Dedibox VPS · 2 vCPU · 2 GB · 40 GB",
        tier: Tier::Api,
        port25: Port25::Unknown,
        integration: "OpenAPI, cloud-init, security groups",
        mail_note: "Scaleway Transactional Email in Paris makes a tidy EU outbound relay.",
        url: "https://www.scaleway.com/en/pricing/",
    },
    Host {
        id: "contabo",
        name: "Contabo",
        country: "Germany",
        datacenters: "Munich, Nuremberg · plus EU regions",
        price_from: "€4.99",
        specs: "4 vCPU · 8 GB · 200 GB SSD",
        tier: Tier::Api,
        port25: Port25::Restricted,
        integration: "OAuth2 REST API — four credential fields",
        mail_note: "Shared IP ranges struggle with blocklists. Use an EU relay or expect deliverability work.",
        url: "https://contabo.com/en/vps/",
    },
    Host {
        id: "alphavps",
        name: "AlphaVPS",
        country: "Bulgaria",
        datacenters: "Sofia · plus DE and EU locations",
        price_from: "€2.54",
        specs: "Cloud VPS · 1 vCPU · 1 GB · 20 GB",
        tier: Tier::Guided,
        port25: Port25::Ticket,
        integration: "Order in the VirtFusion panel, paste our recipe",
        mail_note: "Port 25 blocked by default; a support ticket unlocks personal-volume mail.",
        url: "https://alphavps.com/clients/store/cheap-cloud-vps",
    },
    Host {
        id: "euronodes",
        name: "EuroNodes",
        country: "Cyprus (EU)",
        datacenters: "Lisbon · Prague",
        price_from: "€1.99",
        specs: "Hourly VPS · 1 vCPU · 1 GB · 20 GB",
        tier: Tier::Guided,
        port25: Port25::Ticket,
        integration: "Panel “Recipes” cloud-init box, automatic initial PTR",
        mail_note: "Port 25 closed by default; ticket unlocks personal low-volume mail.",
        url: "https://euronodes.com",
    },
    Host {
        id: "netcup",
        name: "Netcup",
        country: "Germany",
        datacenters: "Nuremberg · Vienna",
        price_from: "value tier",
        specs: "G11 / G12 VPS · generous RAM per euro",
        tier: Tier::Guided,
        port25: Port25::Toggle,
        integration: "cloud-init on G11/G12, mail toggle in the SCP panel",
        mail_note: "Mail block is a self-serve toggle — no 30-day wait.",
        url: "https://www.netcup.com/en/server/vps",
    },
];

/// One entry of the signed app registry (`registry/apps.json`).
#[derive(Deserialize, Debug, Clone)]
pub struct RegistryApp {
    pub id: String,
    pub title: String,
    pub publisher: String,
    pub blurb: String,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub recommended: String,
}

impl RegistryApp {
    /// `/assets/icons/wordcraft.png`
    pub fn icon_path(&self) -> String {
        format!("/assets/icons/{}.png", self.id)
    }

    pub fn extension_list(&self) -> String {
        if self.extensions.is_empty() {
            "no file types".to_string()
        } else {
            self.extensions
                .iter()
                .map(|e| format!(".{e}"))
                .collect::<Vec<_>>()
                .join("  ")
        }
    }
}

/// The registry file as shipped in the drive image.
#[derive(Deserialize, Debug, Clone)]
pub struct Registry {
    pub serial: u64,
    pub apps: Vec<RegistryApp>,
}

impl Registry {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// The wizard's form, posted to `/api/deploy` (one-click) or `/api/cloud-init` (recipe only).
#[derive(Serialize, Deserialize, Clone)]
pub struct DeployRequest {
    pub provider: String,
    pub mode: DeployMode,
    /// `cloud.example.eu`. Ignored when `skip_domain` is set.
    #[serde(default)]
    pub domain: String,
    /// Optional: Let's Encrypt contact for certificate expiry notices.
    #[serde(default)]
    pub email: String,
    /// Held in memory for the duration of the task only, never persisted or logged.
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub region: Option<String>,
    /// True when the user attaches a domain later; the server then uses `<ip>.sslip.io`.
    #[serde(default)]
    pub skip_domain: bool,
}

// Hand-written so the API token can never end up in a log line.
impl std::fmt::Debug for DeployRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeployRequest")
            .field("provider", &self.provider)
            .field("mode", &self.mode)
            .field("domain", &self.domain)
            .field("email", &self.email)
            .field(
                "api_token",
                &if self.api_token.is_empty() {
                    "<none>"
                } else {
                    "<redacted>"
                },
            )
            .field("region", &self.region)
            .field("skip_domain", &self.skip_domain)
            .finish()
    }
}

impl DeployRequest {
    /// The domain to configure, or `None` for the `<ip>.sslip.io` fallback.
    pub fn effective_domain(&self) -> Option<String> {
        let d = self
            .domain
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if self.skip_domain || d.is_empty() {
            None
        } else {
            Some(d)
        }
    }

    /// Checks the fields both the wizard and the server rely on. The error is user-facing.
    pub fn validate(&self) -> Result<(), String> {
        if host(&self.provider).is_none() {
            return Err("Pick a host from the list.".into());
        }
        if let Some(d) = self.effective_domain() {
            if !valid_domain(&d) {
                return Err(format!(
                    "“{d}” does not look like a domain name, e.g. cloud.example.eu."
                ));
            }
        } else if !self.skip_domain {
            return Err("Add your domain, or tick “set it up later”.".into());
        }
        let email = self.email.trim();
        if !email.is_empty() && !valid_email(email) {
            return Err("That email address does not look right.".into());
        }
        Ok(())
    }
}

/// A host name with at least two labels, letters, digits and inner hyphens only.
pub fn valid_domain(d: &str) -> bool {
    let labels: Vec<&str> = d.split('.').collect();
    d.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && !labels.last().unwrap().chars().all(|c| c.is_ascii_digit())
}

/// Deliberately simple: `local@domain`, no spaces or quotes, a valid domain.
pub fn valid_email(e: &str) -> bool {
    match e.rsplit_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && local.len() <= 64
                && local
                    .chars()
                    .all(|c| c.is_ascii_graphic() && !"\"'`<>()[],;:@\\".contains(c))
                && valid_domain(domain)
        }
        None => false,
    }
}

/// `203.0.113.10` -> `203-0-113-10.sslip.io`, the name the recipe uses when no domain is set.
pub fn sslip_domain(ipv4: &str) -> String {
    format!("{}.sslip.io", ipv4.replace('.', "-"))
}

/// Looks a host up by id.
pub fn host(id: &str) -> Option<&'static Host> {
    HOSTS.iter().find(|h| h.id == id)
}

/// Reply from `/api/deploy`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeployAccepted {
    pub job_id: String,
    pub status: String,
}

/// Reply from `/api/cloud-init`: the recipe to paste into the host's panel.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CloudInitRecipe {
    pub yaml: String,
    /// `https://cloud.example.eu`, or `None` when it will be `https://<ip>.sslip.io`.
    pub url: Option<String>,
}

/// Reply from `/api/deploy/:id`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JobStatus {
    pub job_id: String,
    /// queued, creating, booting, installing, done, failed, unknown
    pub status: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub ip: Option<String>,
    /// `https://...` once the server has an address.
    #[serde(default)]
    pub url: Option<String>,
    /// The generated admin login; only kept in the server's memory while the job lives.
    #[serde(default)]
    pub admin_user: Option<String>,
    #[serde(default)]
    pub admin_password: Option<String>,
}

/// DNS records the wizard hands the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecord {
    pub kind: &'static str,
    pub name: String,
    pub value: String,
    pub note: &'static str,
}

/// Builds the copy-paste DNS table for a domain.
pub fn dns_records(domain: &str, mail: bool, ip: &str) -> Vec<DnsRecord> {
    let mut rows = vec![DnsRecord {
        kind: "A",
        name: if domain.is_empty() {
            "<domain>".to_string()
        } else {
            domain.to_string()
        },
        value: if ip.is_empty() {
            "<server IPv4>".to_string()
        } else {
            ip.to_string()
        },
        note: "Points the drive at your server",
    }];

    if mail {
        let mail_host = if domain.is_empty() {
            "mail.<domain>".to_string()
        } else {
            format!("mail.{domain}")
        };
        rows.push(DnsRecord {
            kind: "A",
            name: mail_host.clone(),
            value: if ip.is_empty() {
                "<server IPv4>".to_string()
            } else {
                ip.to_string()
            },
            note: "Mail + JMAP host",
        });
        rows.push(DnsRecord {
            kind: "MX",
            name: "@".to_string(),
            value: format!("10 {mail_host}"),
            note: "Incoming mail",
        });
        rows.push(DnsRecord {
            kind: "TXT",
            name: "@".to_string(),
            value: format!("v=spf1 mx a:{mail_host} ~all"),
            note: "SPF — authorises your server to send",
        });
        rows.push(DnsRecord {
            kind: "TXT",
            name: "_dmarc".to_string(),
            value: if domain.is_empty() {
                "v=DMARC1; p=quarantine; rua=mailto:postmaster@<domain>".to_string()
            } else {
                format!("v=DMARC1; p=quarantine; rua=mailto:postmaster@{domain}")
            },
            note: "DMARC — start at quarantine",
        });
        rows.push(DnsRecord {
            kind: "TXT",
            name: "stalwart._domainkey".to_string(),
            value: "v=DKIM1; k=ed25519; p=<printed after first boot>".to_string(),
            note: "DKIM — the wizard prints the real key once Stalwart is up",
        });
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_only_has_a_single_record() {
        let rows = dns_records("cloud.example.eu", false, "203.0.113.10");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "A");
        assert_eq!(rows[0].value, "203.0.113.10");
    }

    #[test]
    fn mail_mode_adds_mx_spf_dmarc_dkim() {
        let rows = dns_records("example.eu", true, "203.0.113.10");
        let kinds: Vec<_> = rows.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&"MX"));
        assert_eq!(rows.iter().filter(|r| r.kind == "TXT").count(), 3);
        let mx = rows.iter().find(|r| r.kind == "MX").unwrap();
        assert_eq!(mx.value, "10 mail.example.eu");
    }

    #[test]
    fn missing_ip_is_placeheld_not_invented() {
        let rows = dns_records("example.eu", true, "");
        assert_eq!(rows[0].value, "<server IPv4>");
    }

    #[test]
    fn registry_lists_ten_apps() {
        let reg = Registry::parse(include_str!("../../site/static/assets/apps.json")).unwrap();
        assert_eq!(reg.apps.len(), 10);
        assert!(reg.apps.iter().any(|a| a.id == "wordcraft"));
        assert!(reg
            .apps
            .iter()
            .all(|a| a.icon_path().starts_with("/assets/icons/")));
    }

    #[test]
    fn domains_and_emails() {
        assert!(valid_domain("cloud.example.eu"));
        assert!(valid_domain("xn--bcher-kva.example"));
        assert!(!valid_domain("example"));
        assert!(!valid_domain("-a.example.eu"));
        assert!(!valid_domain("a..eu"));
        assert!(!valid_domain("cloud.example.eu\nruncmd"));
        assert!(!valid_domain("1.2.3.4"));
        assert!(valid_email("me@example.eu"));
        assert!(!valid_email("me@example"));
        assert!(!valid_email("m e@example.eu"));
        assert!(!valid_email("me\"@example.eu"));
        assert_eq!(sslip_domain("203.0.113.10"), "203-0-113-10.sslip.io");
    }

    #[test]
    fn request_validation_and_debug_hides_token() {
        let mut r = DeployRequest {
            provider: "hetzner".into(),
            mode: DeployMode::Drive,
            domain: "Cloud.Example.EU.".into(),
            email: String::new(),
            api_token: "super-secret".into(),
            region: None,
            skip_domain: false,
        };
        assert!(r.validate().is_ok());
        assert_eq!(r.effective_domain().as_deref(), Some("cloud.example.eu"));
        assert!(!format!("{r:?}").contains("super-secret"));
        r.domain.clear();
        assert!(r.validate().is_err());
        r.skip_domain = true;
        assert!(r.validate().is_ok());
        assert_eq!(r.effective_domain(), None);
        r.provider = "hostkey".into();
        assert!(r.validate().is_err());
    }

    #[test]
    fn catalog_covers_every_tier() {
        assert!(HOSTS.iter().any(|h| h.tier == Tier::Api));
        assert!(HOSTS.iter().any(|h| h.tier == Tier::Guided));
        assert!(!HOSTS.iter().any(|h| h.name.contains("Hostkey")));
    }
}
