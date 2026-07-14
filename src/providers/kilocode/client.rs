use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChatCompletionResponse, Usage};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::Value;

use super::auth::KcOAuthCredential;
use super::constants;

pub struct KlClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl KlClient {
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(constants::DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("HTTP client"),
            first_chunk_timeout: std::time::Duration::from_secs(constants::STREAM_FIRST_CHUNK_TIMEOUT_SECS),
            stall_timeout: std::time::Duration::from_secs(constants::STREAM_STALL_TIMEOUT_SECS),
        }
    }

    fn headers(
        &self,
        builder: reqwest::RequestBuilder,
        cred: &KcOAuthCredential,
    ) -> reqwest::RequestBuilder {
        let b = builder
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json");
        if let Some(org_id) = &cred.org_id {
            b.header("X-Kilocode-OrganizationID", org_id)
        } else {
            b
        }
    }

    pub async fn send_collect(
        &self,
        body: Value,
        cred: &KcOAuthCredential,
    ) -> Result<ChatCompletionResponse, GatewayError> {
        if cred.access_token.is_empty() {
            return Err(GatewayError::ProviderError("KiloCode: access_token missing".into()));
        }
        let resp = self
            .headers(self.http.post(constants::CHAT_URL), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("KiloCode HTTP: {e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GatewayError::ProviderHttpError {
                status: status.as_u16(),
                body: text,
                provider: constants::PROVIDER_ID.to_string(),
            });
        }
        serde_json::from_str(&text).map_err(|e| {
            GatewayError::ProviderError(format!("KiloCode parse: {e} — body: {}", &text[..text.len().min(200)]))
        })
    }

    pub async fn send_stream(
        &self,
        body: Value,
        cred: &KcOAuthCredential,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.is_empty() {
            return Err(GatewayError::ProviderError("KiloCode: access_token missing".into()));
        }
        let response = self
            .headers(self.http.post(constants::CHAT_URL), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("KiloCode HTTP: {e}")))?;
        if !response.status().is_success() {
            let s = response.status().as_u16();
            let t = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError {
                status: s,
                body: t,
                provider: constants::PROVIDER_ID.to_string(),
            });
        }

        let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("model").to_string();
        let mut upstream = response.bytes_stream();
        let ft = self.first_chunk_timeout;
        let st = self.stall_timeout;

        let stream = async_stream::try_stream! {
            let mut buf = String::new();
            let mut first = true;
            loop {
                let wait = if first { ft } else { st };
                first = false;
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("KiloCode timeout: {}s", wait.as_secs())))?;
                let Some(bytes) = next else { break };
                let bytes = bytes.map_err(|e| GatewayError::ProviderError(format!("KiloCode read: {e}")))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(end) = buf.find("\n\n") {
                    let frame = buf[..end].to_string();
                    buf = buf[end + 2..].to_string();
                    for line in frame.lines() {
                        let Some(data) = line.trim().strip_prefix("data:") else { continue };
                        let data = data.trim();
                        if data == "[DONE]" { continue; }
                        match serde_json::from_str::<ChatCompletionChunk>(data) {
                            Ok(chunk) => yield chunk,
                            Err(e) => {
                                tracing::warn!("KiloCode SSE parse: {e}");
                                continue;
                            }
                        }
                    }
                }
            }
        };

        Ok(stream.boxed())
    }
}
