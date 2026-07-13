use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub icon_path: String,
    pub category: String,
    pub icon_url: String,
    pub color: String,
    /// OAuth flow type: "device_code" | "authorization_code" | null
    pub oauth_flow: Option<String>,
}
