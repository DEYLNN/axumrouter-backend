use async_trait::async_trait;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::db::models::ApiKey;
use crate::engine::helpers::lock_key_on_error;
use crate::error::GatewayError;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult, FailedKeyAttempt};
use crate::providers::traits::Provider;
use crate::types::chat::ChatCompletionRequest;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::auth::GrokCliOAuthCredential;
use super::client::GcliClient;
use super::constants;
use super::mapper::GcliMapper;
use super::oauth;

pub struct GcliProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: GcliClient,
    mapper: GcliMapper,
    db: Arc<SqlitePool>,
}

impl GcliProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>, db: Arc<SqlitePool>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string(), "oauth".to_string()],
            icon_path: String::new(),
            category: constants::CATEGORY.to_string(),
            icon_name: constants::ICON_NAME.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: Some("authorization_code".to_string()),
            validate_url: constants::provider_spec().validate_url.to_string(),
        };
        Self {
            metadata,
            keys: KeyManager::new(keys),
            client: GcliClient::new(constants::DEFAULT_TIMEOUT_SECS),
            mapper: GcliMapper,
            db,
        }
    }

    fn models_static(&self) -> Vec<Model> {
        constants::MODELS.iter().map(|m| Model {
            id: format!("{}/{}", constants::PROVIDER_ID, m.id),
            object: "model".to_string(),
            owned_by: constants::PROVIDER_ID.to_string(),
            context_length: Some(m.max_tokens),
        }).collect()
    }

    fn exhausted(&self) -> GatewayError {
        let lock_summary = self.keys.locked_keys()
            .into_iter()
            .map(|(id, remaining, reason)| format!("{} locked {}s: {}", id, remaining, reason))
            .collect::<Vec<_>>()
            .join("; ");
        GatewayError::NoAvailableKeys(if lock_summary.is_empty() {
            "No Grok Build OAuth credentials available — connect via OAuth first".to_string()
        } else {
            format!("All Grok Build OAuth credentials exhausted — {}", lock_summary)
        })
    }

    /// Try to refresh a credential in-place. Returns Ok(true) if refreshed, Ok(false) if not needed, Err if permanent fail.
    async fn try_refresh(&self, cred: &mut GrokCliOAuthCredential, key_id: &str) -> Result<bool, GatewayError> {
        if !cred.needs_refresh() {
            return Ok(false);
        }
        tracing::info!("grok-cli key '{}' needs refresh, attempting...", key_id);
        match oauth::refresh_access_token(&cred.refresh_token).await {
            Ok(new_token) => {
                // Update credential in-place
                if let Some(at) = new_token.get("access_token").and_then(|v| v.as_str()) {
                    cred.access_token = at.to_string();
                }
                if let Some(rt) = new_token.get("refresh_token").and_then(|v| v.as_str()) {
                    if !rt.is_empty() {
                        cred.refresh_token = rt.to_string();
                    }
                }
                if let Some(exp) = new_token.get("expires_in").and_then(|v| v.as_u64()) {
                    cred.expires_in = exp;
                    let exp_at = chrono::Utc::now() + chrono::Duration::seconds(exp as i64);
                    cred.expires_at = Some(exp_at.to_rfc3339());
                }
                // Re-serialize and update DB
                if let Ok(updated_json) = serde_json::to_string(cred) {
                    let _ = sqlx::query("UPDATE api_keys SET key_value = ?1, updated_at = datetime('now') WHERE id = ?2")
                        .bind(&updated_json).bind(key_id)
                        .execute(&*self.db).await;
                }
                tracing::info!("grok-cli key '{}' refreshed successfully", key_id);
                Ok(true)
            }
            Err(e) => {
                if e.starts_with("permanent:") {
                    tracing::error!("grok-cli key '{}' refresh permanent fail: {}", key_id, e);
                    Err(GatewayError::ProviderError(format!("grok-cli token refresh permanent: {}", e)))
                } else {
                    tracing::warn!("grok-cli key '{}' refresh temporary fail: {}", key_id, e);
                    // Temporary — let retry handle it
                    Err(GatewayError::ProviderError(format!("grok-cli token refresh failed: {}", e)))
                }
            }
        }
    }
}

#[async_trait]
impl Provider for GcliProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        for attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let mut cred = match GrokCliOAuthCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => {
                    self.keys.lock_key(&key.id, 401, e.to_string());
                    failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
            };

            // Auto-refresh if needed
            if let Err(e) = self.try_refresh(&mut cred, &key_id).await {
                if e.to_string().contains("permanent") {
                    self.keys.lock_key(&key.id, 401, e.to_string());
                    failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
                // Temporary — try anyway, might work
            }

            let body = self.mapper.to_response_request(request.clone());
            match self.client.send_collect(body, &cred).await {
                Ok(response) => {
                    self.keys.unlock(&key_id);
                    return Ok(ChatResult { response, used_key_id: Some(key_id), failed_keys: failed });
                }
                Err(e) => {
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    tracing::warn!("grok-cli key '{}' failed attempt {}/{}, kind={:?}", key_id, attempt + 1, total, c.kind);
                    if c.retryable {
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
            let mut cred = match GrokCliOAuthCredential::parse(&key.key_value) {
                Ok(c) => c,
                Err(e) => {
                    self.keys.lock_key(&key.id, 401, e.to_string());
                    failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
            };

            // Auto-refresh if needed
            if let Err(e) = self.try_refresh(&mut cred, &key_id).await {
                if e.to_string().contains("permanent") {
                    self.keys.lock_key(&key.id, 401, e.to_string());
                    failed.push(FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
            }

            let body = self.mapper.to_response_request(request.clone());
            match self.client.send_stream(body, &cred).await {
                Ok(stream) => {
                    self.keys.unlock(&key_id);
                    return Ok(ChatStreamResult { stream, used_key_id: Some(key_id), failed_keys: failed });
                }
                Err(e) => {
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    tracing::warn!("grok-cli key '{}' failed attempt {}/{}, kind={:?}", key_id, attempt + 1, total, c.kind);
                    if c.retryable {
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
        GrokCliOAuthCredential::parse(&key.key_value).map(|_| ())
    }
    fn locked_keys(&self) -> Vec<(String, u64, String)> { self.keys.locked_keys() }
    fn total_keys(&self) -> usize { self.keys.total_count() }
    fn active_keys(&self) -> usize { self.keys.active_count() }
}
