pub mod migrations;
pub mod models;

use sqlx::sqlite::SqlitePool;
use std::path::Path;
use chrono::Local;

/// Initialize database — run migrations, return pool.
/// Auto-backups the DB file to data/backups/ before destructive schema changes.
pub async fn init(database_url: &str) -> anyhow::Result<SqlitePool> {
    // Auto-backup: if SQLite file path mode (sqlite:path), copy file to data/backups/.
    if let Some(path) = sqlite_file_path(database_url) {
        let p = Path::new(&path);
        if p.exists() {
            if let Some(parent) = p.parent() {
                let backup_dir = parent.join("backups");
                if !backup_dir.exists() {
                    let _ = std::fs::create_dir_all(&backup_dir);
                }
                let ts = Local::now().format("%Y%m%d_%H%M%S");
                let filename = p.file_name().and_then(|n| n.to_str()).unwrap_or("axumrouter.db");
                let backup_path = backup_dir.join(format!("{}.{}.bak", filename, ts));
                if let Err(e) = std::fs::copy(p, &backup_path) {
                    tracing::warn!("Backup failed: {} (path: {:?})", e, backup_path);
                } else {
                    tracing::info!("Backup created: {}", backup_path.display());

                    // Garbage-collect old backups (keep last 20).
                    if let Ok(entries) = std::fs::read_dir(&backup_dir) {
                        let mut files: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .and_then(|x| x.to_str())
                                    .map(|x| x == "bak")
                                    .unwrap_or(false)
                            })
                            .collect();
                        files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
                        while files.len() > 20 {
                            if let Some(old) = files.first() {
                                let _ = std::fs::remove_file(old.path());
                                files.remove(0);
                            }
                        }
                    }
                }
            }
        }
    }

    let pool = SqlitePool::connect(database_url).await?;
    migrations::run(&pool).await?;
    Ok(pool)
}

/// Extracts the filesystem path from a `sqlite:path[?mode=...]` URL.
/// Returns None if the URL is `:memory:` or unparseable.
fn sqlite_file_path(database_url: &str) -> Option<String> {
    if database_url.starts_with("sqlite::memory:") {
        return None;
    }
    let stripped = database_url
        .strip_prefix("sqlite:")
        .unwrap_or(database_url);
    let path = stripped.split('?').next().unwrap_or(stripped);
    if path.is_empty() || path == ":memory:" {
        None
    } else {
        Some(path.to_string())
    }
}

/// Load active API keys for a provider
pub async fn load_provider_keys(pool: &SqlitePool, provider_id: &str) -> anyhow::Result<Vec<models::ApiKey>> {
    let keys = sqlx::query_as::<_, models::ApiKey>(
        "SELECT id, provider_id, key_value, label, is_active, last_used_at, consecutive_use_count, created_at, updated_at
         FROM api_keys WHERE provider_id = ? AND is_active != 0 ORDER BY created_at"
    )
    .bind(provider_id)
    .fetch_all(pool)
    .await?;

    Ok(keys)
}

/// Count keys for a provider. `active_only = true` ignores inactive (locked) rows.
pub async fn count_provider_keys(pool: &SqlitePool, provider_id: &str, active_only: bool) -> i64 {
    let sql = if active_only {
        "SELECT COUNT(*) FROM api_keys WHERE provider_id = ? AND is_active = 1"
    } else {
        "SELECT COUNT(*) FROM api_keys WHERE provider_id = ?"
    };
    sqlx::query_scalar(sql)
        .bind(provider_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

/// Count active keys across all providers.
#[allow(dead_code)]
pub async fn count_total_active_keys(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM api_keys WHERE is_active = 1")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

// ── Usage tracking ──
//
// One row per upstream API call. Populated by services::usage_tracking
// via UsageTracker. Engines (openai_compat, anthropic_compat, custom
// providers) call UsageTracker::save() after their response/stream
// finalizes. Aggregations power both the admin Usage page and the
// gateway key `max_tokens` quota check (see services/gateway.rs).

#[derive(Debug, Clone)]
pub struct UsageEntry {
    pub provider_id: String,
    pub model_id: String,
    pub gateway_key_id: String,
    pub provider_api_key_id: Option<String>,
    pub endpoint: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub latency_ms: i64,
    /// Time-to-first-token in milliseconds. Only populated for streaming.
    pub ttft_ms: Option<i64>,
    pub status: String,
    pub status_code: i32,
    pub error_message: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
}

pub async fn save_request_usage(pool: &SqlitePool, e: &UsageEntry) {
    let total = e.prompt_tokens + e.completion_tokens;
    let _ = sqlx::query(
        "INSERT INTO usage (id, provider_id, model_id, gateway_key_id, status, status_code,
                            prompt_tokens, completion_tokens, total_tokens, latency_ms,
                            endpoint, error_message, request_body, response_body,
                            provider_api_key_id, ttft_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(format!("usage_{}", uuid::Uuid::new_v4().simple()))
    .bind(&e.provider_id)
    .bind(&e.model_id)
    .bind(&e.gateway_key_id)
    .bind(&e.status)
    .bind(e.status_code)
    .bind(e.prompt_tokens)
    .bind(e.completion_tokens)
    .bind(total)
    .bind(e.latency_ms)
    .bind(&e.endpoint)
    .bind(&e.error_message)
    .bind(&e.request_body)
    .bind(&e.response_body)
    .bind(&e.provider_api_key_id)
    .bind(e.ttft_ms)
    .execute(pool)
    .await;
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UsageStatsRow {
    pub total_requests: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
}

pub async fn usage_stats(pool: &SqlitePool) -> UsageStatsRow {
    sqlx::query_as::<_, UsageStatsRow>(
        "SELECT
            COUNT(*) AS total_requests,
            COALESCE(SUM(prompt_tokens), 0) AS total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) AS total_completion_tokens,
            COALESCE(SUM(total_tokens), 0) AS total_tokens
         FROM usage",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(UsageStatsRow {
        total_requests: 0,
        total_prompt_tokens: 0,
        total_completion_tokens: 0,
        total_tokens: 0,
    })
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UsagePerKeyRow {
    pub gateway_key_id: String,
    /// Label of the gateway key (admin-issued) — `None` when the row has no
    /// `gateway_key_id` or the key was deleted. Rendered in the Per-Key card.
    pub label: Option<String>,
    /// Truncated/masked key value for at-a-glance identification in admin UI.
    pub key_value: Option<String>,
    pub requests: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

pub async fn usage_per_key(pool: &SqlitePool) -> Vec<UsagePerKeyRow> {
    sqlx::query_as::<_, UsagePerKeyRow>(
        "SELECT
            u.gateway_key_id,
            gk.label AS label,
            gk.key_value AS key_value,
            COUNT(*) AS requests,
            COALESCE(SUM(u.prompt_tokens), 0) AS prompt_tokens,
            COALESCE(SUM(u.completion_tokens), 0) AS completion_tokens,
            COALESCE(SUM(u.total_tokens), 0) AS total_tokens
         FROM usage u
         LEFT JOIN gateway_keys gk ON gk.id = u.gateway_key_id
         GROUP BY u.gateway_key_id
         ORDER BY requests DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct UsageLogRow {
    pub id: String,
    pub created_at: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    /// Serialized as `api_key_id` for FE compatibility (Logs page expects this field).
    #[serde(rename = "api_key_id", alias = "gateway_key_id")]
    pub gateway_key_id: Option<String>,
    pub endpoint: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub latency_ms: i64,
    pub status: Option<String>,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub key_label: Option<String>,
    /// Label of the underlying provider_api_key that handled this request.
    /// `None` when the row has no `provider_api_key_id` (no key was attempted).
    pub provider_key_label: Option<String>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
}

pub async fn usage_logs_page(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Vec<UsageLogRow> {
    sqlx::query_as::<_, UsageLogRow>(
        "SELECT u.id, u.created_at, u.provider_id, u.model_id, u.gateway_key_id, u.endpoint,
                u.prompt_tokens, u.completion_tokens, u.total_tokens, u.latency_ms,
                u.status, u.status_code, u.error_message,
                u.request_body, u.response_body,
                COALESCE(gk.label, '') AS key_label,
                COALESCE(ak.label, '') AS provider_key_label
         FROM usage u
         LEFT JOIN gateway_keys gk ON gk.id = u.gateway_key_id
         LEFT JOIN api_keys ak ON ak.id = u.provider_api_key_id
         ORDER BY u.created_at DESC
         LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn count_usage_logs(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM usage")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

/// Fetch the single most-recently inserted row. Used by `UsageTracker`
/// to broadcast the just-saved row to SSE subscribers.
pub async fn fetch_latest_usage(pool: &SqlitePool) -> Option<UsageLogRow> {
    sqlx::query_as::<_, UsageLogRow>(
        "SELECT u.id, u.created_at, u.provider_id, u.model_id, u.gateway_key_id, u.endpoint,
                u.prompt_tokens, u.completion_tokens, u.total_tokens, u.latency_ms,
                u.status, u.status_code, u.error_message,
                u.request_body, u.response_body,
                COALESCE(gk.label, '') AS key_label,
                COALESCE(ak.label, '') AS provider_key_label
         FROM usage u
         LEFT JOIN gateway_keys gk ON gk.id = u.gateway_key_id
         LEFT JOIN api_keys ak ON ak.id = u.provider_api_key_id
         ORDER BY u.id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}


/// Check if a model is blocked for a provider.
pub async fn is_model_blocked(pool: &SqlitePool, provider_id: &str, model_id: &str) -> bool {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM blocked_models WHERE provider_id = ? AND model_id = ?"
    )
    .bind(provider_id)
    .bind(model_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    row.is_some()
}

/// Check if a model is globally disabled (outer middleware).
pub async fn is_model_disabled(pool: &SqlitePool, model_id: &str) -> bool {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT model_id FROM disabled_models WHERE model_id = ?"
    )
    .bind(model_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    row.is_some()
}

/// Block a model for a provider. Returns true if inserted (true if inserted—false when already existed) false=was already blocking
pub async fn block_model(pool: &SqlitePool, provider_id: &str, model_id: &str) -> anyhow::Result<bool> {
    let id = format!("{}/{}", provider_id, model_id);
    let result = sqlx::query(
        "INSERT OR IGNORE INTO blocked_models (id, provider_id, model_id) VALUES (?, ?, ?)"
    )
    .bind(&id)
    .bind(provider_id)
    .bind(model_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Unblock a model for a provider.
pub async fn unblock_model(pool: &SqlitePool, provider_id: &str, model_id: &str) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM blocked_models WHERE provider_id = ? AND model_id = ?")
        .bind(provider_id)
        .bind(model_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Custom providers ──

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CustomProviderRow {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub base_url: String,
    pub validate_url: String,
    pub color: String,
    pub timeout_secs: i64,
    pub first_chunk_timeout_secs: i64,
    pub stall_timeout_secs: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CustomProviderModelRow {
    pub provider_id: String,
    pub model_id: String,
    pub ctx: i64,
    pub vision: i64,
    pub tools: i64,
}

pub async fn list_custom_providers(pool: &SqlitePool) -> anyhow::Result<Vec<CustomProviderRow>> {
    Ok(sqlx::query_as::<_, CustomProviderRow>(
        "SELECT * FROM custom_providers ORDER BY created_at"
    ).fetch_all(pool).await?)
}

pub async fn get_custom_provider(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<CustomProviderRow>> {
    Ok(sqlx::query_as::<_, CustomProviderRow>(
        "SELECT * FROM custom_providers WHERE id = ?"
    ).bind(id).fetch_optional(pool).await?)
}

pub async fn list_custom_provider_models(pool: &SqlitePool, provider_id: &str) -> anyhow::Result<Vec<CustomProviderModelRow>> {
    Ok(sqlx::query_as::<_, CustomProviderModelRow>(
        "SELECT * FROM custom_provider_models WHERE provider_id = ? ORDER BY model_id"
    ).bind(provider_id).fetch_all(pool).await?)
}

pub async fn create_custom_provider(
    pool: &SqlitePool, id: &str, name: &str, prefix: &str,
    base_url: &str, validate_url: &str, color: &str,
    timeout: i64, fc_timeout: i64, stall_timeout: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO custom_providers (id, name, prefix, base_url, validate_url, color, timeout_secs, first_chunk_timeout_secs, stall_timeout_secs) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    ).bind(id).bind(name).bind(prefix).bind(base_url).bind(validate_url).bind(color).bind(timeout).bind(fc_timeout).bind(stall_timeout)
    .execute(pool).await?;
    Ok(())
}

pub async fn delete_custom_provider(pool: &SqlitePool, id: &str) -> anyhow::Result<bool> {
    // Root provider row
    let r = sqlx::query("DELETE FROM custom_providers WHERE id = ?").bind(id).execute(pool).await?;
    if r.rows_affected() > 0 {
        // Cascade — API keys, model definitions are orphaned otherwise
        let _ = sqlx::query("DELETE FROM api_keys WHERE provider_id = ?").bind(id).execute(pool).await;
        let _ = sqlx::query("DELETE FROM custom_provider_models WHERE provider_id = ?").bind(id).execute(pool).await;
    }
    Ok(r.rows_affected() > 0)
}

pub async fn add_custom_provider_model(pool: &SqlitePool, provider_id: &str, model_id: &str, ctx: i64, vision: i64, tools: i64) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO custom_provider_models (provider_id, model_id, ctx, vision, tools) VALUES (?, ?, ?, ?, ?)"
    ).bind(provider_id).bind(model_id).bind(ctx).bind(vision).bind(tools)
    .execute(pool).await?;
    Ok(())
}

pub async fn remove_custom_provider_model(pool: &SqlitePool, provider_id: &str, model_id: &str) -> anyhow::Result<bool> {
    let r = sqlx::query(
        "DELETE FROM custom_provider_models WHERE provider_id = ? AND model_id = ?"
    ).bind(provider_id).bind(model_id).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}

// ── Custom Models (per-provider user-added model entries) ──

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CustomModelRow {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub ctx: i64,
    pub vision: i64,
    pub tools: i64,
    pub created_at: String,
}

pub async fn list_custom_models(pool: &SqlitePool, provider_id: &str) -> Vec<CustomModelRow> {
    sqlx::query_as::<_, CustomModelRow>(
        "SELECT * FROM custom_models WHERE provider_id = ? ORDER BY model_id"
    )
    .bind(provider_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn add_custom_model(
    pool: &SqlitePool,
    provider_id: &str,
    model_id: &str,
    display_name: &str,
    ctx: i64,
    vision: i64,
    tools: i64,
) -> anyhow::Result<bool> {
    let id = format!("{}_{}", provider_id, model_id.replace('/', "_"));
    let r = sqlx::query(
        "INSERT OR REPLACE INTO custom_models (id, provider_id, model_id, display_name, ctx, vision, tools) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(provider_id)
    .bind(model_id)
    .bind(display_name)
    .bind(ctx)
    .bind(vision)
    .bind(tools)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}

pub async fn remove_custom_model(pool: &SqlitePool, provider_id: &str, model_id: &str) -> anyhow::Result<bool> {
    let r = sqlx::query(
        "DELETE FROM custom_models WHERE provider_id = ? AND model_id = ?"
    )
    .bind(provider_id)
    .bind(model_id)
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}
