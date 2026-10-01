use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZbCredential {
    pub api_key: String,
}

impl ZbCredential {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("Empty credential".into());
        }
        Ok(ZbCredential { api_key: trimmed.to_string() })
    }
}
