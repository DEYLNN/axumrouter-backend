use axum::Router;

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

    // Public docs — no auth
    let docs = Router::new().route(
        "/admin/api/docs/key-injection",
        axum::routing::get(crate::admin::routes::keys::serve_key_injection_docs),
    );

    // Public OpenAI-compatible API  (/v1/*)
    let api = crate::api::routes(shared.clone());

    // Admin dashboard JSON API
    let admin = crate::admin::api::admin_routes(shared.clone())
        .merge(crate::admin::login::routes(shared.clone()))
        .merge(crate::admin::oauth::routes(shared.clone()));

    // Static assets and SPA fallback
    //
    // Provider icons live in `frontend/public/providers/` (single source of
    // truth, owned by FE). Serve via absolute path so cwd doesn't matter —
    // binary can be launched from `backend/` (cargo run) or elsewhere.
    let static_assets = axum::Router::new()
        .nest_service(
            "/public/providers",
            ServeDir::new("../frontend/public/providers"),
        );

    // SPA: serve admin build assets, fallback to index.html for client-side routing
    use tower_http::services::{ServeDir, ServeFile};
    let spa = axum::Router::new()
        .nest_service("/admin", ServeDir::new("public/admin")
            .fallback(ServeFile::new("public/admin/index.html")));

    health
        .merge(docs)
        .merge(api)
        .merge(admin)
        .merge(static_assets)
        .merge(spa)
        .layer(from_fn(middleware::logging::logging_middleware))
        .layer(from_fn_with_state(shared.clone(), middleware::auth::auth_middleware))
        .layer(CorsLayer::permissive())
}
