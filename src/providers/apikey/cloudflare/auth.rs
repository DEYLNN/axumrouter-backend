use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfCredential {
    #[serde(default, alias = "apiKey")]
    pub api_key: String,
    #[serde(default, alias = "apiKey")]
    pub apiToken: String,
    #[serde(default, alias = "accountId")]
    pub account_id: String,
    #[serde(default)]
    pub accountId: String,
}

impl CfCredential {
    pub fn parse(raw: &str) -> Result<Self, String> {
        // Try JSON first
        if let Ok(cred) = serde_json::from_str::<CfCredential>(raw) {
            let account_id = if !cred.account_id.is_empty() { cred.account_id.clone() }
                else if !cred.accountId.is_empty() { cred.accountId.clone() }
                else { return Err("Missing accountId".into()); };
            let api_key = if !cred.api_key.is_empty() { cred.api_key.clone() }
                else { cred.apiToken.clone() };
            if api_key.is_empty() { return Err("Missing apiKey".into()); }
            return Ok(CfCredential {
                api_key: api_key.clone(),
                apiToken: api_key.clone(),
                account_id: account_id.clone(),
                accountId: account_id,
            });
        }
        // Try as plain API key (legacy format without accountId)
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(CfCredential {
                api_key: trimmed.to_string(),
                apiToken: trimmed.to_string(),
                account_id: String::new(),
                accountId: String::new(),
            });
        }
        Err("Empty credential".into())
    }

    pub fn effective_api_key(&self) -> &str {
        if !self.api_key.is_empty() { &self.api_key }
        else { &self.apiToken }
    }

    pub fn effective_account_id(&self) -> &str {
        if !self.account_id.is_empty() { &self.account_id }
        else { &self.accountId }
    }
}
