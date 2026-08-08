use std::sync::Arc;
use axum::routing::{get, post};
use axum::Router;
use crate::state::AppState;

mod gb;
mod kc;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // KiloCode OAuth
        .route("/admin/api/oauth/kc/start", get(kc::start))
        .route("/admin/api/oauth/kc/poll", post(kc::poll))
        // Grok Build OAuth
        .route("/admin/api/oauth/gb/start", get(gb::start))
        .route("/admin/api/oauth/gb/poll", post(gb::poll))
        .route("/admin/api/oauth/gb/refresh/:key_id", post(gb::refresh))
        .with_state(state)
}
