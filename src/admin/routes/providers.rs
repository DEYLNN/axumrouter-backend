use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Serialize)]
pub struct ProviderListItem {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub color: String,
    pub icon_url: String,
    pub category: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub total_keys: i64,
    pub active_keys: i64,
    pub locked_keys: i64,
    pub oauth_flow: Option<String>,
}

pub async fn api_providers(State(state): State<Arc<AppState>>) -> Json<Vec<ProviderListItem>> {
    let pm = state.provider_manager.read().await;
    let names = pm.provider_names();
    let mut out = Vec::new();
    for id in names {
        if let Some(p) = pm.get(id) {
            let meta = p.metadata();
            let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id=?")
                .bind(id).fetch_one(&state.db).await.unwrap_or(0);
            let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id=? AND is_active=1")
                .bind(id).fetch_one(&state.db).await.unwrap_or(0);
            let locked: i64 = total - active;
            out.push(ProviderListItem {
                id: id.to_string(),
                name: meta.display_name.clone(),
                display_name: meta.display_name.clone(),
                color: meta.color.clone(),
                icon_url: meta.icon_url.clone(),
                category: meta.category.clone(),
                provider_type: meta.category.clone(),
                total_keys: total,
                active_keys: active,
                locked_keys: locked,
                oauth_flow: meta.oauth_flow.clone(),
            });
        }
    }
    Json(out)
}

pub async fn api_provider_detail(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> Json<serde_json::Value> {
    let pm = state.provider_manager.read().await;
    if let Some(p) = pm.get(&provider_id) {
        let meta = p.metadata();
        let total_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id=?")
            .bind(&provider_id).fetch_one(&state.db).await.unwrap_or(0);
        let active_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id=? AND is_active=1")
            .bind(&provider_id).fetch_one(&state.db).await.unwrap_or(0);
        let locked_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE provider_id=? AND is_active=0")
            .bind(&provider_id).fetch_one(&state.db).await.unwrap_or(0);
        let key_type: String = meta.category.clone(); // Use provider metadata, not DB query
        let models: Vec<serde_json::Value> = match p.list_models().await {
            Ok(list) => {
                // Batch-fetch blocked models for this provider
                let blocked: std::collections::HashSet<String> = sqlx::query_scalar(
                    "SELECT model_id FROM blocked_models WHERE provider_id = ?"
                )
                .bind(&provider_id)
                .fetch_all(&state.db)
                .await
                .unwrap_or_default()
                .into_iter()
                .collect();

                list.into_iter().map(|m| {
                    let model_name = m.id.split('/').last().unwrap_or(&m.id).to_string();
                    serde_json::json!({
                        "id": m.id,
                        "name": model_name,
                        "available": true,
                        "blocked": blocked.contains(&model_name),
                    })
                }).collect()
            }
            _ => vec![],
        };
        let keys: Vec<serde_json::Value> = sqlx::query_as::<_, (String, String, Option<String>, String, bool)>(
            "SELECT id, key_value, label, key_type, is_active FROM api_keys WHERE provider_id = ? ORDER BY created_at DESC"
        )
        .bind(&provider_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(id, key_value, label, key_type, is_active)| {
            let preview = if key_value.len() > 12 {
                format!("{}...{}", &key_value[..6], &key_value[key_value.len()-4..])
            } else {
                key_value
            };
            serde_json::json!({
                "id": id,
                "label": label,
                "key_type": key_type,
                "is_active": is_active,
                "is_locked": !is_active,
                "masked": preview,
            })
        })
        .collect();
        Json(serde_json::json!({
            "id": provider_id,
            "display_name": meta.display_name,
            "color": meta.color,
            "icon_url": meta.icon_url,
            "category": meta.category,
            "base_url": meta.version,
            "total_keys": total_keys,
            "active_keys": active_keys,
            "locked_keys": locked_keys,
            "type": key_type,
            "oauth_flow": meta.oauth_flow,
            "description": "",
            "models": models,
            "keys": keys,
        }))
    } else {
        Json(serde_json::json!({"error": "Provider not found"}))
    }
}

// ── Test model endpoint ──

#[derive(Deserialize)]
pub struct TestModelRequest {
    pub model: String,
}

#[derive(Serialize)]
pub struct TestModelResponse {
    pub ok: bool,
    pub response: String,
    pub model: String,
    pub latency_ms: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub error: Option<String>,
}

pub async fn api_test_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<TestModelRequest>,
) -> Json<TestModelResponse> {
    let pm = state.provider_manager.read().await;
    let provider = match pm.get(&provider_id) {
        Some(p) => p,
        None => return Json(TestModelResponse {
            ok: false, response: String::new(), model: req.model,
            latency_ms: 0, prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
            error: Some("Provider not found".into()),
        }),
    };

    let chat_request = crate::types::chat::ChatCompletionRequest {
        model: req.model.clone(),
        messages: vec![crate::types::chat::Message {
            role: "user".into(),
            content: Some("Reply with exactly: Hello world".into()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            reasoning_content: None,
        }],
        temperature: Some(0.0),
        max_tokens: Some(20),
        stream: Some(false),
        stream_options: None,
        tools: None,
        tool_choice: None,
        top_p: None,
    };

    let start = std::time::Instant::now();
    match provider.chat_completion(chat_request).await {
        Ok(result) => {
            let latency = start.elapsed().as_millis() as i64;
            let response_text = result.response.choices.first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let usage = result.response.usage.unwrap_or(crate::types::chat::Usage {
                prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
            });
            // Log test to usage table
            let _ = crate::db::log_usage(
                &state.db, &provider_id, result.used_key_id.as_deref(),
                &req.model, "success", Some(200),
                usage.prompt_tokens as i64, usage.completion_tokens as i64,
                Some(latency), None, None, None, None,
            ).await;
            Json(TestModelResponse {
                ok: true,
                response: response_text,
                model: req.model,
                latency_ms: latency,
                prompt_tokens: usage.prompt_tokens as i64,
                completion_tokens: usage.completion_tokens as i64,
                total_tokens: usage.total_tokens as i64,
                error: None,
            })
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as i64;
            let _ = crate::db::log_usage(
                &state.db, &provider_id, None,
                &req.model, "error", Some(500),
                0, 0, Some(latency), Some(e.to_string()), None, None, None,
            ).await;
            Json(TestModelResponse {
                ok: false, response: String::new(), model: req.model,
                latency_ms: latency,
                prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
                error: Some(e.to_string()),
            })
        },
    }
}

// ── Block / Unblock model ──

#[derive(Deserialize)]
pub struct BlockModelRequest {
    pub model: String,
}

#[derive(Serialize)]
pub struct BlockModelResponse {
    pub ok: bool,
    pub message: String,
}

pub async fn api_block_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<BlockModelRequest>,
) -> Json<BlockModelResponse> {
    match crate::db::block_model(&state.db, &provider_id, &req.model).await {
        Ok(_) => Json(BlockModelResponse { ok: true, message: "Blocked".into() }),
        Err(e) => Json(BlockModelResponse { ok: false, message: e.to_string() }),
    }
}

pub async fn api_unblock_model(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Json(req): Json<BlockModelRequest>,
) -> Json<BlockModelResponse> {
    match crate::db::unblock_model(&state.db, &provider_id, &req.model).await {
        Ok(_) => Json(BlockModelResponse { ok: true, message: "Unblocked".into() }),
        Err(e) => Json(BlockModelResponse { ok: false, message: e.to_string() }),
    }
}