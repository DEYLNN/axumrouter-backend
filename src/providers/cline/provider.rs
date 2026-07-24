use async_trait::async_trait;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::engine::helpers::lock_key_on_error;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult, FailedKeyAttempt};
use crate::providers::traits::Provider;
use crate::types::chat::ChatCompletionRequest;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::auth::ClCredential;
use super::client::ClClient;
use super::constants;

pub struct ClProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: ClClient,
}

impl ClProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string()],
            icon_path: String::new(),
            category: constants::CATEGORY.to_string(),
            icon_name: constants::ICON_NAME.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: None,
            validate_url: constants::provider_spec().validate_url.to_string(),
        };
        Self { metadata, keys: KeyManager::new(keys), client: ClClient::new() }
    }

    fn models_static(&self) -> Vec<Model> {
        // models loaded from DB / TOML — use generic openai_compat model list
        // For now, hardcode the known model from providers.toml
        vec![
            Model {
                id: "cl/tencent/hy3".to_string(),
                object: "model".to_string(),
                owned_by: constants::PROVIDER_ID.to_string(),
                context_length: Some(128000),
            },
            Model {
                id: "cl/cline-pass/deepseek-v4-flash".to_string(),
                object: "model".to_string(),
                owned_by: constants::PROVIDER_ID.to_string(),
                context_length: Some(1000000),
            },
        ]
    }

    fn exhausted(&self) -> GatewayError {
        let lock_summary = self.keys.locked_keys()
            .into_iter().map(|(id, remaining, reason)| format!("{} locked {}s: {}", id, remaining, reason))
            .collect::<Vec<_>>().join("; ");
        GatewayError::NoAvailableKeys(if lock_summary.is_empty() {
            "No Cline keys available".into()
        } else {
            format!("All Cline keys exhausted — {}", lock_summary)
        })
    }
}

#[async_trait]
impl Provider for ClProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        for _attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let cred = match ClCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => { self.keys.lock_key(&key.id, 401, e.to_string()); failed.push(FailedKeyAttempt { key_id, error: GatewayError::ProviderError(e) }); continue; }
            };
            // Strip "cl/" prefix for upstream
            let model_name = request.model.strip_prefix("cl/").unwrap_or(&request.model);
            let mut body = serde_json::json!({
                "model": model_name,
                "messages": request.messages.iter().filter_map(|m| serde_json::to_value(m).ok()).collect::<Vec<_>>(),
                "stream": false,
                "max_tokens": request.max_tokens.unwrap_or(2048),
            });
            if let Some(v) = request.temperature { body["temperature"] = serde_json::json!(v); }
            if let Some(v) = request.top_p { body["top_p"] = serde_json::json!(v); }
            if let Some(ref v) = request.tools { body["tools"] = serde_json::to_value(v).unwrap_or_default(); }
            if let Some(ref v) = request.tool_choice { body["tool_choice"] = v.clone(); }
            match self.client.send_collect(body, &cred).await {
                Ok(response) => { self.keys.unlock(&key_id); return Ok(ChatResult { response, used_key_id: Some(key_id), failed_keys: failed }); }
                Err(e) => {
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    if c.retryable {
                        failed.push(FailedKeyAttempt { key_id, error: e });
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(self.exhausted())
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<ChatStreamResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        for _attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let cred = match ClCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => { self.keys.lock_key(&key.id, 401, e.to_string()); failed.push(FailedKeyAttempt { key_id, error: GatewayError::ProviderError(e) }); continue; }
            };
            let model_name = request.model.strip_prefix("cl/").unwrap_or(&request.model);
            let mut body = serde_json::json!({
                "model": model_name,
                "messages": request.messages.iter().filter_map(|m| serde_json::to_value(m).ok()).collect::<Vec<_>>(),
                "stream": true,
                "max_tokens": request.max_tokens.unwrap_or(2048),
            });
            if let Some(v) = request.temperature { body["temperature"] = serde_json::json!(v); }
            if let Some(v) = request.top_p { body["top_p"] = serde_json::json!(v); }
            if let Some(ref v) = request.tools { body["tools"] = serde_json::to_value(v).unwrap_or_default(); }
            if let Some(ref v) = request.tool_choice { body["tool_choice"] = v.clone(); }
            match self.client.send_stream(body, &cred).await {
                Ok(stream) => { self.keys.unlock(&key_id); return Ok(ChatStreamResult { stream, used_key_id: Some(key_id), failed_keys: failed }); }
                Err(e) => {
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    if c.retryable {
                        failed.push(FailedKeyAttempt { key_id, error: e });
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(self.exhausted())
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> { Ok(self.models_static()) }
    async fn health_check(&self) -> Result<bool, GatewayError> { Ok(self.keys.total_count() > 0) }
    async fn authenticate(&self) -> Result<(), GatewayError> { self.keys.next()?; Ok(()) }
    fn locked_keys(&self) -> Vec<(String, u64, String)> { self.keys.locked_keys() }
    fn total_keys(&self) -> usize { self.keys.total_count() }
    fn active_keys(&self) -> usize { self.keys.active_count() }
}