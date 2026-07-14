use crate::error::GatewayError;
use crate::providers::spec::MaxTokensField;
use crate::engine::openai_compat::config::{OpenAIConfig, ModelDef};
use crate::engine::openai_compat::types::{ChatRequest, ChatResponse, StreamChunk};
use crate::types::chat::{ChatCompletionRequest, ChatCompletionResponse, Choice, Message, Usage, ToolCall};
use crate::types::model::Model;
use std::sync::Arc;

/// Generic mapper for OpenAI-compatible providers.
/// Generic mapper for OpenAI-compatible providers.
#[derive(Clone)]
pub struct Mapper {
    config: Arc<OpenAIConfig>,
}

impl Mapper {
    pub fn new(config: Arc<OpenAIConfig>) -> Self {
        Self { config }
    }

    pub fn to_provider_request(&self, gateway_req: &ChatCompletionRequest) -> ChatRequest {
        let model = gateway_req
            .model
            .strip_prefix(&format!("{}/", self.config.model_prefix))
            .unwrap_or(&gateway_req.model)
            .to_string();

        let quirks = &self.config.quirks;

        // Handle max_tokens field choice
        let (max_tokens, max_completion_tokens) = match quirks.max_tokens_field {
            MaxTokensField::MaxTokens => (gateway_req.max_tokens, None),
            MaxTokensField::MaxCompletionTokens => (None, gateway_req.max_tokens),
        };

        ChatRequest {
            model,
            messages: gateway_req.messages.clone(),
            temperature: gateway_req.temperature,
            max_tokens,
            max_completion_tokens,
            top_p: gateway_req.top_p,
            stream: gateway_req.stream.unwrap_or(false),
            stream_options: if quirks.drop_stream_options {
                None
            } else {
                gateway_req.stream_options.clone()
            },
            tools: if quirks.drop_tools {
                None
            } else {
                gateway_req.tools.clone()
            },
            tool_choice: if quirks.drop_tool_choice {
                None
            } else {
                gateway_req.tool_choice.clone()
            },
        }
    }

    pub fn to_gateway_response(
        &self,
        provider_resp: &ChatResponse,
    ) -> ChatCompletionResponse {
        let choices: Vec<Choice> = provider_resp
            .choices
            .iter()
            .map(|c| Choice {
                index: c.index,
                message: Message {
                    role: c.message.role.clone(),
                    content: c.message.content.clone(),
                    tool_calls: c.message.tool_calls.clone(),
                    tool_call_id: None,
                    name: None,
                    reasoning_content: None,
                },
                finish_reason: c.finish_reason.clone(),
            })
            .collect();

        let usage = provider_resp.usage.clone().map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        ChatCompletionResponse {
            id: provider_resp.id.clone(),
            object: provider_resp.object.clone(),
            created: provider_resp.created,
            model: format!("{}/{}", self.config.model_prefix, provider_resp.model),
            choices,
            usage,
        }
    }

    pub fn parse_stream_chunk(&self, line: &str) -> Result<StreamChunk, GatewayError> {
        if line.is_empty() {
            return Err(GatewayError::ProviderError("Empty SSE line".into()));
        }
        let data = line
            .strip_prefix("data: ")
            .ok_or_else(|| GatewayError::ProviderError("SSE line missing 'data: ' prefix".into()))?;

        if data == "[DONE]" {
            return Err(GatewayError::ProviderError("Stream done".into()));
        }

        serde_json::from_str::<StreamChunk>(data).map_err(|e| {
            GatewayError::ProviderError(format!("Failed to parse SSE chunk: {} - data: {}", e, data))
        })
    }

    pub fn to_gateway_chunk(
        &self,
        chunk: &StreamChunk,
    ) -> crate::types::chat::ChatCompletionChunk {
        let delta = chunk
            .choices
            .first()
            .map(|c| c.delta.clone())
            .unwrap_or_else(|| crate::engine::openai_compat::types::StreamDelta {
                role: None,
                content: None,
                tool_calls: None,
            });

        crate::types::chat::ChatCompletionChunk {
            id: chunk.id.clone().unwrap_or_default(),
            object: "chat.completion.chunk".into(),
            created: chunk.created.unwrap_or_else(|| std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
            model: chunk.model.clone().unwrap_or_default(),
            choices: vec![crate::types::chat::ChunkChoice {
                index: chunk.choices.first().map(|c| c.index).unwrap_or(0),
                delta: crate::types::chat::Delta {
                    role: delta.role,
                    content: delta.content,
                    tool_calls: delta.tool_calls,
                },
                finish_reason: chunk.choices.first().and_then(|c| c.finish_reason.clone()),
            }],
            usage: chunk.usage.clone().map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        }
    }

    pub fn models_static(&self) -> Vec<Model> {
        self.config
            .models
            .iter()
            .map(|m| Model {
                id: format!("{}/{}", self.config.model_prefix, m.id),
                object: "model".to_string(),
                owned_by: self.config.provider_name.to_string(),
                context_length: Some(m.max_tokens),
            })
            .collect()
    }

    pub fn token_economy(&self, _model_id: &str) -> (u32, u32) {
        (65536, 4096)
    }
}
