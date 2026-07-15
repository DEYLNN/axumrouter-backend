use std::sync::Arc;
use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::state::AppState;
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::Utc;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/login", post(login_handler))
        .with_state(state)
}

async fn login_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl axum::response::IntoResponse {
    let expected = match &state.config.auth.admin_password {
        Some(p) if !p.is_empty() => p,
        _ => return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({"error": "login_disabled", "message": "Admin password not set"})),
        )),
    };

    if req.password != *expected {
        return Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid_password"})),
        ));
    }

    let secret = state.config.auth.jwt_secret.clone()
        .unwrap_or_else(|| "default-dev-secret-change-in-prod".into());

    let now = Utc::now().timestamp();
    let claims = serde_json::json!({
        "sub": "admin",
        "iat": now,
        "exp": now + 86400, // 24h
    });

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    ).map_err(|_| (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": "token_generation_failed"})),
    ))?;

    Ok(Json(json!({"token": token})))
}
