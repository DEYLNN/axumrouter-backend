use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "lc";
pub const PROVIDER_NAME: &str = "LongCat";
pub const MODEL_PREFIX: &str = "lc";
pub const BASE_URL: &str = "https://api.longcat.chat/openai";
pub const VALIDATE_URL: &str = "https://api.longcat.chat/openai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF8C00";
pub const ICON_NAME: &str = "longcat.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://api.longcat.chat",
        api_key_url: "https://api.longcat.chat",
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
    ModelDef { id: "LongCat-2.0-Preview", name: "LongCat 2.0 Preview", max_tokens: 128000, supports_vision: false, supports_tools: true },
];
