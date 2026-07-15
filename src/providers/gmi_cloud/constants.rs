use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "gmi";
pub const PROVIDER_NAME: &str = "GMI Cloud";
pub const MODEL_PREFIX: &str = "gmi";
pub const BASE_URL: &str = "https://api.gmi-serving.com";
pub const VALIDATE_URL: &str = "https://api.gmi-serving.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#0EA5E9";
pub const ICON_NAME: &str = "gmi-cloud.jpg";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://api.gmi-serving.com",
        api_key_url: "https://api.gmi-serving.com",
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
    ModelDef { id: "x-ai/grok-4.5", name: "Grok 4.5", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "tencent/Hy3", name: "Hy3", max_tokens: 262144, supports_vision: false, supports_tools: true },
];
