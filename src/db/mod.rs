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
        "SELECT * FROM api_keys WHERE provider_id = ? AND is_active != 0 ORDER BY created_at"
    )
    .bind(provider_id)
    .fetch_all(pool)
    .await?;
    
    Ok(keys)
}

/// Log usage record
pub async fn log_usage(
    pool: &SqlitePool,
    provider_id: &str,
    api_key_id: Option<&str>,
    model_id: &str,
    status: &str,
    status_code: Option<i64>,
    prompt_tokens: i64,
    completion_tokens: i64,
    latency_ms: Option<i64>,
    error_message: Option<String>,
    request_body: Option<String>,
    response_body: Option<String>,
    gateway_key_id: Option<&str>,
) -> anyhow::Result<String> {
    let usage_id = format!("usage_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let total_tokens = prompt_tokens + completion_tokens;

    tracing::info!("Logging usage: {} {} tokens={}", usage_id, model_id, total_tokens);

    sqlx::query(
        r#"
        INSERT INTO usage (
            id, provider_id, api_key_id, model_id, status, status_code,
            prompt_tokens, completion_tokens, total_tokens, latency_ms,
            error_message, request_body, response_body, gateway_key_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&usage_id)
    .bind(provider_id)
    .bind(api_key_id)
    .bind(model_id)
    .bind(status)
    .bind(status_code)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(latency_ms)
    .bind(error_message)
    .bind(request_body)
    .bind(response_body)
    .bind(gateway_key_id)
    .execute(pool)
    .await?;

    tracing::info!("Usage logged successfully");
    Ok(usage_id)
}

/// Update token usage for an existing usage row (used when streaming finishes).
pub async fn update_usage_tokens(
    pool: &SqlitePool,
    usage_id: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    response_body: Option<String>,
) -> anyhow::Result<()> {
    let total_tokens = prompt_tokens + completion_tokens;
    sqlx::query(
        r#"
        UPDATE usage
        SET prompt_tokens = ?, completion_tokens = ?, total_tokens = ?, response_body = ?, status = 'success'
        WHERE id = ?
        "#
    )
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total_tokens)
    .bind(response_body)
    .bind(usage_id)
    .execute(pool)
    .await?;
    Ok(())
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
