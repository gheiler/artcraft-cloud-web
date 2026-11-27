//! artcraftcloud.com in one binary: the server-rendered pages (from the `site` crate), the
//! `/deploy` wizard's static files, and the deploy API.

pub mod cloud_init;
pub mod jobs;
pub mod providers;
pub mod verify;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use site::Page;
use tower::ServiceExt as _;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, set_header::SetResponseHeaderLayer,
};
use types::{sslip_domain, CloudInitRecipe, DeployAccepted, DeployRequest};

use crate::cloud_init::Settings;
use crate::jobs::Jobs;
use crate::providers::hetzner::Hetzner;

#[derive(Clone)]
pub struct Config {
    /// `crates/site/static`: the CSS, fonts, icons and screenshots, served at `/assets`.
    pub static_dir: PathBuf,
    /// `crates/wizard/dist`: the trunk build of the wizard, served at `/deploy`.
    pub wizard_dir: PathBuf,
    /// Base URL of the Hetzner API (a mock in tests).
    pub hetzner_api: String,
    /// When true, the API token `demo` runs a simulated deploy (for screenshots and local work),
    /// and `/api/verify` answers documentation IPs (203.0.113.0/24, …) with canned results.
    pub allow_demo: bool,
    /// Behind a reverse proxy (Caddy): take the client address for rate limits from the last
    /// `X-Forwarded-For` entry instead of the TCP peer.
    pub trust_proxy: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        Config {
            static_dir: env("ACW_STATIC_DIR", "crates/site/static").into(),
            wizard_dir: env("ACW_WIZARD_DIR", "crates/wizard/dist").into(),
            hetzner_api: env("ACW_HETZNER_API", providers::hetzner::API),
            allow_demo: env("ACW_ALLOW_DEMO", "0") == "1",
            trust_proxy: env("ACW_TRUST_PROXY", "0") == "1",
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub jobs: Jobs,
    pub dns: Arc<verify::PublicDns>,
    pub limits: Arc<verify::Limits>,
}

pub fn app(config: Config) -> Router {
    let state = AppState {
        config: config.clone(),
        jobs: Jobs::default(),
        dns: Arc::new(verify::PublicDns::default()),
        limits: Arc::new(verify::Limits::default()),
    };
    let assets = Router::new()
        .fallback_service(ServeDir::new(config.static_dir.join("assets")))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600"),
        ));
    Router::new()
        .route("/", get(page))
        .route("/apps", get(page))
        .route("/hosts", get(page))
        .route("/guide", get(page))
        .route("/deploy", get(wizard_index))
        .route("/deploy/", get(wizard_index))
        .route("/deploy/{*file}", get(wizard_file))
        .nest("/assets", assets)
        .route("/favicon.svg", get(favicon))
        .route("/robots.txt", get(robots))
        .route("/sitemap.xml", get(sitemap))
        .route("/healthz", get(|| async { "ok" }))
        .route("/api/healthz", get(|| async { "ok" }))
        .route("/api/cloud-init", post(cloud_init_recipe))
        .route("/api/deploy", post(deploy))
        .route("/api/deploy/{id}", get(job_status))
        .route("/api/verify", post(verify_server))
        .fallback(not_found)
        .layer(CompressionLayer::new())
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .with_state(state)
}

fn year() -> i32 {
    // Days since 1970 -> civil year (Howard Hinnant's algorithm), to avoid a date crate.
    let days = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + if month <= 2 { 1 } else { 0 }) as i32
}

async fn page(uri: Uri) -> Html<String> {
    Html(site::render_page(Page::from_path(uri.path()), year()))
}

async fn not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Html(site::render_page(Page::NotFound, year())),
    )
        .into_response()
}

async fn wizard_index(State(state): State<AppState>) -> Response {
    match tokio::fs::read_to_string(state.config.wizard_dir.join("index.html")).await {
        Ok(html) => {
            let head = site::head(
                "Deploy your own cloud · artcraftCloud",
                "Pick what to run and a European host; we boot your artcraftCloud with one API token or give you the recipe to paste.",
                "/deploy",
            );
            Html(
                html.replacen("<meta name=\"artcraft-head\">", &head, 1)
                    .replacen("<meta name=\"artcraft-head\" />", &head, 1),
            )
            .into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "The wizard is not built yet: run `trunk build` in crates/wizard.",
        )
            .into_response(),
    }
}

/// `/deploy/wizard-<hash>.js` and friends from the trunk build.
async fn wizard_file(
    State(state): State<AppState>,
    Path(file): Path<String>,
    req: axum::extract::Request,
) -> Response {
    if file.is_empty() || file == "index.html" {
        return wizard_index(State(state)).await;
    }
    if file
        .split('/')
        .any(|part| part.is_empty() || part.starts_with('.'))
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = state.config.wizard_dir.join(&file);
    match tower_http::services::ServeFile::new(path)
        .oneshot(req)
        .await
    {
        Ok(resp) if resp.status() == StatusCode::OK => {
            // File names carry a content hash, so they can be cached for good.
            let mut resp = resp.into_response();
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            resp
        }
        Ok(resp) => resp.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn favicon(State(state): State<AppState>) -> Response {
    match tokio::fs::read(state.config.static_dir.join("assets/brand/favicon.svg")).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/svg+xml")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn robots() -> impl IntoResponse {
    format!(
        "User-agent: *\nAllow: /\nDisallow: /api/\nSitemap: {}/sitemap.xml\n",
        site::SITE_URL
    )
}

async fn sitemap() -> impl IntoResponse {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#,
    );
    for path in Page::ALL.iter().map(|p| p.path()).chain(["/deploy"]) {
        xml.push_str(&format!("<url><loc>{}{}</loc></url>", site::SITE_URL, path));
    }
    xml.push_str("</urlset>");
    ([(header::CONTENT_TYPE, "application/xml")], xml)
}

fn bad_request(msg: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": msg.into() })),
    )
        .into_response()
}

/// POST /api/cloud-init: the recipe to paste into a guided host's panel. No token involved.
async fn cloud_init_recipe(Json(req): Json<DeployRequest>) -> Response {
    if let Err(e) = req.validate() {
        return bad_request(e);
    }
    let domain = req.effective_domain();
    let settings = Settings::new(domain.clone(), &req.email, req.mode);
    match cloud_init::render(&settings) {
        Ok(yaml) => Json(CloudInitRecipe {
            yaml,
            url: domain.map(|d| format!("https://{d}")),
        })
        .into_response(),
        Err(e) => bad_request(e),
    }
}

/// POST /api/deploy: starts a one-click deploy and returns a job id to poll.
async fn deploy(State(state): State<AppState>, Json(req): Json<DeployRequest>) -> Response {
    if let Err(e) = req.validate() {
        return bad_request(e);
    }
    let host = types::host(&req.provider).expect("validated");
    if !host.one_click() {
        return bad_request(format!(
            "{} is not automated yet; use the recipe instead.",
            host.name
        ));
    }
    let token = req.api_token.trim().to_string();
    if token.is_empty() {
        return bad_request(format!("{} needs an API token.", host.name));
    }
    let demo = state.config.allow_demo && token == "demo";
    let Some(job_id) = state.jobs.create() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({ "error": "Too many deploys right now. Try again in a few minutes." }))).into_response();
    };
    tracing::info!(job = %job_id, provider = %req.provider, mode = ?req.mode, demo, "deploy started");

    let jobs = state.jobs.clone();
    let id = job_id.clone();
    let api = state.config.hetzner_api.clone();
    tokio::spawn(async move {
        // The token moves into this task and is dropped when it returns.
        let result = if demo {
            run_demo(&jobs, &id, &req).await
        } else {
            run_hetzner(&jobs, &id, &req, &token, &api).await
        };
        if let Err(e) = result {
            tracing::warn!(job = %id, error = %e, "deploy failed");
            jobs.update(&id, |s| {
                s.status = "failed".into();
                s.detail = Some(e);
            });
        }
    });

    Json(DeployAccepted {
        job_id,
        status: "started".into(),
    })
    .into_response()
}

async fn run_hetzner(
    jobs: &Jobs,
    id: &str,
    req: &DeployRequest,
    token: &str,
    api: &str,
) -> Result<(), String> {
    let password = cloud_init::generate_password();
    let mut settings = Settings::new(req.effective_domain(), &req.email, req.mode);
    settings.admin_password = Some(password.clone());
    let user_data = cloud_init::render(&settings)?;
    jobs.update(id, |s| {
        s.status = "creating".into();
        s.detail = Some("Asking Hetzner for a server".into());
    });

    let hetzner = Hetzner::with_base(api);
    let name = format!("artcraft-{}", &id[..8]);
    let server = hetzner
        .create_server(token, &name, &user_data, req.region.as_deref())
        .await?;
    tracing::info!(job = %id, server = server.id, server_type = %server.server_type, location = %server.location, "server created");
    jobs.update(id, |s| {
        s.status = "booting".into();
        s.detail = Some(format!(
            "Server {} ({}, {}) is starting",
            name, server.server_type, server.location
        ));
        s.ip = server.ipv4.clone();
    });

    // Wait for "running" (usually under a minute). Cloud-init then installs in 5–10 minutes; the
    // wizard polls /api/verify (DNS + https://<host>/healthz) for that part.
    let mut ip = server.ipv4.clone();
    let mut running = false;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_secs(5)).await;
        match hetzner.server(token, server.id).await {
            Ok((status, v4)) => {
                if v4.is_some() {
                    ip = v4;
                }
                if status == "running" {
                    running = true;
                    break;
                }
            }
            Err(e) => tracing::info!(job = %id, error = %e, "status check failed, retrying"),
        }
    }
    if !running {
        return Err(
            "The server did not reach “running” within 5 minutes. Check it in the Hetzner console."
                .into(),
        );
    }
    let ip = ip.ok_or("Hetzner did not give the server an IPv4 address.")?;
    finish(jobs, id, req, &ip, password);
    Ok(())
}

/// Fake run for screenshots and local development (`ACW_ALLOW_DEMO=1`, token `demo`).
async fn run_demo(jobs: &Jobs, id: &str, req: &DeployRequest) -> Result<(), String> {
    for (status, detail) in [
        ("creating", "Asking Hetzner for a server (demo)"),
        ("booting", "Server artcraft-demo (cx23, nbg1) is starting"),
    ] {
        jobs.update(id, |s| {
            s.status = status.into();
            s.detail = Some(detail.into());
        });
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
    finish(
        jobs,
        id,
        req,
        "203.0.113.10",
        cloud_init::generate_password(),
    );
    Ok(())
}

fn finish(jobs: &Jobs, id: &str, req: &DeployRequest, ip: &str, password: String) {
    let domain = req.effective_domain().unwrap_or_else(|| sslip_domain(ip));
    jobs.update(id, |s| {
        s.status = "installing".into();
        s.detail = Some(
            "The server is up and installing artcraftCloud (5–10 minutes on first start)".into(),
        );
        s.ip = Some(ip.to_string());
        s.url = Some(format!("https://{domain}"));
        s.admin_user = Some("admin".into());
        s.admin_password = Some(password);
    });
}

/// GET /api/deploy/:id
async fn job_status(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.jobs.get(&id) {
        Some(status) => ([(header::CACHE_CONTROL, "no-store")], Json(status)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "job_id": id, "status": "unknown" })),
        )
            .into_response(),
    }
}

/// The address rate limits are counted against.
fn client_ip(req: &axum::extract::Request, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(last) = req
            .headers()
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .map(str::trim)
            .rfind(|v| !v.is_empty())
        {
            return last.to_string();
        }
    }
    req.extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip().to_string())
        .unwrap_or_else(|| "local".into())
}

fn too_many(msg: &str) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, "60")],
        Json(serde_json::json!({ "error": msg })),
    )
        .into_response()
}

/// POST /api/verify: DNS records and `https://<host>/healthz` for a new server (see `verify.rs`).
async fn verify_server(State(state): State<AppState>, req: axum::extract::Request) -> Response {
    let client = client_ip(&req, state.config.trust_proxy);
    if !state
        .limits
        .hit(&format!("c:{client}"), state.limits.per_client_per_minute)
    {
        return too_many("Too many checks from your address. Wait a minute and try again.");
    }
    let body = match axum::body::to_bytes(req.into_body(), 4096).await {
        Ok(b) => b,
        Err(_) => return bad_request("Request too large."),
    };
    let Ok(vr) = serde_json::from_slice::<types::VerifyRequest>(&body) else {
        return bad_request("Expected a JSON verify request.");
    };
    let target = match verify::Target::parse(&vr, state.config.allow_demo) {
        Ok(t) => t,
        Err(e) => return bad_request(e),
    };
    if !state.limits.hit(
        &format!("t:{}", target.ipv4),
        state.limits.per_target_per_minute,
    ) {
        return too_many(
            "This server was checked a lot in the last minute. Wait a minute and try again.",
        );
    }
    let Ok(_permit) = state.limits.concurrent.try_acquire() else {
        return too_many("Lots of checks running right now. Try again in a few seconds.");
    };
    let demo = state.config.allow_demo && target.ipv4.is_documentation();
    let report = if demo {
        let (dns, probe) = verify::demo(&target);
        verify::verify(&dns, &probe, &target).await
    } else {
        let probe = verify::HttpProbe::default();
        verify::verify(state.dns.as_ref(), &probe, &target).await
    };
    tracing::info!(host = %report.host, ready = report.ready, demo, "verify");
    ([(header::CACHE_CONTROL, "no-store")], Json(report)).into_response()
}

#[cfg(test)]
mod tests;
