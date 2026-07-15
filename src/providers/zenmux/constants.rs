use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "zmx";
pub const PROVIDER_NAME: &str = "Zenmux";
pub const MODEL_PREFIX: &str = "zmx";
pub const BASE_URL: &str = "https://zenmux.ai";
pub const VALIDATE_URL: &str = "https://zenmux.ai/api/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#7C3AED";
pub const ICON_NAME: &str = "zenmux.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://zenmux.ai",
        api_key_url: "https://zenmux.ai",
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
    ModelDef { id: "stepfun/step-3.7-flash-free", name: "Step 3.7 Flash Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
];
