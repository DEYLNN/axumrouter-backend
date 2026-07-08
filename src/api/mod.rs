pub mod chat;
pub mod models;
pub mod responses;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

use crate::state::AppState;
use crate::types::provider::ProviderMetadata;
use serde::Serialize;

/// All `/v1/*` routes mounted at root.
pub fn routes(state: Arc<AppState>) -> Router {
    // Build sub-routers that still have state attached, then strip state off
    // so we can attach handlers without state before `with_state`.
    let chat = crate::api::chat::routes(state.clone()); // has /v1/chat/completions, with_state
    let models = crate::api::models::routes(state.clone()); // has /v1/models, with_state
    
    Router::new()
        .merge(chat)
        .merge(models)
        .route("/v1/health", get(v1_health).with_state(state.clone()))
        .route("/v1/providers", get(list_providers).with_state(state.clone()))
}

async fn v1_health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pm = state.provider_manager.read().await;
    let provider_names: Vec<String> = pm.list_providers().iter().map(|p| p.name.clone()).collect();
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "providers": provider_names,
        "auth": "bearer_token",
    }))
}

#[derive(Serialize)]
struct ProviderInfo {
    id: String,
    display_name: String,
    version: String,
    capabilities: Vec<String>,
    total_keys: usize,
    active_keys: usize,
}

async fn list_providers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let pm = state.provider_manager.read().await;
    let provider_metas: Vec<ProviderMetadata> = pm.list_providers();
    let mut infos: Vec<ProviderInfo> = Vec::new();
    for meta in provider_metas {
        let total = pm.total_keys_for(&meta.name).unwrap_or(0);
        let active = pm.active_keys_for(&meta.name).unwrap_or(0);
        infos.push(ProviderInfo {
            id: meta.name,
            display_name: meta.display_name,
            version: meta.version,
            capabilities: meta.capabilities,
            total_keys: total,
            active_keys: active,
        });
    }
    Json(serde_json::json!({
        "object": "list",
        "data": infos,
    }))
}
