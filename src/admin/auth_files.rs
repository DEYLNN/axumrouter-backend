use std::sync::Arc;
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/admin/api/auth-files", get(get_auth_files_json))
        .route("/admin/auth-files/import", post(import_auth_files))
        .route("/admin/auth-files/delete", post(delete_selected))
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
