use serde::{Deserialize, Serialize};
use crate::error::GatewayError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CxOAuthCredential {
    #[serde(default)] pub access_token: String,
    #[serde(default)] pub refresh_token: String,
    #[serde(default)] pub id_token: String,
    #[serde(default)] pub expires_at: String,
    #[serde(default)] pub expired: String,
    #[serde(default)] pub email: String,
    #[serde(default)] pub chatgpt_account_id: String,
    #[serde(default)] pub account_id: String,
    #[serde(default)] pub disabled: bool,
}

impl CxOAuthCredential {
    pub fn parse(raw: &str) -> Result<Self, GatewayError> {
        let cred: Self = serde_json::from_str(raw)
            .map_err(|e| GatewayError::ProviderError(format!("Codex key must be OAuth JSON: {}", e)))?;
        if cred.disabled { return Err(GatewayError::ProviderError("Codex credential disabled".into())); }
        if cred.access_token.trim().is_empty() && cred.refresh_token.trim().is_empty() {
            return Err(GatewayError::ProviderError("Codex OAuth JSON missing access_token/refresh_token".into()));
        }
        Ok(cred)
    }

    pub fn account_id(&self) -> Option<&str> {
        if !self.chatgpt_account_id.is_empty() { Some(&self.chatgpt_account_id) }
        else if !self.account_id.is_empty() { Some(&self.account_id) }
        else { None }
    }
}
