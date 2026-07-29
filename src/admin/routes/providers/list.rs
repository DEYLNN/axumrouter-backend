use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct ProviderListItem {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub color: String,
    pub icon_name: String,
    pub category: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub total_keys: i64,
    pub active_keys: i64,
    pub locked_keys: i64,
    pub oauth_flow: Option<String>,
    pub model_count: usize,
}

/// GET /admin/api/providers
pub async fn api_providers(State(state): State<Arc<AppState>>) -> Json<Vec<ProviderListItem>> {
    let pm = state.provider_manager.read().await;
    let names = pm.provider_names();
    let mut out = Vec::new();
    for id in names {
        if let Some(p) = pm.get(id) {
            let meta = p.metadata();
            let total =
                crate::db::count_provider_keys(&state.db, id, false).await;
            let active =
                crate::db::count_provider_keys(&state.db, id, true).await;
            let locked = total - active;
            out.push(ProviderListItem {
                id: id.to_string(),
                name: meta.display_name.clone(),
                display_name: meta.display_name.clone(),
                color: meta.color.clone(),
                icon_name: meta.icon_name.clone(),
                category: meta.category.clone(),
                provider_type: meta.category.clone(),
                total_keys: total,
                active_keys: active,
                locked_keys: locked,
                oauth_flow: meta.oauth_flow.clone(),
                model_count: p.list_models().await.unwrap_or_default().len(),
            });
        }
    }
    Json(out)
}
