use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "qc";
pub const PROVIDER_NAME: &str = "Qwen Cloud";
pub const MODEL_PREFIX: &str = "qc";
pub const BASE_URL: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode";
pub const VALIDATE_URL: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#1677FF";
pub const ICON_NAME: &str = "qwencloud.png";
pub const DOCS_URL: &str = "https://dashscope.console.aliyun.com";
pub const API_KEY_URL: &str = "https://dashscope.console.aliyun.com/apiKey";
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
    ModelDef { id: "qwen3.7-max", name: "Qwen3.7 Max", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.7-plus", name: "Qwen3.7 Plus", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.6-max-preview", name: "Qwen3.6 Max Preview", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.6-plus", name: "Qwen3.6 Plus", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.6-flash", name: "Qwen3.6 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.6-27b", name: "Qwen3.6 27B", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.6-35b-a3b", name: "Qwen3.6 35B A3B", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.5-plus", name: "Qwen3.5 Plus", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3.5-flash", name: "Qwen3.5 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-max", name: "Qwen3 Max", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-235b-a22b", name: "Qwen3 235B A22B", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-32b", name: "Qwen3 32B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-coder-plus", name: "Qwen3 Coder Plus", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen3-coder-flash", name: "Qwen3 Coder Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-5.2", name: "GLM 5.2", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-5.1", name: "GLM 5.1", max_tokens: 202752, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v3.2", name: "DeepSeek V3.2", max_tokens: 163840, supports_vision: false, supports_tools: true },
];
