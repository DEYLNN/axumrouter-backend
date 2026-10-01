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
pub async fn logging_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    state.in_flight.fetch_add(1, Ordering::Relaxed);

    let response = next.run(request).await;

    state.in_flight.fetch_sub(1, Ordering::Relaxed);

    tracing::info!(
        "{} {} → {} ({:.2?})",
        method,
        uri,
        response.status().as_u16(),
        start.elapsed()
    );

    response
}
