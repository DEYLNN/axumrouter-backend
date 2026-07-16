use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "kim";
pub const PROVIDER_NAME: &str = "Kimchi";
pub const MODEL_PREFIX: &str = "kim";
pub const BASE_URL: &str = "https://llm.kimchi.dev/openai";
pub const VALIDATE_URL: &str = "https://llm.kimchi.dev/openai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF5A3D";
pub const ICON_NAME: &str = "kimchi.jpg";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://llm.kimchi.dev",
        api_key_url: "https://llm.kimchi.dev",
        category: CATEGORY,
        color: COLOR,
        icon_name: ICON_NAME,
        default_timeout_secs: DEFAULT_TIMEOUT_SECS,
        stream_first_chunk_timeout_secs: 120,
        stream_stall_timeout_secs: 120,
        models: MODELS,
        quirks: ProviderQuirks::default(),
    }
}

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "kimi-k2.6", name: "Kimi K2.6", max_tokens: 262_144, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax-m3", name: "MiniMax M3", max_tokens: 1_000_000, supports_vision: false, supports_tools: true },
];
