use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::response::IntoResponse;

use crate::error::GatewayError;
use crate::middleware::auth::GatewayKeyInfo;
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

/// Handle non-streaming chat completion.
pub(crate) async fn handle_non_streaming(
    state: &Arc<AppState>,
    gw_key: &GatewayKeyInfo,
    provider: &(dyn crate::providers::traits::Provider + Send + Sync),
    provider_id: &str,
    model: &str,
    provider_request: &ChatCompletionRequest,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let result = provider.chat_completion(provider_request.clone()).await;
    let latency_ms = start.elapsed().as_millis() as i64;

    // Log failed key attempts
    if let Ok(chat_result) = &result {
        for failed in &chat_result.failed_keys {
            if let Err(e) = crate::db::log_usage(
                &state.db, provider_id,
                Some(&failed.key_id),
                model, "error", Some(401),
                0, 0, Some(latency_ms),
                Some(failed.error.to_string()),
                Some(serde_json::to_string(provider_request).unwrap_or_default()),
                None,
            ).await {
                tracing::error!("Failed to log failed-key usage: {}", e);
            }
        }
    }

    match &result {
        Ok(chat_result) => {
            let pt = chat_result.response.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0) as i64;
            let ct = chat_result.response.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0) as i64;

            crate::services::gateway::track_gateway_usage(&state.db, &gw_key.key_id, provider_id, model, pt, ct, latency_ms).await;

            if let Err(e) = crate::db::log_usage(
                &state.db, provider_id,
                chat_result.used_key_id.as_deref(),
                model, "success", Some(200),
                pt, ct, Some(latency_ms), None,
                Some(serde_json::to_string(provider_request).unwrap_or_default()),
                Some(serde_json::to_string(&chat_result.response).unwrap_or_default()),
            ).await {
                tracing::error!("Failed to log usage: {}", e);
            }
            Ok(Json(chat_result.response.clone()).into_response())
        }
        Err(e) => {
            if let Err(err) = crate::db::log_usage(
                &state.db, provider_id, None,
                model, "error", None, 0, 0,
                Some(latency_ms), Some(e.to_string()),
                Some(serde_json::to_string(provider_request).unwrap_or_default()),
                None,
            ).await {
                tracing::error!("Failed to log usage: {}", err);
            }
            // Return error as ProviderError (GatewayError doesn't impl Clone)
            Err(GatewayError::ProviderError(e.to_string()))
        }
    }
}
