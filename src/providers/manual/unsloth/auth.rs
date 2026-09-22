use serde::{Deserialize, Serialize};

// TODO: define credential structure when auth mechanism is known.
// This could be Bearer, custom header, multi-field token, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnslothCredential {
    pub api_key: String,
}

impl UnslothCredential {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("Empty credential".into());
        }
        Ok(UnslothCredential { api_key: trimmed.to_string() })
    }
}
