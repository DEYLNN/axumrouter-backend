use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use crate::state::AppState;

pub async fn start() -> Json<serde_json::Value> {
    match crate::providers::kilocode::oauth::start().await {
        Ok(data) => Json(data),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

pub async fn poll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let device_code = body.get("device_code").and_then(|v| v.as_str()).unwrap_or_default();
    match crate::providers::kilocode::oauth::poll(device_code).await {
        Ok(data) => {
            if data.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                if let Err(e) = crate::providers::kilocode::oauth::save_token(&state, &data).await {
                    return Json(serde_json::json!({"ok": false, "error": e}));
                }
            }
            Json(data)
        }
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
