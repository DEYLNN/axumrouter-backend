use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "alin";
pub const PROVIDER_NAME: &str = "Alibaba Intl";
pub const MODEL_PREFIX: &str = "alin";
pub const BASE_URL: &str = "https://coding-intl.dashscope.aliyuncs.com";
pub const VALIDATE_URL: &str = "https://coding-intl.dashscope.aliyuncs.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF6A00";
pub const ICON_NAME: &str = "alibaba-intl.png";
pub const DOCS_URL: &str = "";
pub const API_KEY_URL: &str = "";
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
    ModelDef { id: "qwen3.5-plus", name: "Qwen 3.5 Plus", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "kimi-k2.5", name: "Kimi K2.5", max_tokens: 262000, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-5", name: "GLM 5", max_tokens: 202752, supports_vision: false, supports_tools: true },
    ModelDef { id: "MiniMax-M2.5", name: "MiniMax M2.5", max_tokens: 204800, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-coder-next", name: "Qwen 3 Coder Next", max_tokens: 262000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-coder-plus", name: "Qwen 3 Coder Plus", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-4.7", name: "GLM 4.7", max_tokens: 202752, supports_vision: false, supports_tools: true },
];
