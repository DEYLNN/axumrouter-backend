use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "yun";
pub const PROVIDER_NAME: &str = "Yunwu AI";
pub const MODEL_PREFIX: &str = "yun";
pub const BASE_URL: &str = "https://yunwu.ai";
pub const VALIDATE_URL: &str = "https://yunwu.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#6366F1";
pub const ICON_NAME: &str = "yunwu.png";
pub const DOCS_URL: &str = "https://yunwu.ai";
pub const API_KEY_URL: &str = "https://yunwu.ai";
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
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1_000_000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1_000_000, supports_vision: false, supports_tools: true },
];
