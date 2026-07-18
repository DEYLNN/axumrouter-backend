use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::providers::manager::ProviderManager;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateComboRequest {
    pub name: String,
    pub description: Option<String>,
    pub tiers: Vec<ComboTier>,
    pub round_robin: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ComboTier {
    pub tier: u32,
    pub provider: String,
    pub model: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct ComboResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tiers: Vec<ComboTier>,
    pub round_robin: bool,
    pub is_active: bool,
    pub min_context: u64,
    pub created_at: String,
}

/// List all combos
pub async fn api_list_combos(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ComboResponse>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, bool, bool, u64, String)>(
        "SELECT id, name, description, tiers, round_robin, is_active, min_context, created_at FROM combos ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, name, desc, tiers_str, rr, active, min_ctx, created)| {
        let tiers: Vec<ComboTier> = serde_json::from_str(&tiers_str).unwrap_or_default();
        ComboResponse {
            id, name, description: desc, tiers,
            round_robin: rr, is_active: active, min_context: min_ctx, created_at: created,
        }
    })
    .collect();
    Json(rows)
}

/// Create combo
pub async fn api_create_combo(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateComboRequest>,
) -> Json<serde_json::Value> {
    let id = format!("combo_{}", &Uuid::new_v4().to_string()[..8]);
    let tiers_json = serde_json::to_string(&req.tiers).unwrap_or_default();
    let rr = req.round_robin.unwrap_or(false) as i32;

    // Calculate min_context from all tier models
    let pm = state.provider_manager.read().await;
    let mut min_ctx = u64::MAX;
    for tier in &req.tiers {
        if let Some(ctx) = find_model_context(&pm, &tier.model).await {
            min_ctx = min_ctx.min(ctx);
        }
    }
    drop(pm);
    if min_ctx == u64::MAX { min_ctx = 0; }

    let result = sqlx::query(
        "INSERT INTO combos (id, name, description, tiers, round_robin, min_context) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description.unwrap_or_default())
    .bind(&tiers_json)
    .bind(rr)
    .bind(min_ctx as i64)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true, "id": id, "name": req.name, "min_context": min_ctx})),
        Err(e) => {
            let msg = if e.to_string().contains("UNIQUE") {
                format!("Combo '{}' already exists", req.name)
            } else {
                format!("Failed: {}", e)
            };
            Json(serde_json::json!({"ok": false, "error": msg}))
        }
    }
}

/// Look up context_length for a model from provider manager
async fn find_model_context(pm: &ProviderManager, model_id: &str) -> Option<u64> {
    let all = pm.list_all_models_unfiltered().await;
    all.iter().find(|m| m.id == model_id).and_then(|m| m.context_length.map(|c| c as u64))
}

/// Update combo tiers
pub async fn api_update_combo(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateComboRequest>,
) -> Json<serde_json::Value> {
    let tiers_json = serde_json::to_string(&req.tiers).unwrap_or_default();

    let pm = state.provider_manager.read().await;
    let mut min_ctx = u64::MAX;
    for tier in &req.tiers {
        if let Some(ctx) = find_model_context(&pm, &tier.model).await {
            min_ctx = min_ctx.min(ctx);
        }
    }
    drop(pm);
    if min_ctx == u64::MAX { min_ctx = 0; }

    let result = sqlx::query(
        "UPDATE combos SET name = ?, description = ?, tiers = ?, min_context = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&req.name)
    .bind(&req.description.unwrap_or_default())
    .bind(&tiers_json)
    .bind(min_ctx as i64)
    .bind(&id)
    .execute(&state.db)
    .await;

    match result {
            Ok(_) => Json(serde_json::json!({"ok": true, "id": id, "name": req.name, "min_context": min_ctx})),
            Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
        }
    }

    /// Delete combo
pub async fn api_delete_combo(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let result = sqlx::query("DELETE FROM combos WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Toggle combo round_robin
pub async fn api_toggle_roundrobin(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let _ = sqlx::query("UPDATE combos SET round_robin = CASE WHEN round_robin = 1 THEN 0 ELSE 1 END, updated_at = datetime('now') WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;
    Json(serde_json::json!({"ok": true}))
}
pub async fn api_toggle_combo(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let _ = sqlx::query("UPDATE combos SET is_active = CASE WHEN is_active = 1 THEN 0 ELSE 1 END, updated_at = datetime('now') WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;
    Json(serde_json::json!({"ok": true}))
}
