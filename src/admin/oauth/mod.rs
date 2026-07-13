use std::sync::Arc;
use axum::routing::{get, post};
use axum::Router;
use crate::state::AppState;

mod cx;
mod xai;
mod fb;
mod np;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Codex OAuth
        .route("/admin/oauth/cx/start", get(cx::start))
        .route("/admin/oauth/cx/callback", get(cx::exchange))
        .route("/admin/oauth/cx/exchange", post(cx::manual))
        .route("/admin/oauth/cx/manual", post(cx::manual))
        // xAI OAuth
        .route("/admin/oauth/xai/start", get(xai::start))
        .route("/admin/oauth/xai/callback", get(xai::exchange))
        .route("/admin/oauth/xai/exchange", post(xai::manual))
        // FreeBuff OAuth
        .route("/admin/oauth/fb/start", get(fb::start))
        .route("/admin/oauth/fb/poll", post(fb::poll))
        // Nous Portal OAuth
        .route("/admin/oauth/np/start", get(np::start))
        .route("/admin/oauth/np/poll", get(np::poll_get))
        .route("/admin/oauth/np/poll", post(np::poll_post))
        .with_state(state)
}
