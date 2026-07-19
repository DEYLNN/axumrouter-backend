use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChunkChoice, Delta, Choice, ChatCompletionResponse, Message, Usage};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};

use super::auth::GrokCliOAuthCredential;
use super::constants;

pub struct GcliClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl GcliClient {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            http: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .expect("Failed to build HTTP client"),
            first_chunk_timeout: std::time::Duration::from_secs(constants::STREAM_FIRST_CHUNK_TIMEOUT_SECS),
            stall_timeout: std::time::Duration::from_secs(constants::STREAM_STALL_TIMEOUT_SECS),
        }
    }

    fn headers(&self, builder: reqwest::RequestBuilder, cred: &GrokCliOAuthCredential) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", constants::USER_AGENT)
            .header("x-grok-client-identifier", constants::CLIENT_IDENTIFIER)
            .header("x-grok-client-version", constants::CLIENT_VERSION)
            .header("x-xai-token-auth", "xai-grok-cli")
    }

    /// POST to Responses API, parse SSE stream back to ChatCompletionChunks
    pub async fn send_stream(&self, body: Value, cred: &GrokCliOAuthCredential) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("grok-cli access_token missing".into()));
        }

        let response = self.headers(self.http.post(constants::BASE_URL), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("grok-cli HTTP error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError { status, body: text, provider: constants::PROVIDER_ID.to_string() });
        }

        let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("grok-build").to_string();
        let mut upstream = response.bytes_stream();
        let first_chunk_timeout = self.first_chunk_timeout;
        let stall_timeout = self.stall_timeout;

        let parsed = async_stream::try_stream! {
            let mut buffer = String::new();
            let mut first = true;
            loop {
                let wait = if first { first_chunk_timeout } else { stall_timeout };
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("grok-cli stream timeout: no chunk within {}s", wait.as_secs())))?;
                let Some(chunk_result) = next else { break; };
                first = false;
                let bytes = chunk_result.map_err(|e| GatewayError::ProviderError(format!("grok-cli stream read error: {}", e)))?;
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Parse SSE frames: data: {...}
                while let Some(frame_end) = buffer.find("\n\n") {
                    let frame = buffer[..frame_end].to_string();
                    buffer = buffer[frame_end + 2..].to_string();
                    for line in frame.lines() {
                        let line = line.trim();
                        if line.is_empty() || !line.starts_with("data:") { continue; }
                        let data = line.strip_prefix("data:").map(|s| s.trim()).unwrap_or("");
                        if data.is_empty() || data == "[DONE]" { continue; }

                        let Ok(v) = serde_json::from_str::<Value>(data) else { continue; };
                        if let Some(err) = v.get("error") {
                            Err(GatewayError::ProviderError(format!("grok-cli stream error: {}", err)))?;
                        }

                        // Responses API → ChatCompletionChunk
                        // Responses SSE events: response.output_text.delta, response.output_item.added, response.done
                        let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

                        match event_type {
                            "response.output_text.delta" => {
                                let content = v.get("delta").and_then(|d| d.as_str()).unwrap_or("");
                                yield ChatCompletionChunk {
                                    id: format!("chatcmpl-grok-{}", chrono::Utc::now().timestamp()),
                                    object: "chat.completion.chunk".to_string(),
                                    created: chrono::Utc::now().timestamp() as u64,
                                    model: model.clone(),
                                    choices: vec![ChunkChoice {
                                        index: 0,
                                        delta: Delta { role: None, content: Some(content.to_string()), reasoning_content: None, tool_calls: None },
                                        finish_reason: None,
                                    }],
                                    usage: None,
                                };
                            }
                            "response.done" => {
                                // Final event — collect usage from response
                                let resp = v.get("response");
                                let usage = resp.and_then(|r| r.get("usage")).map(|u| Usage {
                                    prompt_tokens: u.get("input_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                                    completion_tokens: u.get("output_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                                    total_tokens: 0,
                                });
                                yield ChatCompletionChunk {
                                    id: format!("chatcmpl-grok-{}", chrono::Utc::now().timestamp()),
                                    object: "chat.completion.chunk".to_string(),
                                    created: chrono::Utc::now().timestamp() as u64,
                                    model: model.clone(),
                                    choices: vec![ChunkChoice {
                                        index: 0,
                                        delta: Delta { role: Some("assistant".to_string()), content: None, reasoning_content: None, tool_calls: None },
                                        finish_reason: Some("stop".to_string()),
                                    }],
                                    usage,
                                };
                            }
                            _ => { /* skip other events */ }
                        }
                    }
                }
            }
        };
        Ok(parsed.boxed())
    }

    pub async fn send_collect(&self, body: Value, cred: &GrokCliOAuthCredential) -> Result<ChatCompletionResponse, GatewayError> {
        let mut stream = self.send_stream(body, cred).await?;
        let mut out = String::new();
        let mut last_finish = None;
        while let Some(item) = stream.next().await {
            let chunk = item?;
            for choice in chunk.choices {
                if let Some(content) = choice.delta.content {
                    out.push_str(&content);
                }
                if choice.finish_reason.is_some() {
                    last_finish = choice.finish_reason;
                }
            }
        }

        let msg = Message {
            role: "assistant".to_string(),
            content: if out.is_empty() { None } else { Some(out) },
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning_content: None,
        };

        Ok(ChatCompletionResponse {
            id: format!("chatcmpl-grok-{}", chrono::Utc::now().timestamp()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: "grok-cli".to_string(),
            choices: vec![Choice {
                index: 0,
                message: msg,
                finish_reason: last_finish.or(Some("stop".to_string())),
            }],
            usage: Some(Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
