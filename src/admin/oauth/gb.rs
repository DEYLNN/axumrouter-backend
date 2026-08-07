use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use crate::state::AppState;

pub async fn start() -> Json<serde_json::Value> {
    match crate::providers::oauth::grok_build::oauth::start().await {
        Ok(data) => Json(data),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

pub async fn poll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let device_code = body.get("device_code").and_then(|v| v.as_str()).unwrap_or_default();
    match crate::providers::oauth::grok_build::oauth::poll(device_code).await {
        Ok(data) => {
            // Token bundle: store when access_token present.
            if data.get("access_token").is_some() {
                if let Err(e) = crate::providers::oauth::grok_build::oauth::save_token(&state, &data).await {
                    return Json(serde_json::json!({"error": e}));
                }
                return Json(serde_json::json!({"ok": true, "token": data["access_token"]}));
            }
            Json(data)
        }
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
