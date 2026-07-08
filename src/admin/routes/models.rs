use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ToggleModelRequest {
    pub model_id: String,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct ToggleModelResponse {
    pub ok: bool,
    pub model_id: String,
    pub enabled: bool,
}

/// Toggle a model's enabled/disabled state (global allowlist)
pub async fn api_toggle_model(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ToggleModelRequest>,
) -> Json<ToggleModelResponse> {
    if req.enabled {
        // Enable: remove from disabled list
        let _ = sqlx::query("DELETE FROM disabled_models WHERE model_id = ?")
            .bind(&req.model_id)
            .execute(&state.db)
            .await;
    } else {
        // Disable: add to disabled list
        let _ = sqlx::query("INSERT OR IGNORE INTO disabled_models (model_id) VALUES (?)")
            .bind(&req.model_id)
            .execute(&state.db)
            .await;
    }
    Json(ToggleModelResponse {
        ok: true,
        model_id: req.model_id,
        enabled: req.enabled,
    })
}

/// Get all disabled model IDs
pub async fn api_disabled_models(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT model_id FROM disabled_models")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    Json(rows)
}

/// Get all models grouped by provider with enabled/disabled status
pub async fn api_all_models(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let pm = state.provider_manager.read().await;
    let all_models = pm.list_all_models().await;

    let disabled: std::collections::HashSet<String> = sqlx::query_scalar("SELECT model_id FROM disabled_models")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut providers_map: std::collections::BTreeMap<String, Vec<serde_json::Value>> = std::collections::BTreeMap::new();
    for m in &all_models {
        let parts: Vec<&str> = m.id.split('/').collect();
        let provider = parts.first().unwrap_or(&"unknown").to_string();
        let is_disabled = disabled.contains(&m.id);
        providers_map.entry(provider).or_default().push(serde_json::json!({
            "id": m.id,
            "owned_by": m.owned_by,
            "enabled": !is_disabled,
        }));
    }
    Json(serde_json::json!(providers_map))
}

/// Get all blocked models from the per-provider blocked_models table
pub async fn api_blocked_models(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<serde_json::Value>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT provider_id, model_id FROM blocked_models"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(provider_id, model_id): (String, String)| {
        (provider_id, model_id)
    })
    .collect();
    Json(rows.into_iter().map(|(p, m)| serde_json::json!({ "provider": p, "model": m })).collect())
}
