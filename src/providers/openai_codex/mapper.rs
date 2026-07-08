use serde_json::{json, Value};

use crate::types::chat::ChatCompletionRequest;

pub struct CxMapper;

impl CxMapper {
    pub fn to_responses_request(&self, request: ChatCompletionRequest) -> Value {
        let mut instructions = Vec::new();
        let mut input = Vec::new();

        for msg in request.messages {
            let role = msg.role.as_str();
            let content = msg.content.unwrap_or_default();
            if role == "system" || role == "developer" {
                if !content.is_empty() {
                    instructions.push(content);
                }
                continue;
            }
            let mapped_role = if role == "assistant" { "assistant" } else { "user" };
            input.push(json!({
                "type": "message",
                "role": mapped_role,
                "content": [{ "type": "input_text", "text": content }]
            }));
        }

        if input.is_empty() {
            input.push(json!({
                "type": "message",
                "role": "user",
                "content": [{ "type": "input_text", "text": "..." }]
            }));
        }

        let mut body = json!({
            "model": request.model,
            "input": input,
            "stream": true,
            "store": false
        });

        if !instructions.is_empty() {
            body["instructions"] = json!(instructions.join("\n\n"));
        }
        body
    }
}
