use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use crate::state::AppState;

pub async fn start() -> Json<serde_json::Value> {
    match crate::providers::nous_portal::oauth::start().await {
        Ok(resp) => Json(serde_json::json!(resp)),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

pub async fn poll_get(
    State(state): State<Arc<AppState>>,
    params: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let device_code = params.get("device_code").cloned().unwrap_or_default();
    inner(state, &device_code).await
}

pub async fn poll_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let device_code = body.get("device_code").and_then(|v| v.as_str()).unwrap_or("").to_string();
    inner(state, &device_code).await
}

async fn inner(state: Arc<AppState>, device_code: &str) -> Json<serde_json::Value> {
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
