use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub gateway: GatewayConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub public_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GatewayConfig {
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub base_url: String,
    pub api_key: String,
    pub default_model: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub api_key_header: String,
    pub api_key_prefix: String,
    pub admin_password: Option<String>,
    #[serde(default)]
    pub jwt_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
}
