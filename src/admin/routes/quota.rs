use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct QuotaResponse {
    pub provider_id: Option<String>,
    pub error: Option<String>,
    pub expires_at: Option<String>,
    pub last_refresh: Option<String>,
    pub key_plan: Option<String>,
    pub rate_limits: Vec<serde_json::Value>,
}

/// Fetch Codex WHAM usage via provider module
async fn fetch_codex_usage(access_token: &str) -> (Vec<serde_json::Value>, Option<String>) {
    crate::providers::openai_codex::usage::fetch_wham_usage(access_token).await
}

pub async fn api_usage_quota(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<String>,
) -> Json<QuotaResponse> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT key_value, provider_id FROM api_keys WHERE id = ?"
    )
    .bind(&key_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    match row {
        Some((kv_str, provider_id)) => {
            let kv: serde_json::Value = serde_json::from_str(&kv_str).unwrap_or_default();
            let expires_at = kv["expires_at"].as_str().map(String::from);
            let last_refresh = kv["last_refresh"].as_str().map(String::from);
            let key_plan = kv["plan"].as_str().or(kv["codex_plan"].as_str()).map(String::from);

            // Fetch rate limits for cx (Codex) from WHAM API
            let (rate_limits, wham_plan) = if provider_id == "cx" {
                if let Some(token) = kv["access_token"].as_str() {
                    fetch_codex_usage(token).await
                } else {
                    (vec![], None)
                }
            } else {
                (vec![], None)
            };

            // WHAM plan_type overrides local key_plan
            let key_plan = wham_plan.or(key_plan);

            Json(QuotaResponse {
                provider_id: Some(provider_id),
                error: None,
                expires_at,
                last_refresh,
                key_plan,
                rate_limits,
            })
        }
        None => Json(QuotaResponse {
            provider_id: None,
            error: Some("Key not found".into()),
            expires_at: None,
            last_refresh: None,
            key_plan: None,
            rate_limits: vec![],
        }),
    }
}

pub async fn api_refresh_token(
    State(state): State<Arc<AppState>>,
    Path(key_id): Path<String>,
) -> Json<serde_json::Value> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT key_value, provider_id FROM api_keys WHERE id = ?"
    )
    .bind(&key_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (kv_str, provider_id) = match row {
        Some(r) => r,
        None => return Json(serde_json::json!({"ok": false, "success": false, "error": "Key not found"})),
    };

    let mut kv: serde_json::Value = serde_json::from_str(&kv_str).unwrap_or_default();
    let refresh_token = kv["refresh_token"].as_str().map(String::from);

    let refresh_token = match refresh_token {
        Some(t) if !t.is_empty() => t,
        _ => return Json(serde_json::json!({"ok": false, "success": false, "error": "No refresh_token available"})),
    };

    let (token_url, client_id) = match provider_id.as_str() {
        "xai" => ("https://auth.x.ai/oauth2/token", "b1a00492-073a-47ea-816f-4c329264a828"),
        "cx"  => ("https://auth.openai.com/oauth/token", "app_EMoamEEZ73f0CkXaXp7hrann"),
        _ => return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Unsupported provider: {}", provider_id)})),
    };

    let client = reqwest::Client::new();
    let mut params = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", &refresh_token),
    ];
    if !client_id.is_empty() {
        params.push(("client_id", client_id));
    }

    let resp = match client.post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Network error: {}", e)})),
    };

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Refresh failed: HTTP {} — {}", status, text)}));
    }

    let tokens: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Parse error: {}", e)})),
    };

    let new_access_token = tokens["access_token"].as_str().unwrap_or("").to_string();
    let new_refresh_token = tokens["refresh_token"].as_str().unwrap_or("").to_string();

    if new_access_token.is_empty() {
        return Json(serde_json::json!({"ok": false, "success": false, "error": "No access_token in response"}));
    }

    // Update key_value
    kv["access_token"] = serde_json::Value::String(new_access_token);
    if !new_refresh_token.is_empty() {
        kv["refresh_token"] = serde_json::Value::String(new_refresh_token);
    }
    if let Some(expires_in) = tokens["expires_in"].as_u64() {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64);
        kv["expires_at"] = serde_json::Value::String(expires_at.to_rfc3339());
        kv["expires_in"] = serde_json::Value::Number(serde_json::Number::from(expires_in));
    }
    kv["last_refresh"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());

    let updated = kv.to_string();
    let _ = sqlx::query("UPDATE api_keys SET key_value = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(&updated)
        .bind(&key_id)
        .execute(&state.db)
        .await;

    let mut pm = state.provider_manager.write().await;
    let _ = pm.reload_provider(&provider_id).await;

    Json(serde_json::json!({
        "ok": true,
        "success": true,
        "message": "Token refreshed",
        "expires_at": kv["expires_at"],
        "last_refresh": kv["last_refresh"],
    }))
}
