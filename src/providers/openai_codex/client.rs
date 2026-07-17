use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChunkChoice, Delta, Choice, ChatCompletionResponse, Message, Usage};
use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::Client;
use serde_json::Value;

use super::auth::CxOAuthCredential;
use super::constants;

pub struct CxClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl CxClient {
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

    fn headers(&self, builder: reqwest::RequestBuilder, cred: &CxOAuthCredential) -> reqwest::RequestBuilder {
        let mut b = builder
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("originator", "codex_cli_rs")
            .header("User-Agent", constants::USER_AGENT)
            .header("session_id", if cred.email.is_empty() { "default" } else { cred.email.as_str() });
        if let Some(account_id) = cred.account_id() {
            b = b.header("chatgpt-account-id", account_id);
        }
        b
    }

    pub async fn send_stream(&self, body: Value, cred: &CxOAuthCredential) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("Codex access_token missing; refresh flow not available yet".into()));
        }

        let response = self.headers(self.http.post(constants::BASE_URL), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("Codex HTTP error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderHttpError { status, body: text, provider: constants::PROVIDER_ID.to_string() });
        }

        let model = body.get("model").and_then(|v| v.as_str()).unwrap_or("codex").to_string();
        let mut upstream = response.bytes_stream();
        let first_chunk_timeout = self.first_chunk_timeout;
        let stall_timeout = self.stall_timeout;

        let parsed = async_stream::try_stream! {
            let mut buffer = String::new();
            let mut first = true;
            loop {
                let wait = if first { first_chunk_timeout } else { stall_timeout };
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("Codex stream timeout: no chunk within {}s", wait.as_secs())))?;
                let Some(chunk_result) = next else { break; };
                first = false;
                let bytes = chunk_result.map_err(|e| GatewayError::ProviderError(format!("Codex stream read error: {}", e)))?;
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
                            Err(GatewayError::ProviderError(format!("Codex stream error: {}", err)))?;
                        }
                        if let Some(text) = extract_delta_text(&v) {
                            if text.is_empty() { continue; }
                            yield chunk(&model, Some(text), None);
                        }
                        if is_done(&v) {
                            yield chunk(&model, None, Some("stop".to_string()));
                        }
                    }
                }
            }
        };
        Ok(parsed.boxed())
    }

    pub async fn send_collect(&self, body: Value, cred: &CxOAuthCredential) -> Result<ChatCompletionResponse, GatewayError> {
        let mut stream = self.send_stream(body, cred).await?;
        let mut out = String::new();
        while let Some(item) = stream.next().await {
            let chunk = item?;
            for choice in chunk.choices {
                if let Some(content) = choice.delta.content {
                    out.push_str(&content);
                }
            }
        }
        Ok(ChatCompletionResponse {
            id: format!("chatcmpl-cx-{}", chrono::Utc::now().timestamp()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: "cx".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message { role: "assistant".to_string(), content: Some(out), tool_calls: None, tool_call_id: None, name: None, reasoning_content: None },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }),
        })
    }
}

fn chunk(model: &str, content: Option<String>, finish_reason: Option<String>) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: format!("chatcmpl-cx-{}", chrono::Utc::now().timestamp()),
        object: "chat.completion.chunk".to_string(),
        created: chrono::Utc::now().timestamp() as u64,
        model: model.to_string(),
        choices: vec![ChunkChoice { index: 0, delta: Delta { role: None, content, reasoning_content: None, tool_calls: None }, finish_reason }],
        usage: None,
    }
}

fn extract_delta_text(v: &Value) -> Option<String> {
    let typ = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
    if typ.contains("output_text.delta") || typ.contains("response.output_text.delta") {
        return v.get("delta").and_then(|x| x.as_str()).map(|s| s.to_string());
    }
    if typ.contains("message.delta") || typ.contains("content.delta") {
        return v.get("delta").and_then(|x| x.as_str()).map(|s| s.to_string());
    }
    None
}

fn is_done(v: &Value) -> bool {
    let typ = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
    typ == "response.completed" || typ == "response.done"
}
