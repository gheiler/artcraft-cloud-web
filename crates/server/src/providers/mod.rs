//! Clients for the hosts we can provision with an API token.
pub mod hetzner;

/// A freshly created server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedServer {
    pub id: u64,
    pub ipv4: Option<String>,
    pub server_type: String,
    pub location: String,
}
