use crate::error::GatewayError;
use crate::middleware::auth::GatewayKeyInfo;
use sqlx::SqlitePool;

/// Layer 1 + Layer 2: Check if model is allowed for this gateway key.
/// - Layer 1 (global): already done before calling this (disabled_models)
/// - Layer 2 (per-key):
pub async fn check_model_access(
    gw_key: &GatewayKeyInfo,
    model: &str,
) -> Result<(), GatewayError> {
    match gw_key.access_type.as_str() {
        "allow" => {
            if !gw_key.allowed_models.contains(&model.to_string()) {
                return Err(GatewayError::ModelNotFound {
                    provider: "gateway".to_string(),
                    model: model.to_string(),
                });
            }
        }
        "deny" => {
            if gw_key.allowed_models.contains(&model.to_string()) {
                return Err(GatewayError::ModelNotFound {
                    provider: "gateway".to_string(),
                    model: model.to_string(),
                });
            }
        }
        _ => {} // "full" = all models
    }
    Ok(())
}

/// Insert a gateway usage row for tracking token consumption per key.
pub async fn track_gateway_usage(
    db: &SqlitePool,
    gateway_key_id: &str,
    provider_id: &str,
    model_id: &str,
    prompt_tokens: i64,
    completion_tokens: i64,
    latency_ms: i64,
) {
    let total = prompt_tokens + completion_tokens;
    let _ = sqlx::query(
        "INSERT INTO usage (id, provider_id, model_id, status, status_code, prompt_tokens, completion_tokens, total_tokens, latency_ms, gateway_key_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(format!("usage_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap()))
    .bind(provider_id)
    .bind(model_id)
    .bind("success")
    .bind(200i64)
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(total)
    .bind(latency_ms)
    .bind(gateway_key_id)
    .execute(db)
    .await;
}

/// Layer 2: Check if gateway key has exceeded its max_tokens limit.
/// max_tokens=0 means unlimited. Single JOIN query.
pub async fn check_token_limit(
    db: &SqlitePool,
    gateway_key_id: &str,
) -> Result<(), GatewayError> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "SELECT gk.max_tokens, COALESCE(SUM(u.total_tokens), 0) FROM gateway_keys gk LEFT JOIN usage u ON u.gateway_key_id = gk.id WHERE gk.id = ? GROUP BY gk.id"
    )
    .bind(gateway_key_id)
    .fetch_optional(db)
    .await
    .unwrap_or(None);

    let (max_tokens, used) = match row {
        Some((max, used)) => (max, used),
        None => return Ok(()), // key not found, shouldn't happen in practice
    };

    if max_tokens <= 0 { return Ok(()); } // 0 = unlimited

    if used >= max_tokens {
        return Err(GatewayError::TokenLimitExceeded { used, max: max_tokens });
    }

    Ok(())
}
