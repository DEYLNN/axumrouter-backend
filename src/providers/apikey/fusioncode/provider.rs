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

use super::auth::FsnCredential;
use super::client::FsnClient;
use super::constants;

/// InferX provider — custom key handling:
/// errors NEVER lock or deactivate a key (upstream 429s are transient,
/// server allows immediate retry). On error the key is simply skipped for
/// THIS request via `excluded`; `mark_success`/DB state stay untouched.
/// KeyManager still owns round-robin selection — untouched for other providers.
pub struct FsnProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: FsnClient,
}

impl FsnProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>, db: Arc<SqlitePool>) -> Self {
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
            model_prefix: None,
            validate_url: constants::provider_spec().validate_url.to_string(),
        };
        Self { metadata, keys: KeyManager::new_with_pool(keys, constants::PROVIDER_ID, Some((*db).clone())), client: FsnClient::new() }
    }

    fn models_static(&self) -> Vec<Model> {
        constants::MODELS.iter().map(|m| Model {
            id: format!("{}/{}", constants::PROVIDER_ID, m.id),
            object: "model".to_string(),
            owned_by: constants::PROVIDER_ID.to_string(),
            context_length: Some(m.context_length),
        }).collect()
    }

    fn build_body(&self, request: &ChatCompletionRequest, stream: bool) -> serde_json::Value {
        let model_name = request.model.strip_prefix("fsn/").unwrap_or(&request.model);
        let mut body = serde_json::json!({
            "model": model_name,
            "messages": request.messages.iter().filter_map(|m| serde_json::to_value(m).ok()).collect::<Vec<_>>(),
            "stream": stream,
            "max_tokens": request.max_tokens.unwrap_or(2048),
        });
        if let Some(v) = request.temperature { body["temperature"] = serde_json::json!(v); }
        if let Some(v) = request.top_p { body["top_p"] = serde_json::json!(v); }
        if let Some(ref v) = request.tools { body["tools"] = serde_json::to_value(v).unwrap_or_default(); }
        if let Some(ref v) = request.tool_choice { body["tool_choice"] = v.clone(); }
        // Forward stream_options (include_usage) — required for upstream to send
        // the terminal usage chunk; without it usage tracking sees 0 tokens.
        if let Some(ref v) = request.stream_options { body["stream_options"] = v.clone(); }
        body
    }

    fn exhausted(&self) -> GatewayError {
        GatewayError::NoAvailableKeys(format!(
            "All FusionCode keys exhausted ({} key(s) tried, none locked — errors are non-locking for this provider)",
            self.keys.total_count()
        ))
    }
}

#[async_trait]
impl Provider for FsnProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        let mut excluded: Vec<String> = Vec::new();
        let body = self.build_body(&request, false);
        for _attempt in 0..total.max(1) {
            // Errors don't lock → next_excluding sees the same key as available;
            // excluded list is what forces rotation to the next key.
            let key = match self.keys.next_excluding(None, &excluded) { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let cred = match FsnCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => { excluded.push(key_id.clone()); failed.push(FailedKeyAttempt { key_id, error: GatewayError::ProviderError(e) }); continue; }
            };
            match self.client.send_collect(body.clone(), &cred).await {
                Ok(response) => { self.keys.mark_success(&key_id); return Ok(ChatResult { response, used_key_id: Some(key_id), failed_keys: failed }); }
                Err(e) => {
                    // NO lock_key / NO cooldown / NO auto-deactivate — key stays usable.
                    excluded.push(key_id.clone());
                    failed.push(FailedKeyAttempt { key_id, error: e });
                    continue;
                }
            }
        }
        Err(self.exhausted())
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<ChatStreamResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        let mut excluded: Vec<String> = Vec::new();
        let body = self.build_body(&request, true);
        for _attempt in 0..total.max(1) {
            let key = match self.keys.next_excluding(None, &excluded) { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let cred = match FsnCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => { excluded.push(key_id.clone()); failed.push(FailedKeyAttempt { key_id, error: GatewayError::ProviderError(e) }); continue; }
            };
            match self.client.send_stream(body.clone(), &cred).await {
                Ok(stream) => { self.keys.mark_success(&key_id); return Ok(ChatStreamResult { stream, used_key_id: Some(key_id), failed_keys: failed, last_attempted_key_id: None }); }
                Err(e) => {
                    excluded.push(key_id.clone());
                    failed.push(FailedKeyAttempt { key_id, error: e });
                    continue;
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
