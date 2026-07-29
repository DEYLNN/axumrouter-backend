use serde_json::Value;

use crate::types::chat::{ChatCompletionChunk, ChatCompletionResponse, Choice, Message, ToolCall};

// ── SSE chunk accumulator for tool calls ──

pub struct AccumulatingToolCall {
    pub id: String,
    pub fn_name: String,
    pub fn_args: String,
    pub tool_type: Option<String>,
}

impl AccumulatingToolCall {
    fn new() -> Self {
        Self { id: String::new(), fn_name: String::new(), fn_args: String::new(), tool_type: None }
    }
}

/// Reassemble SSE chunks into a single ChatCompletionResponse.
/// Handles tool-call accumulation, content null when tool_calls present,
/// and truncated tool_calls on finish_reason="length".
pub fn assemble_from_chunks(chunks: &[ChatCompletionChunk], model: &str) -> ChatCompletionResponse {
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut finish_reason = None;
    let mut usage = None;
    let mut resp_id = String::new();
    let mut resp_created = 0;
    let mut resp_model = String::new();
    let mut tool_calls_acc: Vec<AccumulatingToolCall> = Vec::new();

    for chunk in chunks {
        if resp_id.is_empty() && !chunk.id.is_empty() { resp_id = chunk.id.clone(); }
        if resp_created == 0 && chunk.created > 0 { resp_created = chunk.created; }
        if resp_model.is_empty() && !chunk.model.is_empty() { resp_model = chunk.model.clone(); }
        if let Some(u) = &chunk.usage { usage = Some(u.clone()); }
        for choice in &chunk.choices {
            if let Some(delta_content) = &choice.delta.content {
                if !delta_content.is_empty() { content.push_str(delta_content); }
            }
            if let Some(rc) = &choice.delta.reasoning_content {
                if !rc.is_empty() { reasoning.push_str(rc); }
            }
            if let Some(fr) = &choice.finish_reason {
                finish_reason = Some(fr.clone());
            }
            if let Some(tcs) = &choice.delta.tool_calls {
                for tc in tcs {
                    let idx = tc.index as usize;
                    if idx >= tool_calls_acc.len() {
                        tool_calls_acc.resize_with(idx + 1, AccumulatingToolCall::new);
                    }
                    let acc = &mut tool_calls_acc[idx];
                    if let Some(id) = &tc.id { if !id.is_empty() { acc.id = id.clone(); } }
                    if let Some(t) = &tc.type_ { acc.tool_type = Some(t.clone()); }
                    if let Some(fn_name) = tc.function.as_ref().and_then(|f| f.name.as_ref()) {
                        if !fn_name.is_empty() { acc.fn_name.push_str(fn_name); }
                    }
                    if let Some(fn_args) = tc.function.as_ref().and_then(|f| f.arguments.as_ref()) {
                        acc.fn_args.push_str(fn_args);
                    }
                }
            }
        }
    }

    let finish = finish_reason.unwrap_or_else(|| "stop".to_string());

    // If finish_reason is 'length' and we have tool_calls, the arguments might be
    // truncated mid-JSON. Drop tool_calls to prevent Hermes from executing incomplete ones.
    let has_tool_calls = !tool_calls_acc.is_empty() && tool_calls_acc.iter().any(|a| !a.id.is_empty());
    if finish == "length" && has_tool_calls {
        tool_calls_acc.clear();
    }

    // OpenAI spec: content must be null (not "") when tool_calls present.
    let content_str = if content.is_empty() && tool_calls_acc.iter().any(|a| !a.id.is_empty()) {
        None
    } else if content.is_empty() && tool_calls_acc.iter().all(|a| a.id.is_empty()) {
        Some(" ".to_string())
    } else {
        Some(content)
    };

    let mut message = Message {
        role: "assistant".to_string(),
        content: content_str,
        tool_calls: None,
        tool_call_id: None,
        name: None,
        reasoning_content: None,
    };

    if !tool_calls_acc.is_empty() {
        let calls: Vec<ToolCall> = tool_calls_acc.into_iter().filter(|a| !a.id.is_empty()).map(|a| ToolCall {
            id: a.id,
            function: crate::types::chat::ToolCallFunction {
                name: a.fn_name,
                arguments: a.fn_args,
            },
            type_: a.tool_type.unwrap_or_else(|| "function".to_string()),
        }).collect();
        if !calls.is_empty() {
            message.tool_calls = Some(calls);
        }
    }

    // Inject reasoning_content if present
    if !reasoning.is_empty() {
        message.reasoning_content = Some(reasoning);
    }

    ChatCompletionResponse {
        id: if resp_id.is_empty() { format!("chatcmpl-freebuff-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()) } else { resp_id },
        object: "chat.completion".to_string(),
        created: if resp_created == 0 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() } else { resp_created },
        model: if resp_model.is_empty() { model.to_string() } else { resp_model },
        choices: vec![Choice { index: 0, message, finish_reason: Some(finish) }],
        usage,
    }
}

/// Ensure assistant messages with tool_calls have a reasoning_content field.
/// FreeBuff free tier agents may require this placeholder.
pub fn ensure_reasoning_content(messages: &mut Value) {
    if let Some(arr) = messages.as_array_mut() {
        for msg in arr.iter_mut() {
            if msg["role"] != "assistant" { continue; }
            let has_tc = msg.get("tool_calls").and_then(|tc| tc.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
            if !has_tc { continue; }
            let has_rc = msg.get("reasoning_content").and_then(|rc| rc.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
            if !has_rc {
                msg["reasoning_content"] = Value::String(" ".to_string());
            }
        }
    }
}
