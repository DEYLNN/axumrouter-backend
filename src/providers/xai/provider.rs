use async_trait::async_trait;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::providers::error_classifier::classify_provider_error;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult, FailedKeyAttempt};
use crate::providers::traits::Provider;
use crate::types::chat::ChatCompletionRequest;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::auth::XaiOAuthCredential;
use super::client::XaiClient;
use super::constants;
use super::mapper::XaiMapper;

pub struct XaiProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: XaiClient,
    mapper: XaiMapper,
}

impl XaiProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string(), "oauth".to_string()],
            icon_path: String::new(),
            category: constants::CATEGORY.to_string(),
            icon_url: constants::ICON_URL.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: Some("authorization_code".to_string()),
        };
        Self {
            metadata,
            keys: KeyManager::new(keys),
            client: XaiClient::new(constants::DEFAULT_TIMEOUT_SECS),
            mapper: XaiMapper,
        }
    }

    fn models_static(&self) -> Vec<Model> {
        constants::MODELS.iter().map(|m| Model {
            id: format!("{}/{}", constants::PROVIDER_ID, m.id),
            object: "model".to_string(),
            owned_by: constants::PROVIDER_ID.to_string(),
            context_length: None,
            }).collect()
    }

    fn exhausted(&self) -> GatewayError {
        let lock_summary = self.keys.locked_keys()
            .into_iter()
            .map(|(id, remaining, reason)| format!("{} locked {}s: {}", id, remaining, reason))
            .collect::<Vec<_>>()
            .join("; ");
        GatewayError::no_available_keys(if lock_summary.is_empty() {
            "No xAI OAuth credentials available — import OAuth JSON as provider key or connect via OAuth".to_string()
        } else {
            format!("All xAI OAuth credentials exhausted — {}", lock_summary)
        })
    }
}

#[async_trait]
impl Provider for XaiProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        for attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let cred = match XaiOAuthCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => {
                    self.keys.lock_key(&key.id, 401, e.to_string());
                    failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
            };
            let body = self.mapper.to_chat_request(request.clone());
            match self.client.send_collect(body, &cred).await {
                Ok(response) => {
                    self.keys.unlock(&key_id);
                    return Ok(ChatResult { response, used_key_id: Some(key_id), failed_keys: failed });
                }
                Err(e) => {
                    let classified = classify_provider_error(&e);
                    if classified.retryable {
                        let status = classified.lock_status.unwrap_or(classified.status.unwrap_or(503));
                        tracing::warn!("xAI key '{}' failed attempt {}/{}, kind={:?}", key_id, attempt + 1, total, classified.kind);
                        self.keys.lock_key(&key.id, status, e.to_string());
                        failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
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
        for attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let cred = match XaiOAuthCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => {
                    self.keys.lock_key(&key.id, 401, e.to_string());
                    failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
            };
            let body = self.mapper.to_chat_request(request.clone());
            match self.client.send_stream(body, &cred).await {
                Ok(stream) => {
                    self.keys.unlock(&key_id);
                    return Ok(ChatStreamResult { stream, used_key_id: Some(key_id), failed_keys: failed });
                }
                Err(e) => {
                    let classified = classify_provider_error(&e);
                    if classified.retryable {
                        let status = classified.lock_status.unwrap_or(classified.status.unwrap_or(503));
                        tracing::warn!("xAI key '{}' failed attempt {}/{}, kind={:?}", key_id, attempt + 1, total, classified.kind);
                        self.keys.lock_key(&key.id, status, e.to_string());
                        failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
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
    async fn authenticate(&self) -> Result<(), GatewayError> {
        let key = self.keys.next()?;
        XaiOAuthCredential::parse(&key.key_value).map(|_| ())
    }
    fn locked_keys(&self) -> Vec<(String, u64, String)> { self.keys.locked_keys() }
    fn total_keys(&self) -> usize { self.keys.total_count() }
    fn active_keys(&self) -> usize { self.keys.active_count() }
}
