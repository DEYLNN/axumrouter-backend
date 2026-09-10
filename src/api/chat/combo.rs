use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::response::IntoResponse;

use crate::error::GatewayError;
use crate::middleware::auth::GatewayKeyInfo;
use crate::services::usage_tracking::{CanonicalUsage, record_error, record_success};
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

struct ComboConfig {
    tiers: Vec<String>,
    strategy: String,
    min_context: i64,
}

async fn fetch_combo(db: &sqlx::SqlitePool, combo_name: &str) -> Result<ComboConfig, GatewayError> {
    let row = sqlx::query_as::<_, (String, String, bool, i64)>(
        "SELECT tiers, strategy, is_active, min_context FROM combos WHERE name = ? OR id = ?"
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

    let (tiers_str, strategy, is_active, min_context) = row;
    if !is_active {
        return Err(GatewayError::ModelNotFound {
            provider: "combo".to_string(),
            model: combo_name.to_string(),
        });
    }

    let tiers: Vec<String> = serde_json::from_str(&tiers_str)
        .map_err(|_| GatewayError::Internal("Invalid combo tiers".into()))?;

    if tiers.is_empty() {
        return Err(GatewayError::Internal("Combo has no tiers".into()));
    }

    Ok(ComboConfig { tiers, strategy, min_context })
}

/// Handle combo/xxx requests — non-streaming.
/// Resolves combo to real provider+model, dispatches. Usage tracked under real provider.
pub(crate) async fn handle_combo_request(
    state: Arc<AppState>,
    gw_key: GatewayKeyInfo,
    request: ChatCompletionRequest,
    combo_name: String,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let cfg = fetch_combo(&state.db, &combo_name).await?;
    let pm = state.provider_manager.read().await;
    let mut last_error = None;

    let mut tier_order: Vec<usize> = (0..cfg.tiers.len()).collect();
    if cfg.strategy == "round_robin" {
        use rand::seq::SliceRandom;
        tier_order.shuffle(&mut rand::thread_rng());
    }

    for idx in tier_order {
        let tier_model = &cfg.tiers[idx];
        let (raw_prefix, model_name) = match tier_model.split_once('/') {
            Some((p, m)) => (p, m),
            None => continue,
        };

        let provider_id = match pm.resolve_provider_id(raw_prefix) {
            Some(id) => id,
            None => continue,
        };
        let provider = match pm.get(&provider_id) {
            Some(p) => p,
            None => continue,
        };

        let mut tier_req = request.clone();
        tier_req.model = model_name.to_string();

        // Clamp max_tokens to min_context
        if cfg.min_context > 0 {
            if let Some(mt) = tier_req.max_tokens {
                if mt as i64 > cfg.min_context {
                    tier_req.max_tokens = Some(cfg.min_context as u32);
                }
            }
        }

        match provider.chat_completion(tier_req).await {
            Ok(result) => {
                let usage = match &result.response.usage {
                    Some(u) => CanonicalUsage {
                        prompt_tokens: u.prompt_tokens as i64,
                        completion_tokens: u.completion_tokens as i64,
                    },
                    None => CanonicalUsage::default(),
                };
                record_success(
                    &state.usage_tracker,
                    start,
                    &provider_id,
                    tier_model,
                    &gw_key.key_id,
                    result.used_key_id.as_deref(),
                    "/v1/chat/completions",
                    usage,
                    None,
                ).await;
                return Ok(Json(result.response).into_response());
            }
            Err(e) => {
                let status = e.http_status().unwrap_or(502);
                record_error(
                    &state.usage_tracker,
                    start,
                    &provider_id,
                    tier_model,
                    &gw_key.key_id,
                    None,
                    "/v1/chat/completions",
                    status as i32,
                    &e.to_string(),
                ).await;
                last_error = Some(e);
            }
        }
    }

    drop(pm);
    let err_msg = match last_error {
        Some(e) => format!("All combo tiers failed. Last error: {}", e),
        None => "All combo tiers failed: no tiers defined or all skipped".into(),
    };
    Err(GatewayError::ProviderError(err_msg))
}

/// Handle combo/xxx streaming requests.
/// Resolves to a real provider and delegates to the standard streaming handler.
pub(crate) async fn handle_combo_request_stream(
    state: Arc<AppState>,
    gw_key: GatewayKeyInfo,
    request: ChatCompletionRequest,
    combo_name: String,
    start: Instant,
) -> Result<axum::response::Response, GatewayError> {
    let cfg = fetch_combo(&state.db, &combo_name).await?;
    let pm = state.provider_manager.read().await;
    let mut last_error = None;

    let mut tier_order: Vec<usize> = (0..cfg.tiers.len()).collect();
    if cfg.strategy == "round_robin" {
        use rand::seq::SliceRandom;
        tier_order.shuffle(&mut rand::thread_rng());
    }

    for idx in tier_order {
        let tier_model = &cfg.tiers[idx];
        let (raw_prefix, model_name) = match tier_model.split_once('/') {
            Some((p, m)) => (p, m),
            None => continue,
        };

        let provider_id = match pm.resolve_provider_id(raw_prefix) {
            Some(id) => id,
            None => continue,
        };
        let provider = match pm.get(&provider_id) {
            Some(p) => p,
            None => continue,
        };

        let mut tier_req = request.clone();
        tier_req.model = model_name.to_string();
        tier_req.stream = Some(true);
        if tier_req.stream_options.is_none() {
            tier_req.stream_options = Some(serde_json::json!({"include_usage": true}));
        }

        // Clamp max_tokens to min_context
        if cfg.min_context > 0 {
            if let Some(mt) = tier_req.max_tokens {
                if mt as i64 > cfg.min_context {
                    tier_req.max_tokens = Some(cfg.min_context as u32);
                }
            }
        }

        match provider.chat_completion_stream(tier_req).await {
            Ok(result) => {
                // Delegate to the standard streaming handler — it handles
                // SSE wrapping, usage tracking, and response formatting.
                // pm stays borrowed; handle_streaming takes a reference.
                return crate::api::chat::streaming::handle_streaming(
                    &state, &gw_key, provider, &provider_id, tier_model, &request, start,
                ).await;
            }
            Err(e) => {
                let status = e.http_status().unwrap_or(502);
                record_error(
                    &state.usage_tracker,
                    start,
                    &provider_id,
                    tier_model,
                    &gw_key.key_id,
                    None,
                    "/v1/chat/completions",
                    status as i32,
                    &e.to_string(),
                ).await;
                last_error = Some(e);
            }
        }
    }

    drop(pm);
    let err_msg = match last_error {
        Some(e) => format!("All combo tiers failed. Last error: {}", e),
        None => "All combo tiers failed: no tiers defined or all skipped".into(),
    };
    Err(GatewayError::ProviderError(err_msg))
}
