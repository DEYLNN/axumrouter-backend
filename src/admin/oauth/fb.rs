use std::sync::Arc;
use axum::extract::State;
use axum::Json;
use crate::state::AppState;

pub async fn start() -> Json<serde_json::Value> {
    match crate::providers::freebuff::oauth::start().await {
        Ok(data) => Json(data),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

pub async fn poll(
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
