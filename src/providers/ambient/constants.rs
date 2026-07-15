use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "amb";
pub const PROVIDER_NAME: &str = "Ambient";
pub const MODEL_PREFIX: &str = "amb";
pub const BASE_URL: &str = "https://api.ambient.xyz";
pub const VALIDATE_URL: &str = "https://api.ambient.xyz/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#111827";
pub const ICON_NAME: &str = "ambient.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://api.ambient.xyz",
        api_key_url: "https://api.ambient.xyz",
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
    ModelDef { id: "zai-org/GLM-5.1-FP8", name: "GLM 5.1 FP8", max_tokens: 202752, supports_vision: false, supports_tools: true },
];
