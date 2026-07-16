use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "llm7";
pub const PROVIDER_NAME: &str = "LLM7";
pub const MODEL_PREFIX: &str = "llm7";
pub const BASE_URL: &str = "https://api.llm7.io";
pub const VALIDATE_URL: &str = "https://api.llm7.io/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#0EA5E9";
pub const ICON_NAME: &str = "llm7.png";
pub const DOCS_URL: &str = "https://llm7.io";
pub const API_KEY_URL: &str = "https://llm7.io";
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
    ModelDef { id: "gpt-oss-20b", name: "GPT OSS 20B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "codestral-latest", name: "Codestral", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "GLM-4.6V-Flash", name: "GLM 4.6V Flash", max_tokens: 131072, supports_vision: true, supports_tools: true },
    ModelDef { id: "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo", name: "Llama 3.1 8B Turbo", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "ministral-8b-2512", name: "Ministral 8B", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "devstral-small-2:24b-cloud", name: "Devstral Small 2 24B", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v3.1:671b-terminus", name: "DeepSeek V3.1 Terminus", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
