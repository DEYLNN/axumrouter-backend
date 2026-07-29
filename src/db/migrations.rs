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

        CREATE INDEX IF NOT EXISTS idx_api_keys_provider ON api_keys(provider_id, is_active);

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

    // Migration v4: add max_tokens to gateway_keys
    match sqlx::query(    "ALTER TABLE gateway_keys ADD COLUMN max_tokens INTEGER NOT NULL DEFAULT 0")
    .execute(pool)
    .await {
        Ok(_) => tracing::debug!("Migration applied: ALTER TABLE"),
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

    // Migration v8: custom OpenAI-compatible providers
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS custom_providers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            prefix TEXT NOT NULL,
            base_url TEXT NOT NULL,
            validate_url TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT '#6366F1',
            timeout_secs INTEGER NOT NULL DEFAULT 120,
            first_chunk_timeout_secs INTEGER NOT NULL DEFAULT 200,
            stall_timeout_secs INTEGER NOT NULL DEFAULT 360,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS custom_provider_models (
            provider_id TEXT NOT NULL REFERENCES custom_providers(id) ON DELETE CASCADE,
            model_id TEXT NOT NULL,
            ctx INTEGER NOT NULL DEFAULT 4096,
            vision INTEGER NOT NULL DEFAULT 0,
            tools INTEGER NOT NULL DEFAULT 1,
            UNIQUE(provider_id, model_id)
        )
    "#).execute(pool).await?;

    // Usage table — per-request token tracking row, populated by
    // services::usage_tracking via UsageTracker. All engines
    // (openai_compat, anthropic_compat, custom providers) MUST call
    // `UsageTracker::save(...)` after their response/stream finalizes
    // so the Usage page can show per-key stats and the gateway key
    // `max_tokens` quota check can SUM total_tokens.
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS usage (
            id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            gateway_key_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'success',
            status_code INTEGER DEFAULT 200,
            prompt_tokens INTEGER NOT NULL DEFAULT 0,
            completion_tokens INTEGER NOT NULL DEFAULT 0,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            latency_ms INTEGER NOT NULL DEFAULT 0,
            endpoint TEXT,
            error_message TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
    "#).execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_key ON usage(gateway_key_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_provider ON usage(provider_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_model ON usage(model_id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_created ON usage(created_at DESC)").execute(pool).await?;

    // Additive schema migrations (2026-07-28): usage tracking extension.
    // SQLite ALTER TABLE ADD COLUMN has no IF NOT EXISTS — ignore duplicate-column
    // errors so re-running migrations on an old DB is a no-op.
    let _ = sqlx::query("ALTER TABLE usage ADD COLUMN endpoint TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE usage ADD COLUMN provider_api_key_id TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE usage ADD COLUMN ttft_ms INTEGER")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE usage ADD COLUMN request_body TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE usage ADD COLUMN response_body TEXT")
        .execute(pool).await;
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN consecutive_use_count INTEGER NOT NULL DEFAULT 0")
        .execute(pool).await;

    // Migration v9: custom_models — per-provider user-added model entries
    // that get merged into the model list without needing to edit TOML or DB rows.
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS custom_models (
            id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            model_id TEXT NOT NULL,
            display_name TEXT NOT NULL DEFAULT '',
            ctx INTEGER NOT NULL DEFAULT 4096,
            vision INTEGER NOT NULL DEFAULT 0,
            tools INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(provider_id, model_id)
        )
    "#).execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_custom_models_provider ON custom_models(provider_id)").execute(pool).await?;

    // Migration v10: persistent key lock state — cooldown, error, backoff
    // Mirrors 9router's per-account error-state tracking but persisted across restarts.
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN locked_until TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN last_error_status INTEGER").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN last_error_message TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN last_error_at TEXT").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE api_keys ADD COLUMN backoff_level INTEGER NOT NULL DEFAULT 0").execute(pool).await;

    tracing::info!("Database migrations complete");
    Ok(())
}
