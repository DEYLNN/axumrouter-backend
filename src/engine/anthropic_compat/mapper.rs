use std::sync::Arc;

use crate::engine::anthropic_compat::config::AnthropicConfig;
use crate::engine::anthropic_compat::types::*;
use crate::error::GatewayError;
use crate::providers::spec::MaxTokensField;
use crate::types::chat::*;
use crate::types::model::Model;

/// Mapper: OpenAI ChatCompletion ↔ Claude /v1/messages
#[derive(Clone)]
pub struct Mapper {
    config: Arc<AnthropicConfig>,
}

impl Mapper {
    pub fn new(config: Arc<AnthropicConfig>) -> Self {
        Self { config }
    }

    pub fn to_provider_request(&self, gw: &ChatCompletionRequest) -> AnthropicRequest {
        let model = gw.model.strip_prefix(&format!("{}/", self.config.model_prefix))
            .unwrap_or(&gw.model).to_string();

        // Extract system messages
        let (system_msgs, msgs): (Vec<&Message>, Vec<&Message>) = gw.messages.iter()
            .partition(|m| m.role == "system");

        let system: Option<Vec<SystemBlock>> = if system_msgs.is_empty() {
            None
        } else {
            Some(system_msgs.iter().map(|m| SystemBlock {
                type_: "text".into(),
                text: m.content.clone().unwrap_or_default(),
                cache_control: None,
            }).collect())
        };

        let quirks = &self.config.quirks;
        let (max_tokens, _) = match quirks.max_tokens_field {
            MaxTokensField::MaxTokens => (gw.max_tokens, None::<u32>),
            MaxTokensField::MaxCompletionTokens => (gw.max_tokens, None),
        };

        let messages: Vec<AnthropicMessage> = msgs.iter().map(|m| {
            let role = if m.role == "assistant" { "assistant" } else { "user" };
            let mut content = Vec::new();

            // Text content
            if let Some(text) = &m.content {
                if !text.is_empty() {
                    content.push(ContentBlock::Text { text: text.clone() });
                }
            }

            // Tool calls (assistant → tool_use)
            if let Some(tcs) = &m.tool_calls {
                for tc in tcs {
                    content.push(ContentBlock::ToolUse {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        input: serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null),
                    });
                }
            }

            // Tool result (tool → tool_result)
            if m.role == "tool" {
                content.push(ContentBlock::ToolResult {
                    tool_use_id: m.tool_call_id.clone().unwrap_or_default(),
                    content: m.content.clone().unwrap_or_default(),
                    is_error: None,
                });
            }

            AnthropicMessage { role: role.into(), content }
        }).collect();

        // Tools
        let tools = gw.tools.as_ref().map(|ts| ts.iter().map(|t| AnthropicTool {
            name: t.function.name.clone(),
            description: t.function.description.clone(),
            input_schema: t.function.parameters.clone(),
        }).collect());

        // Thinking
        let thinking = None; // could map from reasoning_effort later

        AnthropicRequest {
            model,
            max_tokens,
            temperature: quirks.force_temperature.or(gw.temperature).or(quirks.default_temperature),
            stream: gw.stream.unwrap_or(false),
            system,
            messages,
            tools,
            tool_choice: gw.tool_choice.clone(),
            thinking,
        }
    }

    pub fn to_gateway_response(&self, resp: &AnthropicResponse) -> ChatCompletionResponse {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in &resp.content {
            match block {
                ResponseContentBlock::Text { text } => content.push_str(text),
                ResponseContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        type_: "function".into(),
                        function: ToolCallFunction {
                            name: name.clone(),
                            arguments: serde_json::to_string(input).unwrap_or_default(),
                        },
                    });
                }
                ResponseContentBlock::Thinking { thinking, .. } => {
                    // Include thinking as reasoning_content in the message
                    if !content.is_empty() { content.push('\n'); }
                    content.push_str(&format!("<think>{}</think>", thinking));
                }
            }
        }

        let finish_reason = resp.stop_reason.as_deref().map(|r| match r {
            "end_turn" => "stop",
            "max_tokens" => "length",
            "tool_use" => "tool_calls",
            "stop_sequence" => "stop",
            _ => "stop",
        }).map(String::from);

        let usage = resp.usage.as_ref().map(|u| Usage {
            prompt_tokens: u.input_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens,
        });

        ChatCompletionResponse {
            id: resp.id.clone(),
            object: "chat.completion".into(),
            created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            model: format!("{}/{}", self.config.model_prefix, resp.model),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: if content.is_empty() { None } else { Some(content) },
                    reasoning_content: None,
                    tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                    tool_call_id: None,
                    name: None,
                },
                finish_reason,
            }],
            usage,
        }
    }

    pub fn parse_stream_event(&self, line: &str) -> Result<AnthropicStreamEvent, GatewayError> {
        let data = line.strip_prefix("data: ")
            .ok_or_else(|| GatewayError::ProviderError("SSE line missing 'data: ' prefix".into()))?;

        if data == "[DONE]" || data.trim().is_empty() {
            return Err(GatewayError::ProviderError("Stream done".into()));
        }

        let event_type = serde_json::from_str::<serde_json::Value>(data)
            .map_err(|e| GatewayError::ProviderError(format!("Parse error: {}", e)))?
            .get("type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| GatewayError::ProviderError("Missing type field in SSE".into()))?;

        match event_type.as_str() {
            "message_start" => {
                let e: MessageStartEvent = serde_json::from_str(data).map_err(|e| GatewayError::ProviderError(format!("Parse message_start: {}", e)))?;
                Ok(AnthropicStreamEvent::MessageStart(e))
            }
            "content_block_start" => {
                let e: ContentBlockStartEvent = serde_json::from_str(data).map_err(|e| GatewayError::ProviderError(format!("Parse content_block_start: {}", e)))?;
                Ok(AnthropicStreamEvent::ContentBlockStart(e))
            }
            "content_block_delta" => {
                let e: ContentBlockDeltaEvent = serde_json::from_str(data).map_err(|e| GatewayError::ProviderError(format!("Parse content_block_delta: {}", e)))?;
                Ok(AnthropicStreamEvent::ContentBlockDelta(e))
            }
            "content_block_stop" => {
                let e: ContentBlockStopEvent = serde_json::from_str(data).map_err(|e| GatewayError::ProviderError(format!("Parse content_block_stop: {}", e)))?;
                Ok(AnthropicStreamEvent::ContentBlockStop(e))
            }
            "message_delta" => {
                let e: MessageDeltaEvent = serde_json::from_str(data).map_err(|e| GatewayError::ProviderError(format!("Parse message_delta: {}", e)))?;
                Ok(AnthropicStreamEvent::MessageDelta(e))
            }
            "message_stop" => {
                Ok(AnthropicStreamEvent::MessageStop)
            }
            "ping" => Ok(AnthropicStreamEvent::Ping),
            _ => Err(GatewayError::ProviderError(format!("Unknown SSE event: {}", event_type))),
        }
    }

    /// Convert a parsed Anthropic stream event to OpenAI chunk(s)
    pub fn to_gateway_chunks(&self, event: &AnthropicStreamEvent, state: &mut StreamState) -> Vec<ChatCompletionChunk> {
        let mut chunks = Vec::new();
        match event {
            AnthropicStreamEvent::MessageStart(e) => {
                state.message_id = e.message.id.clone();
                state.model = e.message.model.clone();
                chunks.push(self._make_chunk(state, Delta { role: Some("assistant".into()), content: None, reasoning_content: None, tool_calls: None }, None));
            }
            AnthropicStreamEvent::ContentBlockStart(e) => {
                match &e.content_block {
                    ResponseContentBlock::Text { text } => {
                        state.text_block_index = Some(e.index);
                        if !text.is_empty() {
                            chunks.push(self._make_chunk(state, Delta { role: None, content: Some(text.clone()), reasoning_content: None, tool_calls: None }, None));
                        }
                    }
                    ResponseContentBlock::ToolUse { id, name, .. } => {
                        let idx = state.tool_call_index;
                        state.tool_call_index += 1;
                        chunks.push(self._make_chunk(state, Delta {
                            role: None, content: None, reasoning_content: None,
                            tool_calls: Some(vec![ChunkToolCall {
                                index: idx,
                                id: Some(id.clone()),
                                type_: Some("function".into()),
                                function: Some(ChunkToolCallFunction { name: Some(name.clone()), arguments: None }),
                            }]),
                        }, None));
                        state.pending_tool_calls.insert(e.index, idx);
                    }
                    ResponseContentBlock::Thinking { thinking, .. } => {
                        if !thinking.is_empty() {
                            chunks.push(self._make_chunk(state, Delta { role: None, content: Some("<think>".into()), reasoning_content: None, tool_calls: None }, None));
                            chunks.push(self._make_chunk(state, Delta {
                                role: None,
                                reasoning_content: None,
                                content: None,
                                tool_calls: None,
                            }, None));
                            // reasoning_content is not in Delta struct currently, so we put it inline
                            // Actually let's use content for simplicity
                            chunks.push(self._make_chunk(state, Delta { role: None, content: Some(thinking.clone()), reasoning_content: None, tool_calls: None }, None));
                        }
                    }
                }
            }
            AnthropicStreamEvent::ContentBlockDelta(e) => {
                match &e.delta {
                    ContentDelta::TextDelta { text } => {
                        chunks.push(self._make_chunk(state, Delta { role: None, content: Some(text.clone()), reasoning_content: None, tool_calls: None }, None));
                    }
                    ContentDelta::InputJsonDelta { partial_json } => {
                        if let Some(tc_idx) = state.pending_tool_calls.get(&e.index) {
                            chunks.push(self._make_chunk(state, Delta {
                                role: None, content: None, reasoning_content: None,
                                tool_calls: Some(vec![ChunkToolCall {
                                    index: *tc_idx,
                                    id: None, type_: None,
                                    function: Some(ChunkToolCallFunction { name: None, arguments: Some(partial_json.clone()) }),
                                }]),
                            }, None));
                        }
                    }
                    ContentDelta::ThinkingDelta { thinking } => {
                        chunks.push(self._make_chunk(state, Delta { role: None, content: Some(thinking.clone()), reasoning_content: None, tool_calls: None }, None));
                    }
                }
            }
            AnthropicStreamEvent::ContentBlockStop(e) => {
                if state.text_block_index == Some(e.index) {
                    state.text_block_index = None;
                }
            }
            AnthropicStreamEvent::MessageDelta(e) => {
                let fr = e.delta.stop_reason.as_deref().map(|r| match r {
                    "end_turn" => "stop",
                    "max_tokens" => "length",
                    "tool_use" => "tool_calls",
                    "stop_sequence" => "stop",
                    _ => "stop",
                }).map(String::from);

                let usage = e.usage.as_ref().map(|u| Usage {
                    prompt_tokens: u.input_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens,
                    completion_tokens: u.output_tokens,
                    total_tokens: u.input_tokens + u.output_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens,
                });

                chunks.push(ChatCompletionChunk {
                    id: format!("chatcmpl-{}", state.message_id),
                    object: "chat.completion.chunk".into(),
                    created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
                    model: state.model.clone(),
                    choices: vec![ChunkChoice { index: 0, delta: Delta { role: None, content: None, reasoning_content: None, tool_calls: None }, finish_reason: fr }],
                    usage,
                });
            }
            AnthropicStreamEvent::MessageStop => {
                if !state.finish_sent {
                    chunks.push(self._make_chunk(state, Delta { role: None, content: None, reasoning_content: None, tool_calls: None }, Some("stop".into())));
                    state.finish_sent = true;
                }
            }
            AnthropicStreamEvent::Ping => {}
        }
        chunks
    }

    fn _make_chunk(&self, state: &StreamState, delta: Delta, finish_reason: Option<String>) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: format!("chatcmpl-{}", state.message_id),
            object: "chat.completion.chunk".into(),
            created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            model: state.model.clone(),
            choices: vec![ChunkChoice { index: 0, delta, finish_reason }],
            usage: None,
        }
    }

    pub fn models_static(&self) -> Vec<Model> {
        self.config.models.iter().map(|m| Model {
            id: format!("{}/{}", self.config.model_prefix, m.id),
            object: "model".to_string(),
            owned_by: self.config.provider_name.to_string(),
            context_length: Some(m.max_tokens),
        }).collect()
    }
}

/// Streaming state for Anthropic → OpenAI chunk conversion
pub struct StreamState {
    pub message_id: String,
    pub model: String,
    pub tool_call_index: u32,
    pub text_block_index: Option<u32>,
    pub pending_tool_calls: std::collections::HashMap<u32, u32>,
    pub finish_sent: bool,
}

impl StreamState {
    pub fn new() -> Self {
        Self {
            message_id: String::new(),
            model: String::new(),
            tool_call_index: 0,
            text_block_index: None,
            pending_tool_calls: std::collections::HashMap::new(),
            finish_sent: false,
        }
    }
}
