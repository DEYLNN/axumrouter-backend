pub const PROVIDER_ID: &str = "cl";
pub const PROVIDER_NAME: &str = "Cline";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#5B9BD5";
pub const ICON_NAME: &str = "cl.png";
pub const BASE_URL: &str = "https://api.cline.bot/api";
pub const DEFAULT_TIMEOUT_SECS: u64 = 64;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 90;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 180;
pub const USER_AGENT: &str = "axumrouter/1.0";

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "cline",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: "https://api.cline.bot/api/v1/models",
        compatible_api: "openai-chat",
        supports_streaming: true,
        supports_tools: true,
        supports_vision: false,
        color: COLOR,
        icon_name: ICON_NAME,
        usage_url: None,
        quirks: Default::default(),
    }
}