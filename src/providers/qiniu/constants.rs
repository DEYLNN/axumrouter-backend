use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "qiniu";
pub const PROVIDER_NAME: &str = "Qiniu";
pub const MODEL_PREFIX: &str = "qiniu";
pub const BASE_URL: &str = "https://api.qnaigc.com";
pub const VALIDATE_URL: &str = "https://api.qnaigc.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#3B82F6";
pub const ICON_NAME: &str = "qiniu.png";
pub const DOCS_URL: &str = "https://portal.qiniu.com";
pub const API_KEY_URL: &str = "https://portal.qiniu.com";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: DOCS_URL,
        api_key_url: API_KEY_URL,
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
    ModelDef { id: "deepseek-v3", name: "DeepSeek V3", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
