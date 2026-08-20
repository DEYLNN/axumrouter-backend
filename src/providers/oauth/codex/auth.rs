/// Credentials parsed from the OpenAI Codex OAuth token bundle.
///
/// Codex's auth.openai.com token returns `access_token` + `refresh_token`
/// (offline_access scope) + `id_token`. We stash refresh_token + expires_at
/// so the OAuth layer can refresh proactively before the access token
/// expires (9router uses 5-day refresh lead).
#[derive(Debug, Clone)]
pub struct CxOAuthCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Stored for completeness / future claims verification; not read at
    /// request time (Codex needs access_token only for upstream calls).
    #[allow(dead_code)]
    pub id_token: Option<String>,
    pub expires_at: Option<i64>,
    pub email: Option<String>,
    /// OpenAI account id (`chatgpt-account-id` header). Some responses embed
    /// `chatgpt_account_id`/`account_id`; used for multi-account routing.
    pub account_id: Option<String>,
}

impl CxOAuthCredential {
    pub fn parse(kv: &str) -> Result<Self, String> {
        let val: serde_json::Value =
            serde_json::from_str(kv).map_err(|e| format!("Codex: invalid key_value JSON: {e}"))?;
        let access_token = val["access_token"]
            .as_str()
            .or_else(|| val["accessToken"].as_str())
            .unwrap_or_default()
            .to_string();
        if access_token.is_empty() {
            return Err("Codex: missing access_token".to_string());
        }
        let refresh_token = val["refresh_token"]
            .as_str()
            .or_else(|| val["refreshToken"].as_str())
            .map(String::from);
        let id_token = val["id_token"].as_str().map(String::from);
        let expires_at = val["expires_at"]
            .as_i64()
            .or_else(|| val["expiresAt"].as_i64());
        let email = val["email"].as_str().map(String::from);
        let account_id = val["chatgpt_account_id"]
            .as_str()
            .or_else(|| val["chatgptAccountId"].as_str())
            .or_else(|| val["account_id"].as_str())
            .map(String::from);
        Ok(Self {
            access_token,
            refresh_token,
            id_token,
            expires_at,
            email,
            account_id,
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