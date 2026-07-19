use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
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
    pub icon_name: String,
    pub category: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub total_keys: i64,
    pub active_keys: i64,
    pub locked_keys: i64,
    pub oauth_flow: Option<String>,
    pub model_count: usize,
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
                icon_name: meta.icon_name.clone(),
                category: meta.category.clone(),
                provider_type: meta.category.clone(),
                total_keys: total,
                active_keys: active,
                locked_keys: locked,
                oauth_flow: meta.oauth_flow.clone(),
                model_count: p.list_models().await.unwrap_or_default().len(),
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
        let key_type: String = meta.category.clone();
        let models: Vec<serde_json::Value> = match p.list_models().await {
            Ok(list) => {
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
                        "context_length": m.context_length,
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

        let runtime_locked: std::collections::HashMap<String, (u64, String)> = p.locked_keys()
            .into_iter()
            .map(|(k, s, r)| (k, (s, r)))
            .collect();
        let keys: Vec<serde_json::Value> = keys.into_iter().map(|mut k| {
            if let Some(id) = k["id"].as_str() {
                if let Some((remaining, reason)) = runtime_locked.get(id) {
                    k["is_locked"] = serde_json::Value::Bool(true);
                    k["locked_reason"] = serde_json::Value::String(reason.clone());
                    k["locked_remaining"] = serde_json::Value::Number(serde_json::Number::from(*remaining));
                }
            }
            k
        }).collect();
        Json(serde_json::json!({
            "id": provider_id,
            "display_name": meta.display_name,
            "color": meta.color,
            "icon_name": meta.icon_name,
            "category": meta.category,
            "base_url": meta.version,
            "validate_url": meta.validate_url,
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

// ���─ Validate models — proxy to validate_url with first active API key ──

pub async fn api_validate_models(
    State(state): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let key_id = params.get("key_id").map(|s| s.as_str());

    // Get specific key or first active
    let key_row = if let Some(kid) = key_id {
        sqlx::query_as::<_, (String,)>(
            "SELECT key_value FROM api_keys WHERE id = ? AND provider_id = ? AND is_active = 1 LIMIT 1"
        )
        .bind(kid).bind(&provider_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    } else {
        sqlx::query_as::<_, (String,)>(
            "SELECT key_value FROM api_keys WHERE provider_id = ? AND is_active = 1 LIMIT 1"
        )
        .bind(&provider_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    };

    let key_value = match key_row {
        Some((kv,)) => kv,
        None => return Json(serde_json::json!({"ok": false, "error": "No active API key"})),
    };

    // Look up validate_url from provider metadata
    let pm = state.provider_manager.read().await;
    let meta = pm.get(&provider_id).map(|p| p.metadata());
    let validate_url = meta.as_ref().map(|m| m.validate_url.clone()).unwrap_or_default();
    drop(pm);

    if validate_url.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "No validate_url for this provider"}));
    }

    let client = reqwest::Client::new();
    let resp = match client.get(&validate_url)
        .header("Authorization", format!("Bearer {}", key_value))
        .header("User-Agent", "AxumRouter/1.0")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("HTTP error: {}", e)})),
    };

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Json(serde_json::json!({"ok": false, "error": format!("HTTP {}: {}", status, body)}));
    }

    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("Read body: {}", e)})),
    };

    let parsed = serde_json::from_str::<serde_json::Value>(&body);
    match parsed {
        Ok(json) => {
            let models = try_extract_models(&json);
            Json(serde_json::json!({
                "ok": true,
                "models": models,
                "raw": json,
            }))
        }
        Err(_) => {
            Json(serde_json::json!({
                "ok": true,
                "models": [],
                "raw": body,
            }))
        }
    }
}

/// Try to extract model list from various JSON response shapes.
fn try_extract_models(v: &serde_json::Value) -> Vec<serde_json::Value> {
    // OpenAI format: { "data": [{ "id": "...", ... }] }
    // Also handles { "object": "list", "data": [...] }
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        let has_id = arr.iter().any(|m| m.get("id").is_some());
        if has_id {
            return arr.iter().map(normalize_model).collect();
        }
    }
    // { "models": [...] }
    if let Some(arr) = v.get("models").and_then(|d| d.as_array()) {
        return arr.iter().map(normalize_model).collect();
    }
    // Plain array: [{ "id": "...", ... }]
    if let Some(arr) = v.as_array() {
        if arr.iter().any(|m| m.get("id").is_some()) {
            return arr.iter().map(normalize_model).collect();
        }
    }
    vec![]
}

fn normalize_model(m: &serde_json::Value) -> serde_json::Value {
    let id = m.get("id").and_then(|s| s.as_str()).unwrap_or("?");
    serde_json::json!({
        "id": id,
        "name": m.get("name").and_then(|s| s.as_str()).unwrap_or(id),
        "owned_by": m.get("owned_by").and_then(|s| s.as_str()),
        "context_length": m.get("context_length").or_else(|| m.get("max_tokens")),
    })
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

    let known_keys: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM api_keys WHERE provider_id=? AND is_active=1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&provider_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let db_key_id = known_keys.first().cloned();

    match provider.chat_completion(chat_request).await {
        Ok(result) => {
            let latency = start.elapsed().as_millis() as i64;
            let response_text = result.response.choices.first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let usage = result.response.usage.unwrap_or(crate::types::chat::Usage {
                prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
            });
            let _ = crate::db::log_usage(
                &state.db, &provider_id, result.used_key_id.as_deref(),
                &req.model, "success", Some(200),
                usage.prompt_tokens as i64, usage.completion_tokens as i64,
                Some(latency), None, None, None, None,
            ).await;
            for failed in &result.failed_keys {
                let _ = crate::db::log_usage(
                    &state.db, &provider_id, Some(&failed.key_id),
                    &req.model, "error", Some(401),
                    0, 0, Some(latency),
                    Some(failed.error.to_string()),
                    None, None, None,
                ).await;
            }
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
            let failed_key = provider.locked_keys().first().map(|(id, _, _)| id.clone())
                .or_else(|| db_key_id.clone());
            let _ = crate::db::log_usage(
                &state.db, &provider_id, failed_key.as_deref(),
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
