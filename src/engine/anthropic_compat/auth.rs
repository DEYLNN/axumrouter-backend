use crate::providers::spec::AuthHeader;

pub struct ApiKeyAuth {
    api_key: String,
}

impl ApiKeyAuth {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn to_header(&self, auth_header: AuthHeader) -> (String, String) {
        match auth_header {
            AuthHeader::Bearer => ("Authorization".into(), format!("Bearer {}", self.api_key)),
            AuthHeader::XApiKey => ("x-api-key".into(), self.api_key.clone()),
        }
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Anthropic uses anthropic-version header
    pub fn anthropic_version(&self) -> &'static str {
        "2023-06-01"
    }
}
