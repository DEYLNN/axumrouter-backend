use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "poll";
pub const PROVIDER_NAME: &str = "Pollinations";
pub const MODEL_PREFIX: &str = "poll";
pub const BASE_URL: &str = "https://text.pollinations.ai/openai/v1";
pub const VALIDATE_URL: &str = "https://text.pollinations.ai/openai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#22C55E";
pub const ICON_NAME: &str = "pollinations.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://pollinations.ai",
        api_key_url: "",
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
    ModelDef { id: "openai-fast", name: "GPT-OSS 20B (Pollinations)", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
