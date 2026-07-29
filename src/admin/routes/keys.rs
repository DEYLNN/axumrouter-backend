use std::sync::Arc;

use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct KeysQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Serialize)]
pub struct KeyListItem {
    pub id: String,
    pub provider_id: String,
    pub label: Option<String>,
    pub key_type: String,
    pub is_active: bool,
    pub key_preview: String,
    pub key_value: String,
    pub locked_until: Option<String>,
    pub last_error_status: Option<i64>,
    pub last_error_message: Option<String>,
    pub last_error_at: Option<String>,
    pub backoff_level: i64,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct KeysListResponse {
    pub keys: Vec<KeyListItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

pub async fn api_list_keys(
    State(state): State<Arc<AppState>>,
    Query(q): Query<KeysQuery>,
) -> Json<KeysListResponse> {
    let page = q.page.unwrap_or(1).max(1);
    let per_page = q.per_page.unwrap_or(100).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String, bool, Option<String>, Option<i64>, Option<String>, Option<String>, i64, String)>(
        "SELECT id, provider_id, label, COALESCE(key_type, 'apikey'), key_value, is_active, locked_until, last_error_status, last_error_message, last_error_at, backoff_level, created_at
         FROM api_keys ORDER BY provider_id, created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let keys = rows.into_iter().map(|(id, provider_id, label, key_type, key_value, is_active, locked_until, last_error_status, last_error_message, last_error_at, backoff_level, created_at)| {
        let preview = if key_value.len() > 12 {
            format!("{}...{}", &key_value[..6], &key_value[key_value.len() - 4..])
        } else {
            key_value.clone()
        };
        KeyListItem {
            id, provider_id, label, key_type, is_active,
            key_preview: preview, key_value,
            locked_until, last_error_status, last_error_message, last_error_at, backoff_level, created_at,
        }
    }).collect();

    Json(KeysListResponse { keys, total, page, per_page })
}

#[derive(Deserialize)]
pub struct AddKeyRequest {
    pub provider_id: String,
    pub key_value: String,
    pub label: Option<String>,
}

#[derive(Serialize)]
pub struct AddKeyResponse {
    pub success: bool,
    pub message: String,
}

pub async fn api_add_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddKeyRequest>,
) -> Json<AddKeyResponse> {
    let id = format!("key_{}", &Uuid::new_v4().to_string()[..8]);
    let label = req.label.unwrap_or_default();
    let key_type = "apikey";

    let result = sqlx::query(
        "INSERT INTO api_keys (id, provider_id, key_value, label, is_active, key_type) VALUES (?, ?, ?, ?, 1, ?)",
    )
    .bind(&id)
    .bind(&req.provider_id)
    .bind(&req.key_value)
    .bind(&label)
    .bind(key_type)
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            // Reload provider to pick up new key
            let mut pm = state.provider_manager.write().await;
            let _ = pm.reload_provider(&req.provider_id).await;
            drop(pm);

            // Auto-disable all models ONLY on the first key (0 → 1 transition).
            // Subsequent keys must NOT clobber the admin's manual toggle state.
            // Each new model must be explicitly enabled via /admin/api/models/toggle.
            let existing_key_count = crate::db::count_provider_keys(&state.db, &req.provider_id, true).await;
            if existing_key_count <= 1 {
                let pm = state.provider_manager.read().await;
                let models = pm.list_all_models_unfiltered().await;
                // Use the provider's actual model-prefix (may differ from the
                // DB row id for custom providers — e.g. `nx` vs `custom_nx`).
                let prefix = pm.get(&req.provider_id)
                    .and_then(|p| p.metadata().model_prefix)
                    .map(|pf| format!("{}/", pf))
                    .unwrap_or_else(|| format!("{}/", req.provider_id));
                let _ = pm;
                for m in models {
                    if m.id.starts_with(&prefix) {
                        let _ = sqlx::query(
                            "INSERT OR IGNORE INTO disabled_models (model_id) VALUES (?)",
                        )
                        .bind(&m.id)
                        .execute(&state.db)
                        .await;
                    }
                }
            }

            Json(AddKeyResponse {
                success: true,
                message: format!(
                    "Key {} added. Models auto-disabled; toggle them in Models page.",
                    id
                ),
            })
        }
        Err(e) => Json(AddKeyResponse {
            success: false,
            message: format!("Failed: {}", e),
        }),
    }
}

#[derive(Deserialize)]
pub struct DeleteKeyRequest {
    pub provider_id: Option<String>,
    pub key_id: String,
}

pub async fn api_delete_key(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DeleteKeyRequest>,
) -> Json<AddKeyResponse> {
    // Get provider_id from the key we're about to delete
    let provider_id: Option<String> = if let Some(pid) = &req.provider_id {
        Some(pid.clone())
    } else {
        sqlx::query_scalar("SELECT provider_id FROM api_keys WHERE id=?")
            .bind(&req.key_id)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None)
    };

    let result = sqlx::query("DELETE FROM api_keys WHERE id=?")
        .bind(&req.key_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(_) => {
            // Reload provider
            if let Some(pid) = &provider_id {
                let mut pm = state.provider_manager.write().await;
                let _ = pm.reload_provider(pid).await;
            }
            Json(AddKeyResponse {
                success: true,
                message: "Key deleted".into(),
            })
        }
        Err(e) => Json(AddKeyResponse {
            success: false,
            message: format!("Failed: {}", e),
        }),
    }
}
