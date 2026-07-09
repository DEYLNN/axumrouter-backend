use axum::Router;
use tower_http::services::ServeDir;
use tower_http::cors::CorsLayer;

use axum::middleware::{from_fn, from_fn_with_state};
use crate::middleware;
use crate::state::AppState;

pub fn build(state: AppState) -> Router {
    let shared = std::sync::Arc::new(state);

    // Health check — lightweight, no auth
    let health = Router::new().route(
        "/health",
        axum::routing::get(|| async { r#"{"status":"ok"}"# }),
    );

    // Public OpenAI-compatible API  (/v1/*)
    let api = crate::api::routes(shared.clone());

    // Admin dashboard JSON API + auth_files
    let admin = crate::admin::api::admin_routes(shared.clone())
        .merge(crate::admin::auth_files::routes(shared.clone()))
        .merge(crate::admin::oauth::routes(shared.clone()));

    // Static assets and SPA fallback
    let static_assets = axum::Router::new()
        .nest_service("/public/providers", ServeDir::new("public/providers"));

    // SPA: serve admin build assets, fallback to index.html for client-side routing
    use tower_http::services::{ServeDir, ServeFile};
    let spa = axum::Router::new()
        .nest_service("/admin", ServeDir::new("public/admin")
            .fallback(ServeFile::new("public/admin/index.html")));

    health
        .merge(api)
        .merge(admin)
        .merge(static_assets)
        .merge(spa)
        .layer(CorsLayer::permissive())
        .layer(from_fn(middleware::logging::logging_middleware))
        .layer(from_fn_with_state(shared.clone(), middleware::auth::auth_middleware))
}
