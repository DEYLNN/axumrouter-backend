pub const PROVIDER_ID: &str = "agr";
pub const PROVIDER_NAME: &str = "Agent Router";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#6366F1";
pub const ICON_NAME: &str = "default_provider.png";
pub const BASE_URL: &str = "https://agentrouter.org/v1";
pub const BACKUP_URL: &str = "https://ps.air-outer.com/v1"; // backup domain if primary blocked
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 200;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;
pub const USER_AGENT: &str = "hermes-agent/1.0"; // CRITICAL: AgentRouter blocks non-supported clients

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
    ModelDef {
        id: "gpt-6-astra",
        context_length: 1000000,
    },
    ModelDef {
        id: "gpt-6.1-sol",
        context_length: 1000000,
    },
];

/// Models that only support /v1/responses (OpenAI Responses API), not /v1/chat/completions.
pub const RESPONSES_ONLY_MODELS: &[&str] = &["gpt-6.1-sol", "gpt-6-astra"];

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "agentrouter",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: "https://agentrouter.org/v1/models",
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
