use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "tr";
pub const PROVIDER_NAME: &str = "TokenRouter";
pub const MODEL_PREFIX: &str = "tr";
pub const BASE_URL: &str = "https://api.tokenrouter.com";
pub const VALIDATE_URL: &str = "https://api.tokenrouter.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#111827";
pub const ICON_NAME: &str = "tokenrouter.jpg";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://api.tokenrouter.com",
        api_key_url: "https://api.tokenrouter.com",
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
    ModelDef { id: "MiniMax-M3", name: "MiniMax M3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek/deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek/deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "z-ai/glm-5.2-free", name: "Z-AI GLM 5.2 Free", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
