// Chat completions module — split for maintainability.
pub mod combo;
pub mod non_streaming;
pub mod streaming;

use axum::extract::{State, Extension};
use axum::routing::post;
use axum::{Json, Router};
use std::sync::Arc;
use std::time::Instant;

use crate::error::GatewayError;
use crate::middleware::auth::GatewayKeyInfo;
use crate::services::caveman;
use crate::services::ponytail;
use crate::services::rtk;
use crate::services::tool_normalizer::normalize_tool_messages;
use crate::state::AppState;
use crate::types::chat::ChatCompletionRequest;

async fn log_and_return(
    db: &sqlx::SqlitePool,
    model_ref: &str,
    err: GatewayError,
    status: u16,
) -> GatewayError {
    // Extract provider from model_ref (e.g. "mst/mistral-small-latest" → "mst")
    let provider = model_ref.split('/').next().filter(|p| !p.is_empty()).unwrap_or("gateway");
    let _ = crate::db::log_usage(
        db, provider, None, model_ref,
        "error", Some(status as i64), 0, 0, None,
        Some(err.to_string()), None, None, None,
    ).await;
    err
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state)
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Extension(gw_key): Extension<GatewayKeyInfo>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<axum::response::Response, GatewayError> {
    let start = Instant::now();
    let model = request.model.clone();

    // ── Pre-checks ──

    if crate::db::is_model_disabled(&state.db, &model).await {
        return Err(log_and_return(&state.db, model.as_str(), GatewayError::ModelNotFound {
            provider: "gateway".to_string(), model: model.clone(),
        }, 404).await);
    }

    if let Err(e) = crate::services::gateway::check_model_access(&gw_key, &model).await {
        return Err(log_and_return(&state.db, &model, e, 404).await);
    }

    if let Err(e) = crate::services::gateway::check_token_limit(&state.db, &gw_key.key_id).await {
        return Err(log_and_return(&state.db, &model, e, 429).await);
    }

    let (provider_id, model_name) = match model.split_once('/') {
        Some((pid, rest)) => (pid, rest),
        None => return Err(log_and_return(&state.db, model.as_str(), GatewayError::InvalidModelFormat(model.clone()), 400).await),
    };
    let is_streaming = request.stream.unwrap_or(false);

    // Combo routing
    if provider_id == "combo" {
        if is_streaming {
            return combo::handle_combo_request_stream(state, request, model_name.to_string(), start).await;
        }
        return combo::handle_combo_request(state, request, model_name.to_string(), start).await;
    }

    // Blocked model check
    if crate::db::is_model_blocked(&state.db, provider_id, model_name).await {
        return Err(log_and_return(&state.db, &model, GatewayError::ModelNotFound {
            provider: provider_id.to_string(), model: model_name.to_string(),
        }, 404).await);
    }

    if request.messages.is_empty() {
        return Err(log_and_return(&state.db, &model, GatewayError::EmptyMessages, 400).await);
    }

    // ── Resolve provider ──

    let pm = state.provider_manager.read().await;
    let provider = match pm.get(provider_id) {
        Some(p) => p,
        None => {
            drop(pm);
            return Err(log_and_return(&state.db, &model, GatewayError::ProviderNotFound(provider_id.to_string()), 404).await);
        }
    };

    let all_models = provider.list_models().await.map_err(|_| GatewayError::Internal("Failed to list models".into()))?;
    if !all_models.iter().any(|m| m.id == model) {
        drop(pm);
        return Err(log_and_return(&state.db, &model, GatewayError::ModelNotFound {
            provider: provider_id.to_string(), model: model_name.to_string(),
        }, 404).await);
    }

    // ── Prepare request ──

    let mut provider_request = request.clone();
    provider_request.model = model_name.to_string();
    provider_request.stream = Some(is_streaming);
    // Inject stream_options for streaming — upstream may return usage chunks
    if is_streaming && provider_request.stream_options.is_none() {
        provider_request.stream_options = Some(serde_json::json!({"include_usage": true}));
    }
    normalize_tool_messages(&mut provider_request.messages);

    // RTK: compress tool_result content before routing
    rtk::compress(&state.db, &mut provider_request.messages).await;
    // Caveman: inject terse system prompt
    caveman::inject(&state.db, &mut provider_request.messages).await;
    // Ponytail: inject "lazy senior dev" minimalism prompt
    ponytail::inject(&state.db, &mut provider_request.messages).await;

    // Reasoning placeholder: inject reasoning_content: " " on assistant messages
    // Signals upstream (OCG/deepseek) to separate thinking from visible content.
    // Without this, model puts "The user wants..." thinking into content field.
    for msg in &mut provider_request.messages {
        if msg.role == "assistant" && msg.reasoning_content.is_none() {
            msg.reasoning_content = Some(" ".to_string());
        }
    }

    // ── Route to handler ──

    if is_streaming {
        streaming::handle_streaming(&state, &gw_key, provider, provider_id, &model, &provider_request, start).await
    } else {
        non_streaming::handle_non_streaming(&state, &gw_key, provider, provider_id, &model, &provider_request, start).await
    }
}
