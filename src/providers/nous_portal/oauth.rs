// Nous Portal OAuth — Device Code Flow
use std::sync::Arc;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

use super::constants;

fn poll_state() -> &'static Mutex<HashMap<String, serde_json::Value>> {
    static STORE: OnceLock<Mutex<HashMap<String, serde_json::Value>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Deserialize)]
pub struct PollRequest {
    pub device_code: String,
}

pub async fn start() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let body = format!(
        "client_id={}&scope={}",
        urlencoding::encode(constants::CLIENT_ID),
        urlencoding::encode(constants::SCOPE),
    );
    let resp = client
        .post(constants::DEVICE_CODE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;

    if data.get("error").is_some() {
        return Err(format!("{}", data.get("error_description").and_then(|v| v.as_str()).unwrap_or("nous_error")));
    }

    let device_code = data["device_code"].as_str().unwrap_or("").to_string();
    let verification_uri = data["verification_uri"].as_str()
        .or(data["verification_uri_complete"].as_str())
        .unwrap_or("https://portal.nousresearch.com/activate").to_string();
    let verification_uri_complete = data["verification_uri_complete"].as_str()
        .or(data["verification_uri"].as_str())
        .unwrap_or(&verification_uri).to_string();

    // Store for polling
    poll_state().lock().unwrap().insert(device_code.clone(), data.clone());

    Ok(serde_json::json!({
        "verification_uri_complete": verification_uri_complete,
        "verification_uri": verification_uri,
        "device_code": device_code,
        "user_code": data["user_code"],
        "expires_in": data["expires_in"],
        "interval": data.get("interval").and_then(|v| v.as_u64()).unwrap_or(5),
    }))
}

pub async fn poll(device_code: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let body = format!(
        "grant_type={}&client_id={}&device_code={}",
        urlencoding::encode("urn:ietf:params:oauth:grant-type:device_code"),
        urlencoding::encode(constants::CLIENT_ID),
        urlencoding::encode(device_code),
    );
    let resp = client
        .post(constants::TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;

    let text = resp.text().await.unwrap_or_default();
    let data: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

    if let Some(err) = data.get("error") {
        let desc = data.get("error_description").and_then(|v| v.as_str()).unwrap_or("");
        if err.as_str() == Some("authorization_pending") || err.as_str() == Some("slow_down") {
            return Ok(serde_json::json!({"error": err, "message": desc}));
        }
        return Err(format!("{}: {}", err.as_str().unwrap_or("unknown"), desc));
    }

    if data.get("access_token").is_some() {
        poll_state().lock().unwrap().remove(device_code);
        return Ok(data);
    }

    Ok(serde_json::json!({"error": "authorization_pending", "message": "Waiting for authorization"}))
}

pub async fn save_token(state: &Arc<AppState>, token: &serde_json::Value) -> Result<(), String> {
    let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let now = chrono::Utc::now().to_rfc3339();

    let mut enriched = token.clone();
    if let Some(exp_in) = token.get("expires_in").and_then(|v| v.as_u64()) {
        let exp_at = chrono::Utc::now() + chrono::Duration::seconds(exp_in as i64);
        enriched.as_object_mut().map(|obj| {
            obj.insert("expires_at".into(), serde_json::Value::String(exp_at.to_rfc3339()));
            obj.insert("last_refresh".into(), serde_json::Value::String(now.clone()));
            obj.insert("inference_base_url".into(), serde_json::Value::String(constants::INFERENCE_URL.to_string()));
        });
    }

    let kv = serde_json::to_string(&enriched).map_err(|e| format!("Serialize: {}", e))?;
    let email = token.get("email").and_then(|v| v.as_str()).unwrap_or("nous-portal").to_string();

    sqlx::query(
        "INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'np', ?, ?, 1, 'oauth', ?, ?)"
    )
    .bind(&kid).bind(&kv).bind(&email).bind(&now).bind(&now)
    .execute(&state.db).await.map_err(|e| format!("DB: {}", e))?;

    state.provider_manager.write().await.reload_provider("np").await;
    Ok(())
}
