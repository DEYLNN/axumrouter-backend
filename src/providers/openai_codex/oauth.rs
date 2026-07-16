use std::sync::Arc;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::state::AppState;

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

pub async fn start() -> OAuthStartResponse {
    use rand::RngCore;
    let oauth_state = uuid::Uuid::new_v4().to_string();
    let mut code_verifier_bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut code_verifier_bytes);
    let code_verifier = URL_SAFE_NO_PAD.encode(&code_verifier_bytes);
    let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()).as_slice());
    
    pkce_store().lock().unwrap().insert(oauth_state.clone(), code_verifier);

    let client_id = super::constants::CLIENT_ID;
    let redirect_uri = urlencoding::encode("http://localhost:1455/auth/callback");
    let scope = urlencoding::encode("openid profile email offline_access");
    
    let url = format!(
        "https://auth.openai.com/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&id_token_add_organizations=true&codex_cli_simplified_flow=true&originator=codex_cli_rs",
        client_id, redirect_uri, scope, oauth_state, code_challenge
    );
    
    OAuthStartResponse { url, id: Some(oauth_state) }
}

#[derive(Deserialize)]
pub struct ManualCodeRequest {
    pub code: String,
    #[serde(alias = "id")]
    pub state: Option<String>,
}

pub async fn exchange_code(
    code: &str,
    oauth_state: &str,
) -> Result<serde_json::Value, String> {
    let code_verifier = pkce_store().lock().unwrap().remove(oauth_state).unwrap_or_default();
    let client_id = super::constants::CLIENT_ID;
    let redirect_uri = "http://localhost:1455/auth/callback";

    let client = reqwest::Client::new();
    let resp = client.post("https://auth.openai.com/oauth/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", &code_verifier),
        ])
        .send().await.map_err(|e| format!("HTTP: {}", e))?;
    
    let token: serde_json::Value = resp.json().await.map_err(|e| format!("Parse: {}", e))?;
    
    if token.get("error").is_some() {
        return Err(format!("{:?}", token.get("error")));
    }
    
    Ok(token)
}

pub async fn save_token(state: &Arc<AppState>, token: &serde_json::Value) -> Result<(), String> {
    let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let now = chrono::Utc::now().to_rfc3339();
    // Convert expires_in (seconds) to absolute expires_at ISO string
    let mut enriched = token.clone();
    if let Some(exp_in) = token.get("expires_in").and_then(|v| v.as_u64()) {
        let exp_at = chrono::Utc::now() + chrono::Duration::seconds(exp_in as i64);
        enriched.as_object_mut().map(|obj| {
            obj.insert("expires_at".into(), serde_json::Value::String(exp_at.to_rfc3339()));
        });
    }
    let kv = serde_json::to_string(&enriched).map_err(|e| format!("Serialize: {}", e))?;
    
    // Extract email from id_token for label
    let label = token.get("id_token").and_then(|t| {
        let s = t.as_str()?;
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 { return None; }
        let decoded = URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
        let payload: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
        payload.get("email").and_then(|v| v.as_str()).map(|e| e.to_string())
    }).unwrap_or_else(|| format!("codex-{}", &kid[4..12]));

    sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'cx', ?, ?, 1, 'oauth', ?, ?)")
        .bind(&kid).bind(&kv).bind(&label).bind(&now).bind(&now)
        .execute(&state.db).await.map_err(|e| format!("DB: {}", e))?;
    
    let _ = state.provider_manager.write().await.reload_provider("cx").await;
    Ok(())
}
