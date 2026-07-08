// Chat completions module — split for maintainability.
pub mod combo;
pub mod non_streaming;
pub mod streaming;

use axum::extract::{State, Extension};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use std::sync::Arc;
use std::time::Instant;

use crate::error::GatewayError;
use crate::middleware::auth::GatewayKeyInfo;
use crate::services::tool_normalizer::normalize_tool_messages;
use crate::state::AppState;
use crate::types::chat::{ChatCompletionRequest, ChatCompletionResponse};

async fn log_and_return(
    db: &sqlx::SqlitePool,
    model_ref: &str,
    err: GatewayError,
    status: u16,
) -> GatewayError {
    let _ = crate::db::log_usage(
        db, "gateway", None, model_ref,
        "error", Some(status as i64), 0, 0, None,
        Some(err.to_string()), None, None,
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

    let parts: Vec<&str> = model.split('/').collect();
    if parts.len() != 2 {
        return Err(log_and_return(&state.db, model.as_str(), GatewayError::InvalidModelFormat(model.clone()), 400).await);
    }

    let provider_id = parts[0];
    let model_name = parts[1];
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
    normalize_tool_messages(&mut provider_request.messages);

    // Caveman injection
    let caveman_level: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'caveman_enabled'")
        .fetch_one(&state.db)
        .await
        .unwrap_or_else(|_| "off".to_string());
    if caveman_level != "off" {
        let prompt = match caveman_level.as_str() {
            "lite" => "Be concise. Remove filler words but keep proper grammar. Answer directly.",
            "ultra" => "Respond in ultra-terse telegraphic style. No articles, no pronouns, no verbs when possible. Max compression. Only key information.",
            _ => "Respond concisely and directly. No pleasantries, no explanations, no fluff. Drop articles, use fragments. Get straight to the point with minimal words.",
        };
        provider_request.messages.insert(0, crate::types::chat::Message {
            role: "system".to_string(), content: Some(prompt.to_string()),
            tool_calls: None, tool_call_id: None, name: None,
        });
    }

    // ── Route to handler ──

    if is_streaming {
        streaming::handle_streaming(&state, provider, provider_id, &model, &provider_request, start).await
    } else {
        non_streaming::handle_non_streaming(&state, &gw_key, provider, provider_id, &model, &provider_request, start).await
    }
}
