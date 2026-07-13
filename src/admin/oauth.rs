use std::sync::Arc;
use axum::extract::{State, Query};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Serialize};
use crate::state::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Codex OAuth
        .route("/admin/oauth/cx/start", get(oauth_cx_start))
        .route("/admin/oauth/cx/callback", get(oauth_cx_exchange))
        .route("/admin/oauth/cx/exchange", post(oauth_cx_manual))
        .route("/admin/oauth/cx/manual", post(oauth_cx_manual))
        // xAI OAuth
        .route("/admin/oauth/xai/start", get(oauth_xai_start))
        .route("/admin/oauth/xai/callback", get(oauth_xai_exchange))
        .route("/admin/oauth/xai/exchange", post(oauth_xai_manual))
        // FreeBuff OAuth
        .route("/admin/oauth/fb/start", get(oauth_fb_start))
        .route("/admin/oauth/fb/poll", post(oauth_fb_poll))
        // Nous Portal OAuth
        .route("/admin/oauth/np/start", get(oauth_np_start))
        .route("/admin/oauth/np/poll", get(oauth_np_poll_get))
        .route("/admin/oauth/np/poll", post(oauth_np_poll_post))
        .with_state(state)
}

// ── Codex (cx) OAuth ──

async fn oauth_cx_start() -> Json<crate::providers::openai_codex::oauth::OAuthStartResponse> {
    Json(crate::providers::openai_codex::oauth::start().await)
}

async fn oauth_cx_exchange(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let code = params.get("code").cloned().unwrap_or_default();
    let oauth_state = params.get("state").cloned().unwrap_or_default();
    let token = crate::providers::openai_codex::oauth::exchange_code(&code, &oauth_state).await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    crate::providers::openai_codex::oauth::save_token(&state, &token).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Redirect::to("/admin/auth-files"))
}

async fn oauth_cx_manual(
    State(state): State<Arc<AppState>>,
    Json(body): Json<crate::providers::openai_codex::oauth::ManualCodeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    match crate::providers::openai_codex::oauth::exchange_code(&body.code, &body.state.unwrap_or_default()).await {
        Ok(token) => {
            crate::providers::openai_codex::oauth::save_token(&state, &token).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
            Ok(Json(serde_json::json!({"success":true,"message":"Codex OAuth connected"})))
        }
        Err(e) => Ok(Json(serde_json::json!({"success":false,"error":e}))),
    }
}

// ── xAI OAuth ──

async fn oauth_xai_start() -> Json<serde_json::Value> {
    let resp = crate::providers::xai::oauth::start().await;
    Json(serde_json::json!({"url": resp.url, "id": resp.id}))
}

async fn oauth_xai_exchange(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let code = params.get("code").cloned().unwrap_or_default();
    let oauth_state = params.get("state").cloned().unwrap_or_default();
    let token = crate::providers::xai::oauth::exchange_code(&code, &oauth_state).await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    crate::providers::xai::oauth::save_token(&state, &token).await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Redirect::to("/admin/auth-files"))
}

async fn oauth_xai_manual(
    State(state): State<Arc<AppState>>,
    Json(body): Json<crate::providers::xai::oauth::ManualCodeRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    match crate::providers::xai::oauth::exchange_code(&body.code, &body.state.unwrap_or_default()).await {
        Ok(token) => {
            crate::providers::xai::oauth::save_token(&state, &token).await
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
            Ok(Json(serde_json::json!({"success":true,"message":"xAI OAuth connected"})))
        }
        Err(e) => Ok(Json(serde_json::json!({"success":false,"error":e}))),
    }
}

// ── FreeBuff OAuth ──

async fn oauth_fb_start() -> Json<serde_json::Value> {
    match crate::providers::freebuff::oauth::start().await {
        Ok(data) => Json(data),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn oauth_fb_poll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let device_code = body.get("device_code").and_then(|v| v.as_str()).unwrap_or_default();
    let fingerprint_hash = body.get("fingerprint_hash").and_then(|v| v.as_str())
        .or_else(|| body.get("_fingerprintHash").and_then(|v| v.as_str()))
        .unwrap_or_default();
    let expires_at_raw = body.get("expires_at").or_else(|| body.get("_expiresAt"));
    let expires_at: String = match expires_at_raw {
        Some(v) => v.as_str().map(|s| s.to_string())
            .or_else(|| v.as_i64().map(|n| n.to_string()))
            .unwrap_or_default(),
        None => String::new(),
    };

    match crate::providers::freebuff::oauth::poll(device_code, fingerprint_hash, &expires_at).await {
        Ok(data) => {
            if data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Err(e) = crate::providers::freebuff::oauth::save_token(&state, &data).await {
                    return Json(serde_json::json!({"ok": false, "error": e}));
                }
            }
            Json(data)
        }
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

// ── Nous Portal OAuth ──

async fn oauth_np_start() -> Json<serde_json::Value> {
    match crate::providers::nous_portal::oauth::start().await {
        Ok(resp) => Json(serde_json::json!(resp)),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn oauth_np_poll_get(
    State(state): State<Arc<AppState>>,
    params: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let device_code = params.get("device_code").cloned().unwrap_or_default();
    np_poll_inner(state, &device_code).await
}

async fn oauth_np_poll_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let device_code = body.get("device_code").and_then(|v| v.as_str()).unwrap_or("").to_string();
    np_poll_inner(state, &device_code).await
}

async fn np_poll_inner(state: Arc<AppState>, device_code: &str) -> Json<serde_json::Value> {
    if device_code.is_empty() {
        return Json(serde_json::json!({"error": "Missing device_code"}));
    }
    match crate::providers::nous_portal::oauth::poll(device_code).await {
        Ok(data) => {
            if data.get("access_token").is_some() {
                if let Err(e) = crate::providers::nous_portal::oauth::save_token(&state, &data).await {
                    return Json(serde_json::json!({"error": e}));
                }
                return Json(serde_json::json!({"success": true, "accessToken": data["access_token"], "message": "Nous Portal connected"}));
            }
            Json(data)
        }
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
