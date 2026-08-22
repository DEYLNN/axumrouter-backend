use std::sync::Arc;

use axum::{extract::{Path, State}, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct OAuthKey {
    pub id: String,
    pub provider_id: String,
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct QuotaResponse {
    pub provider_id: Option<String>,
    pub error: Option<String>,
    pub expires_at: Option<String>,
    pub last_refresh: Option<String>,
    pub key_plan: Option<String>,
    pub rate_limits: Vec<serde_json::Value>,
    pub reset_credits: serde_json::Value,
}

pub async fn api_oauth_keys(State(state): State<Arc<AppState>>) -> Json<Vec<OAuthKey>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT id, provider_id, label FROM api_keys WHERE COALESCE(key_type, 'apikey') = 'oauth' ORDER BY provider_id, created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    Json(rows.into_iter().map(|(id, provider_id, label)| OAuthKey { id, provider_id, label }).collect())
}

pub async fn api_usage_quota(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<String>,
) -> Json<QuotaResponse> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT key_value, provider_id, created_at FROM api_keys WHERE id = ? AND COALESCE(key_type, 'apikey') = 'oauth'",
    )
    .bind(&key_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let Some((raw, provider_id, created_at)) = row else {
        return Json(QuotaResponse { provider_id: None, error: Some("OAuth key not found".into()), expires_at: None, last_refresh: None, key_plan: None, rate_limits: vec![], reset_credits: serde_json::json!({"available_count": 0, "applicable_available_count": 0}) });
    };
    let kv: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let expires_at = kv.get("expires_at").or_else(|| kv.get("expiresAt")).and_then(|v| v.as_str()).map(String::from);
    let last_refresh = kv.get("last_refresh").and_then(|v| v.as_str()).map(String::from).or_else(|| Some(created_at));
    let key_plan = kv.get("plan").or_else(|| kv.get("codex_plan")).and_then(|v| v.as_str()).map(String::from);

    let (rate_limits, wham_plan, reset_credits) = if provider_id == "cx" {
        match kv.get("access_token").or_else(|| kv.get("accessToken")).and_then(|v| v.as_str()) {
            Some(token) => crate::providers::oauth::codex::usage::fetch_wham_usage(token).await,
            None => (vec![], None, serde_json::json!({"available_count": 0, "applicable_available_count": 0})),
        }
    } else { (vec![], None, serde_json::json!({"available_count": 0, "applicable_available_count": 0})) };

    Json(QuotaResponse { provider_id: Some(provider_id), error: None, expires_at, last_refresh, key_plan: wham_plan.or(key_plan), rate_limits, reset_credits })
}

pub async fn api_refresh_token(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<String>,
) -> Json<serde_json::Value> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT key_value, provider_id FROM api_keys WHERE id = ? AND COALESCE(key_type, 'apikey') = 'oauth'",
    ).bind(&key_id).fetch_optional(&state.db).await.unwrap_or(None);
    let Some((raw, provider_id)) = row else { return Json(serde_json::json!({"ok": false, "success": false, "error": "OAuth key not found"})); };
    if provider_id != "cx" { return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Unsupported OAuth provider: {provider_id}")})); }
    let mut kv: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let Some(refresh_token) = kv.get("refresh_token").or_else(|| kv.get("refreshToken")).and_then(|v| v.as_str()).filter(|v| !v.is_empty()) else {
        return Json(serde_json::json!({"ok": false, "success": false, "error": "No refresh_token available"}));
    };
    let response = match reqwest::Client::new().post(crate::providers::oauth::codex::constants::OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[("grant_type", "refresh_token"), ("refresh_token", refresh_token), ("client_id", crate::providers::oauth::codex::constants::CLIENT_ID)])
        .send().await { Ok(r) => r, Err(e) => return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Network error: {e}")})) };
    if !response.status().is_success() { return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Refresh failed: HTTP {}", response.status())})); }
    let tokens: serde_json::Value = match response.json().await { Ok(v) => v, Err(e) => return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Parse error: {e}")})) };
    let Some(access) = tokens.get("access_token").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) else { return Json(serde_json::json!({"ok": false, "success": false, "error": "No access_token in response"})); };
    kv["access_token"] = serde_json::Value::String(access.into());
    if let Some(refresh) = tokens.get("refresh_token").and_then(|v| v.as_str()).filter(|v| !v.is_empty()) { kv["refresh_token"] = serde_json::Value::String(refresh.into()); }
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(seconds) = tokens.get("expires_in").and_then(|v| v.as_i64()) { kv["expires_at"] = serde_json::Value::String((chrono::Utc::now() + chrono::Duration::seconds(seconds)).to_rfc3339()); }
    kv["last_refresh"] = serde_json::Value::String(now);
    let updated = kv.to_string();
    let expires_at = kv.get("expires_at").cloned().unwrap_or(serde_json::Value::Null);
    let last_refresh = kv.get("last_refresh").cloned().unwrap_or(serde_json::Value::Null);
    if sqlx::query("UPDATE api_keys SET key_value = ?, updated_at = datetime('now') WHERE id = ?").bind(updated).bind(&key_id).execute(&state.db).await.is_err() { return Json(serde_json::json!({"ok": false, "success": false, "error": "Failed to persist refreshed token"})); }
    let mut manager = state.provider_manager.write().await;
    let _ = manager.reload_provider(&provider_id).await;
    Json(serde_json::json!({"ok": true, "success": true, "message": "Token refreshed", "expires_at": expires_at, "last_refresh": last_refresh}))
}
