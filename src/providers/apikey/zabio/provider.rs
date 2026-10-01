use async_trait::async_trait;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult, FailedKeyAttempt};
use crate::providers::traits::Provider;
use crate::types::chat::ChatCompletionRequest;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::auth::ZbCredential;
use super::client::ZbClient;
use super::constants;

/// Zabio provider — errors do not lock or deactivate keys. Each request
/// tries every key at most once, then returns the last upstream error.
pub struct ZbProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: ZbClient,
}

impl ZbProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>, db: Arc<SqlitePool>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                "chat".to_string(),
                "models".to_string(),
                "streaming".to_string(),
            ],
            icon_path: String::new(),
            category: constants::CATEGORY.to_string(),
            icon_name: constants::ICON_NAME.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: None,
            model_prefix: None,
            validate_url: constants::provider_spec().validate_url.to_string(),
        };
        Self {
            metadata,
            keys: KeyManager::new_with_pool(keys, constants::PROVIDER_ID, Some((*db).clone())),
            client: ZbClient::new(),
        }
    }

    fn models_static(&self) -> Vec<Model> {
        constants::MODELS
            .iter()
            .map(|m| Model {
                id: format!("{}/{}", constants::PROVIDER_ID, m.id),
                object: "model".to_string(),
                owned_by: constants::PROVIDER_ID.to_string(),
                context_length: Some(m.context_length),
            })
            .collect()
    }

    fn build_body(&self, request: &ChatCompletionRequest, stream: bool) -> serde_json::Value {
        let model_name = request.model.strip_prefix("zb/").unwrap_or(&request.model);
        let messages = self.repair_tool_messages(&request.messages);
        let mut body = serde_json::json!({
            "model": model_name,
            "messages": messages.iter().filter_map(|m| serde_json::to_value(m).ok()).collect::<Vec<_>>(),
            "stream": stream,
            "max_tokens": request.max_tokens.unwrap_or(2048),
        });
        if let Some(v) = request.temperature {
            body["temperature"] = serde_json::json!(v);
        }
        if let Some(v) = request.top_p {
            body["top_p"] = serde_json::json!(v);
        }
        if let Some(ref v) = request.tools {
            body["tools"] = serde_json::to_value(v).unwrap_or_default();
        }
        if let Some(ref v) = request.tool_choice {
            body["tool_choice"] = v.clone();
        }
        // Force include_usage on stream — required for upstream to send
        // the terminal usage chunk; without it usage tracking sees 0 tokens.
        if stream {
            body["stream_options"] = serde_json::json!({"include_usage": true});
        } else if let Some(ref v) = request.stream_options {
            body["stream_options"] = v.clone();
        }
        body
    }

    /// Repair conversation history: inject dummy tool responses for any
    /// assistant tool_calls that lack a matching tool message.
    /// Upstream DeepSeek/Together rejects 400 if assistant has tool_calls
    /// but no tool response follows.
    fn repair_tool_messages(&self, messages: &[crate::types::chat::Message]) -> Vec<crate::types::chat::Message> {
        use std::collections::HashSet;

        // Collect all tool_call_ids that have a tool response
        let mut answered: HashSet<String> = HashSet::new();
        for m in messages {
            if m.role == "tool" {
                if let Some(ref id) = m.tool_call_id {
                    answered.insert(id.clone());
                }
            }
        }

        let mut repaired: Vec<crate::types::chat::Message> = Vec::new();
        for m in messages {
            repaired.push(m.clone());
            // If this is an assistant message with tool_calls, check each
            // tool_call for missing responses. Inject dummy right after.
            if m.role == "assistant" {
                if let Some(ref tcs) = m.tool_calls {
                    for tc in tcs {
                        if !answered.contains(&tc.id) {
                            tracing::warn!(
                                "zb: injecting dummy tool response for orphaned tool_call_id={}",
                                tc.id
                            );
                            repaired.push(crate::types::chat::Message {
                                role: "tool".to_string(),
                                content: Some("{\"error\":\"no_tool_result\"}".to_string()),
                                reasoning_content: None,
                                tool_calls: None,
                                tool_call_id: Some(tc.id.clone()),
                                name: Some(tc.function.name.clone()),
                            });
                        }
                    }
                }
            }
        }
        repaired
    }
}

#[async_trait]
impl Provider for ZbProvider {
    fn metadata(&self) -> ProviderMetadata {
        self.metadata.clone()
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatResult, GatewayError> {
        let body = self.build_body(&request, false);
        let mut failed: Vec<FailedKeyAttempt> = Vec::new();
        let mut excluded: Vec<String> = Vec::new();
        let mut last_error: Option<String> = None;
        let mut last_status: Option<u16> = None;
        let total = self.keys.total_count();
        let mut last_attempted_key_id: Option<String> = None;
        for _ in 0..total {
            let key = match self.keys.next_excluding(None, &excluded) {
                Ok(key) => key,
                Err(_) => break,
            };
            let key_id = key.id.clone();
            last_attempted_key_id = Some(key_id.clone());
            let cred = match ZbCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => {
                    excluded.push(key_id.clone());
                    last_status = Some(400);
                    last_error = Some(e.clone());
                    failed.push(FailedKeyAttempt {
                        key_id,
                        error: GatewayError::ProviderError(e),
                    });
                    continue;
                }
            };
            match self.client.send_collect(body.clone(), &cred).await {
                Ok(response) => {
                    self.keys.mark_success(&key_id);
                    return Ok(ChatResult {
                        response,
                        used_key_id: Some(key_id),
                        failed_keys: failed,
                    });
                }
                Err(e) => {
                    excluded.push(key_id.clone());
                    last_status = e.http_status();
                    last_error = Some(e.to_string());
                    failed.push(FailedKeyAttempt { key_id, error: e });
                    continue;
                }
            }
        }
        if let Some(key_id) = last_attempted_key_id {
            return Err(GatewayError::ProviderHttpError {
                status: last_status.unwrap_or(503),
                body: last_error.unwrap_or_default(),
                provider: constants::PROVIDER_ID.to_string(),
                key_id: Some(key_id),
            });
        }
        Err(GatewayError::NoAvailableKeys(
            "No Zabio keys configured".into(),
        ))
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatStreamResult, GatewayError> {
        let body = self.build_body(&request, true);
        let mut failed: Vec<FailedKeyAttempt> = Vec::new();
        let mut excluded: Vec<String> = Vec::new();
        let mut last_attempted_key_id: Option<String> = None;
        let mut last_error: Option<String> = None;
        let mut last_status: Option<u16> = None;
        let total = self.keys.total_count();
        if total == 0 {
            return Err(GatewayError::NoAvailableKeys(
                "No Zabio keys configured".into(),
            ));
        }
        for _ in 0..total {
            let key = match self.keys.next_excluding(None, &excluded) {
                Ok(key) => key,
                Err(_) => break,
            };
            let key_id = key.id.clone();
            last_attempted_key_id = Some(key_id.clone());
            let cred = match ZbCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => {
                    excluded.push(key_id.clone());
                    last_status = Some(400);
                    last_error = Some(e.clone());
                    failed.push(FailedKeyAttempt {
                        key_id,
                        error: GatewayError::ProviderError(e),
                    });
                    continue;
                }
            };
            match self.client.send_stream(body.clone(), &cred).await {
                Ok(stream) => {
                    self.keys.mark_success(&key_id);
                    return Ok(ChatStreamResult {
                        stream,
                        used_key_id: Some(key_id),
                        failed_keys: failed,
                        last_attempted_key_id,
                    });
                }
                Err(e) => {
                    excluded.push(key_id.clone());
                    last_status = e.http_status();
                    last_error = Some(e.to_string());
                    failed.push(FailedKeyAttempt { key_id, error: e });
                    continue;
                }
            }
        }
        Err(GatewayError::ProviderHttpError {
            status: last_status.unwrap_or(503),
            body: last_error.unwrap_or_default(),
            provider: constants::PROVIDER_ID.to_string(),
            key_id: last_attempted_key_id,
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
        Ok(self.models_static())
    }
    async fn health_check(&self) -> Result<bool, GatewayError> {
        Ok(self.keys.total_count() > 0)
    }
    async fn authenticate(&self) -> Result<(), GatewayError> {
        self.keys.next()?;
        Ok(())
    }
    fn locked_keys(&self) -> Vec<(String, u64, String)> {
        self.keys.locked_keys()
    }
    fn total_keys(&self) -> usize {
        self.keys.total_count()
    }
    fn active_keys(&self) -> usize {
        self.keys.active_count()
    }
}
