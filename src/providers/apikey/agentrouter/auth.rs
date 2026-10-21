use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgrCredential {
    pub api_key: String,
}

impl AgrCredential {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("Empty credential".into());
        }
        Ok(AgrCredential { api_key: trimmed.to_string() })
    }
}
