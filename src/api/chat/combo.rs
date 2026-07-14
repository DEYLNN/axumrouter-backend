use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::response::IntoResponse;

use crate::error::GatewayError;
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

struct ComboConfig {
    tiers: Vec<serde_json::Value>,
}

async fn fetch_combo(db: &sqlx::SqlitePool, combo_name: &str) -> Result<ComboConfig, GatewayError> {
    let row = sqlx::query_as::<_, (String, String, String, bool, bool)>(
        "SELECT id, name, tiers, round_robin, is_active FROM combos WHERE name = ? OR id = ?"
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

    let (_id, _name, tiers_str, _round_robin, is_active) = row;
    if !is_active {
        return Err(GatewayError::ModelNotFound {
            provider: "combo".to_string(),
            model: combo_name.to_string(),
        });
    }

    let tiers = serde_json::from_str(&tiers_str)
        .map_err(|_| GatewayError::Internal("Invalid combo tiers".into()))?;
    Ok(ComboConfig { tiers })
}

/// Handle combo/xxx requests — non-streaming.
pub(crate) async fn handle_combo_request(
    state: Arc<AppState>,
    request: ChatCompletionRequest,
    combo_name: String,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let cfg = fetch_combo(&state.db, &combo_name).await?;
    let pm = state.provider_manager.read().await;
    let mut last_error = None;

    for tier_val in &cfg.tiers {
        let provider_id = tier_val["provider"].as_str().unwrap_or("");
        let model_id = tier_val["model"].as_str().unwrap_or("");

        if provider_id.is_empty() || model_id.is_empty() { continue; }

        let provider = match pm.get(provider_id) {
            Some(p) => p,
            None => continue,
        };

        if model_id == request.model { continue; }

        let mut tier_req = request.clone();
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
                ).await;
                return Ok(Json(result.response).into_response());
            }
            Err(e) => {
                let latency = start.elapsed().as_millis() as i64;
                let _ = crate::db::log_usage(
                    &state.db, provider_id, None,
                    &model_id, "error", Some(502),
                    0, 0, Some(latency), Some(e.to_string()), None, None,
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

    for tier_val in &cfg.tiers {
        let provider_id = tier_val["provider"].as_str().unwrap_or("");
        let model_id = tier_val["model"].as_str().unwrap_or("");

        if provider_id.is_empty() || model_id.is_empty() { continue; }
        let provider = match pm.get(provider_id) { Some(p) => p, None => continue };
        if model_id == request.model { continue; }

        let mut tier_req = request.clone();
        tier_req.model = model_id.to_string();

        match provider.chat_completion_stream(tier_req).await {
            Ok(result) => {
                let latency = start.elapsed().as_millis() as i64;
                let _ = crate::db::log_usage(
                    &state.db, provider_id,
                    result.used_key_id.as_deref(),
                    &model_id, "streaming", Some(200),
                    0, 0, Some(latency), None,
                    Some(serde_json::to_string(&request).unwrap_or_default()),
                    None,
                ).await;

                use axum::response::sse::Event;
                use futures::StreamExt;
                use std::convert::Infallible;

                let stream = result.stream.map(|chunk| {
                    match chunk {
                        Ok(c) => {
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
                    0, 0, Some(latency), Some(e.to_string()), None, None,
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
