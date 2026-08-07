use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChatCompletionResponse};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::Value;

use super::auth::GbOAuthCredential;
use super::constants;
use super::mapper::{self, ResponsesStreamParser};

pub struct GbClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl GbClient {
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

    /// Build the upstream request with all the wire-specific headers.
    fn build_request(
        &self,
        body: Value,
        cred: &GbOAuthCredential,
    ) -> reqwest::RequestBuilder {
        self.http
            .post(constants::RESPONSES_URL)
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("User-Agent", constants::GROK_CLI_USER_AGENT)
            .header("x-grok-client-identifier", constants::GROK_CLI_CLIENT_IDENTIFIER)
            .header("x-grok-client-version", constants::GROK_CLI_VERSION)
            .json(&body)
    }

    /// Non-streaming POST. Returns the full Responses API JSON.
    /// Caller maps it to ChatCompletionResponse.
    pub async fn send_collect(
        &self,
        body: Value,
        cred: &GbOAuthCredential,
    ) -> Result<Value, GatewayError> {
        if cred.access_token.is_empty() {
            return Err(GatewayError::ProviderError("GrokBuild: access_token missing".into()));
        }
        // Force stream=false for collect path.
        let mut body = body;
        body["stream"] = serde_json::json!(false);

        let resp = self
            .build_request(body, cred)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("GrokBuild HTTP: {e}")))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GatewayError::ProviderHttpError {
                status: status.as_u16(),
                body: text,
                provider: constants::PROVIDER_ID.to_string(),
                key_id: None,
            });
        }
        serde_json::from_str(&text).map_err(|e| {
            GatewayError::ProviderError(format!(
                "GrokBuild parse: {e} — body: {}",
                &text[..text.len().min(200)]
            ))
        })
    }

    /// Build a ChatCompletions-shaped value from the Responses output.
    /// Used by send_collect to give callers a familiar response shape.
    pub fn collect_to_chat_response(&self, upstream: &Value) -> ChatCompletionResponse {
        let text = extract_output_text(upstream);
        ChatCompletionResponse {
            id: upstream
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("gb")
                .to_string(),
            object: "chat.completion".to_string(),
            created: now_secs_u64(),
            model: upstream
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("grok-4.5")
                .to_string(),
            choices: vec![crate::types::chat::Choice {
                index: 0,
                message: crate::types::chat::Message {
                    role: "assistant".to_string(),
                    content: Some(text),
                    reasoning_content: None,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        }
    }

    /// Streaming POST. Returns a stream of Chat Completions chunks
    /// (Responses API SSE events translated on the fly).
    pub async fn send_stream(
        &self,
        body: Value,
        cred: &GbOAuthCredential,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.is_empty() {
            return Err(GatewayError::ProviderError("GrokBuild: access_token missing".into()));
        }

        let response = self
            .build_request(body, cred)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("GrokBuild HTTP: {e}")))?;

        if !response.status().is_success() {
            let s = response.status().as_u16();
            let t = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError {
                status: s,
                body: t,
                provider: constants::PROVIDER_ID.to_string(),
                key_id: None,
            });
        }

        let mut upstream = response.bytes_stream();
        let ft = self.first_chunk_timeout;
        let st = self.stall_timeout;

        // Stateful parser — survives across SSE frames so function_call
        // events can be correlated with the right tool_call index.
        let parser = ResponsesStreamParser::new();

        let stream = async_stream::try_stream! {
            let mut buf = String::new();
            let mut first = true;
            let mut parser = parser;
            loop {
                let wait = if first { ft } else { st };
                first = false;
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("GrokBuild timeout: {}s", wait.as_secs())))?;
                let Some(bytes) = next else { break };
                let bytes = bytes.map_err(|e| GatewayError::ProviderError(format!("GrokBuild read: {e}")))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(end) = buf.find("\n\n") {
                    let frame = buf[..end].to_string();
                    buf = buf[end + 2..].to_string();
                    for line in frame.lines() {
                        let Some(data) = line.trim().strip_prefix("data:") else { continue };
                        let data = data.trim();
                        if data.is_empty() || data == "[DONE]" { continue };
                        match serde_json::from_str::<Value>(data) {
                            Ok(event) => {
                                // Some events emit zero or more chunks.
                                for chunk in parser.process_event(&event) {
                                    match serde_json::from_value::<ChatCompletionChunk>(chunk) {
                                        Ok(c) => yield c,
                                        Err(e) => {
                                            tracing::warn!("GrokBuild chunk parse: {e}");
                                            continue;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("GrokBuild SSE parse: {e}");
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

fn extract_output_text(upstream: &Value) -> String {
    // Responses API: output is an array of items, the message item has
    // `content: [{type: "output_text", text: "..."}]`.
    if let Some(output) = upstream.get("output").and_then(|v| v.as_array()) {
        let mut text = String::new();
        for item in output {
            if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                for c in content {
                    if c.get("type").and_then(|v| v.as_str()) == Some("output_text") {
                        if let Some(t) = c.get("text").and_then(|v| v.as_str()) {
                            text.push_str(t);
                        }
                    }
                }
            }
        }
        return text;
    }
    String::new()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn now_secs_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
