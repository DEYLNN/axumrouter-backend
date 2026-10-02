use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::state::AppState;

/// RAII guard: decrements in-flight on Drop — runs even on panic/cancel.
struct InFlightGuard<'a>(&'a AtomicI64);
impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

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

    // Only track in-flight for chat completions, not models/health/providers polling
    let is_chat = uri.path() == "/v1/chat/completions";
    let _guard = if is_chat {
        state.in_flight.fetch_add(1, Ordering::Relaxed);
        Some(InFlightGuard(&state.in_flight))
    } else {
        None
    };

    let response = next.run(request).await;

    tracing::info!(
        "{} {} → {} ({:.2?})",
        method,
        uri,
        response.status().as_u16(),
        start.elapsed()
    );

    response
}
