use serde_json::{json, Value};

use crate::types::chat::ChatCompletionRequest;

use super::constants;

/// Translate OpenAI Chat Completions request → OpenAI Responses API request.
///
/// Differences:
/// - `messages` becomes `input` (array of message items).
/// - `system`/`developer` become `instructions` (joined by blank line).
/// - `stream: true` is forced (Codex Responses API streams).
/// - Model id is mapped from `cx/<frontend>` → backend id from MODELS table.
pub struct CxMapper;

impl CxMapper {
    pub fn to_responses_request(&self, request: ChatCompletionRequest) -> Value {
        let model_id = request.model.trim_start_matches("cx/");
        let backend_model = constants::MODELS
            .iter()
            .find(|m| m.id == model_id)
            .map(|m| m.backend_model)
            .unwrap_or(model_id);

        let mut instructions = Vec::new();
        let mut input: Vec<Value> = Vec::new();

        for msg in request.messages {
            let role = msg.role.as_str();
            let content = msg.content.clone().unwrap_or_default();

            if role == "system" || role == "developer" {
                if !content.is_empty() {
                    instructions.push(content);
                }
                continue;
            }

            // Preserve `assistant` role + use `output_text` (output direction).
            // User/system/developer → `input_text` (input direction).
            let (mapped_role, content_type) = if role == "assistant" {
                ("assistant", "output_text")
            } else {
                ("user", "input_text")
            };
            input.push(json!({
                "type": "message",
                "role": mapped_role,
                "content": [{ "type": content_type, "text": content }]
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
            "model": backend_model,
            "input": input,
            "stream": true,
            "store": false,
            "reasoning": {
                "effort": request.reasoning_effort.as_deref().unwrap_or(constants::DEFAULT_REASONING_EFFORT),
                "summary": "auto",
            },
        });

        if !instructions.is_empty() {
            body["instructions"] = json!(instructions.join("\n\n"));
        }

        // Include reasoning encrypted content — required by the Codex backend
        // for reasoning models; without it no reasoning events are emitted.
        if body["reasoning"]["effort"].as_str() != Some("none") {
            body["include"] = json!(["reasoning.encrypted_content"]);
        }

        // Codex Responses API rejects these params — drop them (mirrors 9router).
        // max_output_tokens is the offending one (400 "Unsupported parameter").
        // temperature/top_p are also unsupported on this endpoint.

        // Forward tools — Codex Responses API uses FLAT shape:
        //   { type: "function", name, description, parameters }
        // (Chat Completions wraps in { function: { name, … } }). Codex backend
        // returns 422 "missing field `name`" without the reshape.
        if let Some(tools) = request.tools.as_ref() {
            let flat: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "name": t.function.name,
                        "description": t.function.description.clone().unwrap_or_default(),
                        "parameters": t.function.parameters.clone().unwrap_or_else(|| {
                            json!({ "type": "object", "properties": {} })
                        }),
                    })
                })
                .collect();
            if !flat.is_empty() {
                body["tools"] = Value::Array(flat);
            }
        }
        if let Some(tc) = request.tool_choice.as_ref() {
            body["tool_choice"] = tc.clone();
        }

        body
    }
}