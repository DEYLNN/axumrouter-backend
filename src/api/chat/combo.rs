use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::response::IntoResponse;

use crate::error::GatewayError;
use crate::middleware::auth::GatewayKeyInfo;
use crate::providers::manager::ProviderManager;
use crate::services::usage_tracking::{CanonicalUsage, record_error, record_success};
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

struct ComboConfig {
    tiers: Vec<String>,
    strategy: String,
    #[allow(dead_code)]
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

// ── Token estimation ──

/// Estimate input tokens using cl100k_base BPE.
/// Sums role + content tokens, adds ~4 tokens overhead per message.
pub(crate) fn estimate_input_tokens(request: &ChatCompletionRequest) -> usize {
    let bpe = match tiktoken_rs::cl100k_base() {
        Ok(b) => b,
        Err(_) => return 0,
    };
    let mut total = 0usize;
    for msg in &request.messages {
        total += bpe.encode_with_special_tokens(&msg.role).len();
        if let Some(ref content) = msg.content {
            total += bpe.encode_with_special_tokens(content).len();
        }
        total += 4; // overhead: role tags, separators
    }
    total
}

// ── Tier context lookup ──

/// Look up context_length for a tier model from the provider model list.
pub(crate) async fn get_tier_ctx(pm: &ProviderManager, tier_model: &str) -> Option<u32> {
    let models = pm.list_all_models().await;
    models
        .iter()
        .find(|m| m.id == tier_model)
        .and_then(|m| m.context_length)
        // If model not found in registry, return None — caller treats as "unknown ctx, don't skip"
}

/// Get the combo's virtual context_length based on strategy.
/// fallback & round_robin → min(ctx tiers) — conservative, avoid overflow.
/// balanced → max(ctx tiers) — will pick the tier that fits.
pub(crate) async fn combo_ctx(pm: &ProviderManager, tiers: &[String], strategy: &str) -> u32 {
    let mut ctxs = Vec::with_capacity(tiers.len());
    for tier in tiers {
        ctxs.push(get_tier_ctx(pm, tier).await.unwrap_or(0));
    }
    let known: Vec<u32> = ctxs.iter().copied().filter(|&c| c > 0).collect();
    if known.is_empty() {
        return 0;
    }
    if strategy == "balanced" {
        *known.iter().max().unwrap_or(&0)
    } else {
        // fallback & round_robin → min ctx
        *known.iter().min().unwrap_or(&0)
    }
}

// ── Strategy: compute tier order ──

/// Compute the tier index order based on strategy.
/// - fallback: DB order, no skip. combo ctx = min(ctx tiers).
/// - round_robin: shuffled, skip ctx < estimate. combo ctx = min(ctx tiers).
/// - balanced: sort by ctx ascending, skip ctx < estimate. combo ctx = max(ctx tiers).
async fn compute_tier_order(
    pm: &ProviderManager,
    cfg: &ComboConfig,
    estimated_tokens: usize,
) -> Vec<usize> {
    let ctxs: Vec<u32> = {
        let mut v = Vec::with_capacity(cfg.tiers.len());
        for tier in &cfg.tiers {
            v.push(get_tier_ctx(pm, tier).await.unwrap_or(0));
        }
        v
    };

    let mut order: Vec<usize> = (0..cfg.tiers.len()).collect();

    match cfg.strategy.as_str() {
        "round_robin" => {
            use rand::seq::SliceRandom;
            order.shuffle(&mut rand::thread_rng());
            // Skip tiers where ctx is known (>0) and smaller than estimated_tokens.
            if estimated_tokens > 0 {
                order.retain(|&i| ctxs[i] == 0 || ctxs[i] as usize >= estimated_tokens);
            }
        }
        "balanced" => {
            // Sort by ctx ascending
            order.sort_by_key(|&i| ctxs[i]);
            // Skip tiers where ctx is known (>0) and smaller than estimated_tokens.
            if estimated_tokens > 0 {
                order.retain(|&i| ctxs[i] == 0 || ctxs[i] as usize >= estimated_tokens);
            }
        }
        _ => {
            // "fallback" — keep DB order, no skip
        }
    }

    order
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

    let estimated = if cfg.strategy == "balanced" {
        estimate_input_tokens(&request)
    } else {
        0 // fallback & round_robin don't need estimation — they use min(ctx tiers)
    };
    let tier_order = compute_tier_order(&pm, &cfg, estimated).await;

    tracing::info!(
        "COMBO: name={} strategy={} tiers={:?} estimated_tokens={} order={:?}",
        combo_name, cfg.strategy, cfg.tiers, estimated, tier_order
    );

    for idx in tier_order {
        let tier_model = &cfg.tiers[idx];
        let (raw_prefix, model_name) = match tier_model.split_once('/') {
            Some((p, m)) => (p, m),
            None => continue,
        };

        let provider_id = match pm.resolve_provider_id(raw_prefix) {
            Some(id) => id,
            None => {
                tracing::warn!("COMBO: resolve_provider_id failed for raw_prefix={:?}", raw_prefix);
                continue;
            }
        };
        let provider = match pm.get(&provider_id) {
            Some(p) => p,
            None => {
                tracing::warn!("COMBO: provider not found in active map: {:?}", provider_id);
                continue;
            }
        };

        let mut tier_req = request.clone();
        tier_req.model = model_name.to_string();

        tracing::info!("COMBO: sending to provider={} model={}", provider_id, tier_req.model);

        match provider.chat_completion(tier_req).await {
            Ok(result) => {
                tracing::info!("COMBO: tier[{}] OK from provider={}", idx, provider_id);
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
                tracing::warn!("COMBO: tier[{}] FAILED provider={} model={} error={}", idx, provider_id, model_name, e);
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

    let estimated = if cfg.strategy == "balanced" {
        estimate_input_tokens(&request)
    } else {
        0 // fallback & round_robin don't need estimation — they use min(ctx tiers)
    };
    let tier_order = compute_tier_order(&pm, &cfg, estimated).await;

    tracing::info!(
        "COMBO STREAM: name={} strategy={} tiers={:?} estimated_tokens={} order={:?}",
        combo_name, cfg.strategy, cfg.tiers, estimated, tier_order
    );

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

        match crate::api::chat::streaming::handle_streaming(
            &state, &gw_key, provider, &provider_id, tier_model, &tier_req, start,
        ).await {
            Ok(resp) => {
                drop(pm);
                return Ok(resp);
            }
            Err(e) => {
                tracing::warn!("COMBO STREAM: tier[{}] FAILED provider={} model={} error={}", idx, provider_id, model_name, e);
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
