use std::sync::Arc;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// Track current fingerprint hash/expiry for polling
fn poll_state() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub async fn start() -> Result<serde_json::Value, String> {
    let fingerprint_id = uuid::Uuid::new_v4().to_string();
    let client = reqwest::Client::new();
    let resp = client.post("https://www.codebuff.com/api/auth/cli/code")
        .json(&serde_json::json!({"fingerprintId": &fingerprint_id}))
        .header("Accept", "application/json")
        .send().await.map_err(|e| format!("HTTP: {}", e))?;
    let text = resp.text().await.map_err(|e| format!("Body: {}", e))?;
    let data: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    
    // Store fingerprint hash for polling
    if let Some(fh) = data.get("fingerprintHash").and_then(|v| v.as_str()) {
        poll_state().lock().unwrap().insert("fingerprint_hash".into(), fh.to_string());
    }
    if let Some(ea) = data.get("expiresAt") {
        let val = ea.as_i64().map(|n| n.to_string()).or_else(|| ea.as_str().map(|s| s.to_string())).unwrap_or_default();
        poll_state().lock().unwrap().insert("expires_at".into(), val);
    }

    Ok(serde_json::json!({
        "device_code": fingerprint_id,
        "user_code": "",
        "verification_uri": data.get("loginUrl").and_then(|v| v.as_str()).unwrap_or("https://www.codebuff.com"),
        "verification_uri_complete": data.get("loginUrl").and_then(|v| v.as_str()).unwrap_or(""),
        "expires_in": 600,
        "interval": 4,
        "fingerprint_hash": data.get("fingerprintHash"),
        "expires_at": data.get("expiresAt"),
        "_fingerprintHash": data.get("fingerprintHash"),
        "_expiresAt": data.get("expiresAt"),
        "_loginUrl": data.get("loginUrl"),
    }))
}

pub async fn poll(device_code: &str, fingerprint_hash: &str, expires_at: &str) -> Result<serde_json::Value, String> {
    let (fh, ea) = if fingerprint_hash.is_empty() || expires_at.is_empty() {
        let ps = poll_state().lock().unwrap();
        let fh = ps.get("fingerprint_hash").cloned().unwrap_or_default();
        let ea = ps.get("expires_at").cloned().unwrap_or_default();
        if fh.is_empty() || ea.is_empty() {
            return Ok(serde_json::json!({"error": "authorization_pending", "message": "Waiting for credentials"}));
        }
        (fh, ea)
    } else {
        (fingerprint_hash.to_string(), expires_at.to_string())
    };

    let url = format!("https://www.codebuff.com/api/auth/cli/status?fingerprintId={}&fingerprintHash={}&expiresAt={}",
        device_code, fh, ea);
    let client = reqwest::Client::new();
    let resp = client.get(&url).header("Accept", "application/json")
        .send().await.map_err(|e| format!("HTTP: {}", e))?;

    // 401 means not yet authorized (pending)
    if resp.status().as_u16() == 401 {
        return Ok(serde_json::json!({"error": "authorization_pending", "message": "Waiting for credentials"}));
    }

    let text = resp.text().await.map_err(|e| format!("Body: {}", e))?;
    let data: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({"error":"parse_failed"}));

    // Check if authorized
    if data.get("user").and_then(|u| u.get("authToken")).and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false) {
        let user_obj = data.get("user");
        let email = user_obj.and_then(|u| u.get("email")).and_then(|v| v.as_str()).unwrap_or("freebuff");
        let auth_token = user_obj.and_then(|u| u.get("authToken")).and_then(|v| v.as_str()).unwrap_or("");
        return Ok(serde_json::json!({
            "ok": true,
            "access_token": auth_token,
            "email": email,
            "_raw": data,
        }));
    }

    // Not yet authorized — return pending (not raw error)
    Ok(serde_json::json!({"error": "authorization_pending", "message": "Waiting for credentials"}))
}

pub async fn save_token(state: &Arc<crate::state::AppState>, data: &serde_json::Value) -> Result<(), String> {
    let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let now = chrono::Utc::now().to_rfc3339();
    let email = data.get("email").and_then(|v| v.as_str()).unwrap_or("freebuff");
    let raw = data.get("_raw").cloned().unwrap_or_default();
    let access_token = data.get("access_token").and_then(|v| v.as_str()).unwrap_or("");
    // Merge access_token into stored data so provider.rs can read kv["access_token"]
    let mut kv_obj = raw.as_object().cloned().unwrap_or_default();
    kv_obj.insert("access_token".into(), serde_json::Value::String(access_token.to_string()));
    if !email.is_empty() {
        kv_obj.insert("email".into(), serde_json::Value::String(email.to_string()));
    }
    let kv = serde_json::to_string(&serde_json::Value::Object(kv_obj)).map_err(|e| format!("Serialize: {}", e))?;

    sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'fb', ?, ?, 1, 'oauth', ?, ?)")
        .bind(&kid).bind(&kv).bind(email).bind(&now).bind(&now)
        .execute(&state.db).await.map_err(|e| format!("DB: {}", e))?;

    let _ = state.provider_manager.write().await.reload_provider("fb").await;
    Ok(())
}
