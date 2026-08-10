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
    let deleted = crate::db::remove_custom_model(&state.db, &id, &model_id).await.unwrap_or(false);
    Json(serde_json::json!({"ok": deleted}))
}
