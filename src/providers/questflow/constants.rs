use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "qf";
pub const PROVIDER_NAME: &str = "Questflow";
pub const MODEL_PREFIX: &str = "qf";
pub const BASE_URL: &str = "https://app.questflow.ai/openapi";
pub const VALIDATE_URL: &str = "https://app.questflow.ai/openapi/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#0E9384";
pub const ICON_NAME: &str = "questflow.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://app.questflow.ai",
        api_key_url: "https://app.questflow.ai",
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
    ModelDef { id: "gpt-4o-mini", name: "GPT-4o Mini", max_tokens: 128000, supports_vision: false, supports_tools: true },
];
