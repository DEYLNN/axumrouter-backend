use serde_json::{json, Value};

use crate::types::chat::ChatCompletionRequest;

use super::constants;

pub struct KlMapper;

impl KlMapper {
    /// Map our model id (kc/nvidia-nemotron-...) to KC's backend model (nvidia/nemotron-...:free)
    pub fn to_chat_request(&self, request: ChatCompletionRequest) -> Value {
        let model_id = request.model.trim_start_matches("kc/");
        let backend_model = constants::MODELS
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.backend_model)
            .unwrap_or(model_id);
        let mut body = json!({
            "model": backend_model,
            "messages": request.messages,
            "stream": request.stream.unwrap_or(true),
        });
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
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
        if let Some(stream_options) = request.stream_options {
            body["stream_options"] = stream_options;
        }
        body
    }
}
