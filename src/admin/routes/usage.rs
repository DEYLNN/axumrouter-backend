use std::sync::Arc;
use std::convert::Infallible;

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    Json,
};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;

/// Aggregate stats for the Usage page top cards.
/// GET /admin/api/usage/stats
pub async fn api_usage_stats(
    State(state): State<Arc<AppState>>,
) -> Json<crate::db::UsageStatsRow> {
    Json(crate::db::usage_stats(&state.db).await)
}

#[derive(Debug, Deserialize)]
pub struct UsageKeysQuery {
    pub gateway_key_id: Option<String>,
}

/// Per-key breakdown for the Usage page Per-Key table.
/// GET /admin/api/usage/keys
pub async fn api_usage_keys(
    State(state): State<Arc<AppState>>,
    Query(_q): Query<UsageKeysQuery>,
) -> Json<Vec<crate::db::UsagePerKeyRow>> {
    // For Phase 1 we return all rows; gateway_key_id filter is a follow-up
    // when the FE asks for per-row drilldown.
    Json(crate::db::usage_per_key(&state.db).await)
}

#[derive(Debug, Deserialize)]
pub struct UsageLogsQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// Paginated recent-request list for the Usage page live feed.
/// GET /admin/api/logs?page=1&limit=50
pub async fn api_usage_logs(
    State(state): State<Arc<AppState>>,
    Query(q): Query<UsageLogsQuery>,
) -> Json<serde_json::Value> {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * limit;
    let logs = crate::db::usage_logs_page(&state.db, limit, offset).await;
    let total = crate::db::count_usage_logs(&state.db).await;
    let total_pages = if limit == 0 { 1 } else { (total + limit - 1) / limit };
    Json(serde_json::json!({
        "logs": logs,
        "total": total,
        "page": page,
        "total_pages": total_pages,
    }))
}

/// Server-Sent Events push — every new usage row is forwarded to
/// connected FE clients in real-time. Open with EventSource on the FE.
///
/// GET /admin/api/usage/stream
pub async fn api_usage_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.usage_broadcast.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(row) => {
                    let json = serde_json::to_string(&row).unwrap_or_default();
                    yield Ok(Event::default().event("usage").data(json));
                }
                Err(RecvError::Lagged(_)) => {
                    // Subscriber slow — skip missed events, keep going.
                    continue;
                }
                Err(RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// DELETE all usage rows. Use with caution — this is irreversible.
/// POST /admin/api/logs/clear
pub async fn api_clear_logs(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let _ = sqlx::query("DELETE FROM usage")
        .execute(&state.db)
        .await;
    Json(serde_json::json!({ "ok": true }))
}
