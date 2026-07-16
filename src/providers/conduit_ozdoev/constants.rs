use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "co";
pub const PROVIDER_NAME: &str = "Conduit Ozdoev";
pub const MODEL_PREFIX: &str = "co";
pub const BASE_URL: &str = "https://conduit.ozdoev.net/api";
pub const VALIDATE_URL: &str = "https://conduit.ozdoev.net/api/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#0EA5E9";
pub const ICON_NAME: &str = "conduit.jpg";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://conduit.ozdoev.net",
        api_key_url: "https://conduit.ozdoev.net",
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
    ModelDef { id: "anthropic/claude-sonnet-4-6", name: "Claude Sonnet 4.6", max_tokens: 1_000_000, supports_vision: true, supports_tools: true },
    ModelDef { id: "claude-haiku-4-5", name: "Claude Haiku 4.5", max_tokens: 256_000, supports_vision: true, supports_tools: true },
];
