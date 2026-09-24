pub const PROVIDER_ID: &str = "ocf";
pub const PROVIDER_NAME: &str = "OpenCode Free";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#E87040";
pub const ICON_NAME: &str = "ocf.webp";
pub const BASE_URL: &str = "https://opencode.ai/zen";
pub const VALIDATE_URL: &str = "https://opencode.ai/zen/v1/models";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 200;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;
pub const USER_AGENT: &str = "opencode/1.18.31";

/// OCF thinking models — reasoning always hidden downstream.
pub const THINKING_TAGS: &[&str] = &["think", "thinking", "reasoning"];

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub context_length: u32,
}

pub const MODELS: &[ModelDef] = &[
    ModelDef {
        id: "north-mini-code-free",
        context_length: 128000,
    },
    ModelDef {
        id: "nemotron-3-ultra-free",
        context_length: 1000000,
    },
    ModelDef {
        id: "mimo-v2.5-free",
        context_length: 1000000,
    },
    ModelDef {
        id: "deepseek-v4-flash-free",
        context_length: 1000000,
    },
    ModelDef {
        id: "ling-3.0-flash-free",
        context_length: 262144,
    },
    ModelDef {
        id: "muse-spark-1.2-contributor-free",
        context_length: 128000,
    },
    ModelDef {
        id: "muse-spark-1.3-contributor-free",
        context_length: 128000,
    },
    ModelDef {
        id: "union-alpha",
        context_length: 128000,
    },
    ModelDef {
        id: "jev-1.13-free",
        context_length: 128000,
    },
];

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "opencode",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
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
