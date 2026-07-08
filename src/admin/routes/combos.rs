use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub created_at: String,
}

/// List all combos
pub async fn api_list_combos(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ComboResponse>> {
    let rows = sqlx::query_as::<_, (String, String, String, String, bool, bool, String)>(
        "SELECT id, name, description, tiers, round_robin, is_active, created_at FROM combos ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, name, desc, tiers_str, rr, active, created)| {
        let tiers: Vec<ComboTier> = serde_json::from_str(&tiers_str).unwrap_or_default();
        ComboResponse {
            id, name, description: desc, tiers,
            round_robin: rr, is_active: active, created_at: created,
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

    let result = sqlx::query(
        "INSERT INTO combos (id, name, description, tiers, round_robin) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.description.unwrap_or_default())
    .bind(&tiers_json)
    .bind(rr)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true, "id": id, "name": req.name})),
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

/// Toggle combo active
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
