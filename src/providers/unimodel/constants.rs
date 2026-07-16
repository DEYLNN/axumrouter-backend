use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "um";
pub const PROVIDER_NAME: &str = "Unimodel";
pub const MODEL_PREFIX: &str = "um";
pub const BASE_URL: &str = "https://www.unimodel.ai";
pub const VALIDATE_URL: &str = "https://www.unimodel.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#8B5CF6";
pub const ICON_NAME: &str = "unimodel.jpg";
pub const DOCS_URL: &str = "https://unimodel.ai";
pub const API_KEY_URL: &str = "https://unimodel.ai";
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
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-5.2", name: "GLM 5.2", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-5.1", name: "GLM 5.1", max_tokens: 202752, supports_vision: false, supports_tools: true },
];
