use std::sync::Arc;
use axum::routing::{get, post};
use axum::Router;
use crate::state::AppState;

mod kc;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // KiloCode OAuth
        .route("/admin/api/oauth/kc/start", get(kc::start))
        .route("/admin/api/oauth/kc/poll", post(kc::poll))
        .with_state(state)
}