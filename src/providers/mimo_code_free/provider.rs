use async_trait::async_trait;
use std::sync::Arc;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::providers::result::{ChatResult, ChatStreamResult};
use crate::providers::traits::Provider;
use crate::types::chat::{ChatCompletionRequest, ChatCompletionResponse, ChatCompletionChunk};
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::client::McfClient;
use super::constants;

pub struct McfProvider {
    metadata: ProviderMetadata,
    client: Arc<McfClient>,
}

impl McfProvider {
    pub fn new_with_keys(_keys: Vec<ApiKey>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string()],
            icon_path: format!("/public/providers/{}.png", constants::PROVIDER_ID),
            category: constants::CATEGORY.to_string(),
            icon_url: constants::ICON_URL.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: None,
        };
        Self {
            metadata,
            client: Arc::new(McfClient::new(constants::DEFAULT_TIMEOUT_SECS)),
        }
    }
}

#[async_trait]
impl Provider for McfProvider {
    fn metadata(&self) -> ProviderMetadata {
        self.metadata.clone()
    }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatResult, GatewayError> {
        let mut body = serde_json::to_value(&request)
            .map_err(|e| GatewayError::ProviderError(format!("Serialize: {}", e)))?;
        // Strip provider prefix from model name
        if let Some(model) = body.get("model").and_then(|v| v.as_str()) {
            let short = model.split('/').nth(1).unwrap_or(model);
            body["model"] = serde_json::Value::String(short.to_string());
        }

        let resp = self.client.chat(body).await
            .map_err(|e| GatewayError::ProviderError(e))?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(GatewayError::ProviderError(format!("MiMo {} — {}", status.as_u16(), &text[..text.len().min(300)])));
        }

        // MiMo returns SSE format even for non-streaming — strip "data:" prefix
        let json_str = if let Some(stripped) = text.strip_prefix("data:") {
            stripped.trim()
        } else {
            &text
        };

        let completion: ChatCompletionResponse = serde_json::from_str(json_str)
            .map_err(|e| GatewayError::ProviderError(format!("MiMo parse: {} — {}", e, &json_str[..json_str.len().min(200)])))?;

        Ok(ChatResult {
            response: completion,
            used_key_id: None,
            failed_keys: vec![],
        })
    }

    async fn chat_completion_stream(&self, request: ChatCompletionRequest) -> Result<ChatStreamResult, GatewayError> {
        let mut body = serde_json::to_value(&request)
            .map_err(|e| GatewayError::ProviderError(format!("Serialize: {}", e)))?;
        // Strip provider prefix from model name
        if let Some(model) = body.get("model").and_then(|v| v.as_str()) {
            let short = model.split('/').nth(1).unwrap_or(model);
            body["model"] = serde_json::Value::String(short.to_string());
        }
        body["stream"] = serde_json::Value::Bool(true);

        let resp = self.client.chat(body).await
            .map_err(|e| GatewayError::ProviderError(e))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("MiMo stream {} — {}", status.as_u16(), &text[..text.len().min(300)])));
        }

        // Convert reqwest byte stream to SSE chunk stream
        use futures::stream::StreamExt;
        let byte_stream = resp.bytes_stream();
        let chunk_stream = byte_stream.filter_map(|item| {
            let bytes = match item {
                Ok(b) => b,
                Err(_) => return futures::future::ready(None),
            };
            let text = String::from_utf8_lossy(&bytes).to_string();
            if !text.starts_with("data: ") {
                return futures::future::ready(None);
            }
            let json_str = text.trim_start_matches("data: ").trim().to_string();
            if json_str == "[DONE]" {
                return futures::future::ready(None);
            }
            match serde_json::from_str::<ChatCompletionChunk>(&json_str) {
                Ok(chunk) => futures::future::ready(Some(Ok(chunk))),
                Err(e) => futures::future::ready(Some(Err(
                    GatewayError::ProviderError(format!("MiMo SSE: {}", e))
                ))),
            }
        });

        Ok(ChatStreamResult {
            stream: Box::pin(chunk_stream),
            used_key_id: None,
            failed_keys: vec![],
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
        Ok(constants::MODELS.iter().map(|m| Model {
            id: format!("{}/{}", constants::MODEL_PREFIX, m.id),
            object: "model".to_string(),
            owned_by: constants::PROVIDER_ID.to_string(),
        }).collect())
    }

    async fn health_check(&self) -> Result<bool, GatewayError> {
        // Try bootstrap to verify connectivity
        match self.client.chat(serde_json::json!({
            "model": "mimo-auto",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 1,
        })).await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn authenticate(&self) -> Result<(), GatewayError> {
        Ok(()) // No auth needed (JWT bootstrap is per-request)
    }

    fn total_keys(&self) -> usize { 1 } // dummy
    fn active_keys(&self) -> usize { 1 } // dummy
}
