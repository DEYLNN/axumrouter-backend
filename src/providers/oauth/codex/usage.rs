use reqwest::Client;
use serde_json::Value;

/// Fetch Codex WHAM usage data (rate limits + session info).
/// Not yet wired to an admin endpoint — reserved for the FE Usage page
/// (mirrors legacy usage.rs). Kept so the wire contract isn't lost.
#[allow(dead_code)]
pub async fn fetch_wham_usage(access_token: &str) -> (Vec<Value>, Option<String>, Value) {
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
            Err(_) => return (vec![], None, serde_json::json!({"available_count": 0, "applicable_available_count": 0})),
        },
        Err(_) => return (vec![], None, serde_json::json!({"available_count": 0, "applicable_available_count": 0})),
    };

    let plan_type = json["plan_type"].as_str().map(String::from);
    let reset_credits = serde_json::json!({
        "available_count": json["rate_limit_reset_credits"]["available_count"].as_i64().unwrap_or(0),
        "applicable_available_count": json["rate_limit_reset_credits"]["applicable_available_count"].as_i64().unwrap_or(0),
    });
    let mut limits = Vec::new();

    fn append_window(limits: &mut Vec<Value>, name: &str, window: &Value) {
        let used_pct = window["used_percent"].as_f64()
            .or_else(|| window["percent_used"].as_f64()).unwrap_or(0.0).clamp(0.0, 100.0);
        let limit_secs = window["limit_window_seconds"].as_i64().unwrap_or(0);
        let reset_after = window["reset_after_seconds"].as_i64();
        let reset_at = window["reset_at"].as_i64();
        let limit = limit_secs.max(1);
        let used = (limit as f64 * used_pct / 100.0) as i64;
        let reset_at_str = reset_at
            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.to_rfc3339());

        limits.push(serde_json::json!({
            "name": name,
            "limit": limit,
            "remaining": (limit - used).max(0),
            "used": used.max(0),
            "period_seconds": limit_secs,
            "reset_after_seconds": reset_after,
            "reset_at": reset_at_str,
        }));
    }

    fn append_windows(limits: &mut Vec<Value>, prefix: &str, snapshot: &Value) {
        let body = snapshot.get("rate_limit").unwrap_or(snapshot);
        let primary = body.get("primary_window").or_else(|| body.get("primary"));
        let secondary = body.get("secondary_window").or_else(|| body.get("secondary"));
        if let Some(window) = primary {
            append_window(limits, &format!("{prefix}session"), window);
        }
        if let Some(window) = secondary {
            append_window(limits, &format!("{prefix}weekly"), window);
        }
    }

    append_windows(&mut limits, "primary ", &json["rate_limit"]);

    // Codex reserve/review models use separate windows from normal GPT models.
    // Keep them as distinct rows so FE can show primary and gpt-reserve allocation.
    let review = json.get("code_review_rate_limit")
        .or_else(|| json.get("review_rate_limit"))
        .or_else(|| json.get("rate_limits_by_limit_id").and_then(|v| {
            v.get("code_review").or_else(|| v.get("codex_review")).or_else(|| v.get("review"))
        }))
        .or_else(|| json.get("additional_rate_limits").and_then(|v| v.as_array()).and_then(|items| items.iter().find(|item| {
            let id = item.get("limit_name").or_else(|| item.get("metered_feature")).or_else(|| item.get("id"))
                .and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
            id == "code_review" || id == "codex_review" || id == "review" || id.contains("review")
        })));
    if let Some(review) = review {
        append_windows(&mut limits, "gpt-reserve ", review);
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

    (limits, plan_type, reset_credits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_window_candidate_uses_gpt_reserve_source() {
        let payload = serde_json::json!({
            "rate_limits_by_limit_id": {
                "code_review": {"primary_window": {"used_percent": 25, "limit_window_seconds": 18000}}
            }
        });
        let candidate = payload["rate_limits_by_limit_id"].get("code_review");
        assert!(candidate.is_some());
        assert_eq!(candidate.unwrap()["primary_window"]["used_percent"], 25);
    }
}