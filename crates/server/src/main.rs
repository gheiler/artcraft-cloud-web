use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use types::DeployRequest;

#[derive(Clone)]
struct AppState {
    /// In-memory job registry: job id -> current state.
    jobs: Arc<Mutex<HashMap<String, Job>>>,
    /// Monotonic id generator so every deploy gets a unique job id.
    next_id: Arc<AtomicU64>,
}

#[derive(Clone)]
struct Job {
    status: String,
    detail: Option<String>,
    ip: Option<String>,
}

impl Job {
    fn new(status: &str) -> Self {
        Self {
            status: status.to_string(),
            detail: None,
            ip: None,
        }
    }
}

/// POST /api/deploy
/// Accepts the wizard form, starts a background provisioning task, and
/// returns immediately with a job id the client can poll.
async fn handle_deploy(
    State(state): State<AppState>,
    Json(payload): Json<DeployRequest>,
) -> Json<serde_json::Value> {
    let job_id = format!("job-{}", state.next_id.fetch_add(1, Ordering::SeqCst));

    state
        .jobs
        .lock()
        .unwrap()
        .insert(job_id.clone(), Job::new("queued"));

    let jobs = state.jobs.clone();
    let id = job_id.clone();

    tokio::spawn(async move {
        tracing::info!(
            job = %id,
            provider = %payload.provider,
            mode = ?payload.mode,
            domain = %payload.domain,
            email = %payload.email,
            skip_domain = payload.skip_domain,
            "provisioning started"
        );

        // The API token lives only in this task. It is dropped when the task
        // ends and is never written to SQLite or disk.
        let _token_in_scope = payload.api_token;

        // --- Placeholder for the real provider call -------------------------
        // 1. build cloud-init for payload.domain / payload.mode
        // 2. call the provider API with the token
        // 3. poll the provider until the VM is running
        // 4. DNS + /healthz checks
        // 5. email the one-time admin credentials
        set_job(&jobs, &id, |j| {
            j.status = "installing".into();
            j.detail = Some("installing Docker, Caddy and the drive".into());
        });
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        set_job(&jobs, &id, |j| {
            j.status = "configuring".into();
            j.detail = Some("writing compose.yaml and the Caddyfile".into());
        });
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Pretend the provider told us the public address.
        let ip = "203.0.113.10".to_string();
        set_job(&jobs, &id, |j| {
            j.status = "done".into();
            j.detail = Some("server online".into());
            j.ip = Some(ip.clone());
        });

        // This is where a real "your cloud is ready" email is sent.
        // No SMTP credentials are configured yet, so we only log the intent.
        tracing::info!(
            job = %id,
            to = %payload.email,
            ip = %ip,
            "notification email queued (SMTP not configured)"
        );
        tracing::info!(job = %id, "provisioning finished");
    });

    Json(serde_json::json!({ "job_id": job_id, "status": "started" }))
}

fn set_job(jobs: &Arc<Mutex<HashMap<String, Job>>>, id: &str, f: impl FnOnce(&mut Job)) {
    if let Some(job) = jobs.lock().unwrap().get_mut(id) {
        f(job);
    }
}

/// GET /api/deploy/:id
/// Lets the wizard poll a job until it is finished.
async fn job_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let jobs = state.jobs.lock().unwrap();
    match jobs.get(&id) {
        Some(job) => Json(serde_json::json!({
            "job_id": id,
            "status": job.status,
            "detail": job.detail,
            "ip": job.ip,
        })),
        None => Json(serde_json::json!({
            "job_id": id,
            "status": "unknown",
        })),
    }
}

#[tokio::main]
async fn main() {
    // Structured logs to stdout. Set RUST_LOG=debug for more detail, e.g.
    //   RUST_LOG=debug cargo run -p server
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let state = AppState {
        jobs: Arc::new(Mutex::new(HashMap::new())),
        next_id: Arc::new(AtomicU64::new(1)),
    };

    let app = Router::new()
        .route("/", get(|| async { "artcraftCloud API" }))
        .route("/api/healthz", get(|| async { "ok" }))
        .route("/api/deploy", post(handle_deploy))
        .route("/api/deploy/{id}", get(job_status))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind port 3000");
    tracing::info!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
