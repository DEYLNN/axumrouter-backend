use serde_json::{json, Value};

use crate::types::chat::ChatCompletionRequest;

pub struct GcliMapper;

// Server-generated item id prefixes that /responses cannot resolve when store=false
const SERVER_ID_PATTERN: &[&str] = &["rs_", "fc_", "resp_", "msg_"];

// Hosted tool types executed server-side by Grok CLI backend
const HOSTED_TOOL_TYPES: &[&str] = &[
    "web_search", "x_search", "web_search_preview", "file_search",
    "image_generation", "code_interpreter", "mcp", "local_shell",
];

// Fields accepted by cli-chat-proxy Responses API
const RESPONSES_API_ALLOWLIST: &[&str] = &[
    "model", "input", "instructions", "tools", "tool_choice", "stream",
    "store", "reasoning", "include", "temperature", "top_p", "max_output_tokens",
    "parallel_tool_calls", "text", "metadata", "prompt_cache_key",
];

impl GcliMapper {
    fn is_server_id(id: &str) -> bool {
        SERVER_ID_PATTERN.iter().any(|prefix| id.starts_with(prefix))
    }

    fn is_native_item_id(id: &str) -> bool {
        // Native format: rs|msg|fc_<uuid>
        if id.len() < 40 { return false; }
        let parts: Vec<&str> = id.split('_').collect();
        if parts.len() != 2 { return false; }
        let prefix = parts[0];
        if !["rs", "msg", "fc"].contains(&prefix) { return false; }
        let uuid_part = parts[1];
        uuid_part.len() == 36 && uuid_part.chars().filter(|c| *c == '-').count() == 4
    }

    fn normalize_input_item(item: &Value) -> Option<Value> {
        let obj = item.as_object()?;
        let item_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");

        // Reasoning: keep only if native ID + encrypted_content
        if item_type == "reasoning" {
            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let encrypted = obj.get("encrypted_content").and_then(|v| v.as_str()).unwrap_or("");
            if !Self::is_native_item_id(id) || encrypted.is_empty() {
                return None;
            }
            return Some(item.clone());
        }

        // custom_tool_call → function_call
        if item_type == "custom_tool_call" {
            let call_id = obj.get("call_id").and_then(|v| v.as_str())
                .or_else(|| obj.get("id").and_then(|v| v.as_str()))
                .unwrap_or("");
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if call_id.is_empty() || name.is_empty() { return None; }
            let input_val = obj.get("input").or_else(|| obj.get("arguments")).cloned().unwrap_or(json!(""));
            let args_str = if input_val.is_string() {
                input_val.as_str().unwrap_or("").to_string()
            } else {
                serde_json::to_string(&input_val).unwrap_or_else(|_| "{}".to_string())
            };
            return Some(json!({
                "type": "function_call",
                "call_id": call_id,
                "name": name,
                "arguments": args_str,
            }));
        }

        // function_call_output / custom_tool_call_output
        if item_type == "function_call_output" || item_type == "custom_tool_call_output" {
            let call_id = obj.get("call_id").and_then(|v| v.as_str())
                .or_else(|| obj.get("id").and_then(|v| v.as_str()))
                .unwrap_or("");
            if call_id.is_empty() { return None; }
            let output_val = obj.get("output").cloned().unwrap_or(json!(""));
            let output_str = if output_val.is_string() {
                output_val.as_str().unwrap_or("").to_string()
            } else {
                serde_json::to_string(&output_val).unwrap_or_else(|_| "".to_string())
            };
            return Some(json!({
                "type": "function_call_output",
                "call_id": call_id,
                "output": output_str,
            }));
        }

        // function_call: normalize
        if item_type == "function_call" {
            let call_id = obj.get("call_id").and_then(|v| v.as_str())
                .or_else(|| obj.get("id").and_then(|v| v.as_str()))
                .unwrap_or("");
            let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
            if call_id.is_empty() || name.is_empty() { return None; }
            let args_val = obj.get("arguments").cloned().unwrap_or(json!({}));
            let args_str = if args_val.is_string() {
                args_val.as_str().unwrap_or("").to_string()
            } else {
                serde_json::to_string(&args_val).unwrap_or_else(|_| "{}".to_string())
            };
            let mut result = json!({
                "type": "function_call",
                "call_id": call_id,
                "name": name,
                "arguments": args_str,
            });
            // Keep native ID if present
            if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                if Self::is_native_item_id(id) {
                    result["id"] = json!(id);
                }
            }
            if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
                result["status"] = json!(status);
            }
            return Some(result);
        }

        // Default: strip internal metadata
        let mut clean = item.clone();
        if let Some(obj) = clean.as_object_mut() {
            obj.remove("internal_chat_message_metadata_passthrough");
        }
        Some(clean)
    }

    fn normalize_input(body: &mut Value) {
        let input = match body.get("input").and_then(|v| v.as_array()) {
            Some(arr) => arr.clone(),
            None => return,
        };

        // Normalize items
        let normalized: Vec<Value> = input.iter()
            .filter_map(Self::normalize_input_item)
            .collect();

        // Collect valid call_ids for filtering orphan outputs
        let valid_call_ids: std::collections::HashSet<String> = normalized.iter()
            .filter_map(|item| {
                if item.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                    item.get("call_id").and_then(|v| v.as_str()).map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();

        // Filter: keep all except orphan function_call_output
        let filtered: Vec<Value> = normalized.into_iter()
            .filter(|item| {
                if item.get("type").and_then(|v| v.as_str()) == Some("function_call_output") {
                    let call_id = item.get("call_id").and_then(|v| v.as_str()).unwrap_or("");
                    valid_call_ids.contains(call_id)
                } else {
                    true
                }
            })
            .collect();

        body["input"] = json!(filtered);
    }

    fn strip_stored_item_references(body: &mut Value) {
        let input = match body.get_mut("input").and_then(|v| v.as_array_mut()) {
            Some(arr) => arr,
            None => return,
        };

        // First pass: remove string references and item_reference type
        input.retain(|item| {
            if let Some(s) = item.as_str() {
                return !Self::is_server_id(s);
            }
            if item.get("type").and_then(|v| v.as_str()) == Some("item_reference") {
                return false;
            }
            true
        });

        // Second pass: strip non-native IDs from objects
        for item in input.iter_mut() {
            if let Some(obj) = item.as_object() {
                if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                    if Self::is_server_id(id) && !Self::is_native_item_id(id) {
                        if let Some(item_obj) = item.as_object_mut() {
                            item_obj.remove("id");
                        }
                    }
                }
            }
        }
    }

    fn normalize_tools(body: &mut Value) {
        let tools = match body.get("tools").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr.clone(),
            _ => {
                body.as_object_mut().map(|o| { o.remove("tools"); o.remove("tool_choice"); });
                return;
            }
        };

        let mut valid_names = std::collections::HashSet::new();
        let mut normalized = Vec::new();

        for tool in tools {
            let obj = match tool.as_object() {
                Some(o) => o,
                None => continue,
            };

            let type_ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");

            // Hosted tools: passthrough
            if HOSTED_TOOL_TYPES.contains(&type_) {
                normalized.push(tool.clone());
                continue;
            }

            // Function tools: flatten nested structure
            let is_function = type_ == "function" || type_.is_empty()
                || obj.contains_key("function") || obj.contains_key("name");

            if !is_function && !HOSTED_TOOL_TYPES.contains(&type_) {
                continue;
            }

            // Extract from nested .function or top-level
            let (name, description, parameters) = if let Some(fn_obj) = obj.get("function").and_then(|v| v.as_object()) {
                (
                    fn_obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    fn_obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    fn_obj.get("parameters").cloned().unwrap_or(json!({"type": "object", "properties": {}})),
                )
            } else {
                (
                    obj.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    obj.get("parameters").cloned().unwrap_or(json!({"type": "object", "properties": {}})),
                )
            };

            if name.is_empty() { continue; }

            let mut normalized_tool = json!({
                "type": "function",
                "name": &name[..name.len().min(128)],
            });
            if let Some(desc) = description {
                normalized_tool["description"] = json!(desc);
            }
            normalized_tool["parameters"] = parameters;
            valid_names.insert(name);
            normalized.push(normalized_tool);
        }

        if normalized.is_empty() {
            body.as_object_mut().map(|o| { o.remove("tools"); o.remove("tool_choice"); });
            return;
        }

        body["tools"] = json!(normalized);

        // Normalize tool_choice
        if let Some(choice) = body.get("tool_choice").cloned() {
            if let Some(obj) = choice.as_object() {
                let choice_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if choice_type == "function" || choice_type == "custom" {
                    let name = obj.get("name").and_then(|v| v.as_str())
                        .or_else(|| obj.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()))
                        .unwrap_or("");
                    if !name.is_empty() && valid_names.contains(name) {
                        body["tool_choice"] = json!({"type": "function", "name": &name[..name.len().min(128)]});
                    } else {
                        body.as_object_mut().map(|o| o.remove("tool_choice"));
                    }
                } else if !HOSTED_TOOL_TYPES.contains(&choice_type) {
                    body.as_object_mut().map(|o| o.remove("tool_choice"));
                }
            }
        }
    }

    fn normalize_reasoning(body: &mut Value, model: &str) {
        let supports_effort = !model.starts_with("grok-build"); // grok-build rejects effort

        let reasoning = body.get("reasoning").cloned();
        let reasoning_effort = body.get("reasoning_effort").and_then(|v| v.as_str());

        if reasoning.is_none() || !reasoning.as_ref().unwrap().is_object() {
            let mut new_reasoning = json!({"summary": "concise"});
            if supports_effort {
                let effort = reasoning_effort.unwrap_or("high");
                let normalized_effort = match effort {
                    "low" | "medium" | "high" | "xhigh" => effort,
                    "max" => "xhigh",
                    _ => "high",
                };
                new_reasoning["effort"] = json!(normalized_effort);
            }
            body["reasoning"] = new_reasoning;
        } else {
            let mut reasoning_obj = reasoning.unwrap();
            if supports_effort {
                let effort = reasoning_obj.get("effort").and_then(|v| v.as_str())
                    .or(reasoning_effort)
                    .unwrap_or("high");
                let normalized_effort = match effort {
                    "low" | "medium" | "high" | "xhigh" => effort,
                    "max" => "xhigh",
                    _ => "high",
                };
                reasoning_obj["effort"] = json!(normalized_effort);
            } else {
                reasoning_obj.as_object_mut().map(|o| o.remove("effort"));
            }
            if !reasoning_obj.get("summary").is_some() {
                reasoning_obj["summary"] = json!("concise");
            }
            body["reasoning"] = reasoning_obj;
        }

        body.as_object_mut().map(|o| o.remove("reasoning_effort"));

        // Include encrypted_content for multi-turn continuity
        if body.get("reasoning").and_then(|r| r.get("effort")).and_then(|e| e.as_str()) != Some("none") {
            let include = body.get("include").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let encrypted = json!("reasoning.encrypted_content");
            if !include.contains(&encrypted) {
                let mut new_include = include;
                new_include.push(encrypted);
                body["include"] = json!(new_include);
            }
        }
    }

    fn cleanup_body(body: &mut Value) {
        // Delete Chat Completions leftovers
        let fields_to_delete = [
            "messages", "max_tokens", "max_completion_tokens", "n", "seed",
            "logprobs", "top_logprobs", "frequency_penalty", "presence_penalty",
            "logit_bias", "user", "stream_options", "prompt_cache_retention",
            "safety_identifier", "previous_response_id",
        ];

        if let Some(obj) = body.as_object_mut() {
            for field in fields_to_delete {
                obj.remove(field);
            }

            // Allowlist: remove unknown fields
            let keys: Vec<String> = obj.keys().cloned().collect();
            for key in keys {
                if !RESPONSES_API_ALLOWLIST.contains(&key.as_str()) {
                    obj.remove(&key);
                }
            }
        }
    }

    pub fn to_response_request(&self, request: ChatCompletionRequest) -> Value {
        let model = request.model.trim_start_matches("gcli/").trim_start_matches("gb/");

        let mut input_items: Vec<Value> = Vec::new();

        for msg in &request.messages {
            match msg.role.as_str() {
                "system" => {
                    let content = msg.content.as_deref().unwrap_or("");
                    input_items.push(json!({
                        "role": "system",
                        "content": [{"type": "input_text", "text": content}]
                    }));
                }
                "user" => {
                    let content = msg.content.as_deref().unwrap_or("");
                    input_items.push(json!({
                        "role": "user",
                        "content": [{"type": "input_text", "text": content}]
                    }));
                }
                "assistant" => {
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

        // Guard: empty input → placeholder
        if input_items.is_empty() {
            input_items.push(json!({
                "role": "user",
                "content": [{"type": "input_text", "text": "..."}]
            }));
        }

        let mut body = json!({
            "model": model,
            "input": input_items,
            "stream": true,
            "store": false,
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
                let params = t.function.parameters.clone().unwrap_or(json!({
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
            body["tool_choice"] = tool_choice.clone();
        }

        // Apply normalizations
        Self::normalize_input(&mut body);
        Self::strip_stored_item_references(&mut body);
        Self::normalize_tools(&mut body);
        Self::normalize_reasoning(&mut body, model);
        Self::cleanup_body(&mut body);

        body
    }
}
