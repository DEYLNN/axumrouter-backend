use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChatCompletionResponse, ToolCall, ToolCallFunction, Usage};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::Value;

use super::auth::CxOAuthCredential;
use super::constants;
use super::responses_parser::ResponsesStreamParser;

pub struct CxClient {
    http: Client,
    first_chunk_timeout: std::time::Duration,
    stall_timeout: std::time::Duration,
}

impl CxClient {
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

    /// Build the upstream request with all the codex_cli_rs wire-header quirks.
    fn build_request(&self, body: Value, cred: &CxOAuthCredential) -> reqwest::RequestBuilder {
        let mut b = self
            .http
            .post(constants::RESPONSES_URL)
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("originator", constants::ORIGINATOR)
            .header("User-Agent", constants::USER_AGENT)
            .header(
                "session_id",
                cred.email.as_deref().filter(|s| !s.is_empty()).unwrap_or("default"),
            )
            .json(&body);
        if let Some(account_id) = &cred.account_id {
            b = b.header("chatgpt-account-id", account_id);
        }
        b
    }

    /// Non-streaming POST. Codex Responses API always streams server-side;
    /// we send `stream: true` (per 9router + legacy) and aggregate the SSE
    /// into a single ChatCompletionResponse. The terminal chunk carries
    /// `usage` from `response.completed` — we propagate it so the dispatcher
    /// can record prompt/completion tokens.
    pub async fn send_collect(
        &self,
        body: Value,
        cred: &CxOAuthCredential,
    ) -> Result<ChatCompletionResponse, GatewayError> {
        let mut stream = self.send_stream(body, cred).await?;
        let mut out = String::new();
        let mut reasoning_buf = String::new();
        let mut usage: Option<Usage> = None;
        let mut finish_reason: Option<String> = None;
        // tool_calls accumulate: chunk_index → (id, name, args)
        let mut tool_calls: std::collections::BTreeMap<u32, (String, String, String)> =
            std::collections::BTreeMap::new();

        while let Some(item) = stream.next().await {
            let chunk = item?;
            for choice in chunk.choices {
                if let Some(c) = choice.delta.content {
                    out.push_str(&c);
                }
                if let Some(r) = choice.delta.reasoning_content {
                    reasoning_buf.push_str(&r);
                }
                if let Some(fr) = choice.finish_reason {
                    finish_reason = Some(fr);
                }
                if let Some(tcs) = choice.delta.tool_calls {
                    for tc in tcs {
                        let entry = tool_calls.entry(tc.index).or_insert_with(|| {
                            (String::new(), String::new(), String::new())
                        });
                        if let Some(id) = tc.id {
                            entry.0 = id;
                        }
                        if let Some(func) = tc.function {
                            if let Some(name) = func.name {
                                entry.1 = name;
                            }
                            if let Some(args) = func.arguments {
                                entry.2.push_str(&args);
                            }
                        }
                    }
                }
            }
            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
        }

        let reasoning_content = if reasoning_buf.is_empty() { None } else { Some(reasoning_buf) };
        let tool_calls_vec: Vec<ToolCall> = tool_calls
            .into_iter()
            .filter(|(_, (id, name, _))| !id.is_empty() && !name.is_empty())
            .map(|(_, (id, name, arguments))| ToolCall {
                id,
                type_: "function".to_string(),
                function: ToolCallFunction { name, arguments },
            })
            .collect();
        let tool_calls_opt = if tool_calls_vec.is_empty() { None } else { Some(tool_calls_vec) };

        Ok(ChatCompletionResponse {
            id: format!("chatcmpl-cx-{}", chrono::Utc::now().timestamp()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: "cx".to_string(),
            choices: vec![crate::types::chat::Choice {
                index: 0,
                message: crate::types::chat::Message {
                    role: "assistant".to_string(),
                    content: if out.is_empty() { None } else { Some(out) },
                    reasoning_content,
                    tool_calls: tool_calls_opt,
                    tool_call_id: None,
                    name: None,
                },
                finish_reason,
            }],
            usage,
        })
    }

    /// Streaming POST. Returns a stream of Chat Completions chunks
    /// (Responses API SSE events translated on the fly via ResponsesStreamParser).
    pub async fn send_stream(
        &self,
        body: Value,
        cred: &CxOAuthCredential,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.is_empty() {
            return Err(GatewayError::ProviderError("Codex: access_token missing".into()));
        }

        let response = self
            .build_request(body.clone(), cred)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("Codex HTTP: {e}")))?;

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

        let model = body
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("codex")
            .to_string();
        let mut upstream = response.bytes_stream();
        let ft = self.first_chunk_timeout;
        let st = self.stall_timeout;

        let stream = async_stream::try_stream! {
            let mut buf = String::new();
            let mut first = true;
            let mut parser = ResponsesStreamParser::new();
            loop {
                let wait = if first { ft } else { st };
                first = false;
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("Codex timeout: {}s", wait.as_secs())))?;
                let Some(bytes) = next else { break };
                let bytes = bytes.map_err(|e| GatewayError::ProviderError(format!("Codex read: {e}")))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(end) = buf.find("\n\n") {
                    let frame = buf[..end].to_string();
                    buf = buf[end + 2..].to_string();
                    for line in frame.lines() {
                        let Some(data) = line.trim().strip_prefix("data:") else { continue };
                        let data = data.trim();
                        if data.is_empty() || data == "[DONE]" { continue; }
                        match serde_json::from_str::<Value>(data) {
                            Ok(event) => {
                                if let Some(err) = event.get("error") {
                                    Err(GatewayError::ProviderError(format!("Codex stream error: {err}")))?;
                                }
                                for chunk in parser.process_event(&event, &model) {
                                    yield chunk;
                                }
                                // Terminal — stop reading after the completed event.
                                if event.get("type").and_then(|t| t.as_str())
                                    == Some("response.completed")
                                {
                                    return;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Codex SSE parse: {e}");
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