use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

/// Request/response logging middleware
pub async fn logging_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

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
