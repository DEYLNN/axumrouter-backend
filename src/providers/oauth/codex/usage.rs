use reqwest::Client;
use serde_json::Value;

/// Fetch Codex WHAM usage data (rate limits + session info).
/// Not yet wired to an admin endpoint — reserved for the FE Usage page
/// (mirrors legacy usage.rs). Kept so the wire contract isn't lost.
#[allow(dead_code)]
pub async fn fetch_wham_usage(access_token: &str) -> (Vec<Value>, Option<String>) {
    let client = Client::new();
    let resp = client
        .get(super::constants::WHAM_USAGE_URL)
        .header("Authorization", format!("Bearer {access_token}"))
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

    for (key, name) in [("primary_window", "primary"), ("secondary_window", "secondary")] {
        if let Some(w) = json["rate_limit"][key].as_object() {
            let used_pct = w["used_percent"].as_i64().unwrap_or(0);
            let limit_secs = w["limit_window_seconds"].as_i64().unwrap_or(0);
            let reset_after = w["reset_after_seconds"].as_i64();
            let reset_at = w["reset_at"].as_i64();

            let limit = limit_secs;
            let used = (limit as f64 * used_pct as f64 / 100.0) as i64;
            let remaining = limit - used;

            let reset_at_str = reset_at
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .map(|dt| dt.to_rfc3339());

            limits.push(serde_json::json!({
                "name": name,
                "limit": limit.max(1),
                "remaining": remaining.max(0),
                "used": used.max(0),
                "period_seconds": limit_secs,
                "reset_after_seconds": reset_after,
                "reset_at": reset_at_str,
            }));
        }
    }

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