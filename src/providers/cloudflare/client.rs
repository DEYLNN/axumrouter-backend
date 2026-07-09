use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, Delta, ChunkChoice, Choice, ChatCompletionResponse, Message, Usage};

use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::Value;

use super::auth::CfCredential;
use super::constants;

pub struct CfClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl CfClient {
    pub fn new() -> Self {
        Self {
            http: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(constants::DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to build HTTP client"),
            first_chunk_timeout: std::time::Duration::from_secs(constants::STREAM_FIRST_CHUNK_TIMEOUT_SECS),
            stall_timeout: std::time::Duration::from_secs(constants::STREAM_STALL_TIMEOUT_SECS),
        }
    }

    fn build_url(&self, cred: &CfCredential) -> String {
        let account_id = cred.effective_account_id();
        if account_id.is_empty() {
            // No accountId — use direct URL (legacy key format)
            return "https://api.cloudflare.com/client/v4/accounts/UNKNOWN/ai/v1/chat/completions".into();
        }
        format!("https://api.cloudflare.com/client/v4/accounts/{}/ai/v1/chat/completions", account_id)
    }

    fn headers(&self, builder: reqwest::RequestBuilder, cred: &CfCredential) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", cred.effective_api_key()))
            .header("Content-Type", "application/json")
            .header("User-Agent", constants::USER_AGENT)
    }

    fn parse_chunk(v: &Value, model: &str, usage: &mut Option<Usage>) -> Option<ChatCompletionChunk> {
        let choices = v.get("choices").and_then(|c| c.as_array()).cloned().unwrap_or_default();
        if choices.is_empty() {
            if let Some(u) = v.get("usage") {
                *usage = Some(Usage {
                    prompt_tokens: u.get("prompt_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                    completion_tokens: u.get("completion_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                    total_tokens: u.get("total_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                });
            }
            return None;
        }
        let choice = &choices[0];
        let idx = choice.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as u32;
        let delta = choice.get("delta").cloned().unwrap_or_default();
        let content = delta.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
        let finish = choice.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
        if content.is_none() && finish.is_none() { return None; }

        Some(ChatCompletionChunk {
            id: format!("chatcmpl-cf-{}", chrono::Utc::now().timestamp()),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: model.to_string(),
            choices: vec![ChunkChoice {
                index: idx,
                delta: Delta { role: None, content, tool_calls: None },
                finish_reason: finish,
            }],
            usage: usage.clone(),
        })
    }

    pub async fn send_stream(&self, body: Value, cred: &CfCredential) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        let url = self.build_url(cred);
        let response = self.headers(self.http.post(&url), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("CF HTTP: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError { status, body: text, provider: "cf".into() });
        }

        let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("@cf/meta/llama-3.1-8b-instruct-fp8-fast").to_string();
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
                    .map_err(|_| GatewayError::ProviderError(format!("CF stream timeout: {}s", wait.as_secs())))?;
                let Some(chunk_result) = next else { break; };
                first = false;
                let bytes = chunk_result.map_err(|e| GatewayError::ProviderError(format!("CF stream read: {}", e)))?;
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(frame_end) = buffer.find("\n\n") {
                    let frame = buffer[..frame_end].to_string();
                    buffer = buffer[frame_end + 2..].to_string();
                    for line in frame.lines() {
                        let Some(data) = line.trim().strip_prefix("data:") else { continue; };
                        let data = data.trim();
                        if data.is_empty() || data == "[DONE]" { continue; }
                        if let Ok(v) = serde_json::from_str::<Value>(data) {
                            if let Some(chunk) = Self::parse_chunk(&v, &model, &mut collected_usage) {
                                yield chunk;
                            }
                        }
                    }
                }
            }
        };
        Ok(parsed.boxed())
    }

    pub async fn send_collect(&self, body: Value, cred: &CfCredential) -> Result<ChatCompletionResponse, GatewayError> {
        let url = self.build_url(cred);
        let response = self.headers(self.http.post(&url), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("CF HTTP: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError { status, body: text, provider: "cf".into() });
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| GatewayError::ProviderError(format!("CF parse: {}", e)))?;

        // Extract content from Cloudflare response
        let content = json.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let finish = json.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("finish_reason"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(ChatCompletionResponse {
            id: json.get("id").and_then(|v| v.as_str()).unwrap_or("cf-unknown").to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: body.get("model").and_then(|v| v.as_str()).unwrap_or("cf").to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content,
                    tool_calls: None,
                    tool_call_id: None,
                    name: None,
                },
                finish_reason: finish.or(Some("stop".to_string())),
            }],
            usage: Some(Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}
