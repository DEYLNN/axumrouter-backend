use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::providers::result::{ChatResult, ChatStreamResult};
use crate::providers::traits::Provider;
use crate::types::chat::{ChatCompletionRequest, ChatCompletionResponse, ChatCompletionChunk};
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::constants;
use super::client::NpClient;

pub struct NpProvider {
    metadata: ProviderMetadata,
    keys: Arc<Mutex<Vec<ApiKey>>>,
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
            keys: Arc::new(Mutex::new(keys)),
            client: Arc::new(NpClient::new(constants::DEFAULT_TIMEOUT_SECS)),
            db,
        }
    }

    /// Ensure access token is fresh — refresh if expiring within 2 minutes.
    async fn ensure_fresh_token(&self) -> Result<(String, String), GatewayError> {
        let mut keys = self.keys.lock().await;
        for key in keys.iter_mut() {
            let mut kv: serde_json::Value = serde_json::from_str(&key.key_value).unwrap_or_default();
            let access_token = kv["access_token"].as_str().unwrap_or("").to_string();
            if access_token.is_empty() { continue; }

            let needs_refresh = match kv["expires_at"].as_str() {
                Some(ea) => {
                    if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(ea) {
                        chrono::Utc::now() > exp  // only when actually expired
                    } else { false }
                }
                None => false, // no expires_at, skip refresh
            };

            if !needs_refresh {
                return Ok((access_token, key.id.clone()));
            }

            // Refresh token
            let refresh_token = kv["refresh_token"].as_str().unwrap_or("").to_string();
            if refresh_token.is_empty() {
                return Err(GatewayError::ProviderError("Token expired and no refresh_token".into()));
            }

            let tokens = super::oauth::refresh_token(&refresh_token).await
                .map_err(|e| GatewayError::ProviderError(e))?;

            let new_at = tokens["access_token"].as_str().unwrap_or("").to_string();

            // Update in-memory
            kv["access_token"] = serde_json::Value::String(new_at.clone());
            if let Some(rt) = tokens["refresh_token"].as_str() {
                kv["refresh_token"] = serde_json::Value::String(rt.to_string());
            }
            if let Some(ei) = tokens["expires_in"].as_u64() {
                let exp_at = chrono::Utc::now() + chrono::Duration::seconds(ei as i64);
                kv["expires_at"] = serde_json::Value::String(exp_at.to_rfc3339());
            }
            kv["last_refresh"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());

            let updated = kv.to_string();
            key.key_value = updated.clone();

            // Persist to DB
            let _ = sqlx::query("UPDATE api_keys SET key_value = ?, updated_at = datetime('now') WHERE id = ?")
                .bind(&updated).bind(&key.id)
                .execute(&*self.db).await;

            return Ok((new_at, key.id.clone()));
        }
        Err(GatewayError::ProviderError("No valid access token".into()))
    }
}

#[async_trait]
impl Provider for NpProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let (token, key_id) = self.ensure_fresh_token().await?;
        let mut body = serde_json::to_value(&request).map_err(|e| GatewayError::ProviderError(format!("Serialize: {}", e)))?;

        if let Some(m) = body["model"].as_str() {
            if let Some(rest) = m.strip_prefix("np/") {
                body["model"] = serde_json::Value::String(rest.to_string());
            }
        }

        let resp = self.client.chat(&token, body).await
            .map_err(|e| GatewayError::ProviderError(e))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GatewayError::ProviderError(format!("Nous {} — {}", status.as_u16(), &text[..text.len().min(300)])));
        }

        let completion: ChatCompletionResponse = serde_json::from_str(&text)
            .map_err(|e| GatewayError::ProviderError(format!("Parse: {}", e)))?;

        Ok(ChatResult { response: completion, used_key_id: Some(key_id), failed_keys: vec![] })
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<ChatStreamResult, GatewayError> {
        let (token, key_id) = self.ensure_fresh_token().await?;
        let mut body = serde_json::to_value(&request).map_err(|e| GatewayError::ProviderError(format!("Serialize: {}", e)))?;
        if let Some(m) = body["model"].as_str() {
            if let Some(rest) = m.strip_prefix("np/") {
                body["model"] = serde_json::Value::String(rest.to_string());
            }
        }
        body["stream"] = serde_json::Value::Bool(true);

        let resp = self.client.chat(&token, body).await
            .map_err(|e| GatewayError::ProviderError(e))?;
        let stream_status = resp.status();
        if !stream_status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("Nous stream {} — {}", stream_status.as_u16(), &text[..text.len().min(300)])));
        }

        use futures::stream::StreamExt;
        let stream = resp.bytes_stream().filter_map(|item| {
            let bytes = match item {
                Ok(b) => b,
                Err(_) => return futures::future::ready(None),
            };
            let text = String::from_utf8_lossy(&bytes);
            if !text.starts_with("data: ") { return futures::future::ready(None); }
            let json_str = text.trim_start_matches("data: ").trim();
            if json_str == "[DONE]" { return futures::future::ready(None); }
            match serde_json::from_str::<ChatCompletionChunk>(json_str) {
                Ok(c) => futures::future::ready(Some(Ok(c))),
                Err(e) => futures::future::ready(Some(Err(GatewayError::ProviderError(format!("SSE: {}", e))))),
            }
        });

        Ok(ChatStreamResult { stream: Box::pin(stream), used_key_id: Some(key_id), failed_keys: vec![] })
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
        Ok(constants::MODELS.iter().map(|m| Model {
            id: format!("np/{}", m.id), object: "model".to_string(), owned_by: "nous".to_string(),
            context_length: Some(m.max_tokens),
        }).collect())
    }

    async fn health_check(&self) -> Result<bool, GatewayError> {
        match self.ensure_fresh_token().await {
            Ok((tok, _)) => {
                let resp = self.client.chat(&tok, serde_json::json!({
                    "model": "Hermes-3-Llama-3.2-3B", "messages": [{"role":"user","content":"hi"}], "max_tokens": 1
                })).await;
                Ok(resp.map_or(false, |r| r.status().is_success()))
            }
            Err(_) => Ok(false),
        }
    }

    async fn authenticate(&self) -> Result<(), GatewayError> { Ok(()) }
    fn total_keys(&self) -> usize { self.keys.try_lock().map(|k| k.len()).unwrap_or(0) }
    fn active_keys(&self) -> usize { self.keys.try_lock().map(|k| k.iter().filter(|x| x.is_active == 1).count()).unwrap_or(0) }
}
