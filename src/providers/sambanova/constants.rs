use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "sn";
pub const PROVIDER_NAME: &str = "SambaNova";
pub const MODEL_PREFIX: &str = "sn";
pub const BASE_URL: &str = "https://api.sambanova.ai";
pub const VALIDATE_URL: &str = "https://api.sambanova.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#7C3AED";
pub const ICON_NAME: &str = "sambanova.png";
pub const DOCS_URL: &str = "https://cloud.sambanova.ai";
pub const API_KEY_URL: &str = "https://cloud.sambanova.ai/apis";
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
    ModelDef { id: "DeepSeek-V3.2", name: "DeepSeek V3.2", max_tokens: 163840, supports_vision: false, supports_tools: true },
    ModelDef { id: "DeepSeek-V3.1", name: "DeepSeek V3.1", max_tokens: 163840, supports_vision: false, supports_tools: true },
    ModelDef { id: "gpt-oss-120b", name: "GPT OSS 120B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "Llama-4-Maverick-17B-128E-Instruct", name: "Llama 4 Maverick", max_tokens: 1048576, supports_vision: true, supports_tools: true },
    ModelDef { id: "Meta-Llama-3.3-70B-Instruct", name: "Llama 3.3 70B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "gemma-3-12b-it", name: "Gemma 3 12B IT", max_tokens: 131072, supports_vision: true, supports_tools: true },
];
