use crate::types::chat::ChatCompletionRequest;

/// Rough token estimate fallback when provider does not return usage.
/// Mirrors 9router fallback principle: provider usage first, estimate only as backup.
pub fn estimate_tokens_from_chars(chars: usize) -> i64 {
    ((chars as f64) / 4.0).ceil().max(1.0) as i64
}

pub fn estimate_prompt_tokens(request: &ChatCompletionRequest) -> i64 {
    let mut chars = request.model.len();
    for msg in &request.messages {
        chars += msg.role.len();
        if let Some(content) = &msg.content {
            chars += content.len();
        }
        if let Some(tool_calls) = &msg.tool_calls {
            chars += serde_json::to_string(tool_calls).map(|s| s.len()).unwrap_or(0);
        }
        if let Some(tool_call_id) = &msg.tool_call_id {
            chars += tool_call_id.len();
        }
    }
    if let Some(tools) = &request.tools {
        chars += serde_json::to_string(tools).map(|s| s.len()).unwrap_or(0);
    }
    estimate_tokens_from_chars(chars)
}
