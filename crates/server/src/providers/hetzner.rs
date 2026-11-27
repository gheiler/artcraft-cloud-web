pub struct HetznerClient {
    api_token: String,
    http_client: reqwest::Client,
}

impl HetznerClient {
    pub fn new(api_token: String) -> Self {
        Self {
            api_token,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn create_server(&self, name: &str, cloud_init: &str) -> Result<String, String> {
        let url = "https://api.hetzner.cloud/v1/servers";
        
        let payload = serde_json::json!({
            "name": name,
            "server_type": "cx23",
            "image": "ubuntu-24.04",
            "user_data": cloud_init,
            "start_after_create": true
        });

        let response = self.http_client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
            Ok(json["server"]["id"].to_string())
        } else {
            Err(format!("Hetzner API error: {}", response.status()))
        }
    }
}
