use serde::{Deserialize, Serialize};
use crate::error::GatewayError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokCliOAuthCredential {
    #[serde(default)] pub access_token: String,
    #[serde(default)] pub refresh_token: String,
    #[serde(default)] pub id_token: String,
    #[serde(default)] pub scope: String,
    #[serde(default)] pub expires_in: u64,
    #[serde(default)] pub email: String,
    #[serde(default)] pub disabled: bool,
    #[serde(default)] pub expires_at: Option<String>,
}

impl GrokCliOAuthCredential {
    pub fn parse(raw: &str) -> Result<Self, GatewayError> {
        let mut cred: Self = serde_json::from_str(raw)
            .map_err(|e| GatewayError::ProviderError(format!("grok-cli key must be OAuth JSON: {}", e)))?;

        // Compute expires_at from expires_in if not present
        if cred.expires_at.is_none() && cred.expires_in > 0 {
            let exp = chrono::Utc::now() + chrono::Duration::seconds(cred.expires_in as i64);
            cred.expires_at = Some(exp.to_rfc3339());
        }

        if cred.disabled {
            return Err(GatewayError::ProviderError("grok-cli credential disabled".into()));
        }
        if cred.access_token.trim().is_empty() && cred.refresh_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("grok-cli OAuth JSON missing access_token/refresh_token".into()));
        }
        Ok(cred)
    }

    pub fn is_expired(&self) -> bool {
        self.access_token.trim().is_empty() && !self.refresh_token.trim().is_empty()
    }

    /// Check if token needs refresh: expired or within 5 min buffer
    pub fn needs_refresh(&self) -> bool {
        // No refresh token? can't refresh.
        if self.refresh_token.trim().is_empty() {
            return false;
        }
        // Access token empty? definitely needs refresh.
        if self.access_token.trim().is_empty() {
            return true;
        }
        // Check expires_at with 5 min buffer
        if let Some(ref exp) = self.expires_at {
            if let Ok(exp_parsed) = chrono::DateTime::parse_from_rfc3339(exp) {
                let buffer = chrono::Duration::minutes(5);
                if chrono::Utc::now() + chrono::Duration::seconds(10) >= exp_parsed - buffer {
                    return true;
                }
            }
        }
        false
    }
}
