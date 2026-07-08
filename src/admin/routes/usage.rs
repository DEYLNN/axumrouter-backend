use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub success_count: i64,
    pub error_count: i64,
}

pub async fn api_usage_oauth_keys(State(state): State<Arc<AppState>>) -> Json<Vec<serde_json::Value>> {
    // All keys with key_type='oauth' (xai, cx, fb etc)
    let rows = sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT id, provider_id, label FROM api_keys WHERE key_type='oauth' ORDER BY created_at DESC"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let data: Vec<serde_json::Value> = rows.into_iter().map(|(id, provider_id, label)| {
        serde_json::json!({ "id": id, "provider_id": provider_id, "label": label })
    }).collect();

    Json(data)
}

pub async fn api_usage_stats(State(state): State<Arc<AppState>>) -> Json<UsageStats> {
    let total_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage")
        .fetch_one(&state.db).await.unwrap_or(0);
    let total_prompt_tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(prompt_tokens), 0) FROM usage"
    )
    .fetch_one(&state.db).await.unwrap_or(0);
    let total_completion_tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(completion_tokens), 0) FROM usage"
    )
    .fetch_one(&state.db).await.unwrap_or(0);
    let total_tokens: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_tokens), 0) FROM usage"
    )
    .fetch_one(&state.db).await.unwrap_or(0);
    let success_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM usage WHERE status='success'"
    )
    .fetch_one(&state.db).await.unwrap_or(0);
    let error_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM usage WHERE status='error'"
    )
    .fetch_one(&state.db).await.unwrap_or(0);

    Json(UsageStats {
        total_requests,
        total_prompt_tokens,
        total_completion_tokens,
        total_tokens,
        success_count,
        error_count,
    })
}

/// Aggregated usage per gateway key
pub async fn api_usage_per_key(State(state): State<Arc<AppState>>) -> Json<Vec<serde_json::Value>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        gateway_key_id: Option<String>,
        label: Option<String>,
        key_value: Option<String>,
        requests: i64,
        prompt_tokens: i64,
        completion_tokens: i64,
        total_tokens: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(u.gateway_key_id, '') as gateway_key_id,
            gk.label,
            gk.key_value,
            COUNT(*) as requests,
            COALESCE(SUM(u.prompt_tokens), 0) as prompt_tokens,
            COALESCE(SUM(u.completion_tokens), 0) as completion_tokens,
            COALESCE(SUM(u.total_tokens), 0) as total_tokens
        FROM usage u
        LEFT JOIN gateway_keys gk ON u.gateway_key_id = gk.id
        WHERE u.gateway_key_id IS NOT NULL
        GROUP BY u.gateway_key_id
        ORDER BY total_tokens DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(rows.iter().map(|r| serde_json::json!({
        "gateway_key_id": r.gateway_key_id,
        "label": r.label,
        "key_value": r.key_value.as_ref().map(|kv| &kv[..12.min(kv.len())]),
        "requests": r.requests,
        "prompt_tokens": r.prompt_tokens,
        "completion_tokens": r.completion_tokens,
        "total_tokens": r.total_tokens,
    })).collect())
}
