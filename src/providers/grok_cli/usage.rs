use reqwest::Client;
use serde_json::{Value, Map};

/// Fetch Grok CLI billing + user info (same pattern as official CLI)
/// Response: (Vec<quota_objects>, Option<plan_name>)
pub async fn fetch_grok_usage(access_token: &str) -> (Vec<Value>, Option<String>) {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    fn add_headers(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
        builder
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("User-Agent", "grok-shell/0.2.99 (linux; x86_64)")
            .header("x-xai-token-auth", "xai-grok-cli")
            .header("x-grok-client-identifier", "grok-shell")
            .header("x-grok-client-version", "0.2.99")
            .header("x-grok-client-mode", "headless")
    }

    let (billing_res, user_res) = tokio::join!(
        add_headers(client.get("https://cli-chat-proxy.grok.com/v1/billing?format=credits"), access_token).send(),
        add_headers(client.get("https://cli-chat-proxy.grok.com/v1/user?include=subscription"), access_token).send(),
    );

    let billing_json: Value = match billing_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        Ok(r) if r.status().as_u16() == 401 || r.status().as_u16() == 403 => {
            return (vec![], Some("Grok CLI auth expired".into()));
        }
        Ok(r) => {
            let status = r.status().as_u16();
            let text = r.text().await.unwrap_or_default();
            return (vec![], Some(format!("Billing API error {}: {}", status, text)));
        }
        Err(e) => return (vec![], Some(format!("Network error: {}", e))),
    };

    let user_json: Value = match user_res {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        _ => Value::Null,
    };

    // Parse billing config
    let cfg = billing_json.get("config")
        .and_then(|c| c.as_object())
        .cloned()
        .unwrap_or_default();
    let cfg_val: Value = Value::Object(cfg);

    // Helper: unwrap { val: n } or plain number
    let unwrap_val = |field: &str| -> f64 {
        cfg_val.get(field)
            .and_then(|v| v.get("val").and_then(|n| n.as_f64()))
            .or_else(|| cfg_val.get(field).and_then(|v| v.as_f64()))
            .unwrap_or(0.0)
    };

    let on_demand_cap = unwrap_val("onDemandCap");
    let on_demand_used = unwrap_val("onDemandUsed");
    let prepaid = unwrap_val("prepaidBalance");

    // Parse period end
    let period_end = cfg_val.get("billingPeriodEnd")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| cfg_val.get("currentPeriod")
            .and_then(|p| p.get("end").and_then(|e| e.as_str()))
            .map(|s| s.to_string()));

    // Determine plan
    let is_unified = cfg_val.get("isUnifiedBillingUser").and_then(|v| v.as_bool()).unwrap_or(false);
    let tier = user_json.get("subscriptionTier")
        .or_else(|| user_json.get("subscription").and_then(|s| s.get("tier")))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let plan = if !tier.is_empty() && !tier.eq_ignore_ascii_case("free") && !tier.eq_ignore_ascii_case("none") && !tier.eq_ignore_ascii_case("null") {
        Some(tier.to_string())
    } else if is_unified {
        Some("Grok Build".into())
    } else {
        None
    };

    let mut quotas = Vec::new();

    // On-demand quota
    if on_demand_cap > 0.0 {
        let remaining = (on_demand_cap - on_demand_used).max(0.0);
        let remaining_pct = (remaining / on_demand_cap * 100.0) as i64;
        quotas.push(serde_json::json!({
            "name": "On-demand",
            "limit": on_demand_cap as i64,
            "remaining": remaining as i64,
            "used": on_demand_used as i64,
            "remainingPercentage": remaining_pct,
            "resetAt": period_end,
            "unlimited": false,
        }));
    } else if on_demand_cap == 0.0 && !is_unified {
        // Exhausted free/promo
        quotas.push(serde_json::json!({
            "name": "On-demand",
            "limit": 1,
            "remaining": 0,
            "used": 1,
            "remainingPercentage": 0,
            "resetAt": period_end,
            "unlimited": false,
        }));
    }

    // Prepaid balance
    if prepaid > 0.0 {
        quotas.push(serde_json::json!({
            "name": "Prepaid",
            "limit": prepaid as i64,
            "remaining": prepaid as i64,
            "used": 0,
            "remainingPercentage": 100,
            "resetAt": Value::Null,
            "unlimited": false,
        }));
    } else if is_unified && on_demand_cap == 0.0 {
        // Unlimited subscription — no numeric cap
        quotas.push(serde_json::json!({
            "name": "Unlimited",
            "limit": 0,
            "remaining": 0,
            "used": 0,
            "remainingPercentage": 100,
            "resetAt": period_end,
            "unlimited": true,
        }));
    }

    (quotas, plan)
}
