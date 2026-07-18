use async_trait::async_trait;
use std::sync::Arc;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::engine::helpers::lock_key_on_error;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult, FailedKeyAttempt};
use crate::providers::traits::Provider;
use crate::types::chat::{ChatCompletionRequest, ChatCompletionResponse, ChatCompletionChunk};
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::constants;
use super::client::NpClient;

pub struct NpProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: Arc<NpClient>,
    db: Arc<sqlx::SqlitePool>,
}

impl NpProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>, db: Arc<sqlx::SqlitePool>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string(), "oauth".to_string()],
            icon_path: format!("/public/providers/{}.png", constants::PROVIDER_ID),
            category: constants::CATEGORY.to_string(),
            icon_name: constants::ICON_NAME.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: Some("device_code".to_string()),
        };
        Self {
            metadata,
            keys: KeyManager::new(keys),
            client: Arc::new(NpClient::new(constants::DEFAULT_TIMEOUT_SECS)),
            db,
        }
    }

    fn strip_prefix<'a>(&self, model: &'a str) -> &'a str {
        model.strip_prefix("np/").unwrap_or(model)
    }

    async fn try_refresh(&self, key: &ApiKey) -> Option<String> {
        let kv: serde_json::Value = serde_json::from_str(&key.key_value).ok()?;
        let access_token = kv["access_token"].as_str()?.to_string();
        if access_token.is_empty() { return None; }
        let needs_refresh = match kv["expires_at"].as_str() {
            Some(ea) => chrono::DateTime::parse_from_rfc3339(ea).map(|exp| chrono::Utc::now() > exp).unwrap_or(false),
            None => true,
        };
        if !needs_refresh { return Some(access_token); }
        let refresh_token = kv["refresh_token"].as_str()?;
        let tokens = super::oauth::refresh_token(refresh_token).await.ok()?;
        let new_at = tokens["access_token"].as_str()?.to_string();
        Some(new_at)
    }
}

#[async_trait]
impl Provider for NpProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        for _attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let token = match self.try_refresh(key).await {
                Some(t) => t,
                None => {
                    self.keys.lock_key(&key_id, 401, "No valid token".into());
                    failed.push(FailedKeyAttempt { key_id, error: GatewayError::ProviderError("No valid token".into()) });
                    continue;
                }
            };
            let mut body = serde_json::to_value(&request).map_err(|e| GatewayError::ProviderError(format!("Serialize: {e}")))?;
            body["model"] = serde_json::Value::String(self.strip_prefix(body["model"].as_str().unwrap_or("")).to_string());
            let resp = match self.client.chat(&token, body).await {
                Ok(r) => r,
                Err(e_msg) => {
                    let e = GatewayError::ProviderError(e_msg);
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    if c.retryable { failed.push(FailedKeyAttempt { key_id, error: e }); continue; }
                    return Err(e);
                }
            };
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                let err = GatewayError::ProviderError(format!("Nous {} — {}", status.as_u16(), &text[..text.len().min(300)]));
                let c = lock_key_on_error(&self.keys, &key_id, &err);
                if c.retryable { failed.push(FailedKeyAttempt { key_id, error: err }); continue; }
                return Err(err);
            }
            let completion: ChatCompletionResponse = serde_json::from_str(&text)
                .map_err(|e| GatewayError::ProviderError(format!("Parse: {e}")))?;
            return Ok(ChatResult { response: completion, used_key_id: Some(key_id), failed_keys: failed });
        }
        Err(GatewayError::NoAvailableKeys("All Nous Portal keys exhausted".into()))
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<ChatStreamResult, GatewayError> {
        let total = self.keys.total_count();
        let mut failed = Vec::new();
        for _attempt in 0..total.max(1) {
            let key = match self.keys.next() { Ok(k) => k, Err(_) => break };
            let key_id = key.id.clone();
            let token = match self.try_refresh(key).await {
                Some(t) => t,
                None => {
                    self.keys.lock_key(&key_id, 401, "No valid token".into());
                    failed.push(FailedKeyAttempt { key_id, error: GatewayError::ProviderError("No valid token".into()) });
                    continue;
                }
            };
            let mut body = serde_json::to_value(&request).map_err(|e| GatewayError::ProviderError(format!("Serialize: {e}")))?;
            body["model"] = serde_json::Value::String(self.strip_prefix(body["model"].as_str().unwrap_or("")).to_string());
            body["stream"] = serde_json::Value::Bool(true);
            let resp = match self.client.chat(&token, body).await {
                Ok(r) => r,
                Err(e_msg) => {
                    let e = GatewayError::ProviderError(e_msg);
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    if c.retryable { failed.push(FailedKeyAttempt { key_id, error: e }); continue; }
                    return Err(e);
                }
            };
            let stream_status = resp.status();
            if !stream_status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                let err = GatewayError::ProviderError(format!("Nous stream {} — {}", stream_status.as_u16(), &text[..text.len().min(300)]));
                let c = lock_key_on_error(&self.keys, &key_id, &err);
                if c.retryable { failed.push(FailedKeyAttempt { key_id, error: err }); continue; }
                return Err(err);
            }
            use futures::stream::StreamExt;
            let stream = resp.bytes_stream().filter_map(|item| {
                let bytes = match item { Ok(b) => b, Err(_) => return futures::future::ready(None) };
                let text = String::from_utf8_lossy(&bytes);
                if !text.starts_with("data: ") { return futures::future::ready(None); }
                let json_str = text.trim_start_matches("data: ").trim();
                if json_str == "[DONE]" { return futures::future::ready(None); }
                match serde_json::from_str::<ChatCompletionChunk>(json_str) {
                    Ok(c) => futures::future::ready(Some(Ok(c))),
                    Err(e) => futures::future::ready(Some(Err(GatewayError::ProviderError(format!("SSE: {e}"))))),
                }
            });
            return Ok(ChatStreamResult { stream: Box::pin(stream), used_key_id: Some(key_id), failed_keys: failed });
        }
        Err(GatewayError::NoAvailableKeys("All Nous Portal keys exhausted".into()))
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
        Ok(constants::models().iter().map(|m| Model {
            id: format!("np/{}", m.id), object: "model".to_string(), owned_by: "nous".to_string(),
            context_length: Some(m.max_tokens),
        }).collect())
    }

    async fn health_check(&self) -> Result<bool, GatewayError> {
        match self.keys.next() {
            Ok(key) => {
                let token = self.try_refresh(key).await.unwrap_or_default();
                let resp = self.client.chat(&token, serde_json::json!({
                    "model": "Hermes-3-Llama-3.2-3B", "messages": [{"role":"user","content":"hi"}], "max_tokens": 1
                })).await;
                Ok(resp.map_or(false, |r| r.status().is_success()))
            }
            Err(_) => Ok(false),
        }
    }

    async fn authenticate(&self) -> Result<(), GatewayError> { Ok(()) }
    fn locked_keys(&self) -> Vec<(String, u64, String)> { self.keys.locked_keys() }
    fn total_keys(&self) -> usize { self.keys.total_count() }
    fn active_keys(&self) -> usize { self.keys.active_count() }
}
