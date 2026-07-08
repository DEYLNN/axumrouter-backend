use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::err_response;
use crate::state::AppState;
use std::sync::Arc;

/// Stored in request extensions by auth middleware for downstream handlers.
#[derive(Clone)]
pub struct GatewayKeyInfo {
    pub key_id: String,
    pub access_type: String,
    pub allowed_models: Vec<String>,
}

/// Auth middleware: protects /v1/* with gateway keys.
/// Public routes (/admin/*, /health) bypass.
/// Logs failed auth attempts to usage table.
pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let path = request.uri().path().to_string();

    // Public paths — bypass auth
    if path.starts_with("/admin")
        || path.starts_with("/health")
        || !path.starts_with("/v1/")
    {
        return next.run(request).await;
    }

    // Extract Bearer token
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    let key_value = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            // Log missing auth
            let _ = crate::db::log_usage(
                &state.db,
                "gateway",
                None,
                "N/A",
                "error",
                Some(401i64),
                0, 0, None,
                Some("missing_authorization: Missing or invalid Authorization header".into()),
                None, None,
            ).await;
            return err_response(
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "missing_authorization",
                "Missing or invalid `Authorization` header. Send `Authorization: Bearer <gateway key>` header.",
            );
        }
    };

    // Resolve gateway key id for logging
    let key_id_opt: Option<String> = sqlx::query_scalar(
        "SELECT id FROM gateway_keys WHERE key_value = ?"
    )
    .bind(key_value)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    // Validate gateway key (inlined from deleted admin/gateway_keys.rs)
    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM gateway_keys WHERE key_value = ? AND is_active = 1)"
    )
    .bind(key_value)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if !valid {
        // Log invalid key attempt
        let _ = crate::db::log_usage(
            &state.db,
            "gateway",
            key_id_opt.as_deref(),
            "N/A",
            "error",
            Some(401i64),
            0, 0, None,
            Some("invalid_api_key: Invalid or inactive gateway API key".into()),
            None, None,
        ).await;
        return err_response(
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "invalid_api_key",
            "Invalid or inactive gateway API key",
        );
    }

    // Look up gateway key permissions and store in request extensions
    if let Some(kid) = &key_id_opt {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT access_type, allowed_models FROM gateway_keys WHERE id = ?"
        )
        .bind(kid)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        if let Some((access_type, allowed_models_str)) = row {
            let allowed_models: Vec<String> = if allowed_models_str.is_empty() {
                vec![]
            } else {
                serde_json::from_str(&allowed_models_str).unwrap_or_default()
            };
            let info = GatewayKeyInfo {
                key_id: kid.clone(),
                access_type,
                allowed_models,
            };
            let mut req = request;
            req.extensions_mut().insert(info);
            next.run(req).await
        } else {
            let mut req = request;
            req.extensions_mut().insert(GatewayKeyInfo {
                key_id: kid.clone(),
                access_type: "full".into(),
                allowed_models: vec![],
            });
            next.run(req).await
        }
    } else {
        // Key not found in DB (shouldn't happen since we validated it), but still let through
        next.run(request).await
    }
}
