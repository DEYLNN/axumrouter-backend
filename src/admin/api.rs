use std::sync::Arc;

use axum::Router;

use crate::state::AppState;
use super::routes;

pub fn admin_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(providers_routes(state.clone()))
        .merge(logs_routes(state.clone()))
        .merge(usage_routes(state.clone()))
        .merge(settings_routes(state.clone()))
        .merge(database_routes(state.clone()))
        .merge(gateway_routes(state.clone()))
        .merge(keys_routes(state.clone()))
        .merge(models_routes(state.clone()))
        .merge(combos_routes(state.clone()))
}

use axum::routing::{get, post, patch, delete};

fn providers_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/providers", get(routes::providers::api_providers))
        .route("/admin/api/providers/:id", get(routes::providers::api_provider_detail))
        .route("/admin/api/providers/:id/validate-models", get(routes::providers::api_validate_models))
        .route("/admin/api/providers/:id/test", post(routes::providers::api_test_model))
        .route("/admin/api/providers/:id/block", post(routes::providers::api_block_model))
        .route("/admin/api/providers/:id/unblock", post(routes::providers::api_unblock_model))
        .with_state(state)
}

fn logs_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/logs", get(routes::logs::api_logs))
        .route("/admin/api/logs/clear", post(routes::logs::api_logs_clear))
        .with_state(state)
}

fn usage_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/usage/quota/:key_id", get(routes::quota::api_usage_quota))
        .route("/admin/api/usage/keys", get(routes::usage::api_usage_oauth_keys))
        .route("/admin/api/usage/stats", get(routes::usage::api_usage_stats))
        .route("/admin/api/usage/stats/keys", get(routes::usage::api_usage_per_key))
        .route("/admin/api/usage/refresh/:key_id", post(routes::quota::api_refresh_token))
        .with_state(state)
}

fn settings_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/settings", get(routes::settings::api_settings))
        .route("/admin/api/settings/toggle", post(routes::settings::api_toggle_setting))
        .with_state(state)
}

fn database_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/database", get(routes::database::api_database_info))
        .route("/admin/api/database/export", get(routes::database::api_database_export))
        .route("/admin/api/database/import", post(routes::database::api_database_import))
        .with_state(state)
}

fn gateway_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/gateway_keys", get(routes::gateway_keys::api_list_keys))
        .route("/admin/api/gateway_keys", post(routes::gateway_keys::api_create_key))
        .route("/admin/api/gateway_keys/:id", delete(routes::gateway_keys::api_delete_key))
        .route("/admin/api/gateway_keys/:id", patch(routes::gateway_keys::api_update_key))
        .with_state(state)
}

fn keys_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/keys", post(routes::keys::api_add_key))
        .route("/admin/api/keys/delete", post(routes::keys::api_delete_key))
        .with_state(state)
}

fn models_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/models/toggle", post(routes::models::api_toggle_model))
        .route("/admin/api/models/disabled", get(routes::models::api_disabled_models))
        .route("/admin/api/models/all", get(routes::models::api_all_models))
        .route("/admin/api/models/blocked", get(routes::models::api_blocked_models))
        .with_state(state)
}

fn combos_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/combos", get(routes::combos::api_list_combos))
        .route("/admin/api/combos", post(routes::combos::api_create_combo))
        .route("/admin/api/combos/:id", post(routes::combos::api_update_combo))
        .route("/admin/api/combos/:id", delete(routes::combos::api_delete_combo))
        .route("/admin/api/combos/:id/toggle", post(routes::combos::api_toggle_combo))
        .route("/admin/api/combos/:id/roundrobin", post(routes::combos::api_toggle_roundrobin))
        .with_state(state)
}
