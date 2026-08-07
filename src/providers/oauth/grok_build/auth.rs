/// Credentials parsed from the Grok Build OAuth token.
///
/// Grok Build's xAI OAuth returns the standard `access_token` + refresh
/// pair. We also stash `refresh_token` and `expires_at` so the OAuth layer
/// can refresh proactively before the access token expires.
#[derive(Debug, Clone)]
pub struct GbOAuthCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub email: Option<String>,
}

impl GbOAuthCredential {
    pub fn parse(kv: &str) -> Result<Self, String> {
        let val: serde_json::Value =
            serde_json::from_str(kv).map_err(|e| format!("GrokBuild: invalid key_value JSON: {e}"))?;
        let access_token = val["access_token"]
            .as_str()
            .or_else(|| val["accessToken"].as_str())
            .ok_or_else(|| "GrokBuild: missing access_token".to_string())?
            .to_string();
        let refresh_token = val["refresh_token"]
            .as_str()
            .or_else(|| val["refreshToken"].as_str())
            .map(String::from);
        let expires_at = val["expires_at"].as_i64().or_else(|| val["expiresAt"].as_i64());
        let email = val["email"].as_str().map(String::from);
        Ok(Self {
            access_token,
            refresh_token,
            expires_at,
            email,
        })
    }

    /// True if we have a refresh token AND it would expire within `lead_secs`.
    pub fn needs_refresh(&self, now_secs: i64, lead_secs: i64) -> bool {
        match (self.refresh_token.as_ref(), self.expires_at) {
            (Some(_), Some(exp)) => exp - now_secs <= lead_secs,
            _ => false,
        }
    }
}
