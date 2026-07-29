use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct TestModelRequest {
    pub model: String,
}

#[derive(Serialize)]
pub struct TestModelResponse {
    pub ok: bool,
    pub response: String,
    pub model: String,
    pub latency_ms: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub error: Option<String>,
}

/// POST /admin/api/providers/:id/test
///
/// Fire a one-shot chat request against the selected model to verify connectivity.
/// Uses the most recently created active API key.
pub async fn api_test_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<TestModelRequest>,
) -> Json<TestModelResponse> {
    let pm = state.provider_manager.read().await;
    let provider = match pm.get(&provider_id) {
        Some(p) => p,
        None => {
            return Json(TestModelResponse {
                ok: false,
                response: String::new(),
                model: req.model,
                latency_ms: 0,
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                error: Some("Provider not found".into()),
            });
        }
    };

    let chat_request = crate::types::chat::ChatCompletionRequest {
        model: req.model.clone(),
        messages: vec![crate::types::chat::Message {
            role: "user".into(),
            content: Some("Reply with exactly: Hello world".into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning_content: None,
        }],
        temperature: Some(0.0),
        max_tokens: Some(20),
        stream: Some(false),
        stream_options: None,
        tools: None,
        tool_choice: None,
        top_p: None,
    };

    let start = std::time::Instant::now();

    let known_keys: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM api_keys WHERE provider_id=? AND is_active=1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&provider_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let _db_key_id = known_keys.first().cloned();

    match provider.chat_completion(chat_request).await {
        Ok(result) => {
            let latency = start.elapsed().as_millis() as i64;
            let response_text = result
                .response
                .choices
                .first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let usage = result.response.usage.unwrap_or(crate::types::chat::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            });

            // Track test request in usage logs — use actual key from provider
            let tracked_usage = crate::services::usage_tracking::CanonicalUsage {
                prompt_tokens: usage.prompt_tokens as i64,
                completion_tokens: usage.completion_tokens as i64,
            };
            crate::services::usage_tracking::record_success(
                &state.usage_tracker,
                start,
                &provider_id,
                &req.model,
                "__test__",
                result.used_key_id.as_deref(),
                "/admin/api/providers/:id/test",
                tracked_usage,
                None,
            ).await;

            // Log failed key attempts
            for failed in &result.failed_keys {
                crate::services::usage_tracking::record_error(
                    &state.usage_tracker,
                    start,
                    &provider_id,
                    &req.model,
                    "__test__",
                    Some(&failed.key_id),
                    "/admin/api/providers/:id/test",
                    failed.error.http_status().unwrap_or(503) as i32,
                    &failed.error.to_string(),
                ).await;
            }

            Json(TestModelResponse {
                ok: true,
                response: response_text,
                model: req.model,
                latency_ms: latency,
                prompt_tokens: usage.prompt_tokens as i64,
                completion_tokens: usage.completion_tokens as i64,
                total_tokens: usage.total_tokens as i64,
                error: None,
            })
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as i64;

            // Track test failure in usage logs
            let err_status = e.http_status().unwrap_or(502) as i32;
            crate::services::usage_tracking::record_error(
                &state.usage_tracker,
                start,
                &provider_id,
                &req.model,
                "__test__",
                None,
                "/admin/api/providers/:id/test",
                err_status,
                &e.to_string(),
            ).await;

            Json(TestModelResponse {
                ok: false,
                response: String::new(),
                model: req.model,
                latency_ms: latency,
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                error: Some(e.to_string()),
            })
        }
    }
}
