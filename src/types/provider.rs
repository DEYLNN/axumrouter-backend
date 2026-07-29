use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub icon_path: String,
    pub category: String,
    pub icon_name: String,
    pub color: String,
    /// OAuth flow type: "device_code" | "authorization_code" | null
    pub oauth_flow: Option<String>,
    /// URL to validate API keys and list models (e.g. /v1/models)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub validate_url: String,
    /// Prefix prepended to model ids by `models_static()` — e.g. `nx` for a
    /// custom provider. `None` when no prefix is added (provider id == prefix).
    /// Lets the chat dispatcher resolve a model-id prefix back to the
    /// provider row id when they differ (custom providers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_prefix: Option<String>,
}
