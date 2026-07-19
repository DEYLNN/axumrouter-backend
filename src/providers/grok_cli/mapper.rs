use serde_json::{json, Value};

use crate::types::chat::ChatCompletionRequest;

pub struct GcliMapper;

impl GcliMapper {
    pub fn to_response_request(&self, request: ChatCompletionRequest) -> Value {
        // Convert OpenAI chat completion → OpenAI Responses API
        // Always stream=true — Grok Responses API is always SSE
        let mut body = json!({
            "model": request.model.trim_start_matches("gcli/").trim_start_matches("gb/"),
            "input": request.messages,
            "stream": true,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_output_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(tools) = request.tools {
            body["tools"] = json!(tools);
        }
        if let Some(tool_choice) = request.tool_choice {
            body["tool_choice"] = tool_choice;
        }

        // Grok-specific headers are in the client
        body
    }
}
