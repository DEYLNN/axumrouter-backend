use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "cer";
pub const PROVIDER_NAME: &str = "Cerebras";
pub const MODEL_PREFIX: &str = "cer";
pub const BASE_URL: &str = "https://api.cerebras.ai";
pub const VALIDATE_URL: &str = "https://api.cerebras.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF4F00";
pub const ICON_NAME: &str = "cerebras.png";
pub const DOCS_URL: &str = "https://www.cerebras.ai";
pub const API_KEY_URL: &str = "https://cloud.cerebras.ai/platform";
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
    ModelDef { id: "qwen-3-coder-480b", name: "Qwen3 Coder 480B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-4-maverick-17b-128e-instruct", name: "Llama 4 Maverick", max_tokens: 131072, supports_vision: true, supports_tools: true },
    ModelDef { id: "gpt-oss-120b", name: "GPT OSS 120B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "zai-glm-4.7", name: "ZAI GLM 4.7", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.3-70b", name: "Llama 3.3 70B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-4-scout-17b-16e-instruct", name: "Llama 4 Scout", max_tokens: 131072, supports_vision: true, supports_tools: true },
    ModelDef { id: "qwen-3-235b-a22b-instruct-2507", name: "Qwen3 235B A22B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwen-3-32b", name: "Qwen3 32B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama3.1-8b", name: "Llama 3.1 8B", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
