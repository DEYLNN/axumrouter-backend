use std::sync::Arc;
use axum::routing::{get, post};
use axum::Router;
use crate::state::AppState;

mod cx;
mod xai;
mod fb;
mod np;

mod kc;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Codex OAuth
        .route("/admin/api/oauth/cx/start", get(cx::start))
        .route("/admin/api/oauth/cx/callback", get(cx::exchange))
        .route("/admin/api/oauth/cx/exchange", post(cx::manual))
        .route("/admin/api/oauth/cx/manual", post(cx::manual))
        // xAI OAuth
        .route("/admin/api/oauth/xai/start", get(xai::start))
        .route("/admin/api/oauth/xai/callback", get(xai::exchange))
        .route("/admin/api/oauth/xai/exchange", post(xai::manual))
        // FreeBuff OAuth
        .route("/admin/api/oauth/fb/start", get(fb::start))
        .route("/admin/api/oauth/fb/poll", post(fb::poll))
        // Nous Portal OAuth
        .route("/admin/api/oauth/np/start", get(np::start))
        .route("/admin/api/oauth/np/poll", get(np::poll_get))
        .route("/admin/api/oauth/np/poll", post(np::poll_post))
        // Kilo Code OAuth
        .route("/admin/api/oauth/kc/start", get(kc::start))
        .route("/admin/api/oauth/kc/poll", post(kc::poll))
        .with_state(state)
}
