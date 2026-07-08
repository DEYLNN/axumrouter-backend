/// Simple API key auth for OpenAI-compatible providers.
/// Supports both Bearer token and X-Api-Key header.
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

impl ApiKeyAuth {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn bearer_token(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Returns the auth header header format used. Config-driven from quirks.
    pub fn to_header(&self, auth_header: crate::providers::spec::AuthHeader) -> (String, String) {
        match auth_header {
            crate::providers::spec::AuthHeader::Bearer => {
                ("Authorization".into(), self.bearer_token())
            }
            crate::providers::spec::AuthHeader::XApiKey => {
                ("x-api-key".into(), self.api_key.clone())
            }
        }
    }
}
