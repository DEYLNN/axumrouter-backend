use std::sync::Arc;
use axum::extract::{State, Query, Path};
use axum::response::{Html, Redirect};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/auth-files", get(get_auth_files_json))
        .route("/admin/auth-files", get(get_auth_files))
        .route("/admin/auth-files/import", post(import_auth_files))
        .route("/admin/auth-files/delete", post(delete_selected))
        // OAuth endpoints
        .route("/admin/oauth/cx/start", get(oauth_cx_start))
        .route("/admin/oauth/cx/callback", get(oauth_cx_exchange))
        .route("/admin/oauth/xai/start", get(oauth_xai_start))
        .route("/admin/oauth/xai/callback", get(oauth_xai_exchange))
        .route("/admin/oauth/fb/start", get(oauth_fb_start))
        .route("/admin/oauth/fb/poll", get(oauth_fb_poll))
        .with_state(state)
}

#[derive(Deserialize)]
struct AuthFilter {
    #[serde(default)] q: String,
    #[serde(default = "default_provider")] provider: String,
}

fn default_provider() -> String { "all".to_string() }

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn html_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// ── JSON API for React FE ──

async fn get_auth_files_json(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let rows: Vec<(String, String, Option<String>, i64, String, String, String)> = sqlx::query_as(
        "SELECT id, key_value, label, is_active, created_at, provider_id, COALESCE(key_type, 'apikey') FROM api_keys ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let entries: Vec<serde_json::Value> = rows.iter().map(|(id, kv, label, active, created, prov, kt)| {
        let parsed: serde_json::Value = serde_json::from_str(kv).unwrap_or_default();
        serde_json::json!({
            "id": id, "key_value": kv, "label": label, "is_active": active,
            "created_at": created, "provider_id": prov, "key_type": kt,
            "email": parsed.get("email").and_then(|v| v.as_str()).unwrap_or(""),
            "plan": parsed.get("chatgptPlanType").or(parsed.get("codex_plan")).and_then(|v| v.as_str()).unwrap_or(""),
            "has_refresh": !parsed.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("").is_empty(),
        })
    }).collect();
    Json(serde_json::json!({ "files": entries }))
}

async fn get_auth_files(
    State(state): State<Arc<AppState>>,
    Query(f): Query<AuthFilter>,
) -> Html<String> {
    let rows: Vec<(String, String, Option<String>, i64, String, String, String)> = sqlx::query_as(
        "SELECT id, key_value, label, is_active, created_at, provider_id, COALESCE(key_type, 'apikey') FROM api_keys ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut entries = Vec::new();
    for (id, kv, label, is_active, _created, prov, _kt) in &rows {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(kv) {
            let email = json_str(&v, "email");
            if !f.q.is_empty() && !email.to_lowercase().contains(&f.q.to_lowercase()) { continue; }
            entries.push((id.clone(), email, prov.clone(), label.clone(), *is_active != 0));
        }
    }

    let mut rows_html = String::new();
    for (id, email, prov, label, active) in &entries {
        let dot = if *active { "bg-green-500" } else { "bg-gray-500" };
        rows_html.push_str(&format!(
            r#"<div class="flex items-center gap-3 px-4 py-3 rounded-xl border border-slate-700/50 bg-black/20">
                <div class="w-2 h-2 {dot} rounded-full shrink-0"></div>
                <div class="min-w-0 flex-1">
                    <div class="text-xs font-mono text-gray-200 truncate">{email}</div>
                    <span class="text-[9px] font-mono text-gray-500">{prov}</span>
                </div>
                <span class="text-[9px] font-mono text-gray-600">{label}</span>
            </div>"#,
            dot=dot, email=html_esc(email), prov=html_esc(prov), label=html_esc(label.as_deref().unwrap_or("")),
        ));
    }
    if rows_html.is_empty() { rows_html = r#"<div class="text-center py-8 text-xs font-mono text-gray-500">No OAuth files.</div>"#.into(); }

    Html(format!(r#"<!DOCTYPE html><html><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Auth Files</title><script src="https://cdn.tailwindcss.com"></script>
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&display=swap" rel="stylesheet">
<style>body{{font-family:'JetBrains Mono',monospace;background:#020617;color:#e5e7eb}}</style></head>
<body><div class="p-6 max-w-5xl mx-auto space-y-4">
<h1 class="text-xl font-mono font-bold text-cyan-300">AUTH FILES</h1>
<div class="text-[10px] font-mono text-gray-500">Total: <span class="text-cyan-300">{total}</span></div>
<form class="flex gap-2" method="get"><input type="text" name="q" value="{q}" placeholder="Search..." class="flex-1 bg-black/60 border border-slate-700 rounded-lg px-3 py-1.5 text-[11px] font-mono text-gray-200 placeholder-gray-600 focus:outline-none focus:border-cyan-500"></form>
<div class="space-y-1.5">{rows}</div>
<div class="flex gap-2 mt-4">
    <a href="/admin/oauth/cx/start" class="px-4 py-2 bg-cyan-500/20 border border-cyan-500/40 rounded text-[10px] font-mono text-cyan-300 hover:bg-cyan-500/30">+ Connect Cursor</a>
    <a href="/admin/oauth/xai/start" class="px-4 py-2 bg-cyan-500/20 border border-cyan-500/40 rounded text-[10px] font-mono text-cyan-300 hover:bg-cyan-500/30">+ Connect xAI</a>
</div></div></body></html>"#,
        total=entries.len(), q=html_esc(&f.q), rows=rows_html))
}

// ── Bulk actions ──

#[derive(Deserialize)] struct BulkAction { ids: Vec<String> }
#[derive(Serialize)] struct BulkResponse { success: bool, message: String }

async fn delete_selected(State(state): State<Arc<AppState>>, Json(action): Json<BulkAction>) -> Json<BulkResponse> {
    let mut done = 0;
    for id in &action.ids {
        let prov: Option<String> = sqlx::query_scalar("SELECT provider_id FROM api_keys WHERE id = ?").bind(id).fetch_optional(&state.db).await.unwrap_or(None);
        if let Some(p) = prov {
            let _ = sqlx::query("DELETE FROM api_keys WHERE id = ?").bind(id).execute(&state.db).await;
            let _ = state.provider_manager.write().await.reload_provider(&p).await;
            done += 1;
        }
    }
    Json(BulkResponse { success: true, message: format!("Deleted {} key(s)", done) })
}

#[derive(Serialize)] struct ImportResponse { success: usize, failed: usize, message: String }

async fn import_auth_files(State(state): State<Arc<AppState>>, Json(body): Json<serde_json::Value>) -> Json<ImportResponse> {
    let items = if let Some(arr) = body.as_array() { arr.clone() } else if body.is_object() { vec![body.clone()] } else { return Json(ImportResponse { success: 0, failed: 1, message: "Invalid".into() }); };
    let mut success = 0; let mut failed = 0;
    for item in &items {
        let kv = serde_json::to_string(item).unwrap_or_default();
        let email = item.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let prov = item.get("provider_id").and_then(|v| v.as_str()).unwrap_or("cx").to_string();
        let count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id = ? AND key_value = ?").bind(&prov).bind(&kv).fetch_optional(&state.db).await.unwrap_or(None);
        if count.unwrap_or(0) > 0 { failed += 1; continue; }
        let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
        let now = chrono::Utc::now().to_rfc3339();
        let _ = sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 'oauth', ?, ?)")
            .bind(&kid).bind(&prov).bind(&kv).bind(&email).bind(&now).bind(&now).execute(&state.db).await;
        success += 1;
    }
    if success > 0 { let _ = state.provider_manager.write().await.reload_provider("cx").await; }
    Json(ImportResponse { success, failed, message: format!("{} ok, {} failed", success, failed) })
}

// ── OAuth flows (simplified) ──

#[derive(Serialize)] struct OAuthStartResponse { url: String }

async fn oauth_cx_start() -> Json<OAuthStartResponse> {
    let state = uuid::Uuid::new_v4().to_string();
    let url = format!("https://auth.openai.com/oauth/authorize?response_type=code&client_id=cursor_app&redirect_uri=http://localhost:3000/admin/oauth/cx/callback&scope=openid+profile+email+offline_access&state={}", state);
    Json(OAuthStartResponse { url })
}

async fn oauth_cx_exchange(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let code = params.get("code").cloned().unwrap_or_default();
    let client = reqwest::Client::new();
    let resp = client.post("https://auth.openai.com/oauth/token")
        .form(&[("grant_type","authorization_code"),("code",&code),("redirect_uri","http://localhost:3000/admin/oauth/cx/callback"),("client_id","cursor_app")])
        .send().await.map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;
    let token: serde_json::Value = resp.json().await.map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;

    let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let now = chrono::Utc::now().to_rfc3339();
    let kv = serde_json::to_string(&token).unwrap_or_default();
    let _ = sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'cx', ?, 'cursor-oauth', 1, 'oauth', ?, ?)")
        .bind(&kid).bind(&kv).bind(&now).bind(&now).execute(&state.db).await;
    let _ = state.provider_manager.write().await.reload_provider("cx").await;
    Ok(Redirect::to("/admin/auth-files"))
}

async fn oauth_xai_start() -> Redirect {
    let state = uuid::Uuid::new_v4().to_string();
    let url = format!("https://auth.x.ai/oauth/authorize?response_type=code&client_id=xai_client&redirect_uri=http://localhost:3000/admin/oauth/xai/callback&scope=openid+profile+email&state={}", state);
    Redirect::to(&url)
}

async fn oauth_xai_exchange(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Redirect, (axum::http::StatusCode, String)> {
    let code = params.get("code").cloned().unwrap_or_default();
    let client = reqwest::Client::new();
    let resp = client.post("https://auth.x.ai/oauth/token")
        .form(&[("grant_type","authorization_code"),("code",&code),("redirect_uri","http://localhost:3000/admin/oauth/xai/callback"),("client_id","xai_client")])
        .send().await.map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;
    let token: serde_json::Value = resp.json().await.map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))?;
    let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let now = chrono::Utc::now().to_rfc3339();
    let kv = serde_json::to_string(&token).unwrap_or_default();
    let _ = sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'xai', ?, 'xai-oauth', 1, 'oauth', ?, ?)")
        .bind(&kid).bind(&kv).bind(&now).bind(&now).execute(&state.db).await;
    let _ = state.provider_manager.write().await.reload_provider("xai").await;
    Ok(Redirect::to("/admin/auth-files"))
}

// ── FreeBuff OAuth (device_code flow) ──

async fn oauth_fb_start() -> Json<serde_json::Value> {
    let fingerprint_id = uuid::Uuid::new_v4().to_string();
    let client = reqwest::Client::new();
    let resp = client.post("https://www.codebuff.com/api/auth/cli/code")
        .json(&serde_json::json!({"fingerprintId":&fingerprint_id}))
        .header("Accept", "application/json")
        .send().await;
    match resp {
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            let data: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
            Json(serde_json::json!({
                "device_code": fingerprint_id,
                "user_code": "",
                "verification_uri": data.get("loginUrl").and_then(|v| v.as_str()).unwrap_or("https://www.codebuff.com"),
                "verification_uri_complete": data.get("loginUrl").and_then(|v| v.as_str()).unwrap_or(""),
                "expires_in": 600,
                "interval": 4,
                "_fingerprintHash": data.get("fingerprintHash"),
                "_expiresAt": data.get("expiresAt"),
                "_loginUrl": data.get("loginUrl"),
            }))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn oauth_fb_poll(
    Query(params): Query<std::collections::HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let device_code = params.get("device_code").cloned().unwrap_or_default();
    let fingerprint_hash = params.get("_fingerprintHash").cloned().unwrap_or_default();
    let expires_at = params.get("_expiresAt").cloned().unwrap_or_default();

    if fingerprint_hash.is_empty() || expires_at.is_empty() {
        return Json(serde_json::json!({"error": "authorization_pending", "message": "Waiting for credentials"}));
    }

    let url = format!("https://www.codebuff.com/api/auth/cli/status?fingerprintId={}&fingerprintHash={}&expiresAt={}",
        device_code, fingerprint_hash, expires_at);
    let client = reqwest::Client::new();
    let resp = client.get(&url).header("Accept", "application/json").send().await;
    match resp {
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            let data: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({"error":"parse_failed"}));
            // If we got credentials back, save them
            if data.get("accessToken").or(data.get("token")).is_some() {
                let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
                let now = chrono::Utc::now().to_rfc3339();
                let kv = serde_json::to_string(&data).unwrap_or_default();
                let _ = sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, 'fb', ?, 'freebuff-oauth', 1, 'oauth', ?, ?)")
                    .bind(&kid).bind(&kv).bind(&now).bind(&now).execute(&state.db).await;
                let _ = state.provider_manager.write().await.reload_provider("fb").await;
            }
            Json(data)
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}
