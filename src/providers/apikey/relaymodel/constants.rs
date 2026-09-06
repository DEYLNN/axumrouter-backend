pub const PROVIDER_ID: &str = "relm";
pub const PROVIDER_NAME: &str = "RelayModel";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#18181B";
pub const ICON_NAME: &str = "relm.jpg";
pub const BASE_URL: &str = "https://api.relaymodels.com/v1";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 200;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;
pub const USER_AGENT: &str = "axumrouter/1.0";

/// All RelayModel models are thinking models — reasoning is always hidden
/// downstream: `reasoning_content` field dropped + think-block tags stripped.
pub const THINKING_TAGS: &[&str] = &["think", "thinking", "reasoning"];

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub context_length: u32,
}

pub const MODELS: &[ModelDef] = &[
    ModelDef {
        id: "deepseek-v4-flash",
        context_length: 1000000,
    },
];

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "relaymodel",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: "https://api.relaymodels.com/v1/models",
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
