use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

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
            Json(AddKeyResponse {
                success: true,
                message: format!("Key {} added", id),
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
