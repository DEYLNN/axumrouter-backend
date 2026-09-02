use std::sync::Arc;
use axum::{extract::{State, Path}, Json};
use serde::Deserialize;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AddCustomModelReq {
    pub model_id: String,
    pub display_name: Option<String>,
    pub ctx: Option<i64>,
    pub vision: Option<bool>,
    pub tools: Option<bool>,
}

pub async fn api_list_custom_models(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Vec<crate::db::CustomModelRow>> {
    // Custom providers store models in custom_provider_models; legacy providers
    // keep using custom_models.
    if crate::db::get_custom_provider(&state.db, &id).await.map(|p| p.is_some()).unwrap_or(false) {
        let rows = crate::db::list_custom_provider_models(&state.db, &id).await.unwrap_or_default();
        return Json(rows.into_iter().map(|m| crate::db::CustomModelRow {
            id: format!("{}_{}", id, m.model_id.replace('/', "_")),
            provider_id: m.provider_id,
            model_id: m.model_id,
            display_name: String::new(),
            ctx: m.ctx,
            vision: m.vision,
            tools: m.tools,
            created_at: String::new(),
        }).collect());
    }
    let rows = crate::db::list_custom_models(&state.db, &id).await;
    Json(rows)
}

pub async fn api_add_custom_model_for_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AddCustomModelReq>,
) -> Json<serde_json::Value> {
    let model_id = req.model_id.trim();
    if model_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "invalid_model_id", "message": "Model ID required"}));
    }
    // Custom providers (custom_providers table) build their engine model list
    // from `custom_provider_models` — manager.rs loads it into config.models.
    // Writing to the legacy `custom_models` table too made the model appear
    // twice in the detail view (`prefix/x` + `custom_<id>/x`). Route by type.
    let is_custom_provider = crate::db::get_custom_provider(&state.db, &id).await.map(|p| p.is_some()).unwrap_or(false);
    if is_custom_provider {
        let dup = crate::db::list_custom_provider_models(&state.db, &id)
            .await
            .map(|rows| rows.iter().any(|m| m.model_id == model_id))
            .unwrap_or(false);
        if dup {
            return Json(serde_json::json!({"ok": false, "error": "duplicate_model", "message": "Model already exists"}));
        }
        if let Err(e) = crate::db::add_custom_provider_model(
            &state.db, &id, model_id,
            req.ctx.unwrap_or(4096),
            if req.vision.unwrap_or(false) { 1 } else { 0 },
            if req.tools.unwrap_or(true) { 1 } else { 0 },
        ).await {
            return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
        }
        let mut pm = state.provider_manager.write().await;
        let _ = pm.reload_custom_provider(&id, &state.db).await;
        return Json(serde_json::json!({"ok": true}));
    }
    // Non-custom providers (TOML/registered): dedup vs TOML + legacy table.
    let toml_duplicate = crate::providers::toml_provider::ProviderList {
        providers: toml::from_str(include_str!("../../../../providers.toml"))
            .map(|list: crate::providers::toml_provider::ProviderList| list.providers)
            .unwrap_or_default(),
    }.providers.iter().find(|p| p.id == id)
        .is_some_and(|p| p.models.iter().any(|m| m.id == model_id));
    let db_duplicate = crate::db::list_custom_models(&state.db, &id).await
        .iter().any(|m| m.model_id == model_id);
    if toml_duplicate || db_duplicate {
        return Json(serde_json::json!({"ok": false, "error": "duplicate_model", "message": "Model already exists"}));
    }
    if let Err(e) = crate::db::add_custom_model(
        &state.db, &id, model_id,
        req.display_name.as_deref().unwrap_or(model_id),
        req.ctx.unwrap_or(4096),
        if req.vision.unwrap_or(false) { 1 } else { 0 },
        if req.tools.unwrap_or(true) { 1 } else { 0 },
    ).await {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    Json(serde_json::json!({"ok": true}))
}

pub async fn api_remove_custom_model_for_provider(
    State(state): State<Arc<AppState>>,
    Path((id, model_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    // Try both tables — custom providers use custom_provider_models,
    // TOML providers use the legacy custom_models.
    let from_cp = crate::db::remove_custom_provider_model(&state.db, &id, &model_id).await.unwrap_or(false);
    let from_legacy = crate::db::remove_custom_model(&state.db, &id, &model_id).await.unwrap_or(false);
    if from_custom_provider_row(&state, &id).await {
        let mut pm = state.provider_manager.write().await;
        let _ = pm.reload_custom_provider(&id, &state.db).await;
    }
    Json(serde_json::json!({"ok": from_cp || from_legacy}))
}

async fn from_custom_provider_row(state: &AppState, id: &str) -> bool {
    crate::db::get_custom_provider(&state.db, id).await.map(|p| p.is_some()).unwrap_or(false)
}
