use reqwest::Client;
use serde_json::Value;

/// Fetch Codex WHAM usage data (rate limits + session info)
/// Response format: { plan_type, rate_limit: { primary_window: {...}, secondary_window: {...} }, ... }
pub async fn fetch_wham_usage(access_token: &str) -> (Vec<Value>, Option<String>) {
    let client = Client::new();
    let resp = client
        .get("https://chatgpt.com/backend-api/wham/usage")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .send()
        .await;

    let json: Value = match resp {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(_) => return (vec![], None),
        },
        Err(_) => return (vec![], None),
    };

    let plan_type = json["plan_type"].as_str().map(String::from);
    let mut limits = Vec::new();

    // Primary window
    if let Some(pw) = json["rate_limit"]["primary_window"].as_object() {
        let used_pct = pw["used_percent"].as_i64().unwrap_or(0);
        let limit_secs = pw["limit_window_seconds"].as_i64().unwrap_or(0);
        let reset_after = pw["reset_after_seconds"].as_i64();
        let reset_at = pw["reset_at"].as_i64();

        let limit = limit_secs;
        let used = (limit as f64 * used_pct as f64 / 100.0) as i64;
        let remaining = limit - used;

        let reset_at_str = reset_at
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.to_rfc3339());

        limits.push(serde_json::json!({
            "name": "primary",
            "limit": limit.max(1),
            "remaining": remaining.max(0),
            "used": used.max(0),
            "period_seconds": limit_secs,
            "reset_after_seconds": reset_after,
            "reset_at": reset_at_str,
        }));
    }

    // Secondary window
    if let Some(sw) = json["rate_limit"]["secondary_window"].as_object() {
        let used_pct = sw["used_percent"].as_i64().unwrap_or(0);
        let limit_secs = sw["limit_window_seconds"].as_i64().unwrap_or(0);
        let reset_at = sw["reset_at"].as_i64();
        let reset_after = sw["reset_after_seconds"].as_i64();

        let limit = limit_secs;
        let used = (limit as f64 * used_pct as f64 / 100.0) as i64;
        let remaining = limit - used;

        let reset_at_str = reset_at
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.to_rfc3339());

        limits.push(serde_json::json!({
            "name": "secondary",
            "limit": limit.max(1),
            "remaining": remaining.max(0),
            "used": used.max(0),
            "period_seconds": limit_secs,
            "reset_after_seconds": reset_after,
            "reset_at": reset_at_str,
        }));
    }

    // Additional rate limits
    if let Some(arr) = json["additional_rate_limits"].as_array() {
        for rl in arr {
            limits.push(serde_json::json!({
                "name": rl["name"].as_str().unwrap_or("additional"),
                "limit": rl["limit"].as_i64().unwrap_or(0),
                "remaining": rl["remaining"].as_i64().unwrap_or(0),
                "used": rl["used"].as_i64().unwrap_or(0),
                "period_seconds": rl["period_seconds"].as_i64(),
                "reset_at": rl["reset_at"].as_str(),
            }));
        }
    }

    (limits, plan_type)
}