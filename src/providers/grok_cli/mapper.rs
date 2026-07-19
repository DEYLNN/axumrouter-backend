use serde_json::{json, Value};

use crate::types::chat::ChatCompletionRequest;

pub struct GcliMapper;

impl GcliMapper {
    pub fn to_response_request(&self, request: ChatCompletionRequest) -> Value {
        let model = request.model.trim_start_matches("gcli/").trim_start_matches("gb/");

        let mut input_items: Vec<Value> = Vec::new();

        for msg in &request.messages {
            match msg.role.as_str() {
                "user" | "system" => {
                    let content = msg.content.as_deref().unwrap_or("");
                    input_items.push(json!({
                        "role": msg.role,
                        "content": [{"type": "input_text", "text": content}]
                    }));
                }
                "assistant" => {
                    // Tool calls dari assistant
                    if let Some(tcs) = &msg.tool_calls {
                        for tc in tcs {
                            let args = &tc.function.arguments;
                            input_items.push(json!({
                                "type": "function_call",
                                "call_id": tc.id,
                                "name": tc.function.name,
                                "arguments": if args.is_empty() { "{}" } else { args.as_str() },
                            }));
                        }
                    }
                    // Content dari assistant
                    if let Some(content) = &msg.content {
                        if !content.is_empty() {
                            input_items.push(json!({
                                "role": "assistant",
                                "content": [{"type": "output_text", "text": content}]
                            }));
                        }
                    }
                }
                "tool" => {
                    let content = msg.content.as_deref().unwrap_or("");
                    let call_id = msg.tool_call_id.as_deref().unwrap_or("");
                    input_items.push(json!({
                        "type": "function_call_output",
                        "call_id": call_id,
                        "output": content,
                    }));
                }
                _ => {}
            }
        }

        let mut body = json!({
            "model": model,
            "input": input_items,
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
        if let Some(tools) = &request.tools {
            let converted: Vec<Value> = tools.iter().map(|t| {
                let params = t.function.parameters.clone().unwrap_or(serde_json::json!({
                    "type": "object",
                    "properties": {}
                }));
                json!({
                    "type": "function",
                    "name": t.function.name,
                    "description": t.function.description.as_deref().unwrap_or(""),
                    "parameters": params,
                })
            }).collect();
            body["tools"] = json!(converted);
        }
        if let Some(tool_choice) = &request.tool_choice {
            // Only pass simple string values; skip OpenAI object format
            if tool_choice.is_string() {
                body["tool_choice"] = tool_choice.clone();
            }
        }

        body
    }
}
