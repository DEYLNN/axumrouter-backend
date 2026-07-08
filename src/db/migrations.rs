use sqlx::sqlite::SqlitePool;

pub async fn run(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            key_value TEXT NOT NULL,
            label TEXT,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            rate_limit INTEGER,
            last_used_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS request_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id TEXT NOT NULL,
            model TEXT NOT NULL,
            status_code INTEGER,
            latency_ms INTEGER,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            error_message TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS usage (
            id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            api_key_id TEXT,
            model_id TEXT NOT NULL,
            status TEXT NOT NULL,
            status_code INTEGER,
            prompt_tokens INTEGER NOT NULL DEFAULT 0,
            completion_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            latency_ms INTEGER,
            error_message TEXT,
            request_body TEXT,
            response_body TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider_id, is_active);
        CREATE INDEX IF NOT EXISTS idx_request_logs_provider ON request_logs(provider_id);
        CREATE INDEX IF NOT EXISTS idx_request_logs_created ON request_logs(created_at);
        CREATE INDEX IF NOT EXISTS idx_usage_provider ON usage(provider_id);
        CREATE INDEX IF NOT EXISTS idx_usage_api_key ON usage(api_key_id);
        CREATE INDEX IF NOT EXISTS idx_usage_created ON usage(created_at);
        CREATE INDEX IF NOT EXISTS idx_usage_status ON usage(status);

        -- Gateway API keys (client-facing auth for /v1/*)
        CREATE TABLE IF NOT EXISTS gateway_keys (
            id TEXT PRIMARY KEY,
            key_value TEXT NOT NULL UNIQUE,
            label TEXT,
            is_active BOOLEAN NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_gateway_keys_active ON gateway_keys(is_active);

        -- Blocked models — models admin has disabled per-provider
        CREATE TABLE IF NOT EXISTS blocked_models (
            id TEXT PRIMARY KEY,  -- "provider/model_name"
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(provider_id, model_id)
        );
        CREATE INDEX IF NOT EXISTS idx_blocked_models_provider ON blocked_models(provider_id);

        -- Disabled models — global model allowlist (outer middleware)
        CREATE TABLE IF NOT EXISTS disabled_models (
            model_id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Gateway key permissions (IF NOT EXISTS via try-catch)
        -- SQLite doesn't support ALTER COLUMN IF NOT EXISTS, but we catch duplicates gracefully
        CREATE TABLE IF NOT EXISTS _gateway_permission_migration_done (id INTEGER PRIMARY KEY);
    "#)
    .execute(pool)
    .await?;

    // Run ALTER TABLE outside the raw SQL to catch duplicate column errors
    match sqlx::query(    "ALTER TABLE gateway_keys ADD COLUMN access_type TEXT NOT NULL DEFAULT 'full'")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: ALTER TABLE"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }
    match sqlx::query(    "ALTER TABLE gateway_keys ADD COLUMN allowed_models TEXT NOT NULL DEFAULT ''")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: ALTER TABLE"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }

    // Migration v3: add gateway_key_id to usage table
    match sqlx::query(    "ALTER TABLE usage ADD COLUMN gateway_key_id TEXT")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: ALTER TABLE"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }

    // Migration v4: add max_tokens to gateway_keys
    match sqlx::query(    "ALTER TABLE gateway_keys ADD COLUMN max_tokens INTEGER NOT NULL DEFAULT 0")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: ALTER TABLE"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }

    // Migration v2: add key_type column
    match sqlx::query(    "ALTER TABLE api_keys ADD COLUMN key_type TEXT NOT NULL DEFAULT 'apikey'")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: ALTER TABLE"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }
    // Backfill existing rows
    match sqlx::query(    "UPDATE api_keys SET key_type = 'oauth' WHERE provider_id = 'cx' AND key_type = 'apikey'")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: UPDATE api_keys"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }

    // Migration v3: settings table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    )
    .execute(pool)
    .await?;
    // Default settings
    match sqlx::query(    "INSERT OR IGNORE INTO settings (key, value) VALUES ('rtk_enabled', 'true')")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: INSERT OR"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }
    match sqlx::query(    "INSERT OR IGNORE INTO settings (key, value) VALUES ('caveman_enabled', 'false')")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: INSERT OR"),
        Err(e) => tracing::warn!("Migration skipped: {}", e),
    }

    // Migration v4: proxies table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS proxies (
            id TEXT PRIMARY KEY,
            label TEXT NOT NULL DEFAULT '',
            protocol TEXT NOT NULL DEFAULT 'http',
            host TEXT NOT NULL,
            port INTEGER NOT NULL DEFAULT 0,
            username TEXT,
            password TEXT,
            country TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            usage_count INTEGER NOT NULL DEFAULT 0,
            last_used TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )"
    )
    .execute(pool)
    .await?;

    // Migration v5: combos table
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS combos (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            tiers TEXT NOT NULL DEFAULT '[]',  -- JSON array
            round_robin INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#)
    .execute(pool)
    .await?;

    tracing::info!("Database migrations complete");
    Ok(())
}
