//! Hetzner Cloud (https://docs.hetzner.cloud). One project API token with Read & Write access.
//!
//! The token is passed in per call and never stored: the job task owns it and drops it when done.

use std::time::Duration;

use serde::Deserialize;

use super::CreatedServer;

pub const API: &str = "https://api.hetzner.cloud/v1";

/// x86 only: the artcraftCloud image is built for linux/amd64. Cheapest first. Hetzner often has
/// no capacity for the cost-optimized CX types, so we fall through to regular-performance ones.
pub const SERVER_TYPES: [&str; 4] = ["cx23", "cx33", "cpx22", "cpx32"];
/// EU locations only: Nuremberg, Falkenstein, Helsinki.
pub const LOCATIONS: [&str; 3] = ["nbg1", "fsn1", "hel1"];

pub struct Hetzner {
    base: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorBody,
}

#[derive(Deserialize)]
struct ApiErrorBody {
    code: String,
    message: String,
}

#[derive(Deserialize)]
struct ServerEnvelope {
    server: Server,
}

#[derive(Deserialize)]
struct Server {
    id: u64,
    status: String,
    public_net: PublicNet,
}

#[derive(Deserialize)]
struct PublicNet {
    ipv4: Option<Ipv4>,
}

#[derive(Deserialize)]
struct Ipv4 {
    ip: String,
}

/// Why a create attempt failed, so the caller knows whether trying elsewhere can help.
#[derive(Debug)]
enum Attempt {
    /// No capacity or this type is not offered here: try the next type or location.
    Unavailable(String),
    /// Bad token, no money on the account, a name clash...: stop and tell the user.
    Fatal(String),
}

impl Hetzner {
    pub fn new() -> Self {
        Self::with_base(API)
    }

    /// For tests against a local mock.
    pub fn with_base(base: &str) -> Self {
        Hetzner {
            base: base.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("artcraftcloud.com deploy wizard")
                .build()
                .expect("reqwest client"),
        }
    }

    /// Creates a server, trying each type and location until Hetzner has capacity.
    /// `location` pins one location instead of trying all of them.
    pub async fn create_server(
        &self,
        token: &str,
        name: &str,
        user_data: &str,
        location: Option<&str>,
    ) -> Result<CreatedServer, String> {
        let locations: Vec<&str> = match location {
            Some(l) => vec![l],
            None => LOCATIONS.to_vec(),
        };
        let mut last = String::from("no server type was available");
        for server_type in SERVER_TYPES {
            for loc in &locations {
                match self
                    .try_create(token, name, user_data, server_type, loc)
                    .await
                {
                    Ok(server) => return Ok(server),
                    Err(Attempt::Unavailable(why)) => {
                        tracing::info!(server_type, location = loc, %why, "hetzner: trying the next option");
                        last = why;
                    }
                    Err(Attempt::Fatal(why)) => return Err(why),
                }
            }
        }
        Err(format!("Hetzner has no capacity for a suitable server right now ({last}). Try again later or pick another host."))
    }

    async fn try_create(
        &self,
        token: &str,
        name: &str,
        user_data: &str,
        server_type: &str,
        location: &str,
    ) -> Result<CreatedServer, Attempt> {
        let body = serde_json::json!({
            "name": name,
            "server_type": server_type,
            "location": location,
            "image": "ubuntu-24.04",
            "user_data": user_data,
            "start_after_create": true,
            "labels": { "created-by": "artcraftcloud" },
            "public_net": { "enable_ipv4": true, "enable_ipv6": true },
        });
        let resp = self
            .http
            .post(format!("{}/servers", self.base))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Attempt::Fatal(format!("Could not reach Hetzner: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            let env: ServerEnvelope = serde_json::from_str(&text)
                .map_err(|e| Attempt::Fatal(format!("Unexpected reply from Hetzner: {e}")))?;
            return Ok(CreatedServer {
                id: env.server.id,
                ipv4: env.server.public_net.ipv4.map(|v| v.ip),
                server_type: server_type.to_string(),
                location: location.to_string(),
            });
        }
        let (code, message) = match serde_json::from_str::<ApiError>(&text) {
            Ok(e) => (e.error.code, e.error.message),
            Err(_) => (
                status.as_str().to_string(),
                text.chars().take(200).collect(),
            ),
        };
        Err(classify(status.as_u16(), &code, &message))
    }

    /// Current status ("initializing", "starting", "running", ...) and public IPv4.
    pub async fn server(&self, token: &str, id: u64) -> Result<(String, Option<String>), String> {
        let resp = self
            .http
            .get(format!("{}/servers/{id}", self.base))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| format!("Could not reach Hetzner: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "Hetzner answered {} while checking the server",
                resp.status()
            ));
        }
        let env: ServerEnvelope = resp
            .json()
            .await
            .map_err(|e| format!("Unexpected reply from Hetzner: {e}"))?;
        Ok((env.server.status, env.server.public_net.ipv4.map(|v| v.ip)))
    }
}

impl Default for Hetzner {
    fn default() -> Self {
        Self::new()
    }
}

fn classify(status: u16, code: &str, message: &str) -> Attempt {
    match (status, code) {
        (401, _) | (_, "unauthorized") => Attempt::Fatal(
            "Hetzner did not accept the API token. Create one with Read & Write access in your project (Security › API tokens).".into(),
        ),
        (403, _) | (_, "forbidden") => Attempt::Fatal(
            "The API token is read-only. Create one with Read & Write access.".into(),
        ),
        (_, "uniqueness_error") => Attempt::Fatal(format!("Hetzner: {message}")),
        (_, "resource_limit_exceeded") => Attempt::Fatal(format!(
            "Your Hetzner project hit a limit ({message}). New accounts may need to raise it in the console."
        )),
        (_, "resource_unavailable") | (_, "placement_error") | (_, "unsupported_location")
        | (_, "server_type_unavailable") | (_, "location_unavailable") => {
            Attempt::Unavailable(format!("{code}: {message}"))
        }
        (_, "invalid_input") if message.contains("server_type") || message.contains("location") => {
            Attempt::Unavailable(format!("{code}: {message}"))
        }
        (429, _) => Attempt::Fatal("Hetzner is rate limiting this token. Wait a minute and try again.".into()),
        _ => Attempt::Fatal(format!("Hetzner refused to create the server: {message} ({code})")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
    use std::sync::{Arc, Mutex};

    /// A fake Hetzner that has no capacity for the first `full` attempts.
    async fn mock(full: usize) -> (String, Arc<Mutex<Vec<serde_json::Value>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let app = Router::new()
            .route(
                "/servers",
                post(
                    |State((seen, full)): State<(Arc<Mutex<Vec<serde_json::Value>>>, usize)>,
                     headers: axum::http::HeaderMap,
                     Json(body): Json<serde_json::Value>| async move {
                        if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some("Bearer good") {
                            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": {"code": "unauthorized", "message": "unable to authenticate"}})));
                        }
                        let n = {
                            let mut s = seen.lock().unwrap();
                            s.push(body.clone());
                            s.len()
                        };
                        if n <= full {
                            return (StatusCode::PRECONDITION_FAILED, Json(serde_json::json!({"error": {"code": "resource_unavailable", "message": "no capacity"}})));
                        }
                        (StatusCode::CREATED, Json(serde_json::json!({
                            "server": {"id": 42, "status": "initializing", "public_net": {"ipv4": {"ip": "203.0.113.10"}}}
                        })))
                    },
                ),
            )
            .with_state((seen.clone(), full));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{addr}"), seen)
    }

    #[tokio::test]
    async fn falls_through_types_and_locations_until_one_has_capacity() {
        let (base, seen) = mock(4).await;
        let server = Hetzner::with_base(&base)
            .create_server("good", "acc-test", "#cloud-config\n", None)
            .await
            .unwrap();
        assert_eq!(server.id, 42);
        assert_eq!(server.ipv4.as_deref(), Some("203.0.113.10"));
        // 3 locations of cx23 failed, then cx33 in nbg1 failed, then cx33 in fsn1 worked.
        assert_eq!(
            (server.server_type.as_str(), server.location.as_str()),
            ("cx33", "fsn1")
        );
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 5);
        assert_eq!(seen[0]["image"], "ubuntu-24.04");
        assert_eq!(seen[0]["user_data"], "#cloud-config\n");
    }

    #[tokio::test]
    async fn a_bad_token_stops_at_once() {
        let (base, seen) = mock(0).await;
        let err = Hetzner::with_base(&base)
            .create_server("bad", "acc-test", "", None)
            .await
            .unwrap_err();
        assert!(err.contains("did not accept the API token"), "{err}");
        assert!(seen.lock().unwrap().is_empty());
    }
}
