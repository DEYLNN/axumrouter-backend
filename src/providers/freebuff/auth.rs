use serde::{Deserialize, Serialize};

use crate::error::GatewayError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbDeviceCodeResponse {
    pub device_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
    pub fingerprint_hash: Option<String>,
    pub expires_at: Option<String>,
    pub login_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbPollResponse {
    pub ok: bool,
    pub access_token: Option<String>,
    pub email: Option<String>,
    pub user_id: Option<String>,
    pub account_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub fingerprint_hash: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbAuthCredentials {
    pub access_token: String,
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub user_id: Option<String>,
    pub fingerprint_id: Option<String>,
    pub fingerprint_hash: Option<String>,
}

pub async fn request_device_code(http: &reqwest::Client) -> Result<FbDeviceCodeResponse, GatewayError> {
    let fingerprint_id = uuid::Uuid::new_v4().to_string();

    let resp = http
        .post(super::constants::CLI_CODE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&serde_json::json!({ "fingerprintId": fingerprint_id }))
        .send()
        .await
        .map_err(|e| GatewayError::ProviderError(format!("FreeBuff device code request failed: {}", e)))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(GatewayError::ProviderError(format!(
            "FreeBuff login init failed: {}",
            text
        )));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| GatewayError::ProviderError(format!("FreeBuff parse error: {}", e)))?;

    let login_url = data["loginUrl"]
        .as_str()
        .ok_or_else(|| GatewayError::ProviderError("FreeBuff: missing loginUrl".into()))?
        .to_string();

    Ok(FbDeviceCodeResponse {
        device_code: fingerprint_id,
        verification_uri: login_url.clone(),
        verification_uri_complete: login_url.clone(),
        expires_in: data["expiresIn"].as_u64().unwrap_or(600),
        interval: std::cmp::max(1, data["pollInterval"].as_u64().unwrap_or(2)),
        fingerprint_hash: data["fingerprintHash"].as_str().map(String::from),
        expires_at: data["expiresAt"]
            .as_str()
            .map(String::from)
            .or_else(|| data["expiresAt"].as_i64().map(|n| n.to_string())),
        login_url: Some(login_url),
    })
}

pub async fn poll_token(
    http: &reqwest::Client,
    device_code: &str,
    fingerprint_hash: Option<&str>,
    expires_at: Option<&str>,
) -> Result<FbPollResponse, GatewayError> {
    let mut url = reqwest::Url::parse(super::constants::CLI_STATUS_URL)
        .map_err(|e| GatewayError::ProviderError(format!("FreeBuff: URL parse error: {}", e)))?;
    url.query_pairs_mut()
        .append_pair("fingerprintId", device_code);
    if let Some(hash) = fingerprint_hash {
        url.query_pairs_mut().append_pair("fingerprintHash", hash);
    }
    if let Some(exp) = expires_at {
        url.query_pairs_mut().append_pair("expiresAt", exp);
    }

    let resp = http
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| GatewayError::ProviderError(format!("FreeBuff poll error: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        if status.as_u16() == 401 {
            return Ok(FbPollResponse {
                ok: false,
                access_token: None,
                email: None,
                user_id: None,
                account_id: None,
                fingerprint_id: None,
                fingerprint_hash: None,
                error: Some("authorization_pending".into()),
            });
        }
        // Try to parse error from body
        let body = resp.text().await.unwrap_or_default();
        let err = if body.contains("expired") || body.contains("already used") {
            "expired_token"
        } else {
            "poll_failed"
        };
        return Ok(FbPollResponse {
            ok: false,
            access_token: None,
            email: None,
            user_id: None,
            account_id: None,
            fingerprint_id: None,
            fingerprint_hash: None,
            error: Some(err.to_string()),
        });
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| GatewayError::ProviderError(format!("FreeBuff poll parse error: {}", e)))?;

    // Check for error in response body
    if let Some(err_val) = data.get("error") {
        if let Some(err_str) = err_val.as_str() {
            let err = match err_str {
                "expired_token" | "access_denied" => err_str,
                _ => "authorization_pending",
            };
            return Ok(FbPollResponse {
                ok: false,
                access_token: None,
                email: None,
                user_id: None,
                account_id: None,
                fingerprint_id: None,
                fingerprint_hash: None,
                error: Some(err.to_string()),
            });
        }
    }

    let user = &data["user"];
    let auth_token = user["authToken"].as_str().map(String::from);

    match auth_token {
        Some(token) => Ok(FbPollResponse {
            ok: true,
            access_token: Some(token),
            email: user["email"].as_str().map(String::from),
            user_id: user["id"].as_str().or(user["userId"].as_str()).map(String::from),
            account_id: user["accountId"].as_str().map(String::from),
            fingerprint_id: Some(device_code.to_string()),
            fingerprint_hash: fingerprint_hash.map(String::from),
            error: None,
        }),
        None => Ok(FbPollResponse {
            ok: false,
            access_token: None,
            email: None,
            user_id: None,
            account_id: None,
            fingerprint_id: None,
            fingerprint_hash: None,
            error: Some("authorization_pending".into()),
        }),
    }
}
