// Nous Portal HTTP client for inference requests
use std::time::Duration;

use super::constants;

pub struct NpClient {
    http: reqwest::Client,
}

impl NpClient {
    pub fn new(timeout_secs: u64) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    pub async fn chat(&self, token: &str, body: serde_json::Value) -> Result<reqwest::Response, String> {
        let url = format!("{}/chat/completions", constants::INFERENCE_URL);
        self.http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json, text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP: {}", e))
    }

    pub async fn list_models(&self, token: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/models", constants::INFERENCE_URL);
        let resp = self.http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("HTTP: {}", e))?;
        resp.json().await.map_err(|e| format!("Parse: {}", e))
    }
}
