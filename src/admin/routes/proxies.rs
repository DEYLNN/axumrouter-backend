use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateProxyReq {
    pub label: String,
    pub protocol: Option<String>,
    pub host: String,
    pub port: i64,
    pub username: Option<String>,
    pub password: Option<String>,
    pub country: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProxyReq {
    pub label: Option<String>,
    pub protocol: Option<String>,
    pub host: Option<String>,
    pub port: Option<i64>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub country: Option<String>,
    pub is_active: Option<i64>,
}

pub async fn api_list_proxies(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query_as::<_, (
        String, String, String, String, i64,
        Option<String>, Option<String>, Option<String>,
        i64, i64, Option<String>, String, String,
    )>(
        "SELECT id, label, protocol, host, port, username, password, country, \
         is_active, usage_count, last_used, created_at, updated_at \
         FROM proxies ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, label, protocol, host, port, username, password, country,
            is_active, usage_count, last_used, created_at, updated_at)| {
        serde_json::json!({
            "id": id, "label": label, "protocol": protocol, "host": host,
            "port": port, "username": username, "password": password,
            "country": country, "is_active": is_active,
            "usage_count": usage_count, "last_used": last_used,
            "created_at": created_at, "updated_at": updated_at,
        })
    })
    .collect();
    Json(rows)
}

pub async fn api_create_proxy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProxyReq>,
) -> Json<serde_json::Value> {
    let id = format!("proxy_{}", Uuid::new_v4());
    let protocol = req.protocol.unwrap_or_else(|| "http".into());

    let result = sqlx::query(
        "INSERT INTO proxies (id, label, protocol, host, port, username, password, country) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&req.label)
    .bind(&protocol)
    .bind(&req.host)
    .bind(req.port)
    .bind(&req.username)
    .bind(&req.password)
    .bind(&req.country)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn api_update_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProxyReq>,
) -> Json<serde_json::Value> {
    if let Some(v) = &req.label {
        let _ = sqlx::query("UPDATE proxies SET label = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = &req.protocol {
        let _ = sqlx::query("UPDATE proxies SET protocol = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = &req.host {
        let _ = sqlx::query("UPDATE proxies SET host = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = req.port {
        let _ = sqlx::query("UPDATE proxies SET port = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = &req.username {
        let _ = sqlx::query("UPDATE proxies SET username = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = &req.password {
        let _ = sqlx::query("UPDATE proxies SET password = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = &req.country {
        let _ = sqlx::query("UPDATE proxies SET country = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    if let Some(v) = req.is_active {
        let _ = sqlx::query("UPDATE proxies SET is_active = ?, updated_at = datetime('now') WHERE id = ?")
            .bind(v).bind(&id).execute(&state.db).await;
    }
    Json(serde_json::json!({"ok": true}))
}

pub async fn api_delete_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let result = sqlx::query("DELETE FROM proxies WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;
    match result {
        Ok(_) => Json(serde_json::json!({"ok": true})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn api_toggle_proxy(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let _ = sqlx::query(
        "UPDATE proxies SET is_active = CASE WHEN is_active = 1 THEN 0 ELSE 1 END, \
         updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&id)
    .execute(&state.db)
    .await;
    Json(serde_json::json!({"ok": true}))
}

// --- Model-proxy assignments ---

pub async fn api_get_model_proxy(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Json<serde_json::Value> {
    let row = sqlx::query_as::<_, (
        String, String, String, String, i64,
        Option<String>, Option<String>, Option<String>, i64,
    )>(
        "SELECT p.id, p.label, p.protocol, p.host, p.port, p.username, p.password, p.country, p.is_active \
         FROM model_proxies mp JOIN proxies p ON mp.proxy_id = p.id \
         WHERE mp.model_id = ? AND mp.enabled = 1 AND p.is_active = 1 LIMIT 1"
    )
    .bind(&model_id)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some((id, label, protocol, host, port, username, password, country, is_active))) => {
            Json(serde_json::json!({
                "model_id": model_id,
                "proxy": {
                    "id": id, "label": label, "protocol": protocol,
                    "host": host, "port": port,
                    "username": username, "password": password,
                    "country": country, "is_active": is_active,
                }
            }))
        }
        _ => Json(serde_json::json!({"model_id": model_id, "proxy": null})),
    }
}

#[derive(Deserialize)]
pub struct AssignModelProxyReq {
    pub model_id: String,
    pub proxy_id: String,
}

pub async fn api_assign_model_proxy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AssignModelProxyReq>,
) -> Json<serde_json::Value> {
    let result = sqlx::query(
        "INSERT OR REPLACE INTO model_proxies (model_id, proxy_id, enabled) VALUES (?, ?, 1)"
    )
    .bind(&req.model_id)
    .bind(&req.proxy_id)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn api_unassign_model_proxy(
    State(state): State<Arc<AppState>>,
    Path(model_id): Path<String>,
) -> Json<serde_json::Value> {
    let result = sqlx::query("DELETE FROM model_proxies WHERE model_id = ?")
        .bind(&model_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => Json(serde_json::json!({"ok": true})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}
