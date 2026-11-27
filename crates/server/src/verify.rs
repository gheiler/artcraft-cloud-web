//! `POST /api/verify`: checks a new server's DNS records and `https://<host>/healthz`.
//!
//! Safety rules, because this endpoint makes the server do network requests for strangers:
//! - HTTP goes **only** to the IP address the user typed (or the one-click job reported), pinned
//!   with `reqwest::ClientBuilder::resolve`, so a domain that resolves to `127.0.0.1` or an
//!   internal range can't be used to reach anything else (no DNS rebinding either).
//! - That IP must be a public unicast address: no private, loopback, link-local, CGNAT,
//!   multicast, benchmarking or documentation ranges (documentation ranges only in demo mode).
//! - Port 443 only, no redirects, no proxy, short timeouts, at most 4 KB of body read.
//! - DNS questions go to public resolvers (Cloudflare) with the cache off, so a fixed record shows
//!   up on the next check; names are validated domain names.
//! - Per-client and per-target rate limits plus a global concurrency cap (see [`Limits`]).

use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use types::{sslip_domain, valid_domain, Check, CheckState, VerifyReport, VerifyRequest};

/// The answer to one DNS question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lookup<T> {
    Found(Vec<T>),
    /// NXDOMAIN or no records of that type.
    Missing,
    /// Timeout, SERVFAIL, network trouble.
    Error(String),
}

pub trait Dns: Send + Sync {
    fn a(&self, name: &str) -> impl Future<Output = Lookup<Ipv4Addr>> + Send;
    fn aaaa(&self, name: &str) -> impl Future<Output = Lookup<Ipv6Addr>> + Send;
    /// `(preference, exchange)` with the exchange lower-cased and without the trailing dot.
    fn mx(&self, name: &str) -> impl Future<Output = Lookup<(u16, String)>> + Send;
    /// One string per TXT record (its character strings joined).
    fn txt(&self, name: &str) -> impl Future<Output = Lookup<String>> + Send;
}

/// What `GET https://<host>/healthz` (sent to the pinned IP) gave back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    /// 200 with artcraftCloud's `{"status":"ok",...}`.
    Ok {
        version: Option<String>,
    },
    /// 200 but not artcraftCloud's answer.
    Foreign,
    Status(u16),
    /// Connected, but the TLS handshake or certificate failed.
    Tls(String),
    /// Nothing listens on 443 (or a firewall rejects it).
    Refused,
    Timeout,
    Other(String),
}

pub trait Probe: Send + Sync {
    fn healthz(&self, host: &str, ip: IpAddr) -> impl Future<Output = Health> + Send;
}

/// A validated [`VerifyRequest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    /// False in sslip.io mode.
    pub custom: bool,
    pub ipv4: Ipv4Addr,
    pub ipv6: Option<Ipv6Addr>,
    pub mail: bool,
    pub dkim_selector: Option<String>,
}

impl Target {
    /// `allow_documentation` lets 192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24 and 2001:db8::/32
    /// through, for demo mode only.
    pub fn parse(req: &VerifyRequest, allow_documentation: bool) -> Result<Target, String> {
        let ipv4: Ipv4Addr = req.ipv4.trim().parse().map_err(|_| {
            "Enter your server's IPv4 address, like 203.0.113.10 (your host shows it in its panel)."
                .to_string()
        })?;
        if !public_ipv4(ipv4, allow_documentation) {
            return Err(format!(
                "{ipv4} is not a public address. Use the server's public IPv4 address from your host's panel."
            ));
        }
        let ipv6 = match req.ipv6.trim() {
            "" => None,
            s => {
                let ip: Ipv6Addr = s
                    .parse()
                    .map_err(|_| format!("“{s}” is not an IPv6 address."))?;
                if !public_ipv6(ip, allow_documentation) {
                    return Err(format!("{ip} is not a public IPv6 address."));
                }
                Some(ip)
            }
        };
        let domain = req.domain.trim().trim_end_matches('.').to_ascii_lowercase();
        let (host, custom) = if req.skip_domain || domain.is_empty() {
            (sslip_domain(&ipv4.to_string()), false)
        } else if valid_domain(&domain) {
            (domain, true)
        } else {
            return Err(format!(
                "“{domain}” does not look like a domain name, e.g. cloud.example.eu."
            ));
        };
        let selector = req
            .dkim_selector
            .trim()
            .trim_end_matches("._domainkey")
            .to_ascii_lowercase();
        let dkim_selector = if selector.is_empty() {
            None
        } else if selector.len() <= 63
            && selector
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            Some(selector)
        } else {
            return Err("The DKIM selector is a short name like 202611e.".into());
        };
        Ok(Target {
            host,
            custom,
            ipv4,
            ipv6,
            mail: req.mail,
            dkim_selector,
        })
    }
}

/// Public unicast IPv4 only.
pub fn public_ipv4(ip: Ipv4Addr, allow_documentation: bool) -> bool {
    let o = ip.octets();
    let reserved = ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_unspecified()
        || o[0] == 0
        || (o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10 CGNAT
        || (o[0] == 192 && o[1] == 0 && o[2] == 0) // 192.0.0.0/24 IETF
        || (o[0] == 198 && (o[1] & 0xfe) == 18) // 198.18.0.0/15 benchmarking
        || o[0] >= 240; // reserved + broadcast
    !reserved && (allow_documentation || !ip.is_documentation())
}

/// Global unicast IPv6 (2000::/3) only, minus Teredo, 6to4 and (unless allowed) documentation.
pub fn public_ipv6(ip: Ipv6Addr, allow_documentation: bool) -> bool {
    let s = ip.segments();
    let global = (s[0] & 0xe000) == 0x2000;
    let teredo = s[0] == 0x2001 && s[1] == 0;
    let six_to_four = s[0] == 0x2002;
    let documentation = s[0] == 0x2001 && s[1] == 0x0db8;
    global && !teredo && !six_to_four && (allow_documentation || !documentation)
}

fn check(
    id: &str,
    label: String,
    state: CheckState,
    found: impl Into<String>,
    expected: Option<String>,
    fix: Option<String>,
) -> Check {
    Check {
        id: id.into(),
        label,
        state,
        found: found.into(),
        expected,
        fix,
    }
}

const DNS_WAIT: &str = "If you just changed it, give DNS a few minutes (up to the record's TTL, often 1 hour) and check again.";

fn join<T: ToString>(v: &[T]) -> String {
    v.iter().map(T::to_string).collect::<Vec<_>>().join(", ")
}

/// The A record of `name` must be exactly `ip`.
async fn check_a<D: Dns>(dns: &D, id: &str, name: &str, ip: Ipv4Addr, custom: bool) -> Check {
    let label = format!("A record for {name}");
    let expected = Some(ip.to_string());
    match dns.a(name).await {
        Lookup::Found(ips) if ips == [ip] => {
            check(id, label, CheckState::Ok, ip.to_string(), expected, None)
        }
        Lookup::Found(ips) if ips.contains(&ip) => {
            let others: Vec<_> = ips.iter().filter(|x| **x != ip).collect();
            check(id, label, CheckState::Warn, join(&ips), expected, Some(format!(
                "Delete the other A record(s) for {name} ({}). Visitors and Let's Encrypt may land on them instead of your server.",
                join(&others)
            )))
        }
        Lookup::Found(ips) => check(
            id,
            label,
            CheckState::Fail,
            join(&ips),
            expected,
            Some(format!(
            "At your domain registrar, change the A record for {name} from {} to {ip}. {DNS_WAIT}",
            join(&ips)
        )),
        ),
        Lookup::Missing if custom => check(
            id,
            label,
            CheckState::Fail,
            "no record",
            expected,
            Some(format!(
                "At your domain registrar, add an A record: name {name}, value {ip}. {DNS_WAIT}"
            )),
        ),
        Lookup::Missing => check(
            id,
            label,
            CheckState::Wait,
            "no answer",
            expected,
            Some("sslip.io did not answer for this name. Check again in a minute.".into()),
        ),
        Lookup::Error(e) => check(
            id,
            label,
            CheckState::Wait,
            format!("lookup failed: {e}"),
            expected,
            Some("The DNS lookup did not finish. Check again in a minute.".into()),
        ),
    }
}

/// An AAAA record is optional, but a stale one breaks HTTPS for IPv6 visitors and certificates.
async fn check_aaaa<D: Dns>(dns: &D, name: &str, ipv6: Option<Ipv6Addr>) -> Check {
    let label = format!("AAAA record (IPv6) for {name}");
    let expected = ipv6.map(|ip| ip.to_string());
    match (dns.aaaa(name).await, ipv6) {
        (Lookup::Missing, _) => check("aaaa", label, CheckState::Ok, "none (IPv4 only, that's fine)", expected, None),
        (Lookup::Found(ips), Some(ip)) if ips == [ip] => check("aaaa", label, CheckState::Ok, ip.to_string(), expected, None),
        (Lookup::Found(ips), Some(ip)) => check("aaaa", label, CheckState::Fail, join(&ips), expected, Some(format!(
            "Change the AAAA record for {name} to {ip}, or delete it. Let's Encrypt and many visitors try IPv6 first, so a wrong AAAA record breaks HTTPS."
        ))),
        (Lookup::Found(ips), None) => check("aaaa", label, CheckState::Warn, join(&ips), None, Some(format!(
            "Make sure {} is this server's IPv6 address (enter it above to check), or delete the AAAA record. A wrong one breaks HTTPS for IPv6 visitors.",
            join(&ips)
        ))),
        (Lookup::Error(e), _) => check("aaaa", label, CheckState::Wait, format!("lookup failed: {e}"), expected, Some(
            "The DNS lookup did not finish. Check again in a minute.".into(),
        )),
    }
}

async fn check_mx<D: Dns>(dns: &D, domain: &str, mail_host: &str) -> Check {
    let label = format!("MX record for {domain}");
    let expected = Some(format!("10 {mail_host}"));
    let add = format!("At your registrar, add an MX record: name @ ({domain}), priority 10, value {mail_host}. {DNS_WAIT}");
    match dns.mx(domain).await {
        Lookup::Found(mut mx) => {
            mx.sort();
            let shown = mx
                .iter()
                .map(|(p, h)| format!("{p} {h}"))
                .collect::<Vec<_>>()
                .join(", ");
            if !mx.iter().any(|(_, h)| h == mail_host) {
                check("mx", label, CheckState::Fail, shown, expected, Some(format!(
                    "Mail for {domain} goes elsewhere. Replace the MX record(s) with priority 10, value {mail_host}. {DNS_WAIT}"
                )))
            } else if mx.len() > 1 {
                check("mx", label, CheckState::Warn, shown, expected, Some(format!(
                    "Other mail servers are listed too, so some mail may not reach your server. Keep only {mail_host} unless that's on purpose."
                )))
            } else {
                check("mx", label, CheckState::Ok, shown, expected, None)
            }
        }
        Lookup::Missing => check(
            "mx",
            label,
            CheckState::Fail,
            "no record",
            expected,
            Some(add),
        ),
        Lookup::Error(e) => check(
            "mx",
            label,
            CheckState::Wait,
            format!("lookup failed: {e}"),
            expected,
            Some("Check again in a minute.".into()),
        ),
    }
}

/// True when the SPF record lets `ip` / `mail_host` send for `domain`. `include:` and `redirect=`
/// are not followed; they make the result a warning instead.
fn spf_authorizes(record: &str, domain: &str, mail_host: &str, ip: Ipv4Addr) -> Option<bool> {
    let mut indirect = false;
    for term in record.split_whitespace().skip(1) {
        let term = term.trim_start_matches('+').to_ascii_lowercase();
        let ok = term == "mx"
            || term == format!("mx:{domain}")
            || term == format!("a:{mail_host}")
            || term == format!("ip4:{ip}")
            || term == format!("ip4:{ip}/32");
        if ok {
            return Some(true);
        }
        if term.starts_with("include:") || term.starts_with("redirect=") {
            indirect = true;
        }
    }
    if indirect {
        None
    } else {
        Some(false)
    }
}

async fn check_spf<D: Dns>(dns: &D, domain: &str, mail_host: &str, ip: Ipv4Addr) -> Check {
    let label = format!("SPF (TXT) for {domain}");
    let want = format!("v=spf1 mx a:{mail_host} ~all");
    let expected = Some(want.clone());
    let records = match dns.txt(domain).await {
        Lookup::Found(t) => t,
        Lookup::Missing => vec![],
        Lookup::Error(e) => {
            return check(
                "spf",
                label,
                CheckState::Wait,
                format!("lookup failed: {e}"),
                expected,
                Some("Check again in a minute.".into()),
            )
        }
    };
    let spf: Vec<&String> = records
        .iter()
        .filter(|r| r.to_ascii_lowercase().starts_with("v=spf1"))
        .collect();
    match spf.as_slice() {
        [] => check("spf", label, CheckState::Fail, "no SPF record", expected, Some(format!(
            "Add a TXT record: name @ ({domain}), value {want}. Without it, other servers treat your mail as spam."
        ))),
        [one] => match spf_authorizes(one, domain, mail_host, ip) {
            Some(true) => check("spf", label, CheckState::Ok, one.to_string(), expected, None),
            Some(false) => check("spf", label, CheckState::Fail, one.to_string(), expected, Some(format!(
                "Your SPF record does not allow your server to send. Add “mx” to it, e.g. {want}."
            ))),
            None => check("spf", label, CheckState::Warn, one.to_string(), expected, Some(
                "This SPF record delegates with include:/redirect=, which we don't follow. Make sure it allows your server, or add “mx”.".into(),
            )),
        },
        many => check("spf", label, CheckState::Fail, many.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" | "), expected, Some(
            "There is more than one SPF record, which makes SPF fail. Merge them into a single TXT record.".into(),
        )),
    }
}

async fn check_dmarc<D: Dns>(dns: &D, domain: &str) -> Check {
    let name = format!("_dmarc.{domain}");
    let label = format!("DMARC (TXT) for {name}");
    let want = format!("v=DMARC1; p=quarantine; rua=mailto:postmaster@{domain}");
    let expected = Some(want.clone());
    let records = match dns.txt(&name).await {
        Lookup::Found(t) => t,
        Lookup::Missing => vec![],
        Lookup::Error(e) => {
            return check(
                "dmarc",
                label,
                CheckState::Wait,
                format!("lookup failed: {e}"),
                expected,
                Some("Check again in a minute.".into()),
            )
        }
    };
    let dmarc: Vec<&String> = records
        .iter()
        .filter(|r| r.to_ascii_lowercase().starts_with("v=dmarc1"))
        .collect();
    match dmarc.as_slice() {
        [] => check(
            "dmarc",
            label,
            CheckState::Fail,
            "no DMARC record",
            expected,
            Some(format!(
                "Add a TXT record: name _dmarc, value {want}. Gmail and Outlook expect one."
            )),
        ),
        [one] if one.to_ascii_lowercase().replace(' ', "").contains("p=") => check(
            "dmarc",
            label,
            CheckState::Ok,
            one.to_string(),
            expected,
            None,
        ),
        [one] => check(
            "dmarc",
            label,
            CheckState::Fail,
            one.to_string(),
            expected,
            Some(format!("The record has no policy (p=). Use {want}.")),
        ),
        _ => check(
            "dmarc",
            label,
            CheckState::Fail,
            "more than one record",
            expected,
            Some("Keep exactly one v=DMARC1 TXT record.".into()),
        ),
    }
}

async fn check_dkim<D: Dns>(dns: &D, domain: &str, selector: Option<&str>) -> Check {
    let Some(sel) = selector else {
        return check("dkim", format!("DKIM for {domain}"), CheckState::Skip, "no selector entered", None, Some(
            "Stalwart creates your DKIM key on first start. Copy the DKIM record from the domain's DNS records in Stalwart's admin, add it at your registrar, then enter its selector (the part before ._domainkey) above.".into(),
        ));
    };
    let name = format!("{sel}._domainkey.{domain}");
    let label = format!("DKIM (TXT) for {name}");
    match dns.txt(&name).await {
        Lookup::Found(t) => {
            let rec = t.join(" ");
            let compact = rec.replace(' ', "");
            let has_key = compact
                .split(';')
                .any(|kv| kv.strip_prefix("p=").is_some_and(|k| !k.is_empty()));
            if has_key {
                check(
                    "dkim",
                    label,
                    CheckState::Ok,
                    "public key published",
                    None,
                    None,
                )
            } else {
                check(
                    "dkim",
                    label,
                    CheckState::Fail,
                    rec,
                    None,
                    Some(
                        "The record has no public key (p=…). Copy it again from Stalwart's admin."
                            .into(),
                    ),
                )
            }
        }
        Lookup::Missing => check(
            "dkim",
            label,
            CheckState::Fail,
            "no record",
            None,
            Some(format!(
                "Add the TXT record {sel}._domainkey with the value Stalwart shows. {DNS_WAIT}"
            )),
        ),
        Lookup::Error(e) => check(
            "dkim",
            label,
            CheckState::Wait,
            format!("lookup failed: {e}"),
            None,
            Some("Check again in a minute.".into()),
        ),
    }
}

async fn check_https<P: Probe>(probe: &P, host: &str, ip: Ipv4Addr, a_ok: bool) -> Check {
    let label = format!("https://{host}/healthz");
    let expected = Some("artcraftCloud answers".to_string());
    let (state, found, fix) = match probe.healthz(host, IpAddr::V4(ip)).await {
        Health::Ok { version } => (
            CheckState::Ok,
            match version {
                Some(v) => format!("artcraftCloud {v} is up"),
                None => "artcraftCloud is up".into(),
            },
            None,
        ),
        Health::Foreign => (
            CheckState::Fail,
            "something else answers".into(),
            Some(format!("{ip} answers on HTTPS, but not as artcraftCloud. Is this the right server, or is another web server using port 443?")),
        ),
        Health::Status(code @ (502..=504)) => (
            CheckState::Wait,
            format!("HTTP {code}"),
            Some("HTTPS works; artcraftCloud is still starting. The first start downloads the apps (about 550 MB) and takes 5–10 minutes.".into()),
        ),
        Health::Status(code) => (
            CheckState::Fail,
            format!("HTTP {code}"),
            Some("Unexpected reply. On the server run: docker compose -f /opt/artcraft/compose.yaml logs".into()),
        ),
        Health::Tls(e) if a_ok => (
            CheckState::Wait,
            format!("no valid certificate yet ({e})"),
            Some("Caddy requests the certificate once the A record points here; give it a minute or two. If it stays like this, make sure ports 80 and 443 are open in your host's firewall.".into()),
        ),
        Health::Tls(e) => (
            CheckState::Wait,
            format!("no valid certificate yet ({e})"),
            Some("Fix the A record first: the certificate can only be issued once the domain points at this server.".into()),
        ),
        Health::Refused => (
            CheckState::Wait,
            "nothing answers on port 443 yet".into(),
            Some("The install is probably still running (5–10 minutes on first start). If it stays like this, check that your host's firewall allows ports 80 and 443, and see /var/log/artcraft-install.log on the server.".into()),
        ),
        Health::Timeout => (
            CheckState::Wait,
            "no answer within 6 seconds".into(),
            Some("The server may still be booting, or a firewall drops port 443. Check again in a minute.".into()),
        ),
        Health::Other(e) => (CheckState::Wait, e, Some("Check again in a minute.".into())),
    };
    check("https", label, state, found, expected, fix)
}

/// Runs every check for `t`.
pub async fn verify<D: Dns, P: Probe>(dns: &D, probe: &P, t: &Target) -> VerifyReport {
    let mut checks = vec![check_a(dns, "a", &t.host, t.ipv4, t.custom).await];
    if t.custom {
        checks.push(check_aaaa(dns, &t.host, t.ipv6).await);
    }
    if t.mail && t.custom {
        let mail_host = format!("mail.{}", t.host);
        checks.push(check_a(dns, "mail_a", &mail_host, t.ipv4, true).await);
        checks.push(check_mx(dns, &t.host, &mail_host).await);
        checks.push(check_spf(dns, &t.host, &mail_host, t.ipv4).await);
        checks.push(check_dmarc(dns, &t.host).await);
        checks.push(check_dkim(dns, &t.host, t.dkim_selector.as_deref()).await);
    } else if t.mail {
        checks.push(check("mail", "Mail records".into(), CheckState::Skip, "no domain yet", None, Some(
            "Mail needs your own domain. Attach one later and these checks (MX, SPF, DMARC, DKIM) run here.".into(),
        )));
    }
    let a_ok = matches!(checks[0].state, CheckState::Ok | CheckState::Warn);
    checks.push(check_https(probe, &t.host, t.ipv4, a_ok).await);
    let ready = !checks
        .iter()
        .any(|c| matches!(c.state, CheckState::Fail | CheckState::Wait));
    VerifyReport {
        host: t.host.clone(),
        url: format!("https://{}", t.host),
        checks,
        ready,
    }
}

// ---------------------------------------------------------------------------------------------
// Real implementations

/// Asks Cloudflare's public resolvers, uncached, with short timeouts.
pub struct PublicDns {
    resolver: hickory_resolver::TokioResolver,
}

impl Default for PublicDns {
    fn default() -> Self {
        use hickory_resolver::config::{ResolverConfig, ResolverOpts};
        use hickory_resolver::name_server::TokioConnectionProvider;
        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(3);
        opts.attempts = 1;
        opts.cache_size = 0;
        let mut builder = hickory_resolver::TokioResolver::builder_with_config(
            ResolverConfig::cloudflare(),
            TokioConnectionProvider::default(),
        );
        *builder.options_mut() = opts;
        PublicDns {
            resolver: builder.build(),
        }
    }
}

fn fqdn(name: &str) -> String {
    format!("{}.", name.trim_end_matches('.'))
}

fn lookup<T, R>(
    r: Result<R, hickory_resolver::ResolveError>,
    f: impl FnOnce(R) -> Vec<T>,
) -> Lookup<T> {
    match r {
        Ok(v) => {
            let v = f(v);
            if v.is_empty() {
                Lookup::Missing
            } else {
                Lookup::Found(v)
            }
        }
        Err(e) if e.is_no_records_found() || e.is_nx_domain() => Lookup::Missing,
        Err(e) => Lookup::Error(e.to_string()),
    }
}

impl Dns for PublicDns {
    async fn a(&self, name: &str) -> Lookup<Ipv4Addr> {
        lookup(self.resolver.ipv4_lookup(fqdn(name)).await, |l| {
            l.iter().map(|a| a.0).collect()
        })
    }
    async fn aaaa(&self, name: &str) -> Lookup<Ipv6Addr> {
        lookup(self.resolver.ipv6_lookup(fqdn(name)).await, |l| {
            l.iter().map(|a| a.0).collect()
        })
    }
    async fn mx(&self, name: &str) -> Lookup<(u16, String)> {
        lookup(self.resolver.mx_lookup(fqdn(name)).await, |l| {
            l.iter()
                .map(|mx| {
                    let h = mx.exchange().to_ascii().to_ascii_lowercase();
                    (mx.preference(), h.trim_end_matches('.').to_string())
                })
                .collect()
        })
    }
    async fn txt(&self, name: &str) -> Lookup<String> {
        lookup(self.resolver.txt_lookup(fqdn(name)).await, |l| {
            l.iter()
                .map(|t| {
                    t.iter()
                        .map(|part| String::from_utf8_lossy(part).into_owned())
                        .collect::<String>()
                })
                .collect()
        })
    }
}

/// `GET <scheme>://<host>:<port>/healthz`, connecting only to the given IP.
pub struct HttpProbe {
    pub scheme: &'static str,
    pub port: u16,
    pub timeout: Duration,
}

impl Default for HttpProbe {
    fn default() -> Self {
        HttpProbe {
            scheme: "https",
            port: 443,
            timeout: Duration::from_secs(6),
        }
    }
}

fn classify(e: &reqwest::Error) -> Health {
    if e.is_timeout() {
        return Health::Timeout;
    }
    let mut chain = format!("{e}");
    let mut src = std::error::Error::source(e);
    while let Some(s) = src {
        chain.push_str(": ");
        chain.push_str(&s.to_string());
        src = s.source();
    }
    let lower = chain.to_ascii_lowercase();
    if lower.contains("connection refused") {
        return Health::Refused;
    }
    let tls = [
        "certificate",
        "tls",
        "handshake",
        "alert",
        "corrupt message",
        "invalidmessage",
        "invalidcontenttype",
    ];
    if tls.iter().any(|m| lower.contains(m)) {
        let short = if lower.contains("expired") {
            "certificate expired"
        } else if lower.contains("notvalidforname") || lower.contains("not valid for") {
            "certificate is for another name"
        } else if lower.contains("unknownissuer")
            || lower.contains("unknown issuer")
            || lower.contains("self")
        {
            "certificate not trusted yet"
        } else {
            "TLS handshake failed"
        };
        Health::Tls(short.into())
    } else if e.is_connect() {
        if lower.contains("timed out") {
            Health::Timeout
        } else {
            Health::Refused
        }
    } else {
        Health::Other("the request failed".into())
    }
}

impl Probe for HttpProbe {
    async fn healthz(&self, host: &str, ip: IpAddr) -> Health {
        let client = match reqwest::Client::builder()
            .resolve(host, SocketAddr::new(ip, self.port))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(self.timeout)
            .timeout(self.timeout)
            .user_agent("artcraftcloud.com verify")
            .build()
        {
            Ok(c) => c,
            Err(_) => return Health::Other("could not build the HTTP client".into()),
        };
        let url = format!("{}://{}:{}/healthz", self.scheme, host, self.port);
        let mut resp = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return classify(&e),
        };
        let status = resp.status().as_u16();
        if status != 200 {
            return Health::Status(status);
        }
        let mut body = Vec::new();
        while let Ok(Some(chunk)) = resp.chunk().await {
            body.extend_from_slice(&chunk);
            if body.len() > 4096 {
                break;
            }
        }
        match serde_json::from_slice::<serde_json::Value>(&body) {
            Ok(v) if v["status"] == "ok" => Health::Ok {
                version: v["version"].as_str().map(str::to_string),
            },
            _ => Health::Foreign,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Fakes: used by the tests and by demo mode (documentation IPs with ACW_ALLOW_DEMO=1).

#[derive(Default, Clone)]
pub struct FakeDns {
    pub a: HashMap<String, Vec<Ipv4Addr>>,
    pub aaaa: HashMap<String, Vec<Ipv6Addr>>,
    pub mx: HashMap<String, Vec<(u16, String)>>,
    pub txt: HashMap<String, Vec<String>>,
    /// Names whose lookups fail with an error.
    pub broken: Vec<String>,
}

fn fake<T: Clone>(m: &HashMap<String, Vec<T>>, broken: &[String], name: &str) -> Lookup<T> {
    if broken.iter().any(|b| b == name) {
        return Lookup::Error("SERVFAIL".into());
    }
    match m.get(name) {
        Some(v) if !v.is_empty() => Lookup::Found(v.clone()),
        _ => Lookup::Missing,
    }
}

impl Dns for FakeDns {
    async fn a(&self, name: &str) -> Lookup<Ipv4Addr> {
        fake(&self.a, &self.broken, name)
    }
    async fn aaaa(&self, name: &str) -> Lookup<Ipv6Addr> {
        fake(&self.aaaa, &self.broken, name)
    }
    async fn mx(&self, name: &str) -> Lookup<(u16, String)> {
        fake(&self.mx, &self.broken, name)
    }
    async fn txt(&self, name: &str) -> Lookup<String> {
        fake(&self.txt, &self.broken, name)
    }
}

pub struct FakeProbe(pub Health);

impl Probe for FakeProbe {
    async fn healthz(&self, _host: &str, _ip: IpAddr) -> Health {
        self.0.clone()
    }
}

/// Demo answers: a domain containing "ready" passes everything; anything else shows a typical
/// half-done setup (stale AAAA, missing SPF, certificate pending).
pub fn demo(t: &Target) -> (FakeDns, FakeProbe) {
    let ready = t.host.contains("ready");
    let mut dns = FakeDns::default();
    let mail_host = format!("mail.{}", t.host);
    dns.a.insert(t.host.clone(), vec![t.ipv4]);
    dns.a.insert(mail_host.clone(), vec![t.ipv4]);
    if !ready {
        dns.aaaa
            .insert(t.host.clone(), vec!["2001:db8::99".parse().unwrap()]);
    }
    dns.mx.insert(t.host.clone(), vec![(10, mail_host.clone())]);
    if ready {
        dns.txt.insert(
            t.host.clone(),
            vec![format!("v=spf1 mx a:{mail_host} ~all")],
        );
    }
    dns.txt.insert(
        format!("_dmarc.{}", t.host),
        vec![format!(
            "v=DMARC1; p=quarantine; rua=mailto:postmaster@{}",
            t.host
        )],
    );
    if let Some(sel) = &t.dkim_selector {
        dns.txt.insert(
            format!("{sel}._domainkey.{}", t.host),
            vec!["v=DKIM1; k=rsa; p=MIIBIjANBgkqh".into()],
        );
    }
    let health = if ready {
        Health::Ok {
            version: Some("0.1.0".into()),
        }
    } else {
        Health::Tls("certificate not trusted yet".into())
    };
    (dns, FakeProbe(health))
}

// ---------------------------------------------------------------------------------------------
// Rate limits

/// Fixed one-minute windows per client IP and per checked server, plus a global cap on
/// concurrent checks. The wizard polls every 15 s, well under these.
pub struct Limits {
    windows: Mutex<HashMap<String, (Instant, u32)>>,
    pub per_client_per_minute: u32,
    pub per_target_per_minute: u32,
    pub concurrent: tokio::sync::Semaphore,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            windows: Mutex::new(HashMap::new()),
            per_client_per_minute: 12,
            per_target_per_minute: 20,
            concurrent: tokio::sync::Semaphore::new(16),
        }
    }
}

impl Limits {
    /// Counts one hit for `key`; false when it is over `max` this minute.
    pub fn hit(&self, key: &str, max: u32) -> bool {
        let mut map = self.windows.lock().unwrap();
        let now = Instant::now();
        if map.len() > 10_000 {
            map.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
        }
        let entry = map.entry(key.to_string()).or_insert((now, 0));
        if now.duration_since(entry.0) >= Duration::from_secs(60) {
            *entry = (now, 0);
        }
        entry.1 += 1;
        entry.1 <= max
    }
}

#[cfg(test)]
mod tests;
