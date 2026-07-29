use async_trait::async_trait;
use std::sync::Arc;

use sqlx::SqlitePool;

use crate::db::models::ApiKey;
use crate::error::GatewayError;
use crate::providers::key_manager::KeyManager;
use crate::providers::result::{ChatResult, ChatStreamResult};
use crate::providers::traits::Provider;
use crate::types::chat::{ChatCompletionRequest, ChatCompletionChunk};
use crate::types::model::Model;
use crate::types::provider::ProviderMetadata;

use super::assemble::assemble_from_chunks;
use super::auth::FbAuthCredentials;
use super::body::build_request_body;
use super::client::FbClient;
use super::constants;
use super::session::{agent_id_for_model, resolve_backend_model, run_lifecycle};

pub struct FbProvider {
    metadata: ProviderMetadata,
    keys: KeyManager,
    client: FbClient,
}

impl FbProvider {
    pub fn new_with_keys(keys: Vec<ApiKey>, db: Arc<SqlitePool>) -> Self {
        let metadata = ProviderMetadata {
            name: constants::PROVIDER_ID.to_string(),
            display_name: constants::PROVIDER_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec!["chat".to_string(), "models".to_string(), "streaming".to_string()],
            icon_path: format!("/public/providers/{}.png", constants::PROVIDER_ID),
            category: constants::CATEGORY.to_string(),
            icon_name: constants::ICON_NAME.to_string(),
            color: constants::COLOR.to_string(),
            oauth_flow: Some("device_code".to_string()),
            validate_url: constants::VALIDATE_URL.to_string(),
            model_prefix: None,
        };
        Self {
            metadata,
            keys: KeyManager::new(keys, constants::PROVIDER_ID),
            client: FbClient::new(constants::DEFAULT_TIMEOUT_SECS),
        }
    }

    fn resolve_credential(&self, key: &ApiKey) -> Result<FbAuthCredentials, GatewayError> {
        let kv: serde_json::Value = serde_json::from_str(&key.key_value)
            .map_err(|_| GatewayError::ProviderError("FreeBuff: invalid key_value JSON".to_string()))?;
        let access_token = kv["access_token"]
            .as_str()
            .or_else(|| kv["accessToken"].as_str())
            .ok_or_else(|| GatewayError::ProviderError("FreeBuff: missing access_token".to_string()))?
            .to_string();
        Ok(FbAuthCredentials {
            access_token,
            email: kv["email"].as_str().map(String::from),
            account_id: kv["account_id"].as_str().or(kv["accountId"].as_str()).map(String::from),
            user_id: kv["user_id"].as_str().or(kv["userId"].as_str()).map(String::from),
            fingerprint_id: kv["fingerprint_id"].as_str().or(kv["fingerprintId"].as_str()).map(String::from),
            fingerprint_hash: kv["fingerprint_hash"].as_str().or(kv["fingerprintHash"].as_str()).map(String::from),
        })
    }

    fn models_static(&self) -> Vec<Model> {
        constants::MODELS.iter().map(|m| Model {
            id: format!("{}/{}", constants::PROVIDER_ID, m.id),
            object: "model".to_string(),
            owned_by: constants::PROVIDER_NAME.to_string(),
            context_length: Some(m.context_length),
        }).collect()
    }
}

#[async_trait]
impl Provider for FbProvider {
    fn metadata(&self) -> ProviderMetadata { self.metadata.clone() }
    fn total_keys(&self) -> usize { self.keys.total_count() }
    fn active_keys(&self) -> usize { self.keys.active_count() }
    fn locked_keys(&self) -> Vec<(String, u64, String)> { self.keys.locked_keys() }

    async fn health_check(&self) -> Result<bool, GatewayError> {
        Ok(self.keys.active_count() > 0)
    }

    async fn authenticate(&self) -> Result<(), GatewayError> {
        if self.keys.active_count() == 0 {
            return Err(GatewayError::ProviderError("FreeBuff: no active keys".to_string()));
        }
        Ok(())
    }

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatResult, GatewayError> {
        let total = self.keys.total_count();
        let mut excluded: Vec<String> = Vec::new();
        let mut failed: Vec<crate::providers::result::FailedKeyAttempt> = Vec::new();
        let mut last_error: Option<String> = None;
        let mut last_status: Option<u16> = None;

        for _attempt in 0..total.max(1) {
            let key = match self.keys.next_excluding(None, &excluded) {
                Ok(k) => k,
                Err(_) => break,
            };
            let key_id = key.id.clone();
            let cred = match self.resolve_credential(&key) {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    self.keys.lock_key(&key_id, 400, msg.clone());
                    excluded.push(key_id.clone());
                    last_error = Some(msg.clone());
                    last_status = Some(400);
                    failed.push(crate::providers::result::FailedKeyAttempt {
                        key_id: key_id.clone(),
                        error: crate::error::GatewayError::ProviderError(msg),
                    });
                    continue;
                }
            };
            let backend_model = resolve_backend_model(&request.model);
            let agent_id = agent_id_for_model(&backend_model);

            let mut retried = false;
            let result: Result<ChatResult, GatewayError> = loop {
                let (run_id, _child_run_id, instance_id) = match run_lifecycle(&self.client, &cred, agent_id, &backend_model)
                    .await
                {
                    Ok(x) => x,
                    Err(e) => {
                        self.keys.lock_key(&key_id, 500, e.to_string());
                        break Err(e);
                    }
                };

                let trace_session_id = uuid::Uuid::new_v4().to_string();
                let profile = constants::agentic_profile_for_backend(&backend_model);

                let body = match build_request_body(
                    &request, &backend_model, profile,
                    &self.client, &run_id, instance_id.as_deref(), &trace_session_id,
                ) {
                    Ok(b) => b,
                    Err(e) => break Err(e),
                };

                let orig_tool_count = body.get("tools").and_then(|t| t.as_array()).map(|a| a.len()).unwrap_or(0);
                let orig_msg_count = body.get("messages").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
                let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                tracing::info!(target: "freebuff", "chat: model={} tools={} messages={}", model_name, orig_tool_count, orig_msg_count);

                // ── retry on 409 (session_superseded) / 428 (session expired) ──
                let stream = match self.client.send_stream(body, &cred).await {
                    Ok(s) => s,
                    Err(e) => {
                        if !retried && (e.to_string().contains("409") || e.to_string().contains("428")) {
                            if let Some(ref iid) = instance_id {
                                let _ = self.client.delete_free_session(&cred, iid).await;
                            }
                            retried = true;
                            continue;
                        }
                        self.keys.lock_key(&key_id, 500, e.to_string());
                        break Err(e);
                    }
                };

                use futures::StreamExt;
                let chunks: Vec<ChatCompletionChunk> = stream
                    .filter_map(|r| futures::future::ready(r.ok()))
                    .collect()
                    .await;

                let response = assemble_from_chunks(&chunks, &request.model);

                // ── debug: log resp stats ──
                let fin = response.choices.first().and_then(|c| c.finish_reason.as_deref()).unwrap_or("?");
                let tool_calls = response.choices.first().and_then(|c| c.message.tool_calls.as_ref()).map(|t| t.len()).unwrap_or(0);
                let u = response.usage.as_ref();
                tracing::info!(target: "freebuff", "chat resp: finish={} tool_calls={} pt={} ct={}",
                    fin, tool_calls,
                    u.map(|u| u.prompt_tokens).unwrap_or(0),
                    u.map(|u| u.completion_tokens).unwrap_or(0));

                let _ = self.client.record_run_step(&cred, &run_id, 2, &[]).await;
                let _ = self.client.finish_run(&cred, &run_id).await;

                self.keys.mark_success(&key_id);
                break Ok(ChatResult {
                    response,
                    used_key_id: Some(key_id.clone()),
                    failed_keys: failed.clone(),
                });
            };

            match result {
                Ok(ok) => return Ok(ok),
                Err(e) => {
                    let msg = e.to_string();
                    let st = 500u16;
                    last_error = Some(msg.clone());
                    last_status = Some(st);
                    failed.push(crate::providers::result::FailedKeyAttempt {
                        key_id: key_id.clone(), error: e,
                    });
                    excluded.push(key_id);
                    continue;
                }
            }
        }
        return Err(GatewayError::ProviderHttpError {
            status: last_status.unwrap_or(503),
            body: last_error.unwrap_or_default(),
            provider: self.metadata.name.clone(),
            key_id: None,
        });
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatStreamResult, GatewayError> {
        let total = self.keys.total_count();
        let mut excluded: Vec<String> = Vec::new();
        let mut failed = Vec::new();
        let mut last_error: Option<String> = None;
        let mut last_status: Option<u16> = None;

        for _attempt in 0..total.max(1) {
            let key = match self.keys.next_excluding(None, &excluded) {
                Ok(k) => k,
                Err(_) => break,
            };
            let key_id = key.id.clone();
            let cred = match self.resolve_credential(&key) {
                Ok(c) => c,
                Err(e) => {
                    let msg = e.to_string();
                    self.keys.lock_key(&key_id, 400, msg.clone());
                    excluded.push(key_id.clone());
                    last_error = Some(msg.clone());
                    last_status = Some(400);
                    failed.push(crate::providers::result::FailedKeyAttempt {
                        key_id: key_id.clone(),
                        error: crate::error::GatewayError::ProviderError(msg),
                    });
                    continue;
                }
            };
            let backend_model = resolve_backend_model(&request.model);
            let agent_id = agent_id_for_model(&backend_model);

            let mut retried = false;
            let result: Result<ChatStreamResult, GatewayError> = loop {
                let (run_id, _child_run_id, instance_id) = match run_lifecycle(&self.client, &cred, agent_id, &backend_model)
                    .await
                {
                    Ok(x) => x,
                    Err(e) => {
                        self.keys.lock_key(&key_id, 500, e.to_string());
                        break Err(e);
                    }
                };

                let trace_session_id = uuid::Uuid::new_v4().to_string();
                let profile = constants::agentic_profile_for_backend(&backend_model);

                let body = match build_request_body(
                    &request, &backend_model, profile,
                    &self.client, &run_id, instance_id.as_deref(), &trace_session_id,
                ) {
                    Ok(b) => b,
                    Err(e) => break Err(e),
                };

                let orig_tool_count = body.get("tools").and_then(|t| t.as_array()).map(|a| a.len()).unwrap_or(0);
                let orig_msg_count = body.get("messages").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
                let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("?");
                tracing::info!(target: "freebuff", "stream: model={} tools={} messages={}", model_name, orig_tool_count, orig_msg_count);

                // ── retry on 409 (session_superseded) ──
                let stream = match self.client.send_stream(body, &cred).await {
                    Ok(s) => s,
                    Err(e) => {
                        if !retried && (e.to_string().contains("409") || e.to_string().contains("428")) {
                            if let Some(ref iid) = instance_id {
                                let _ = self.client.delete_free_session(&cred, iid).await;
                            }
                            retried = true;
                            continue;
                        }
                        self.keys.lock_key(&key_id, 500, e.to_string());
                        break Err(e);
                    }
                };

                let _ = self.client.record_run_step(&cred, &run_id, 2, &[]).await;
                let _ = self.client.finish_run(&cred, &run_id).await;

                self.keys.mark_success(&key_id);
                break Ok(ChatStreamResult {
                    stream,
                    used_key_id: Some(key_id.clone()),
                    failed_keys: failed.clone(),
                    last_attempted_key_id: Some(key_id.clone()),
                });
            };

            match result {
                Ok(ok) => return Ok(ok),
                Err(e) => {
                    let msg = e.to_string();
                    last_error = Some(msg.clone());
                    last_status = Some(500);
                    failed.push(crate::providers::result::FailedKeyAttempt {
                        key_id: key_id.clone(), error: e,
                    });
                    excluded.push(key_id);
                    continue;
                }
            }
        }
        return Err(GatewayError::ProviderHttpError {
            status: last_status.unwrap_or(503),
            body: last_error.unwrap_or_default(),
            provider: self.metadata.name.clone(),
            key_id: None,
        });
    }

    async fn list_models(&self) -> Result<Vec<Model>, GatewayError> {
        Ok(self.models_static())
    }
}
