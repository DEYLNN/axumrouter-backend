use axum::extract::{State, Extension};
use axum::routing::get;
use axum::{Json, Router};
use std::sync::Arc;

use crate::middleware::auth::GatewayKeyInfo;
use crate::state::AppState;
use crate::types::model::ModelListResponse;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/models", get(list_models))
        .with_state(state)
}

async fn list_models(
    State(state): State<Arc<AppState>>,
    Extension(gw_key): Extension<GatewayKeyInfo>,
) -> Json<ModelListResponse> {
    let mut models = state.provider_manager.read().await.list_all_models().await;

    // Layer 1: Filter out globally disabled models (always applied)
    let disabled: std::collections::HashSet<String> = sqlx::query_scalar("SELECT model_id FROM disabled_models")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();
    models.retain(|m| !disabled.contains(&m.id));

    // Layer 2: Filter out providers with zero API keys
    // Models from a provider without keys are unusable — hide them.
    {
        let pm = state.provider_manager.read().await;
        let zero_key_prefixes: Vec<String> = pm.provider_names().iter()
            .filter(|name| pm.total_keys_for(name).unwrap_or(0) == 0)
            .map(|name| format!("{}/", name))
            .collect();
        // release lock before retain
        drop(pm);
        models.retain(|m| !zero_key_prefixes.iter().any(|prefix| m.id.starts_with(prefix)));
    }

    // Layer 3: Apply gateway key access_type permissions
    match gw_key.access_type.as_str() {
        "allow" => {
            // Only show models in allowed_models list
            models.retain(|m| gw_key.allowed_models.contains(&m.id));
        }
        "deny" => {
            // Hide models in allowed_models list (treat as blocked)
            models.retain(|m| !gw_key.allowed_models.contains(&m.id));
        }
        _ => {} // "full" = show all remaining
    }

    Json(ModelListResponse {
        object: "list".into(),
        data: models,
    })
}
