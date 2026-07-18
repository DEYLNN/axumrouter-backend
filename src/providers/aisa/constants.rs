use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "aio";
pub const PROVIDER_NAME: &str = "Aisa One";
pub const MODEL_PREFIX: &str = "aio";
pub const BASE_URL: &str = "https://api.aisa.one";
pub const VALIDATE_URL: &str = "https://api.aisa.one/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#F472B6";
pub const ICON_NAME: &str = "aisa.png";
pub const DOCS_URL: &str = "https://aisa.one";
pub const API_KEY_URL: &str = "https://aisa.one";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

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
    ModelDef { id: "tencent/hy3", name: "Tencent Hy3", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-sonnet-5", name: "Claude Sonnet 5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "kimi-k3", name: "Kimi K3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
