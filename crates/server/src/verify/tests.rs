use super::*;

const IP: &str = "198.51.100.7"; // documentation range, allowed in these tests

fn req(domain: &str, mail: bool) -> VerifyRequest {
    VerifyRequest {
        domain: domain.into(),
        skip_domain: false,
        ipv4: IP.into(),
        ipv6: String::new(),
        mail,
        dkim_selector: String::new(),
    }
}

fn target(r: &VerifyRequest) -> Target {
    Target::parse(r, true).unwrap()
}

fn ip() -> Ipv4Addr {
    IP.parse().unwrap()
}

/// Everything right for `example.eu` with mail.
fn good_dns() -> FakeDns {
    let mut d = FakeDns::default();
    d.a.insert("example.eu".into(), vec![ip()]);
    d.a.insert("mail.example.eu".into(), vec![ip()]);
    d.mx.insert("example.eu".into(), vec![(10, "mail.example.eu".into())]);
    d.txt.insert(
        "example.eu".into(),
        vec![
            "google-site-verification=abc".into(),
            "v=spf1 mx a:mail.example.eu ~all".into(),
        ],
    );
    d.txt.insert(
        "_dmarc.example.eu".into(),
        vec!["v=DMARC1; p=quarantine; rua=mailto:postmaster@example.eu".into()],
    );
    d.txt.insert(
        "202611e._domainkey.example.eu".into(),
        vec!["v=DKIM1; k=ed25519; p=11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=".into()],
    );
    d
}

fn up() -> FakeProbe {
    FakeProbe(Health::Ok {
        version: Some("0.3.0".into()),
    })
}

fn state(r: &VerifyReport, id: &str) -> CheckState {
    r.checks
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("no check {id}"))
        .state
}

fn fix(r: &VerifyReport, id: &str) -> String {
    r.checks
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .fix
        .clone()
        .unwrap_or_default()
}

#[test]
fn rejects_private_and_special_addresses() {
    for bad in [
        "10.0.0.1",
        "172.16.5.5",
        "192.168.1.1",
        "127.0.0.1",
        "169.254.169.254",
        "100.64.0.1",
        "0.0.0.0",
        "255.255.255.255",
        "224.0.0.1",
        "198.18.0.1",
        "192.0.0.8",
        "240.0.0.1",
    ] {
        let mut r = req("example.eu", false);
        r.ipv4 = bad.into();
        assert!(Target::parse(&r, true).is_err(), "{bad} must be rejected");
    }
    // Documentation ranges only in demo mode.
    assert!(Target::parse(&req("example.eu", false), false).is_err());
    let mut r = req("example.eu", false);
    r.ipv4 = "95.217.1.2".into();
    assert!(Target::parse(&r, false).is_ok());
    for bad in [
        "::1",
        "fe80::1",
        "fc00::1",
        "2002:a00:1::1",
        "2001:0:1::1",
        "::ffff:10.0.0.1",
    ] {
        let mut r = req("example.eu", false);
        r.ipv6 = bad.into();
        assert!(Target::parse(&r, true).is_err(), "{bad} must be rejected");
    }
    let mut r = req("example.eu", false);
    r.ipv6 = "2a01:4f8:1c1c::1".into();
    assert!(
        Target::parse(&r, false).is_err(),
        "IPv4 is still a documentation address"
    );
    r.ipv4 = "95.217.1.2".into();
    assert!(Target::parse(&r, false).is_ok());
}

#[test]
fn rejects_bad_domains_and_selectors() {
    for bad in [
        "localhost",
        "exa mple.eu",
        "-x.eu",
        "a..eu",
        "1.2.3.4",
        "x.eu/../",
    ] {
        assert!(Target::parse(&req(bad, false), true).is_err(), "{bad}");
    }
    let mut r = req("example.eu", true);
    r.dkim_selector = "a b".into();
    assert!(Target::parse(&r, true).is_err());
    r.dkim_selector = "202611e._domainkey".into();
    assert_eq!(
        Target::parse(&r, true).unwrap().dkim_selector.as_deref(),
        Some("202611e")
    );
}

#[test]
fn sslip_mode_uses_the_ip_name() {
    let mut r = req("", true);
    r.skip_domain = true;
    let t = target(&r);
    assert_eq!(t.host, "198-51-100-7.sslip.io");
    assert!(!t.custom);
    let mut r = req("cloud.example.eu", false);
    r.skip_domain = true;
    assert_eq!(target(&r).host, "198-51-100-7.sslip.io");
}

#[tokio::test]
async fn all_good_with_mail() {
    let mut r = req("Example.EU.", true);
    r.dkim_selector = "202611e".into();
    let report = verify(&good_dns(), &up(), &target(&r)).await;
    let ids: Vec<_> = report.checks.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        ["a", "aaaa", "mail_a", "mx", "spf", "dmarc", "dkim", "https"]
    );
    assert!(
        report.checks.iter().all(|c| c.state == CheckState::Ok),
        "{report:#?}"
    );
    assert!(report.ready && report.https_ok());
    assert_eq!(report.url, "https://example.eu");
}

#[tokio::test]
async fn missing_dkim_selector_is_skipped_not_failed() {
    let report = verify(&good_dns(), &up(), &target(&req("example.eu", true))).await;
    assert_eq!(state(&report, "dkim"), CheckState::Skip);
    assert!(report.ready);
}

#[tokio::test]
async fn wrong_and_missing_a_records() {
    let mut d = FakeDns::default();
    d.a.insert("example.eu".into(), vec!["203.0.113.99".parse().unwrap()]);
    let report = verify(&d, &up(), &target(&req("example.eu", false))).await;
    assert_eq!(state(&report, "a"), CheckState::Fail);
    assert!(fix(&report, "a").contains("from 203.0.113.99 to 198.51.100.7"));
    assert!(!report.ready);

    let report = verify(
        &FakeDns::default(),
        &up(),
        &target(&req("example.eu", false)),
    )
    .await;
    assert_eq!(state(&report, "a"), CheckState::Fail);
    assert!(fix(&report, "a").contains("add an A record"));

    d.a.insert(
        "example.eu".into(),
        vec![ip(), "203.0.113.99".parse().unwrap()],
    );
    let report = verify(&d, &up(), &target(&req("example.eu", false))).await;
    assert_eq!(state(&report, "a"), CheckState::Warn);
    assert!(report.ready, "an extra A record is a warning");
}

#[tokio::test]
async fn stale_aaaa_breaks_https() {
    let mut d = good_dns();
    d.aaaa
        .insert("example.eu".into(), vec!["2001:db8::99".parse().unwrap()]);
    let report = verify(&d, &up(), &target(&req("example.eu", false))).await;
    assert_eq!(state(&report, "aaaa"), CheckState::Warn);
    let mut r = req("example.eu", false);
    r.ipv6 = "2001:db8::1".into();
    let report = verify(&d, &up(), &target(&r)).await;
    assert_eq!(state(&report, "aaaa"), CheckState::Fail);
    assert!(fix(&report, "aaaa").contains("2001:db8::1"));
    r.ipv6 = "2001:db8::99".into();
    let report = verify(&d, &up(), &target(&r)).await;
    assert_eq!(state(&report, "aaaa"), CheckState::Ok);
}

#[tokio::test]
async fn spf_variants() {
    let t = target(&req("example.eu", true));
    for (record, want) in [
        ("v=spf1 mx ~all", CheckState::Ok),
        ("v=spf1 +mx -all", CheckState::Ok),
        ("v=spf1 ip4:198.51.100.7 -all", CheckState::Ok),
        ("v=spf1 a:mail.example.eu ~all", CheckState::Ok),
        ("v=spf1 include:_spf.google.com ~all", CheckState::Warn),
        ("v=spf1 -all", CheckState::Fail),
    ] {
        let mut d = good_dns();
        d.txt.insert("example.eu".into(), vec![record.into()]);
        let report = verify(&d, &up(), &t).await;
        assert_eq!(state(&report, "spf"), want, "{record}");
    }
    let mut d = good_dns();
    d.txt.insert(
        "example.eu".into(),
        vec!["v=spf1 mx ~all".into(), "v=spf1 a ~all".into()],
    );
    let report = verify(&d, &up(), &t).await;
    assert_eq!(state(&report, "spf"), CheckState::Fail);
    assert!(fix(&report, "spf").contains("more than one"));
    d.txt.remove("example.eu");
    let report = verify(&d, &up(), &t).await;
    assert_eq!(state(&report, "spf"), CheckState::Fail);
    assert!(fix(&report, "spf").contains("v=spf1 mx a:mail.example.eu ~all"));
}

#[tokio::test]
async fn mx_dmarc_dkim_problems() {
    let mut r = req("example.eu", true);
    r.dkim_selector = "202611e".into();
    let t = target(&r);
    let mut d = good_dns();
    d.mx.insert("example.eu".into(), vec![(10, "mx.other.net".into())]);
    d.txt.remove("_dmarc.example.eu");
    d.txt.insert(
        "202611e._domainkey.example.eu".into(),
        vec!["v=DKIM1; p=".into()],
    );
    d.a.remove("mail.example.eu");
    let report = verify(&d, &up(), &t).await;
    assert_eq!(state(&report, "mx"), CheckState::Fail);
    assert_eq!(state(&report, "dmarc"), CheckState::Fail);
    assert_eq!(state(&report, "dkim"), CheckState::Fail);
    assert_eq!(state(&report, "mail_a"), CheckState::Fail);
    d.mx.insert(
        "example.eu".into(),
        vec![
            (20, "backup.other.net".into()),
            (10, "mail.example.eu".into()),
        ],
    );
    let report = verify(&d, &up(), &t).await;
    assert_eq!(state(&report, "mx"), CheckState::Warn);
}

#[tokio::test]
async fn dns_errors_mean_wait() {
    let mut d = good_dns();
    d.broken.push("example.eu".into());
    let report = verify(&d, &up(), &target(&req("example.eu", false))).await;
    assert_eq!(state(&report, "a"), CheckState::Wait);
    assert!(!report.ready);
}

#[tokio::test]
async fn mail_in_sslip_mode_is_skipped() {
    let mut r = req("", true);
    r.skip_domain = true;
    let mut d = FakeDns::default();
    d.a.insert("198-51-100-7.sslip.io".into(), vec![ip()]);
    let report = verify(&d, &up(), &target(&r)).await;
    let ids: Vec<_> = report
        .checks
        .iter()
        .map(|c| (c.id.as_str(), c.state))
        .collect();
    assert_eq!(
        ids,
        [
            ("a", CheckState::Ok),
            ("mail", CheckState::Skip),
            ("https", CheckState::Ok)
        ]
    );
    assert!(report.ready);
}

#[tokio::test]
async fn https_states() {
    let t = target(&req("example.eu", false));
    for (health, want, needle) in [
        (Health::Refused, CheckState::Wait, "5–10 minutes"),
        (Health::Timeout, CheckState::Wait, "firewall"),
        (
            Health::Tls("certificate not trusted yet".into()),
            CheckState::Wait,
            "certificate",
        ),
        (Health::Status(502), CheckState::Wait, "550 MB"),
        (Health::Status(404), CheckState::Fail, "logs"),
        (Health::Foreign, CheckState::Fail, "not as artcraftCloud"),
    ] {
        let report = verify(&good_dns(), &FakeProbe(health.clone()), &t).await;
        assert_eq!(state(&report, "https"), want, "{health:?}");
        assert!(
            fix(&report, "https").contains(needle),
            "{health:?}: {}",
            fix(&report, "https")
        );
        assert!(!report.ready);
    }
    // TLS failing because DNS is wrong points at the A record.
    let report = verify(&FakeDns::default(), &FakeProbe(Health::Tls("x".into())), &t).await;
    assert!(fix(&report, "https").contains("A record first"));
}

#[test]
fn rate_limit_windows() {
    let l = Limits::default();
    for _ in 0..12 {
        assert!(l.hit("c:1", 12));
    }
    assert!(!l.hit("c:1", 12));
    assert!(l.hit("c:2", 12), "other clients are not affected");
}

/// The real probe against a local server: it connects to the pinned IP whatever the name is,
/// reads artcraftCloud's health JSON, and tells "nothing listening" from a wrong reply.
#[tokio::test]
async fn http_probe_pins_the_ip() {
    use axum::{routing::get, Json, Router};
    let app = Router::new().route(
        "/healthz",
        get(|| async { Json(serde_json::json!({"status":"ok","version":"9.9.9"})) }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let probe = HttpProbe {
        scheme: "http",
        port,
        timeout: Duration::from_secs(3),
    };
    let local: IpAddr = "127.0.0.1".parse().unwrap();
    // This name does not resolve anywhere; the probe must not need DNS.
    let h = probe.healthz("does-not-exist.invalid", local).await;
    assert_eq!(
        h,
        Health::Ok {
            version: Some("9.9.9".into())
        }
    );

    let other = Router::new().route("/healthz", get(|| async { "hello" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port2 = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, other).await.unwrap() });
    let probe2 = HttpProbe {
        scheme: "http",
        port: port2,
        timeout: Duration::from_secs(3),
    };
    assert_eq!(probe2.healthz("x.invalid", local).await, Health::Foreign);

    // A port with nothing behind it.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = closed.local_addr().unwrap().port();
    drop(closed);
    let probe3 = HttpProbe {
        scheme: "http",
        port: dead,
        timeout: Duration::from_secs(3),
    };
    assert_eq!(probe3.healthz("x.invalid", local).await, Health::Refused);
}

/// A TLS server with a self-signed certificate is reported as a certificate problem.
#[tokio::test]
async fn https_probe_reports_untrusted_certificates() {
    // A listener that accepts and immediately speaks garbage instead of TLS.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            if let Ok((mut s, _)) = listener.accept().await {
                use tokio::io::AsyncWriteExt;
                let _ = s.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            }
        }
    });
    let probe = HttpProbe {
        scheme: "https",
        port,
        timeout: Duration::from_secs(3),
    };
    match probe
        .healthz("x.invalid", "127.0.0.1".parse().unwrap())
        .await
    {
        Health::Tls(_) | Health::Other(_) => {}
        h => panic!("expected a TLS failure, got {h:?}"),
    }
}
