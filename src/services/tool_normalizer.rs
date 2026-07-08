use crate::types::chat::Message;

fn sanitize_tool_id(id: &str) -> Option<String> {
    let sanitized: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if sanitized.is_empty() { None } else { Some(sanitized) }
}

fn generated_tool_id(msg_index: usize, tool_index: usize, name: Option<&str>) -> String {
    let clean_name = name
        .and_then(sanitize_tool_id)
        .map(|n| format!("_{}", n))
        .unwrap_or_default();
    format!("call_msg{}_tc{}{}", msg_index, tool_index, clean_name)
}

/// Normalize OpenAI-style tool call history before forwarding upstream.
/// Mirrors 9router toolCall concern: IDs must exist/sanitize, assistant content must be None when only tool_calls.
pub fn normalize_tool_messages(messages: &mut Vec<Message>) {
    for (i, msg) in messages.iter_mut().enumerate() {
        if msg.role == "assistant" {
            if msg.tool_calls.is_some() && msg.content.as_deref().map(|s| s.is_empty()).unwrap_or(true) {
                msg.content = None;
            }

            if let Some(tcs) = &mut msg.tool_calls {
                for (j, tc) in tcs.iter_mut().enumerate() {
                    let valid_id = sanitize_tool_id(&tc.id);
                    tc.id = valid_id.unwrap_or_else(|| generated_tool_id(i, j, Some(&tc.function.name)));
                }
            }
        }

        if msg.role == "tool" {
            let current = msg.tool_call_id.as_deref().unwrap_or("");
            msg.tool_call_id = sanitize_tool_id(current).or_else(|| Some(generated_tool_id(i, 0, None)));
        }
    }
}
