use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "gq";
pub const PROVIDER_NAME: &str = "Groq";
pub const MODEL_PREFIX: &str = "gq";
pub const BASE_URL: &str = "https://api.groq.com/openai";
pub const VALIDATE_URL: &str = "https://api.groq.com/openai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#F55036";
pub const ICON_NAME: &str = "groq.png";
pub const DOCS_URL: &str = "https://groq.com";
pub const API_KEY_URL: &str = "https://console.groq.com/keys";
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
    ModelDef { id: "groq/compound", name: "Compound", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "groq/compound-mini", name: "Compound Mini", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.3-70b-versatile", name: "Llama 3.3 70B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "meta-llama/llama-4-maverick-17b-128e-instruct", name: "Llama 4 Maverick", max_tokens: 1048576, supports_vision: true, supports_tools: true },
    ModelDef { id: "qwen/qwen3-32b", name: "Qwen3 32B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "openai/gpt-oss-120b", name: "GPT-OSS 120B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "openai/gpt-oss-20b", name: "GPT-OSS 20B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.1-8b-instant", name: "Llama 3.1 8B Instant", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
