use crate::error::GatewayError;
use crate::types::chat::{ChatCompletionChunk, ChatCompletionResponse};
use futures::stream::BoxStream;
use futures::StreamExt;
use futures::FutureExt;
use reqwest::Client;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use super::auth::FbAuthCredentials;
use super::constants;
use uuid::Uuid;

const WAIT_TIMEOUT_MS: u64 = 25_000;
const WAIT_POLL_MS: u64 = 2_000;

pub struct FbClient {
    http: Client,
}

impl FbClient {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            http: Client::builder()
                .connect_timeout(std::time::Duration::from_secs(timeout_secs))
                .timeout(std::time::Duration::from_secs(timeout_secs + 30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    fn headers(&self, builder: reqwest::RequestBuilder, cred: &FbAuthCredentials) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "ai-sdk/openai-compatible/0.0.0-test/codebuff ai-sdk/provider-utils/3.0.20 runtime/browser")
    }

    fn agent_headers(&self, builder: reqwest::RequestBuilder, cred: &FbAuthCredentials) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", cred.access_token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "Bun/1.3.11")
    }

    /// Start a run and return runId
    pub async fn start_run(&self, cred: &FbAuthCredentials, agent_id: &str) -> Result<String, GatewayError> {
        let url = format!("{}/api/v1/agent-runs", constants::API_BASE_URL);
        let resp = self
            .agent_headers(self.http.post(&url), cred)
            .json(&serde_json::json!({
                "action": "START",
                "agentId": agent_id,
                "ancestorRunIds": [],
            }))
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff start run: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("FreeBuff start run HTTP {}: {}", status, text.chars().take(200).collect::<String>())));
        }

        let data: Value = resp.json()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff start run parse: {}", e)))?;

        data["runId"].as_str()
            .or_else(|| data["data"]["runId"].as_str())
            .map(String::from)
            .ok_or_else(|| GatewayError::ProviderError("FreeBuff: missing runId in start run response".into()))
    }

    /// Record a run step
    pub async fn record_run_step(&self, cred: &FbAuthCredentials, run_id: &str, step_number: u32, child_run_ids: &[String]) -> Result<(), GatewayError> {
        let url = format!("{}/api/v1/agent-runs/{}/steps", constants::API_BASE_URL, run_id);
        let resp = self
            .agent_headers(self.http.post(&url), cred)
            .json(&serde_json::json!({
                "stepNumber": step_number,
                "credits": 0,
                "childRunIds": child_run_ids,
                "messageId": null,
                "status": "completed",
                "startTime": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            }))
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff record run step: {}", e)))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!("FreeBuff record run step {} failed: {}", run_id, text.chars().take(200).collect::<String>());
        }
        Ok(())
    }

    /// Finish a run
    pub async fn finish_run(&self, cred: &FbAuthCredentials, run_id: &str) -> Result<(), GatewayError> {
        let url = format!("{}/api/v1/agent-runs", constants::API_BASE_URL);
        let resp = self
            .agent_headers(self.http.post(&url), cred)
            .json(&serde_json::json!({
                "action": "FINISH",
                "runId": run_id,
                "status": "completed",
                "totalSteps": 2,
                "directCredits": 0,
                "totalCredits": 0,
            }))
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff finish run: {}", e)))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!("FreeBuff finish run failed: {}", text.chars().take(200).collect::<String>());
        }
        Ok(())
    }

    /// Create free session and return instanceId
    pub async fn create_free_session(&self, cred: &FbAuthCredentials, backend_model: &str) -> Result<String, GatewayError> {
        let url = format!("{}/api/v1/freebuff/session", constants::API_BASE_URL);
        let resp = self
            .agent_headers(self.http.post(&url), cred)
            .header("x-freebuff-model", backend_model)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff create session: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            // If 409 (conflict), try deleting current session first
            if status == 409 {
                tracing::warn!("FreeBuff session conflict, attempting cleanup");
                return Err(GatewayError::ProviderError(format!("FreeBuff session conflict: {}", text.chars().take(200).collect::<String>())));
            }
            return Err(GatewayError::ProviderError(format!("FreeBuff create session HTTP {}: {}", status, text.chars().take(200).collect::<String>())));
        }

        let data: Value = resp.json()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff create session parse: {}", e)))?;

        data["instanceId"].as_str()
            .map(String::from)
            .ok_or_else(|| GatewayError::ProviderError("FreeBuff: missing instanceId in session response".into()))
    }

    /// Get current session (no instance_id - returns whatever session exists)
    pub async fn get_current_session(&self, cred: &FbAuthCredentials) -> Result<Value, GatewayError> {
        let url = format!("{}/api/v1/freebuff/session", constants::API_BASE_URL);
        let resp = self
            .agent_headers(self.http.get(&url), cred)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff get current session: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("FreeBuff get current session HTTP {}: {}", code, text.chars().take(200).collect::<String>())));
        }

        let data: Value = resp.json()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff get current session parse: {}", e)))?;

        Ok(data)
    }

    /// Get free session status by instance_id
    pub async fn get_free_session_status(&self, cred: &FbAuthCredentials, instance_id: &str) -> Result<String, GatewayError> {
        let url = format!("{}/api/v1/freebuff/session", constants::API_BASE_URL);
        let resp = self
            .agent_headers(self.http.get(&url), cred)
            .header("x-freebuff-instance-id", instance_id)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff get session: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("FreeBuff get session HTTP {}: {}", code, text.chars().take(200).collect::<String>())));
        }

        let data: Value = resp.json()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff get session parse: {}", e)))?;

        data["status"].as_str()
            .map(String::from)
            .ok_or_else(|| GatewayError::ProviderError("FreeBuff: missing status in session response".into()))
    }

    /// Delete free session
    pub async fn delete_free_session(&self, cred: &FbAuthCredentials, instance_id: &str) -> Result<(), GatewayError> {
        let url = format!("{}/api/v1/freebuff/session", constants::API_BASE_URL);
        let resp = self
            .agent_headers(self.http.delete(&url), cred)
            .header("x-freebuff-instance-id", instance_id)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff delete session: {}", e)))?;

        if !resp.status().is_success() && resp.status().as_u16() != 404 {
            let text = resp.text().await.unwrap_or_default();
            tracing::warn!("FreeBuff delete session failed: {}", text.chars().take(200).collect::<String>());
        }
        Ok(())
    }

    /// Wait for free session to become active (poll with timeout)
    pub async fn wait_for_free_session(
        &self,
        cred: &FbAuthCredentials,
        instance_id: &str,
        backend_model: &str,
    ) -> Result<(), GatewayError> {
        let started = tokio::time::Instant::now();
        loop {
            if started.elapsed() > Duration::from_millis(WAIT_TIMEOUT_MS) {
                return Err(GatewayError::ProviderError("FreeBuff: session wait timeout".into()));
            }

            match self.get_free_session_status(cred, instance_id).await {
                Ok(status) => {
                    if status == "active" {
                        // Check model via get_current_session
                        if let Ok(current) = self.get_current_session(cred).await {
                            let model = current["model"].as_str().unwrap_or("");
                            if model.is_empty() || model == backend_model {
                                return Ok(());
                            }
                            // Wrong model, delete and let caller recreate
                            let _ = self.delete_free_session(cred, instance_id).await;
                            return Err(GatewayError::ProviderError("FreeBuff: session has wrong model, recreate needed".into()));
                        }
                        return Ok(());
                    }
                    if matches!(status.as_str(), "ended" | "superseded" | "none") {
                        return Err(GatewayError::ProviderError("FreeBuff: session ended/superseded, recreate needed".into()));
                    }
                    // Still queued/creating, keep polling
                }
                Err(_) => {
                    // Transient error, keep polling
                }
            }

            sleep(Duration::from_millis(WAIT_POLL_MS)).await;
        }
    }

    /// Ensure a free session is ready (create if needed, wait for active)
    pub async fn ensure_free_session(
        &self,
        cred: &FbAuthCredentials,
        backend_model: &str,
    ) -> Result<String, GatewayError> {
        let mut retried = false;
        loop {
        let instance_id = match self.create_free_session(cred, backend_model).await {
            Ok(id) => id,
            Err(e) => {
                // If conflict, get current session and delete it, then retry
                if e.to_string().contains("session conflict") || e.to_string().contains("409") {
                    if let Ok(current) = self.get_current_session(cred).await {
                        if let Some(existing_id) = current["instanceId"].as_str() {
                            let _ = self.delete_free_session(cred, existing_id).await;
                        }
                    }
                    self.create_free_session(cred, backend_model)
                        .await
                        .map_err(|e2| GatewayError::ProviderError(format!("FreeBuff: failed to create session after cleanup: {}", e2)))?
                } else {
                    return Err(e);
                }
            }
        };

        // Wait for it to become active
        match self.wait_for_free_session(cred, &instance_id, backend_model).await {
            Ok(()) => return Ok(instance_id),
            Err(e) => {
                if retried {
                    return Err(e);
                }
                // Superseded/ended — cleanup and retry once
                let _ = self.delete_free_session(cred, &instance_id).await;
                retried = true;
                continue;
            }
        }
        } // end loop
    }

    /// Build codebuff_metadata block
    pub fn metadata(&self, run_id: &str, instance_id: Option<&str>, trace_session_id: Option<&str>) -> Value {
        let mut meta = serde_json::json!({
            "run_id": run_id,
            "client_id": Uuid::new_v4().to_string(),
            "trace_session_id": trace_session_id.unwrap_or(&Uuid::new_v4().to_string()),
            "cost_mode": "free",
            "n": 1,
        });
        if let Some(iid) = instance_id {
            meta["freebuff_instance_id"] = serde_json::Value::String(iid.to_string());
        }
        meta
    }

    /// Validate agent definitions with FreeBuff (required for free tier)
    pub async fn validate_agents(&self, cred: &FbAuthCredentials) {
        let url = format!("{}/api/agents/validate", constants::API_BASE_URL);
        let agent_defs: Vec<Value> = constants::AGENT_BY_MODEL.iter().map(|(model_id, agent_id)| {
            serde_json::json!({
                "id": agent_id,
                "publisher": "codebuff",
                "model": model_id,
                "displayName": format!("Freebuff {}", model_id),
                "spawnerPrompt": "Freebuff OpenAI-compatible orchestrator",
                "inputSchema": {
                    "prompt": { "type": "string", "description": "A coding task to complete" },
                    "params": { "type": "object", "properties": {}, "required": [] }
                },
                "outputMode": "last_message",
                "includeMessageHistory": true,
                "toolNames": ["spawn_agents"],
                "spawnableAgents": [constants::CONTEXT_PRUNER_AGENT_ID],
                "systemPrompt": "Act as a helpful coding assistant.",
            })
        }).collect();

        let payload = serde_json::json!({
            "agentDefinitions": agent_defs
        });

        let resp = self
            .agent_headers(self.http.post(&url), cred)
            .json(&payload)
            .send()
            .await;

        if let Ok(r) = resp {
            if !r.status().is_success() {
                tracing::warn!("FreeBuff validate agents returned {}", r.status());
            }
        }
    }

    pub async fn send_stream(
        &self,
        body: Value,
        cred: &FbAuthCredentials,
    ) -> Result<BoxStream<'static, Result<ChatCompletionChunk, GatewayError>>, GatewayError> {
        if cred.access_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("FreeBuff: missing access_token".into()));
        }

        let url = format!("{}/api/v1/chat/completions", constants::API_BASE_URL);
        let response = self
            .headers(self.http.post(&url), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff HTTP: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("FreeBuff HTTP {}: {}", status, text.chars().take(200).collect::<String>())));
        }

        let buf = Arc::new(Mutex::new(String::new()));
        let stream = async_stream::stream! {
            use futures::StreamExt;
            let mut byte_stream = response.bytes_stream();
            while let Some(chunk) = byte_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => { yield Err(GatewayError::ProviderError(format!("FreeBuff stream: {}", e))); continue; }
                };
                let text = String::from_utf8_lossy(&chunk);

                // Parse complete lines, hold lock only during extraction
                let parsed: Vec<Result<ChatCompletionChunk, GatewayError>> = {
                    let mut buffer = buf.lock().unwrap();
                    buffer.push_str(&text);
                    let mut results = Vec::new();
                    loop {
                        if let Some(newline) = buffer.find('\n') {
                            let line = buffer[..newline].trim().to_string();
                            *buffer = buffer[newline + 1..].to_string();
                            if line.is_empty() { continue; }
                            if !line.starts_with("data:") { continue; }
                            let data = line[5..].trim();
                            if data.is_empty() || data == "[DONE]" {
                                results.push(Ok(ChatCompletionChunk {
                                    id: String::new(), object: String::new(), created: 0,
                                    model: String::new(), choices: vec![], usage: None,
                                }));
                                continue;
                            }
                            match serde_json::from_str::<ChatCompletionChunk>(data) {
                                Ok(chunk) => results.push(Ok(chunk)),
                                Err(e) => results.push(Err(GatewayError::ProviderError(format!("FreeBuff parse: {}", e)))),
                            }
                        } else {
                            break;
                        }
                    }
                    results
                };
                for result in parsed {
                    yield result;
                }
            }
        };

        Ok(Box::pin(stream))
    }

    pub async fn send_collect(
        &self,
        body: Value,
        cred: &FbAuthCredentials,
    ) -> Result<ChatCompletionResponse, GatewayError> {
        if cred.access_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("FreeBuff: missing access_token".into()));
        }

        let url = format!("{}/api/v1/chat/completions", constants::API_BASE_URL);
        let response = self
            .headers(self.http.post(&url), cred)
            .json(&body)
            .send()
            .await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff HTTP: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(GatewayError::ProviderError(format!("FreeBuff HTTP {}: {}", status, text.chars().take(200).collect::<String>())));
        }

        let full_text = response.text().await
            .map_err(|e| GatewayError::ProviderError(format!("FreeBuff read body: {}", e)))?;

        // Parse SSE from full text
        let mut content = String::new();
        let mut finish_reason = None;
        let mut usage = None;
        let mut resp_id = String::new();
        let mut resp_created: u64 = 0;
        let mut resp_model = String::new();

        for line in full_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if !trimmed.starts_with("data:") { continue; }
            let data = trimmed[5..].trim();
            if data.is_empty() || data == "[DONE]" { continue; }
            if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                if resp_id.is_empty() && !chunk.id.is_empty() { resp_id = chunk.id.clone(); }
                if resp_created == 0 && chunk.created > 0 { resp_created = chunk.created; }
                if resp_model.is_empty() && !chunk.model.is_empty() { resp_model = chunk.model.clone(); }
                if let Some(u) = &chunk.usage { usage = Some(u.clone()); }
                for choice in &chunk.choices {
                    if let Some(delta_content) = &choice.delta.content {
                        content.push_str(delta_content);
                    }
                    if let Some(fr) = &choice.finish_reason {
                        finish_reason = Some(fr.clone());
                    }
                }
            }
        }

        let finish = finish_reason.unwrap_or_else(|| "stop".to_string());

        Ok(ChatCompletionResponse {
            id: if resp_id.is_empty() { format!("chatcmpl-freebuff-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()) } else { resp_id },
            object: "chat.completion".to_string(),
            created: if resp_created == 0 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() } else { resp_created },
            model: if resp_model.is_empty() { "freebuff".to_string() } else { resp_model },
            choices: vec![
                crate::types::chat::Choice {
                    index: 0,
                    message: crate::types::chat::Message {
                        role: "assistant".to_string(),
                        content: Some(content),
                        tool_calls: None,
                        tool_call_id: None,
                        name: None,
                        reasoning_content: None,
                    },
                    finish_reason: Some(finish),
                }
            ],
            usage,
        })
    }
}
