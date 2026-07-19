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
}

impl GrokCliOAuthCredential {
    pub fn parse(raw: &str) -> Result<Self, GatewayError> {
        let cred: Self = serde_json::from_str(raw)
            .map_err(|e| GatewayError::ProviderError(format!("grok-cli key must be OAuth JSON: {}", e)))?;
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
}
