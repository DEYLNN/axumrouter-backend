use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use crate::state::AppState;

/// Request/response logging middleware with in-flight tracking.
/// Only counts /v1/ API requests as in-flight (excludes admin/health polling).
pub async fn logging_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    // Only track in-flight for actual API requests, not admin/health polling
    let is_api = uri.path().starts_with("/v1/");
    if is_api {
        state.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    let response = next.run(request).await;

    if is_api {
        state.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    tracing::info!(
        "{} {} → {} ({:.2?})",
        method,
        uri,
        response.status().as_u16(),
        start.elapsed()
    );

    response
}
