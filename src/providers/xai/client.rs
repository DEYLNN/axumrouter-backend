use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChunkChoice, ChunkToolCall, Delta, Choice, ChatCompletionResponse, Message, Usage};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::Value;

use super::auth::XaiOAuthCredential;
use super::constants;

pub struct XaiClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl XaiClient {
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

    fn headers(&self, builder: reqwest::RequestBuilder, cred: &XaiOAuthCredential) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", constants::USER_AGENT)
    }

    fn parse_tool_calls(delta: &Value) -> Option<Vec<ChunkToolCall>> {
        delta.get("tool_calls")
            .and_then(|tc| tc.as_array())
            .map(|arr| {
                arr.iter().map(|tc| ChunkToolCall {
                    index: tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32,
                    id: tc.get("id").and_then(|s| s.as_str()).map(|s| s.to_string()),
                    type_: tc.get("type").and_then(|s| s.as_str()).map(|s| s.to_string()),
                    function: tc.get("function").map(|f| crate::types::chat::ChunkToolCallFunction {
                        name: f.get("name").and_then(|s| s.as_str()).map(|s| s.to_string()),
                        arguments: f.get("arguments").and_then(|s| s.as_str()).map(|s| s.to_string()),
                    }),
                }).collect()
            })
    }

    pub async fn send_stream(&self, body: Value, cred: &XaiOAuthCredential) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("xAI access_token missing".into()));
        }

        let response = self.headers(self.http.post(constants::BASE_URL), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("xAI HTTP error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError { status, body: text, provider: constants::PROVIDER_ID.to_string() });
        }

        let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("grok").to_string();
        let mut upstream = response.bytes_stream();
        let first_chunk_timeout = self.first_chunk_timeout;
        let stall_timeout = self.stall_timeout;

        let parsed = async_stream::try_stream! {
            let mut buffer = String::new();
            let mut first = true;
            let mut collected_usage: Option<Usage> = None;
            loop {
                let wait = if first { first_chunk_timeout } else { stall_timeout };
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("xAI stream timeout: no chunk within {}s", wait.as_secs())))?;
                let Some(chunk_result) = next else { break; };
                first = false;
                let bytes = chunk_result.map_err(|e| GatewayError::ProviderError(format!("xAI stream read error: {}", e)))?;
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(frame_end) = buffer.find("\n\n") {
                    let frame = buffer[..frame_end].to_string();
                    buffer = buffer[frame_end + 2..].to_string();
                    for line in frame.lines() {
                        let Some(data) = line.trim().strip_prefix("data:") else { continue; };
                        let data = data.trim();
                        if data.is_empty() || data == "[DONE]" { continue; }
                        let Ok(v) = serde_json::from_str::<Value>(data) else { continue; };
                        if let Some(err) = v.get("error") {
                            Err(GatewayError::ProviderError(format!("xAI stream error: {}", err)))?;
                        }

                        // Capture final usage from stream_options
                        if let Some(u) = v.get("usage") {
                            collected_usage = Some(Usage {
                                prompt_tokens: u.get("prompt_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                                completion_tokens: u.get("completion_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                                total_tokens: u.get("total_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                            });
                        }

                        // OpenAI-compatible chunk
                        let choices = v.get("choices").and_then(|c| c.as_array()).cloned().unwrap_or_default();
                        if choices.is_empty() && collected_usage.is_some() {
                            // Final usage-only chunk (no choices) — yield once with usage
                            yield ChatCompletionChunk {
                                id: format!("chatcmpl-xai-{}", chrono::Utc::now().timestamp()),
                                object: "chat.completion.chunk".to_string(),
                                created: chrono::Utc::now().timestamp() as u64,
                                model: model.clone(),
                                choices: vec![],
                                usage: collected_usage.clone(),
                            };
                            continue;
                        }
                        for choice in choices {
                            let idx = choice.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
                            let delta = choice.get("delta").cloned().unwrap_or_default();
                            let content = delta.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                            let finish = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
                            let role = delta.get("role").and_then(|r| r.as_str()).map(|s| s.to_string());
                            let tool_calls = Self::parse_tool_calls(&delta);

                            // Skip only if absolutely nothing useful
                            if content.is_none() && finish.is_none() && role.is_none() && tool_calls.is_none() {
                                continue;
                            }

                            yield ChatCompletionChunk {
                                id: format!("chatcmpl-xai-{}", chrono::Utc::now().timestamp()),
                                object: "chat.completion.chunk".to_string(),
                                created: chrono::Utc::now().timestamp() as u64,
                                model: model.clone(),
                                choices: vec![ChunkChoice {
                                    index: idx,
                                    delta: Delta { role, content, tool_calls },
                                    finish_reason: finish,
                                }],
                                usage: collected_usage.clone(),
                            };
                        }
                    }
                }
            }
        };
        Ok(parsed.boxed())
    }

    pub async fn send_collect(&self, body: Value, cred: &XaiOAuthCredential) -> Result<ChatCompletionResponse, GatewayError> {
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
            id: format!("chatcmpl-xai-{}", chrono::Utc::now().timestamp()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: "xai".to_string(),
            choices: vec![Choice {
                index: 0,
                message: msg,
                finish_reason: last_finish.or(Some("stop".to_string())),
            }],
            usage: Some(Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
