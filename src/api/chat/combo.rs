use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::response::IntoResponse;

use crate::error::GatewayError;
use crate::services::usage_tracking::{estimate_prompt_tokens, estimate_tokens_from_chars};
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

struct ComboConfig {
    tiers: Vec<serde_json::Value>,
    round_robin: bool,
    min_context: u64,
}

async fn fetch_combo(db: &sqlx::SqlitePool, combo_name: &str) -> Result<ComboConfig, GatewayError> {
    let row = sqlx::query_as::<_, (String, String, String, bool, bool, u64)>(
        "SELECT id, name, tiers, round_robin, is_active, min_context FROM combos WHERE name = ? OR id = ?"
    )
    .bind(combo_name)
    .bind(combo_name)
    .fetch_optional(db)
    .await
    .map_err(|_| GatewayError::Internal("DB error".into()))?
    .ok_or_else(|| GatewayError::ModelNotFound {
        provider: "combo".to_string(),
        model: combo_name.to_string(),
    })?;

    let (_id, _name, tiers_str, round_robin, is_active, min_context) = row;
    if !is_active {
        return Err(GatewayError::ModelNotFound {
            provider: "combo".to_string(),
            model: combo_name.to_string(),
        });
    }

    let tiers = serde_json::from_str(&tiers_str)
        .map_err(|_| GatewayError::Internal("Invalid combo tiers".into()))?;
    Ok(ComboConfig { tiers, round_robin, min_context })
}

/// Handle combo/xxx requests �� non-streaming.
pub(crate) async fn handle_combo_request(
    state: Arc<AppState>,
    request: ChatCompletionRequest,
    combo_name: String,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let cfg = fetch_combo(&state.db, &combo_name).await?;
    let pm = state.provider_manager.read().await;
    let mut last_error = None;

    let mut tier_order: Vec<usize> = (0..cfg.tiers.len()).collect();
    if cfg.round_robin {
        use rand::seq::SliceRandom;
        tier_order.shuffle(&mut rand::thread_rng());
    }

    for idx in tier_order {
        let tier_val = &cfg.tiers[idx];
        let provider_id = tier_val["provider"].as_str().unwrap_or("");
        let model_id = tier_val["model"].as_str().unwrap_or("");

        if provider_id.is_empty() || model_id.is_empty() { continue; }

        let provider = match pm.get(provider_id) {
            Some(p) => p,
            None => continue,
        };

        // Clamp max_tokens to min_context to prevent overflow on weakest tier
        let mut tier_req = request.clone();
        if cfg.min_context > 0 {
            if let Some(mt) = tier_req.max_tokens {
                if mt as u64 > cfg.min_context {
                    tier_req.max_tokens = Some(cfg.min_context as u32);
                }
            } else {
                tier_req.max_tokens = Some(cfg.min_context as u32);
            }
        }
        tier_req.model = model_id.to_string();

        match provider.chat_completion(tier_req).await {
            Ok(result) => {
                let latency = start.elapsed().as_millis() as i64;
                let _ = crate::db::log_usage(
                    &state.db, provider_id,
                    result.used_key_id.as_deref(),
                    &model_id, "success", Some(200),
                    result.response.usage.as_ref().map(|u| u.prompt_tokens as i64).unwrap_or(0),
                    result.response.usage.as_ref().map(|u| u.completion_tokens as i64).unwrap_or(0),
                    Some(latency), None,
                    Some(serde_json::to_string(&request).unwrap_or_default()),
                    Some(serde_json::to_string(&result.response).unwrap_or_default()),
                    None,
                ).await;
                return Ok(Json(result.response).into_response());
            }
            Err(e) => {
                let latency = start.elapsed().as_millis() as i64;
                let _ = crate::db::log_usage(
                    &state.db, provider_id, None,
                    &model_id, "error", Some(502),
                    0, 0, Some(latency), Some(e.to_string()), None, None, None,
                ).await;
                last_error = Some(e);
            }
        }
    }

    let err_msg = match last_error {
        Some(e) => format!("All combo tiers failed. Last error: {}", e),
        None => "All combo tiers failed: no tiers defined or all skipped".into(),
    };
    Err(GatewayError::ProviderError(err_msg))
}

/// Handle combo/xxx streaming requests — iterate through tiers.
pub(crate) async fn handle_combo_request_stream(
    state: Arc<AppState>,
    request: ChatCompletionRequest,
    combo_name: String,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let cfg = fetch_combo(&state.db, &combo_name).await?;
    let pm = state.provider_manager.read().await;
    let mut last_error = None;

    let mut tier_order: Vec<usize> = (0..cfg.tiers.len()).collect();
    if cfg.round_robin {
        use rand::seq::SliceRandom;
        tier_order.shuffle(&mut rand::thread_rng());
    }

    for idx in tier_order {
        let tier_val = &cfg.tiers[idx];
        let provider_id = tier_val["provider"].as_str().unwrap_or("");
        let model_id = tier_val["model"].as_str().unwrap_or("");

        if provider_id.is_empty() || model_id.is_empty() { continue; }
        let provider = match pm.get(provider_id) { Some(p) => p, None => continue };

        // Clamp max_tokens to min_context
        let mut tier_req = request.clone();
        if cfg.min_context > 0 {
            if let Some(mt) = tier_req.max_tokens {
                if mt as u64 > cfg.min_context {
                    tier_req.max_tokens = Some(cfg.min_context as u32);
                }
            } else {
                tier_req.max_tokens = Some(cfg.min_context as u32);
            }
        }
        tier_req.model = model_id.to_string();

        match provider.chat_completion_stream(tier_req).await {
            Ok(result) => {
                let latency = start.elapsed().as_millis() as i64;
                let usage_id = crate::db::log_usage(
                    &state.db, provider_id,
                    result.used_key_id.as_deref(),
                    &model_id, "streaming", Some(200),
                    0, 0, Some(latency), None,
                    Some(serde_json::to_string(&request).unwrap_or_default()),
                    None,
                    None,
                ).await.unwrap_or_default();

                use axum::response::sse::Event;
                use futures::StreamExt;
                use std::convert::Infallible;

                let db = state.db.clone();
                let usage_id = std::sync::Arc::new(usage_id);
                let prompt_tokens_est = estimate_prompt_tokens(&request);
                let accumulated_content = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

                let stream = result.stream.then(move |chunk| {
                    let db = db.clone();
                    let usage_id = usage_id.clone();
                    let accumulated_content = accumulated_content.clone();
                    async move {
                        match chunk {
                            Ok(c) => {
                                for choice in &c.choices {
                                    if let Some(content) = &choice.delta.content {
                                        if let Ok(mut acc) = accumulated_content.lock() {
                                            acc.push_str(content);
                                        }
                                    }
                                }

                                let should_update = c.usage.is_some()
                                    || c.choices.iter().any(|c| c.finish_reason.is_some());
                                if should_update && !usage_id.is_empty() {
                                    let (pt, ct) = if let Some(usage) = &c.usage {
                                        (usage.prompt_tokens as i64, usage.completion_tokens as i64)
                                    } else {
                                        let content_len = accumulated_content.lock().map(|s| s.len()).unwrap_or(0);
                                        (prompt_tokens_est, estimate_tokens_from_chars(content_len))
                                    };
                                    let response_body = serde_json::to_string(&c).ok();
                                    let _ = crate::db::update_usage_tokens(&db, &usage_id, pt, ct, response_body).await;
                                }

                                let json = serde_json::to_string(&c).unwrap_or_default();
                                Ok::<_, Infallible>(Event::default().data(json))
                            }
                            Err(e) => {
                                let err_json = serde_json::json!({
                                    "error": {"message": e.to_string(), "type": "stream_error", "code": "stream_error"}
                                });
                                Ok::<_, Infallible>(Event::default().data(err_json.to_string()))
                            }
                        }
                    }
                });

                let done = futures::stream::once(async { Ok::<_, Infallible>(Event::default().data("[DONE]")) });
                let sse = stream.chain(done);
                let mut resp = axum::response::Sse::new(sse)
                    .keep_alive(axum::response::sse::KeepAlive::new()
                        .interval(std::time::Duration::from_secs(15))
                        .text("keep-alive"))
                    .into_response();
                resp.headers_mut().insert(
                    axum::http::header::CACHE_CONTROL,
                    axum::http::HeaderValue::from_static("no-cache, no-transform"),
                );
                resp.headers_mut().insert(
                    axum::http::header::CONNECTION,
                    axum::http::HeaderValue::from_static("keep-alive"),
                );
                resp.headers_mut().insert(
                    "X-Accel-Buffering",
                    axum::http::HeaderValue::from_static("no"),
                );
                return Ok(resp);
            }
            Err(e) => {
                let latency = start.elapsed().as_millis() as i64;
                let _ = crate::db::log_usage(
                    &state.db, provider_id, None,
                    &model_id, "error", Some(502),
                    0, 0, Some(latency), Some(e.to_string()), None, None, None,
                ).await;
                last_error = Some(e);
            }
        }
    }

    let err_msg = match last_error {
        Some(e) => format!("All combo tiers failed. Last error: {}", e),
        None => "All combo tiers failed: no tiers defined or all skipped".into(),
    };
    Err(GatewayError::ProviderError(err_msg))
}
