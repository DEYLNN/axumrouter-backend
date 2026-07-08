use std::sync::Arc;
use std::time::Duration;

use crate::error::GatewayError;
use crate::providers::spec::AuthHeader;
use crate::providers::openai_compat::auth::ApiKeyAuth;
use crate::providers::openai_compat::config::OpenAIConfig;
use crate::providers::openai_compat::types::{ChatRequest, ChatResponse, StreamChunk};

/// Generic HTTP client for OpenAI-compatible providers.
pub struct Client {
    config: Arc<OpenAIConfig>,
    http: reqwest::Client,
}

impl Client {
    pub fn new(config: Arc<OpenAIConfig>) -> Self {
        let timeout = Duration::from_secs(config.default_timeout_secs.max(30));
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("AxumRouter/0.1.0")
            .build()
            .expect("reqwest Client::new");
        Self { config, http }
    }

    fn add_auth_header(
        &self,
        headers: &mut reqwest::header::HeaderMap,
        auth: &ApiKeyAuth,
    ) {
        use reqwest::header::HeaderValue;
        match self.config.quirks.auth_header {
            AuthHeader::Bearer => {
                let val = format!("Bearer {}", auth.api_key());
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    HeaderValue::from_str(&val).expect("Bearer token should be valid header value"),
                );
            }
            AuthHeader::XApiKey => {
                headers.insert(
                    "x-api-key",
                    HeaderValue::from_str(auth.api_key())
                        .expect("API key should be valid header value"),
                );
            }
        }
    }

    pub async fn chat_non_streaming(
        &self,
        auth: &ApiKeyAuth,
        request: &ChatRequest,
    ) -> Result<ChatResponse, GatewayError> {
        let mut headers = reqwest::header::HeaderMap::new();
        self.add_auth_header(&mut headers, auth);
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .json(request)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("HTTP request failed: {}", e)))?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| {
            GatewayError::ProviderError(format!("Failed to read response body: {}", e))
        })?;

        if !status.is_success() {
            return Err(GatewayError::ProviderError(format!(
                "Provider returned HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }

        serde_json::from_str::<ChatResponse>(&body).map_err(|e| {
            GatewayError::ProviderError(format!(
                "Failed to parse chat response: {} - body: {}",
                e,
                &body[..body.len().min(200)]
            ))
        })
    }

    /// Send streaming chat request, returns a stream of SSE bytes.
    pub async fn chat_stream(
        &self,
        auth: &ApiKeyAuth,
        request: &ChatRequest,
    ) -> Result<reqwest::Response, GatewayError> {
        let mut headers = reqwest::header::HeaderMap::new();
        self.add_auth_header(&mut headers, auth);
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let req = self
            .http
            .post(&url)
            .headers(headers)
            .json(request);

        let resp = req
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("HTTP request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!(
                "Provider returned HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }

        Ok(resp)
    }

    pub async fn validate_auth(&self, auth: &ApiKeyAuth) -> Result<(), GatewayError> {
        let mut headers = reqwest::header::HeaderMap::new();
        self.add_auth_header(&mut headers, auth);

        let resp = self
            .http
            .get(self.config.validate_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                GatewayError::ProviderError(format!("Auth validation request failed: {}", e))
            })?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            Err(GatewayError::ProviderError(format!(
                "Auth validation failed (HTTP {}): {}",
                status, body
            )))
        }
    }
}
