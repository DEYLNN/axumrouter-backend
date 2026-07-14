use std::sync::Arc;
use std::time::Instant;

use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use futures::StreamExt;
use std::convert::Infallible;

use crate::error::GatewayError;
use crate::services::usage_tracking::{estimate_prompt_tokens, estimate_tokens_from_chars};
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

/// Handle streaming chat completion.
pub(crate) async fn handle_streaming(
    state: &Arc<AppState>,
    gw_key: &crate::middleware::auth::GatewayKeyInfo,
    provider: &(dyn crate::providers::traits::Provider + Send + Sync),
    provider_id: &str,
    model: &str,
    provider_request: &ChatCompletionRequest,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let stream_result = provider.chat_completion_stream(provider_request.clone()).await;
    let latency_ms = start.elapsed().as_millis() as i64;

    match stream_result {
        Ok(chat_result) => {
            // Log failed key attempts
            for failed in &chat_result.failed_keys {
                let _ = crate::db::log_usage(
                    &state.db, provider_id,
                    Some(&failed.key_id),
                    model, "error", Some(401), 0, 0,
                    Some(latency_ms),
                    Some(failed.error.to_string()),
                    None, None,
                    None,
                ).await;
            }

            let usage_id = crate::db::log_usage(
                &state.db, provider_id,
                chat_result.used_key_id.as_deref(),
                model, "streaming", Some(200),
                0, 0,
                Some(latency_ms), None,
                Some(serde_json::to_string(provider_request).unwrap_or_default()),
                None,
                Some(&gw_key.key_id),
            ).await.unwrap_or_default();

            let db = state.db.clone();
            let usage_id = std::sync::Arc::new(usage_id);
            let prompt_tokens_est = estimate_prompt_tokens(provider_request);
            let accumulated_content = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

            let stream = chat_result.stream.then(move |chunk| {
                let db = db.clone();
                let usage_id = usage_id.clone();
                let accumulated_content = accumulated_content.clone();
                async move {
                    match chunk {
                        Ok(chunk) => {
                            for choice in &chunk.choices {
                                if let Some(content) = &choice.delta.content {
                                    if let Ok(mut acc) = accumulated_content.lock() {
                                        acc.push_str(content);
                                    }
                                }
                            }

                            let should_update = chunk.usage.is_some()
                                || chunk.choices.iter().any(|c| c.finish_reason.is_some());
                            if should_update && !usage_id.is_empty() {
                                let (pt, ct) = if let Some(usage) = &chunk.usage {
                                    (usage.prompt_tokens as i64, usage.completion_tokens as i64)
                                } else {
                                    let content_len = accumulated_content.lock().map(|s| s.len()).unwrap_or(0);
                                    (prompt_tokens_est, estimate_tokens_from_chars(content_len))
                                };
                                let response_body = serde_json::to_string(&chunk).ok();
                                let _ = crate::db::update_usage_tokens(&db, &usage_id, pt, ct, response_body).await;
                            }

                            let json = serde_json::to_string(&chunk).unwrap_or_default();
                            Ok::<_, Infallible>(Event::default().data(json))
                        }
                        Err(e) => {
                            let err_json = serde_json::json!({
                                "error": {
                                    "message": e.to_string(),
                                    "type": "stream_error",
                                    "code": "stream_error"
                                }
                            });
                            Ok::<_, Infallible>(Event::default().data(err_json.to_string()))
                        }
                    }
                }
            });

            let done = futures::stream::once(async {
                Ok::<_, Infallible>(Event::default().data("[DONE]"))
            });

            let sse = stream.chain(done);
            let mut response = Sse::new(sse)
                .keep_alive(axum::response::sse::KeepAlive::new()
                    .interval(std::time::Duration::from_secs(15))
                    .text("keep-alive"))
                .into_response();

            // Anti-buffer headers for CDN/proxy (Cloudflare, nginx, etc.)
            response.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("no-cache, no-transform"),
            );
            response.headers_mut().insert(
                axum::http::header::CONNECTION,
                axum::http::HeaderValue::from_static("keep-alive"),
            );
            response.headers_mut().insert(
                "X-Accel-Buffering",
                axum::http::HeaderValue::from_static("no"),
            );

            Ok(response)
        }
        Err(e) => {
            let _ = crate::db::log_usage(
                &state.db, provider_id, None,
                model, "error", None, 0, 0,
                Some(latency_ms),
                Some(e.to_string()),
                Some(serde_json::to_string(provider_request).unwrap_or_default()),
                None,
                None,
            ).await;
            Err(e)
        }
    }
}
