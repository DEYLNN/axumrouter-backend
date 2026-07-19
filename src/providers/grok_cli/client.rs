use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChunkChoice, Delta, ChunkToolCall, ChunkToolCallFunction, Choice, ChatCompletionResponse, Message, Usage};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

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
            // Track pending tool calls across SSE frames
            let mut pending_tc: HashMap<String, PendingToolCall> = HashMap::new();
            // Track known tool call IDs so we emit the "role + name" header once per ID
            let mut emitted_tc_id = false;

            loop {
                let wait = if first { first_chunk_timeout } else { stall_timeout };
                let next = tokio::time::timeout(wait, upstream.next()).await
                    .map_err(|_| GatewayError::ProviderError(format!("grok-cli stream timeout: no chunk within {}s", wait.as_secs())))?;
                let Some(chunk_result) = next else { break; };
                first = false;
                let bytes = chunk_result.map_err(|e| GatewayError::ProviderError(format!("grok-cli stream read error: {}", e)))?;
                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(frame_end) = buffer.find("\n\n") {
                    let frame = buffer[..frame_end].to_string();
                    buffer = buffer[frame_end + 2..].to_string();

                    let mut event_type = "";
                    let mut data_str = "";

                    for line in frame.lines() {
                        let line = line.trim();
                        if line.starts_with("event:") {
                            event_type = line.strip_prefix("event:").map(|s| s.trim()).unwrap_or("");
                        } else if line.starts_with("data:") {
                            data_str = line.strip_prefix("data:").map(|s| s.trim()).unwrap_or("");
                        }
                    }

                    if data_str.is_empty() || data_str == "[DONE]" { continue; }

                    let Ok(v) = serde_json::from_str::<Value>(data_str) else { continue; };
                    if let Some(err) = v.get("error") {
                        Err(GatewayError::ProviderError(format!("grok-cli stream error: {}", err)))?;
                    }

                    let ev = if !event_type.is_empty() { event_type } else { v.get("type").and_then(|t| t.as_str()).unwrap_or("") };

                    match ev {
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
                        "response.output_text.done" => {
                            let text = v.get("text").and_then(|d| d.as_str()).unwrap_or("");
                            yield ChatCompletionChunk {
                                id: format!("chatcmpl-grok-{}", chrono::Utc::now().timestamp()),
                                object: "chat.completion.chunk".to_string(),
                                created: chrono::Utc::now().timestamp() as u64,
                                model: model.clone(),
                                choices: vec![ChunkChoice {
                                    index: 0,
                                    delta: Delta { role: None, content: Some(text.to_string()), reasoning_content: None, tool_calls: None },
                                    finish_reason: None,
                                }],
                                usage: None,
                            };
                        }
                        "response.output_item.added" => {
                            // New item in output — could be text or function_call
                            if v.get("item").and_then(|i| i.get("type")).and_then(|t| t.as_str()) == Some("function_call") {
                                let id = v["item"].get("id").and_then(|s| s.as_str()).unwrap_or("fc_unknown").to_string();
                                let name = v["item"].get("name").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                pending_tc.insert(id.clone(), PendingToolCall { id, name, arguments: String::new() });
                                emitted_tc_id = false;
                            }
                        }
                        "response.function_call_arguments.delta" => {
                            let tid = v.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
                            let delta = v.get("delta").and_then(|s| s.as_str()).unwrap_or("");
                            if let Some(tc) = pending_tc.get_mut(&tid) {
                                tc.arguments.push_str(delta);
                            }
                            // Emit chunk with accumulating arguments
                            if let Some(tc) = pending_tc.get(&tid) {
                                let mut tcs = Vec::new();
                                if !emitted_tc_id {
                                    tcs.push(ChunkToolCall {
                                        index: 0,
                                        type_: Some("function".to_string()),
                                        id: Some(tc.id.clone()),
                                        function: Some(ChunkToolCallFunction {
                                            name: Some(tc.name.clone()),
                                            arguments: Some("".to_string()),
                                        }),
                                    });
                                }
                                tcs.push(ChunkToolCall {
                                    index: 0,
                                    type_: None,
                                    id: None,
                                    function: Some(ChunkToolCallFunction {
                                        name: None,
                                        arguments: Some(tc.arguments.clone()),
                                    }),
                                });
                                yield ChatCompletionChunk {
                                    id: format!("chatcmpl-grok-{}", chrono::Utc::now().timestamp()),
                                    object: "chat.completion.chunk".to_string(),
                                    created: chrono::Utc::now().timestamp() as u64,
                                    model: model.clone(),
                                    choices: vec![ChunkChoice {
                                        index: 0,
                                        delta: Delta { role: Some("assistant".to_string()), content: None, reasoning_content: None, tool_calls: Some(tcs) },
                                        finish_reason: None,
                                    }],
                                    usage: None,
                                };
                                emitted_tc_id = true;
                            }
                        }
                        "response.completed" | "response.done" => {
                            let resp = v.get("response").or(Some(&v));
                            let usage = resp.and_then(|r| r.get("usage")).map(|u| Usage {
                                prompt_tokens: u.get("input_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                                completion_tokens: u.get("output_tokens").and_then(|n| n.as_u64()).unwrap_or(0) as u32,
                                total_tokens: 0,
                            });

                            // Collect final output text from response.output array
                            let output_text: String = resp
                                .and_then(|r| r.get("output"))
                                .and_then(|o| o.as_array())
                                .map(|arr| {
                                    arr.iter().filter_map(|item| {
                                        item.get("type").and_then(|t| t.as_str()).and_then(|ty| {
                                            if ty == "function_call" { return None; }
                                            Some(())
                                        })?;
                                        let content = item.get("content").and_then(|c| c.as_array())?;
                                        Some(content.iter().filter_map(|part| {
                                            part.get("text").and_then(|t| t.as_str()).map(String::from)
                                        }).collect::<Vec<_>>().concat())
                                    }).collect::<Vec<_>>().concat()
                                })
                                .unwrap_or_default();

                            // Collect final tool calls from output array
                            let tool_calls: Vec<ChunkToolCall> = resp
                                .and_then(|r| r.get("output"))
                                .and_then(|o| o.as_array())
                                .map(|arr| {
                                    arr.iter().filter_map(|item| {
                                        if item.get("type").and_then(|t| t.as_str()) != Some("function_call") {
                                            return None;
                                        }
                                        let id = item.get("id").and_then(|s| s.as_str()).unwrap_or("fc_unknown").to_string();
                                        let name = item.get("name").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                        let args = item.get("arguments").and_then(|s| s.as_str()).unwrap_or("{}").to_string();
                                        Some(ChunkToolCall {
                                            index: 0,
                                            type_: Some("function".to_string()),
                                            id: Some(id),
                                            function: Some(ChunkToolCallFunction { name: Some(name), arguments: Some(args) }),
                                        })
                                    }).collect()
                                })
                                .unwrap_or_default();

                            let delta_has_tool = !tool_calls.is_empty();
                            let delta_content = if delta_has_tool { None } else { Some(output_text) };
                            let delta_role = if delta_has_tool { Some("assistant".to_string()) } else { None };

                            yield ChatCompletionChunk {
                                id: format!("chatcmpl-grok-{}", chrono::Utc::now().timestamp()),
                                object: "chat.completion.chunk".to_string(),
                                created: chrono::Utc::now().timestamp() as u64,
                                model: model.clone(),
                                choices: vec![ChunkChoice {
                                    index: 0,
                                    delta: Delta { role: delta_role, content: delta_content, reasoning_content: None, tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) } },
                                    finish_reason: Some("stop".to_string()),
                                }],
                                usage,
                            };
                        }
                        _ => { /* skip other events */ }
                    }
                }
            }

            // Drain any remaining pending tool calls
        };
        Ok(parsed.boxed())
    }

    pub async fn send_collect(&self, body: Value, cred: &GrokCliOAuthCredential) -> Result<ChatCompletionResponse, GatewayError> {
        let mut stream = self.send_stream(body, cred).await?;
        let mut out = String::new();
        let mut tool_calls = Vec::new();
        let mut last_finish = None;
        let mut usage = Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 };
        while let Some(item) = stream.next().await {
            let chunk = item?;
            for choice in chunk.choices {
                if let Some(content) = choice.delta.content {
                    out.push_str(&content);
                }
                if let Some(tcs) = choice.delta.tool_calls {
                    for tc in tcs {
                        // Accumulate tool calls — group by index
                        let idx = tc.index;
                        if idx as usize >= tool_calls.len() {
                            tool_calls.resize(idx as usize + 1, ToolCallAcc { id: None, name: None, arguments: String::new() });
                        }
                        if let Some(id) = tc.id { tool_calls[idx as usize].id = Some(id); }
                        if let Some(name) = tc.function.as_ref().and_then(|f| f.name.clone()) { tool_calls[idx as usize].name = Some(name); }
                        if let Some(args) = tc.function.as_ref().and_then(|f| f.arguments.clone()) { tool_calls[idx as usize].arguments.push_str(&args); }
                    }
                }
                if choice.finish_reason.is_some() {
                    last_finish = choice.finish_reason;
                }
            }
            if let Some(u) = chunk.usage {
                usage = u;
            }
        }

        let final_tool_calls: Vec<crate::types::chat::ToolCall> = tool_calls.iter().map(|tca| {
            crate::types::chat::ToolCall {
                id: tca.id.clone().unwrap_or_else(|| format!("call_{}", chrono::Utc::now().timestamp())),
                type_: "function".to_string(),
                function: crate::types::chat::ToolCallFunction {
                    name: tca.name.clone().unwrap_or_default(),
                    arguments: if tca.arguments.is_empty() { "{}".to_string() } else { tca.arguments.clone() },
                },
            }
        }).collect();

        let msg = Message {
            role: "assistant".to_string(),
            content: if out.is_empty() && final_tool_calls.is_empty() { None } else if out.is_empty() { None } else { Some(out) },
            tool_calls: if final_tool_calls.is_empty() { None } else { Some(final_tool_calls) },
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
            usage: Some(usage),
        })
    }
}

struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

struct ToolCallAcc {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl Clone for ToolCallAcc {
    fn clone(&self) -> Self {
        ToolCallAcc {
            id: self.id.clone(),
            name: self.name.clone(),
            arguments: self.arguments.clone(),
        }
    }
}
