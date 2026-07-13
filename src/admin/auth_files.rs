use std::sync::Arc;
use axum::extract::{State, Path};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/auth-files", get(get_auth_files_json))
        .route("/admin/api/auth-files/download/:id", get(download_auth_file))
        .route("/admin/api/auth-files/import", post(import_auth_files))
        .route("/admin/api/auth-files/delete", post(delete_selected))
        .with_state(state)
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
        let preview = if kt == "apikey" {
            // API key: try to extract from JSON or show raw
            let key_str = parsed.get("apiKey").or(parsed.get("apiToken")).or(parsed.get("key"))
                .and_then(|v| v.as_str()).unwrap_or(kv);
            key_str.chars().take(16).collect::<String>() + "..." + &key_str.chars().last().map(|c| c.to_string()).unwrap_or_default()
        } else {
            // OAuth: show access token preview
            if let Some(at) = parsed.get("access_token").and_then(|v| v.as_str()) {
                at.chars().take(16).collect::<String>() + "..." + &at.chars().last().map(|c| c.to_string()).unwrap_or_default()
            } else {
                "••••••••".to_string()
            }
        };
        let expires_at: String = {
            // Try string fields first: expires_at, expiresIn, expires_in
            if let Some(v) = parsed.get("expires_at").or(parsed.get("expiresIn")).or(parsed.get("expires_in")) {
                if let Some(s) = v.as_str() { s.to_string() }
                // Fallback: numeric expires_in → compute absolute from created_at
                else if let Some(secs) = v.as_u64() {
                    if let Ok(ct) = chrono::DateTime::parse_from_rfc3339(created) {
                        (ct + chrono::Duration::seconds(secs as i64)).to_rfc3339()
                    } else { String::new() }
                } else { String::new() }
            } else { String::new() }
        };
        serde_json::json!({
            "id": id, "key_value": kv, "label": label, "is_active": active,
            "created_at": created, "provider_id": prov, "key_type": kt,
            "key_preview": preview,
            "email": parsed.get("email").and_then(|v| v.as_str()).unwrap_or(""),
            "plan": parsed.get("chatgptPlanType").or(parsed.get("codex_plan")).and_then(|v| v.as_str()).unwrap_or(""),
            "has_access": parsed.get("access_token").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false),
            "has_refresh": !parsed.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("").is_empty(),
            "expires_at": expires_at,
        })
    }).collect();
    Json(serde_json::json!({ "files": entries }))
}

async fn download_auth_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT key_value, provider_id, COALESCE(key_type, 'apikey') FROM api_keys WHERE id = ?"
    ).bind(&id).fetch_optional(&state.db).await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match row {
        Some((kv, prov, kt)) => {
            let mut val: serde_json::Value = serde_json::from_str(&kv).unwrap_or(serde_json::Value::String(kv));
            if let serde_json::Value::String(s) = &val {
                // Plain key — wrap properly
                val = serde_json::json!({ "key": s });
            }
            val.as_object_mut().map(|obj| {
                obj.insert("provider_id".into(), serde_json::Value::String(prov));
                obj.insert("key_type".into(), serde_json::Value::String(kt));
            });
            Ok(Json(val))
        }
        None => Err((axum::http::StatusCode::NOT_FOUND, "Not found".into())),
    }
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
    let mut success = 0; let mut failed = 0; let mut reloaded = std::collections::HashSet::new();
    for item in &items {
        let kv = serde_json::to_string(item).unwrap_or_default();
        let email = item.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let prov = item.get("provider_id").and_then(|v| v.as_str()).unwrap_or("cx").to_string();
        let kt = item.get("key_type").and_then(|v| v.as_str()).unwrap_or("apikey").to_string();
        let count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id = ? AND key_value = ?").bind(&prov).bind(&kv).fetch_optional(&state.db).await.unwrap_or(None);
        if count.unwrap_or(0) > 0 { failed += 1; continue; }
        let kid = format!("key_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
        let now = chrono::Utc::now().to_rfc3339();
        let _ = sqlx::query("INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type, created_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?, ?)")
            .bind(&kid).bind(&prov).bind(&kv).bind(&email).bind(&kt).bind(&now).bind(&now).execute(&state.db).await;
        reloaded.insert(prov);
        success += 1;
    }
    for p in reloaded { let _ = state.provider_manager.write().await.reload_provider(&p).await; }
    Json(ImportResponse { success, failed, message: format!("{} ok, {} failed", success, failed) })
}
