use super::*;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn config() -> Config {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    Config {
        static_dir: root.join("crates/site/static"),
        wizard_dir: root.join("crates/wizard/dist-does-not-exist"),
        hetzner_api: "http://127.0.0.1:9".into(),
        allow_demo: true,
    }
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, String) {
    let mut req = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(json) => {
            req = req.header("content-type", "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let resp = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn pages_are_server_rendered_html() {
    let app = app(config());
    for (path, needle) in [
        ("/", "Your European server."),
        ("/apps", "WordCraft"),
        ("/hosts", "Hetzner Cloud"),
        ("/guide", "Caddy"),
    ] {
        let (status, html) = call(&app, "GET", path, None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(html.starts_with("<!DOCTYPE html>"), "{path}");
        assert!(html.contains(needle), "{path} should contain {needle}");
        assert!(html.contains("<link rel=\"stylesheet\" href=\"/assets/site.css\">"));
        assert!(!html.contains(".wasm"), "{path} must not load Wasm");
    }
    let (status, html) = call(&app, "GET", "/nope", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(html.contains("That page drifted off"));
}

#[tokio::test]
async fn static_files_robots_and_sitemap() {
    let app = app(config());
    let (status, css) = call(&app, "GET", "/assets/site.css", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(css.contains("--primary"));
    assert_eq!(
        call(&app, "GET", "/favicon.svg", None).await.0,
        StatusCode::OK
    );
    assert!(call(&app, "GET", "/robots.txt", None)
        .await
        .1
        .contains("Disallow: /api/"));
    assert!(call(&app, "GET", "/sitemap.xml", None)
        .await
        .1
        .contains("https://artcraftcloud.com/hosts"));
    // No wizard build in tests: a clear 503 instead of a blank page.
    assert_eq!(
        call(&app, "GET", "/deploy", None).await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

fn request(provider: &str, domain: &str, token: &str) -> serde_json::Value {
    serde_json::json!({
        "provider": provider, "mode": "drive_mail", "domain": domain,
        "email": "me@example.eu", "api_token": token, "skip_domain": domain.is_empty(),
    })
}

#[tokio::test]
async fn cloud_init_recipe_is_filled_in_and_validated() {
    let app = app(config());
    let (status, body) = call(
        &app,
        "POST",
        "/api/cloud-init",
        Some(request("euronodes", "cloud.example.eu", "")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let recipe: CloudInitRecipe = serde_json::from_str(&body).unwrap();
    assert!(recipe
        .yaml
        .contains("      ARTCRAFT_DOMAIN=cloud.example.eu\n"));
    assert!(recipe.yaml.contains("      ARTCRAFT_MAIL=1\n"));
    assert_eq!(recipe.url.as_deref(), Some("https://cloud.example.eu"));

    let (status, body) = call(
        &app,
        "POST",
        "/api/cloud-init",
        Some(request("euronodes", "", "")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let recipe: CloudInitRecipe = serde_json::from_str(&body).unwrap();
    assert!(recipe.yaml.contains("      ARTCRAFT_DOMAIN=\n"));
    assert_eq!(recipe.url, None);

    let (status, body) = call(
        &app,
        "POST",
        "/api/cloud-init",
        Some(request("euronodes", "bad domain", "")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("does not look like a domain"));
}

#[tokio::test]
async fn deploy_checks_host_and_token_then_runs_the_demo() {
    let app = app(config());
    let (status, body) = call(
        &app,
        "POST",
        "/api/deploy",
        Some(request("euronodes", "cloud.example.eu", "x")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("not automated yet"));
    let (status, _) = call(
        &app,
        "POST",
        "/api/deploy",
        Some(request("hetzner", "cloud.example.eu", "")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, body) = call(
        &app,
        "POST",
        "/api/deploy",
        Some(request("hetzner", "", "demo")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let accepted: DeployAccepted = serde_json::from_str(&body).unwrap();
    let mut last = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let (_, body) = call(
            &app,
            "GET",
            &format!("/api/deploy/{}", accepted.job_id),
            None,
        )
        .await;
        let s: types::JobStatus = serde_json::from_str(&body).unwrap();
        if s.status == "installing" {
            last = Some(s);
            break;
        }
    }
    let s = last.expect("demo reaches installing");
    assert_eq!(s.url.as_deref(), Some("https://203-0-113-10.sslip.io"));
    assert_eq!(s.admin_password.as_ref().map(|p| p.len()), Some(24));
    assert_eq!(
        call(&app, "GET", "/api/deploy/unknown", None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_failed_hetzner_call_marks_the_job_failed() {
    // hetzner_api points at a closed port.
    let app = app(config());
    let (_, body) = call(
        &app,
        "POST",
        "/api/deploy",
        Some(request("hetzner", "cloud.example.eu", "real-token")),
    )
    .await;
    let accepted: DeployAccepted = serde_json::from_str(&body).unwrap();
    let mut status = String::new();
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let (_, body) = call(
            &app,
            "GET",
            &format!("/api/deploy/{}", accepted.job_id),
            None,
        )
        .await;
        let s: types::JobStatus = serde_json::from_str(&body).unwrap();
        status = s.status;
        if status == "failed" {
            assert!(s.detail.unwrap().contains("Could not reach Hetzner"));
            break;
        }
    }
    assert_eq!(status, "failed");
}

#[test]
fn year_is_plausible() {
    assert!((2025..2100).contains(&year()));
}
