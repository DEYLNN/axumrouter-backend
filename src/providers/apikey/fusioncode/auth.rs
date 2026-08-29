use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsnCredential {
    pub api_key: String,
}

impl FsnCredential {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("Empty credential".into());
        }
        Ok(FsnCredential { api_key: trimmed.to_string() })
    }
}
