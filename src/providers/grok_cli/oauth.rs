use std::sync::Arc;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

fn pkce_store() -> &'static Mutex<HashMap<String, String>> {
    static STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Serialize)]
pub struct OAuthStartResponse {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Deserialize)]
pub struct ManualCodeRequest {
    pub code: String,
    #[serde(alias = "id")]
    pub state: Option<String>,
}

fn extract_email(token: &serde_json::Value) -> Option<String> {
    let raw = token.get("id_token").and_then(|v| v.as_str())?;
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 3 { return None; }
    let decoded = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    payload.get("email").and_then(|v| v.as_str())
        .or_else(|| payload.get("preferred_username").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

pub async fn start() -> OAuthStartResponse {
    use rand::RngCore;
    let state = uuid::Uuid::new_v4().to_string();
    let mut verifier_bytes = [0u8; 96];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let code_verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);
    let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()).as_slice());
    pkce_store().lock().unwrap().insert(state.clone(), code_verifier);

    let mut nonce_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce: String = nonce_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let url = format!(
        "https://auth.x.ai/oauth2/authorize?response_type=code&client_id=b1a00492-073a-47ea-816f-4c329264a828&redirect_uri=http://127.0.0.1:56121/callback&scope=openid%20profile%20email%20offline_access%20grok-cli%3Aaccess%20api%3Aaccess%20conversations%3Aread%20conversations%3Awrite&state={}&code_challenge={}&code_challenge_method=S256&nonce={}&plan=generic&referrer=grok-build",
        state, code_challenge, nonce
    );
    OAuthStartResponse { url, id: Some(state) }
}

pub async fn exchange_code(code: &str, oauth_state: &str) -> Result<serde_json::Value, String> {
    let code_verifier = {
        let mut store = pkce_store().lock().unwrap();
        if !oauth_state.is_empty() {
            store.remove(oauth_state)
        } else {
            let key = store.keys().next().cloned();
            key.map(|k| store.remove(&k)).flatten()
        }.unwrap_or_default()
    };
    if code_verifier.is_empty() {
        return Err("Session expired — start OAuth again".into());
    }

    let client = reqwest::Client::new();
    let resp = client.post("https://auth.x.ai/oauth2/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", "http://127.0.0.1:56121/callback"),
            ("client_id", "b1a00492-073a-47ea-816f-4c329264a828"),
            ("code_verifier", &code_verifier),
        ])
        .send().await.map_err(|e| format!("HTTP: {}", e))?;

    let token: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    if token.get("error").is_some() {
        return Err(format!("{:?}", token.get("error")));
    }
    Ok(token)
}

/// Refresh xAI access token using refresh_token
pub async fn refresh_access_token(refresh_token: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let resp = client.post("https://auth.x.ai/oauth2/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", "b1a00492-073a-47ea-816f-4c329264a828"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|e| format!("HTTP: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        // Detect permanent errors
        let lower = text.to_lowercase();
        if lower.contains("invalid_grant") || lower.contains("refresh_token_expired") || lower.contains("refresh_token_reused") {
            return Err(format!("permanent: {}", text));
        }
        return Err(format!("HTTP {}: {}", status, text));
    }

    let token: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    Ok(token)
}

pub async fn save_token(state: &Arc<crate::state::AppState>, token: &serde_json::Value) -> Result<(), String> {
    let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let now = chrono::Utc::now().to_rfc3339();
    let mut enriched = token.clone();
    if let Some(exp_in) = token.get("expires_in").and_then(|v| v.as_u64()) {
        let exp_at = chrono::Utc::now() + chrono::Duration::seconds(exp_in as i64);
        enriched.as_object_mut().map(|obj| {
            obj.insert("expires_at".into(), serde_json::Value::String(exp_at.to_rfc3339()));
        });
    }
    let kv = serde_json::to_string(&enriched).map_err(|e| format!("Serialize: {}", e))?;
    let label = extract_email(token).unwrap_or_else(|| format!("grok-{}", &kid[4..12]));

    sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'gb', ?, ?, 1, 'oauth', ?, ?)")
        .bind(&kid).bind(&kv).bind(&label).bind(&now).bind(&now)
        .execute(&state.db).await.map_err(|e| format!("DB: {}", e))?;

    let _ = state.provider_manager.write().await.reload_provider("gb").await;
    Ok(())
}
