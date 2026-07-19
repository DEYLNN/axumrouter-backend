use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;

use crate::db::models::ApiKey;
use crate::engine::anthropic_compat::auth::ApiKeyAuth;
use crate::engine::anthropic_compat::client::Client;
use crate::engine::anthropic_compat::config::AnthropicConfig;
use crate::engine::anthropic_compat::mapper::{Mapper, StreamState};
use crate::error::GatewayError;
use crate::providers::error_classifier::classify_provider_error;
use crate::engine::helpers::lock_key_on_error;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult};
use crate::providers::traits::Provider;
use crate::types::chat::ChatCompletionRequest;
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

pub struct AnthropicCompatibleProvider {
    config: Arc<AnthropicConfig>,
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: Client,
    mapper: Mapper,
}

impl AnthropicCompatibleProvider {
    pub fn new(config: AnthropicConfig, keys: Vec<ApiKey>) -> Self {
        let config = Arc::new(config);
        let metadata = ProviderMetadata {
            name: config.provider_id.to_string(),
            display_name: config.provider_name.to_string(),
            version: format!("{}/v1/messages", config.base_url),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string()],
            icon_path: format!("/public/providers/{}.png", config.provider_id),
            category: config.category.to_string(),
            icon_name: config.icon_name.to_string(),
            color: config.color.to_string(),
            oauth_flow: None,
            validate_url: config.validate_url.clone(),
        };

        Self {
            config: config.clone(),
            metadata,
            keys: KeyManager::new(keys),
            client: Client::new(config.clone()),
            mapper: Mapper::new(config),
        }
    }

    fn build_auth(&self, key: &ApiKey) -> Result<ApiKeyAuth, GatewayError> {
        if key.key_value.trim().is_empty() {
            return Err(GatewayError::ProviderError(format!(
                "Empty API key for {}",
                self.config.provider_id
            )));
        }
        Ok(ApiKeyAuth::new(key.key_value.clone()))
    }
}

#[async_trait]
impl Provider for AnthropicCompatibleProvider {
    fn metadata(&self) -> ProviderMetadata {
        self.metadata.clone()
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatResult, GatewayError> {
        let provider_req = self.mapper.to_provider_request(&request);
        let total = self.keys.total_count();
        let mut attempt = 0usize;

        loop {
            let key = match self.keys.next() {
                Ok(k) => k,
                Err(_) => return Err(GatewayError::ProviderError("All keys locked or no keys configured".into())),
            };
            let key_id = key.id.clone();
            let auth = match self.build_auth(key) {
                Ok(a) => a,
                Err(e) => { self.keys.lock_key(&key_id, 400, e.to_string()); continue; }
            };

            match self.client.chat_non_streaming(&auth, &provider_req).await {
                Ok(resp) => {
                    let gateway_resp = self.mapper.to_gateway_response(&resp);
                    return Ok(ChatResult { response: gateway_resp, used_key_id: Some(key_id), failed_keys: vec![] });
                }
                Err(e) => {
                    attempt += 1;
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    if c.retryable && attempt < total {
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatStreamResult, GatewayError> {
        let provider_req = self.mapper.to_provider_request(&request);
        let total = self.keys.total_count();
        let mut failed = Vec::new();

        for _attempt in 0..total.max(1) {
            let key = match self.keys.next() {
                Ok(k) => k,
                Err(_) => break,
            };
            let key_id = key.id.clone();
            let auth = match self.build_auth(key) {
                Ok(a) => a,
                Err(e) => {
                    self.keys.lock_key(&key_id, 400, e.to_string());
                    failed.push(crate::providers::result::FailedKeyAttempt { key_id: key_id.clone(), error: e });
                    continue;
                }
            };

            match self.client.chat_stream(&auth, &provider_req).await {
                Ok(resp) => {
                    let mapper = self.mapper.clone();
                    let config = self.config.clone();
                    let stream = async_stream::stream! {
                        let mut buffer = String::new();
                        let mut upstream = resp.bytes_stream();
                        let mut first_chunk_received = false;
                        let mut state = StreamState::new();
                        loop {
                            let timeout_dur = if !first_chunk_received {
                                std::time::Duration::from_secs(config.stream_first_chunk_timeout_secs)
                            } else {
                                std::time::Duration::from_secs(config.stream_stall_timeout_secs)
                            };
                            let next = tokio::time::timeout(timeout_dur, upstream.next()).await;
                            let chunk = match next {
                                Ok(Some(Ok(b))) => b,
                                Ok(Some(Err(e))) => {
                                    yield Err(GatewayError::ProviderError(format!("Stream read error: {}", e)));
                                    break;
                                }
                                Ok(None) => break,
                                Err(_) => {
                                    yield Err(GatewayError::ProviderError("Stream timeout".into()));
                                    break;
                                }
                            };
                            buffer.push_str(&String::from_utf8_lossy(&chunk));
                            // Process complete SSE frames
                            while let Some(frame_end) = buffer.find("\n\n") {
                                let frame = buffer[..frame_end].to_string();
                                buffer = buffer[frame_end + 2..].to_string();
                                for line in frame.lines() {
                                    let line = line.trim();
                                    if line.is_empty() { continue; }
                                    match mapper.parse_stream_event(line) {
                                        Ok(event) => {
                                            if !first_chunk_received { first_chunk_received = true; }
                                            let chunks = mapper.to_gateway_chunks(&event, &mut state);
                                            for c in chunks {
                                                yield Ok(c);
                                            }
                                        }
                                        Err(crate::error::GatewayError::ProviderError(ref msg)) if msg == "Stream done" => continue,
                                        Err(e) => {
                                            yield Err(e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    };
                    return Ok(ChatStreamResult {
                        stream: stream.boxed(),
                        used_key_id: Some(key_id),
                        failed_keys: failed,
                    });
                }
                Err(e) => {
                    let c = lock_key_on_error(&self.keys, &key_id, &e);
                    if c.retryable && _attempt + 1 < total.max(1) {
                        failed.push(crate::providers::result::FailedKeyAttempt { key_id: key_id.clone(), error: e });
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(GatewayError::ProviderError("All keys locked or no keys configured".into()))
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
        Ok(self.mapper.models_static())
    }

    async fn health_check(&self) -> Result<bool, GatewayError> {
        if self.keys.total_count() == 0 {
            return Ok(false);
        }
        let key = self.keys.next()?;
        let auth = self.build_auth(key)?;
        self.client.validate_auth(&auth).await.map(|_| true)
    }

    async fn authenticate(&self) -> Result<(), GatewayError> {
        if self.keys.total_count() == 0 {
            return Err(GatewayError::ProviderError(format!("No API keys configured for {}", self.config.provider_name)));
        }
        let key = self.keys.next()?;
        let auth = self.build_auth(key)?;
        self.client.validate_auth(&auth).await
    }

    fn locked_keys(&self) -> Vec<(String, u64, String)> { self.keys.locked_keys() }
    fn total_keys(&self) -> usize { self.keys.total_count() }
    fn active_keys(&self) -> usize { self.keys.active_count() }
}
