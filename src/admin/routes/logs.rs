use std::sync::Arc;

use axum::{
    extract::{State, Query},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LogsQuery {
    page: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct LogEntryJson {
    pub id: String,
    pub provider_id: String,
    pub api_key_id: Option<String>,
    pub key_label: Option<String>,
    pub model_id: String,
    pub status: String,
    pub status_code: Option<i64>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub latency_ms: Option<i64>,
    pub error_message: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntryJson>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
}

pub async fn api_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<LogsQuery>,
) -> Json<LogsResponse> {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let total_pages = ((total as f64) / (limit as f64)).ceil() as i64;

    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String, Option<i64>, i64, i64, i64, Option<i64>, Option<String>, Option<String>, Option<String>, String, Option<String>)>(
        r#"
        SELECT u.id, u.provider_id, u.api_key_id, u.model_id, u.status,
               u.status_code, u.prompt_tokens, u.completion_tokens, u.total_tokens,
               u.latency_ms, u.error_message, u.request_body, u.response_body, u.created_at,
               ak.label
        FROM usage u
        LEFT JOIN api_keys ak ON u.api_key_id = ak.id
        ORDER BY u.created_at DESC
        LIMIT ? OFFSET ?
        "#
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let logs = rows
        .into_iter()
        .map(|(id, provider_id, api_key_id, model_id, status, status_code, prompt_tokens, completion_tokens, total_tokens, latency_ms, error_message, request_body, response_body, created_at, key_label)| LogEntryJson {
            id,
            provider_id,
            api_key_id,
            key_label,
            model_id,
            status,
            status_code,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            latency_ms,
            error_message,
            request_body,
            response_body,
            created_at,
        })
        .collect();

    Json(LogsResponse {
        logs,
        total,
        page,
        total_pages,
    })
}

#[derive(Serialize)]
pub struct ClearResponse {
    pub ok: bool,
    pub deleted: i64,
}

pub async fn api_logs_clear(State(state): State<Arc<AppState>>) -> Json<ClearResponse> {
    let deleted = sqlx::query("DELETE FROM usage")
        .execute(&state.db)
        .await
        .map(|r| r.rows_affected() as i64)
        .unwrap_or(0);

    Json(ClearResponse {
        ok: true,
        deleted,
    })
}
