//! Stateful parser for OpenAI Codex Responses API SSE → OpenAI Chat Completions chunks.
//!
//! Codex emits `response.output_item.added` (type=function_call) to OPEN a tool
//! call (carrying id + name + empty arguments), then incremental
//! `response.function_call_arguments.delta` events to stream the JSON args,
//! then `response.output_item.done` (type=function_call) to close.
//!
//! Per-output_index state lets parallel tool calls (e.g. agent emits 2 tool
//! calls in one turn) keep their tool_call.id + name attached to the right
//! chunk_index, matching the OpenAI Chat Completions streaming convention.

use crate::types::chat::{ChunkChoice, ChatCompletionChunk, ChunkToolCall, ChunkToolCallFunction, Delta};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
struct ToolCallState {
    id: String,
    name: String,
    chunk_index: u32,
    arguments_started: bool,
}

pub struct ResponsesStreamParser {
    /// Per-output_index tool call metadata captured from `output_item.added`.
    tool_state: HashMap<u32, ToolCallState>,
    /// Counter for chunk index assignment (preserves order across output_indexes).
    next_chunk_index: u32,
    /// True once we've emitted a `finish_reason: "tool_calls"` chunk in this
    /// turn. Prevents a duplicate `finish_reason: "stop"` from arriving on the
    /// same turn — OpenAI Chat Completions allows only ONE finish_reason per
    /// chunk, so a duplicate would race and confuse the FE aggregator.
    has_tool_call_finish: bool,
}

impl Default for ResponsesStreamParser {
    fn default() -> Self {
        Self {
            tool_state: HashMap::new(),
            next_chunk_index: 0,
            has_tool_call_finish: false,
        }
    }
}

impl ResponsesStreamParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process one Responses API SSE event. Returns 0..N Chat Completions
    /// chunks (some events emit zero — e.g. status updates).
    pub fn process_event(&mut self, event: &Value, model: &str) -> Vec<ChatCompletionChunk> {
        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let ts = now_secs_u64();
        let mut out: Vec<ChatCompletionChunk> = Vec::new();

        match event_type {
            // Visible output text — goes to the user.
            "response.output_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(|v| v.as_str()) {
                    out.push(text_chunk(model, ts, Some(delta.to_string()), None, None));
                }
            }
            // Per-step reasoning summary text.
            "response.reasoning_summary_text.delta" => {
                if let Some(delta) = event.get("delta").and_then(|v| v.as_str()) {
                    if !delta.is_empty() {
                        out.push(text_chunk(
                            model,
                            ts,
                            None,
                            Some(delta.to_string()),
                            None,
                        ));
                    }
                }
            }
            // Reasoning item opened — marker chunk so the dispatcher sees
            // something happen while reasoning is in progress.
            "response.output_item.added" => {
                if let Some(item) = event.get("item") {
                    let item_type = item.get("type").and_then(|v| v.as_str());
                    match item_type {
                        Some("function_call") => {
                            let output_index = event
                                .get("output_index")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;
                            let id = item
                                .get("id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let name = item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let chunk_index = self.next_chunk_index;
                            self.next_chunk_index += 1;
                            self.tool_state.insert(
                                output_index,
                                ToolCallState {
                                    id: id.clone(),
                                    name: name.clone(),
                                    chunk_index,
                                    arguments_started: false,
                                },
                            );
                            // Inline args (rare but possible if Codex emits them
                            // up-front without delta events).
                            let inline_args = item
                                .get("arguments")
                                .and_then(|v| v.as_str())
                                .map(String::from)
                                .unwrap_or_default();
                            out.push(tool_call_chunk(
                                model,
                                ts,
                                chunk_index,
                                Some(id),
                                Some(name),
                                if inline_args.is_empty() { None } else { Some(inline_args) },
                            ));
                        }
                        Some("reasoning") => {
                            // No-op chunk — reasoning summary deltas come next.
                        }
                        _ => {}
                    }
                }
            }
            // Argument streaming — chunked delta per token.
            "response.function_call_arguments.delta" => {
                let output_index = event
                    .get("output_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if let Some(delta) = event.get("delta").and_then(|v| v.as_str()) {
                    if !delta.is_empty() {
                        if let Some(state) = self.tool_state.get_mut(&output_index) {
                            state.arguments_started = true;
                        }
                        if let Some(state) = self.tool_state.get(&output_index).cloned() {
                            out.push(tool_call_chunk(
                                model,
                                ts,
                                state.chunk_index,
                                None,
                                None,
                                Some(delta.to_string()),
                            ));
                        }
                    }
                }
            }
            // Final argument flush — usually redundant with deltas; forward it.
            "response.function_call_arguments.done" => {
                let output_index = event
                    .get("output_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let already_streamed = self
                    .tool_state
                    .get(&output_index)
                    .map(|s| s.arguments_started)
                    .unwrap_or(false);
                if !already_streamed {
                    if let Some(arguments) =
                        event.get("arguments").and_then(|v| v.as_str())
                    {
                        if !arguments.is_empty() {
                            if let Some(state) =
                                self.tool_state.get(&output_index).cloned()
                            {
                                out.push(tool_call_chunk(
                                    model,
                                    ts,
                                    state.chunk_index,
                                    None,
                                    None,
                                    Some(arguments.to_string()),
                                ));
                            }
                        }
                    }
                }
            }
            // Tool call complete — emit finish_reason "tool_calls" so the FE
            // knows to execute the tool.
            "response.output_item.done" => {
                if let Some(item) = event.get("item") {
                    if item.get("type").and_then(|v| v.as_str())
                        == Some("function_call")
                    {
                        self.has_tool_call_finish = true;
                        out.push(finish_chunk(model, ts, Some("tool_calls")));
                    }
                }
            }
            // Terminal — usage + final finish_reason.
            "response.completed" | "response.incomplete" => {
                let finish = if self.has_tool_call_finish {
                    None // already emitted "tool_calls" — don't double-emit
                } else if event_type == "response.incomplete" {
                    Some("length".to_string())
                } else {
                    Some("stop".to_string())
                };
                let usage = parse_usage(event);
                out.push(text_chunk(model, ts, None, None, finish));
                if let Some(u) = usage {
                    out.push(usage_chunk(model, ts, u));
                }
                // Drop tool_state so a re-used parser (shouldn't happen but safe)
                // doesn't leak ids across turns.
                self.tool_state.clear();
                self.next_chunk_index = 0;
                self.has_tool_call_finish = false;
            }
            _ => {}
        }

        out
    }
}

fn text_chunk(
    model: &str,
    ts: u64,
    content: Option<String>,
    reasoning_content: Option<String>,
    finish_reason: Option<String>,
) -> ChatCompletionChunk {
    ChatCompletionChunk {
        id: chunk_id(),
        object: "chat.completion.chunk".to_string(),
        created: ts,
        model: model.to_string(),
        choices: vec![ChunkChoice {
            index: 0,
            delta: Delta {
                role: None,
                content,
                reasoning_content,
                tool_calls: None,
            },
            finish_reason,
        }],
        usage: None,
    }
}

fn tool_call_chunk(
    model: &str,
    ts: u64,
    chunk_index: u32,
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
) -> ChatCompletionChunk {
    let type_ = if id.is_some() { Some("function".to_string()) } else { None };
    ChatCompletionChunk {
        id: chunk_id(),
        object: "chat.completion.chunk".to_string(),
        created: ts,
        model: model.to_string(),
        choices: vec![ChunkChoice {
            index: 0,
            delta: Delta {
                role: None,
                content: None,
                reasoning_content: None,
                tool_calls: Some(vec![ChunkToolCall {
                    index: chunk_index,
                    id,
                    type_,
                    function: Some(ChunkToolCallFunction { name, arguments }),
                }]),
            },
            finish_reason: None,
        }],
        usage: None,
    }
}

fn finish_chunk(model: &str, ts: u64, finish: Option<&str>) -> ChatCompletionChunk {
    text_chunk(model, ts, None, None, finish.map(String::from))
}

fn usage_chunk(model: &str, ts: u64, usage: Value) -> ChatCompletionChunk {
    let prompt = usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let completion = usage
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let total = usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    ChatCompletionChunk {
        id: chunk_id(),
        object: "chat.completion.chunk".to_string(),
        created: ts,
        model: model.to_string(),
        choices: vec![],
        usage: Some(crate::types::chat::Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
        }),
    }
}

/// Parse Responses API `response.completed` → OpenAI Chat Completions `usage`.
fn parse_usage(event: &Value) -> Option<Value> {
    let usage = event.get("response")?.get("usage")?;
    let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let output = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
    let total = usage
        .get("total_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(input + output);
    let mut usage_obj = serde_json::json!({
        "prompt_tokens": input,
        "completion_tokens": output,
        "total_tokens": total,
    });
    if let Some(reasoning_tokens) = usage
        .get("output_tokens_details")
        .and_then(|d| d.get("reasoning_tokens"))
        .and_then(|v| v.as_u64())
    {
        usage_obj["completion_tokens_details"] = serde_json::json!({
            "reasoning_tokens": reasoning_tokens,
        });
    }
    Some(usage_obj)
}

fn chunk_id() -> String {
    format!("chatcmpl-cx-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0))
}

fn now_secs_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
