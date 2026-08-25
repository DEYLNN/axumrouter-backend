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

fn cb_time(v: &serde_json::Value) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Some(n) = v.as_i64() { return chrono::DateTime::from_timestamp(if n.abs() < 1_000_000_000_000 { n } else { n / 1000 }, 0); }
    v.as_str().and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&chrono::Utc)).or_else(|| chrono::NaiveDateTime::parse_from_str(v.as_str()?, "%Y-%m-%d %H:%M:%S").ok().map(|d| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(d, chrono::Utc)))
}

async fn fetch_cbai_quota(token: &str) -> Result<(Vec<serde_json::Value>, Option<String>), String> {
    let r = reqwest::Client::new().post("https://www.codebuddy.ai/v2/billing/meter/get-user-resource")
        .bearer_auth(token).header("Accept", "application/json").header("Content-Type", "application/json")
        .header("User-Agent", "IDE/2.108.1 CodeBuddy/2.108.1").header("X-Product", "SaaS")
        .header("X-IDE-Type", "IDE").header("X-IDE-Name", "IDE").header("X-Requested-With", "XMLHttpRequest")
        .header("X-Domain", "www.codebuddy.ai").json(&serde_json::json!({})).send().await
        .map_err(|e| format!("CodeBuddy quota request: {e}"))?;
    let status=r.status(); let body:serde_json::Value=r.json().await.map_err(|e|format!("CodeBuddy quota parse: {e}"))?;
    if !status.is_success(){return Err(format!("CodeBuddy quota HTTP {status}"));}
    if body["code"]!=0{return Err(body["msg"].as_str().unwrap_or("CodeBuddy quota error").into());}
    let accounts=body["data"]["Response"]["Data"]["Accounts"].as_array().cloned().unwrap_or_default(); let mut rows=vec![]; let mut plan=None;
    for (i,a) in accounts.iter().enumerate(){
        plan=plan.or_else(||a["PackageName"].as_str().or_else(||a["SubProductName"].as_str()).map(String::from));
        let ce=cb_time(&a["CycleEndTime"]); let cs=cb_time(&a["CycleStartTime"]); let de=cb_time(&a["DeductionEndTime"]);
        let refill=ce.zip(de).map(|(c,d)|d-c>chrono::Duration::days(2)).unwrap_or(false);
        let uk=if refill{"CycleCapacityUsedPrecise"}else{"CapacityUsedPrecise"}; let tk=if refill{"CycleCapacitySizePrecise"}else{"CapacitySizePrecise"};
        let used=a[uk].as_f64().or_else(||a[uk].as_str().and_then(|x|x.parse().ok())).unwrap_or(0.0); let total=a[tk].as_f64().or_else(||a[tk].as_str().and_then(|x|x.parse().ok())).unwrap_or(0.0);
        let name=if refill{let d=cs.zip(ce).map(|(s,e)|(e-s).num_days()).unwrap_or(30);if d<=1{"Daily".into()}else if d<=10{"Weekly".into()}else{"Monthly".into()}}else{format!("Bonus Pack {}",i+1)};
        rows.push(serde_json::json!({"name":name,"limit":total,"remaining":(total-used).max(0.0),"used":used,"period_seconds":cs.zip(ce).map(|(s,e)|(e-s).num_seconds()),"reset_at":ce.or(de).map(|d|d.to_rfc3339())}));
    } Ok((rows,plan))
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
    } else if provider_id == "cbai" {
        match kv.get("access_token").and_then(|v| v.as_str()) { Some(token) => match fetch_cbai_quota(token).await { Ok((limits, plan)) => (limits, plan, serde_json::json!({"available_count":0,"applicable_available_count":0})), Err(e) => return Json(QuotaResponse { provider_id: Some(provider_id), error: Some(e), expires_at, last_refresh, key_plan, rate_limits: vec![], reset_credits: serde_json::json!({"available_count":0,"applicable_available_count":0}) }) }, None => (vec![],None,serde_json::json!({"available_count":0,"applicable_available_count":0})) }
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
    let mut kv: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    if provider_id != "cx" && provider_id != "cbai" { return Json(serde_json::json!({"ok": false, "success": false, "error": format!("Unsupported OAuth provider: {provider_id}")})); }
    if provider_id == "cbai" {
        let Some(refresh_token) = kv.get("refresh_token").and_then(|v| v.as_str()) else { return Json(serde_json::json!({"ok": false, "success": false, "error": "No refresh_token available"})); };
        match crate::providers::oauth::codebuddy_intl::oauth::refresh(refresh_token).await {
            Ok(tokens) => {
                kv["access_token"] = tokens["access_token"].clone();
                if !tokens["refresh_token"].is_null() { kv["refresh_token"] = tokens["refresh_token"].clone(); }
                if let Some(s) = tokens["expires_in"].as_i64() { kv["expires_at"] = serde_json::Value::String((chrono::Utc::now() + chrono::Duration::seconds(s)).to_rfc3339()); }
                kv["last_refresh"] = serde_json::Value::String(chrono::Utc::now().to_rfc3339());
                let expires_at=kv["expires_at"].clone(); let last_refresh=kv["last_refresh"].clone();
                if sqlx::query("UPDATE api_keys SET key_value = ?, updated_at = datetime('now') WHERE id = ?").bind(kv.to_string()).bind(&key_id).execute(&state.db).await.is_err() { return Json(serde_json::json!({"ok":false,"success":false,"error":"Failed to persist refreshed token"})); }
                let _=state.provider_manager.write().await.reload_provider(&provider_id).await;
                return Json(serde_json::json!({"ok":true,"success":true,"message":"Token refreshed","expires_at":expires_at,"last_refresh":last_refresh}));
            },
            Err(e) => return Json(serde_json::json!({"ok":false,"success":false,"error":e})),
        }
    }
    // Existing Codex refresh path.
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
