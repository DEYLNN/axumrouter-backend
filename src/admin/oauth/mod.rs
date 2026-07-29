use std::sync::Arc;
use axum::routing::{get, post};
use axum::Router;
use crate::state::AppState;

mod fb;
mod kc;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // FreeBuff OAuth
        .route("/admin/api/oauth/fb/start", get(fb::start))
        .route("/admin/api/oauth/fb/poll", post(fb::poll))
        // KiloCode OAuth
        .route("/admin/api/oauth/kc/start", get(kc::start))
        .route("/admin/api/oauth/kc/poll", post(kc::poll))
        .with_state(state)
}
