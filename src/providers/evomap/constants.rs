use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "evo";
pub const PROVIDER_NAME: &str = "Evomap";
pub const MODEL_PREFIX: &str = "evo";
pub const BASE_URL: &str = "https://api.evomap.ai";
pub const VALIDATE_URL: &str = "https://api.evomap.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#111827";
pub const ICON_NAME: &str = "evomap.svg";
pub const DOCS_URL: &str = "https://evomap.ai";
pub const API_KEY_URL: &str = "https://evomap.ai";
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
    ModelDef { id: "evomap-gemini-3.1-pro-preview", name: "Gemini 3.1 Pro Preview", max_tokens: 131072, supports_vision: true, supports_tools: true },
    ModelDef { id: "evomap-deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "evomap-claude-opus-4-7", name: "Claude Opus 4.7", max_tokens: 131072, supports_vision: true, supports_tools: true },
    ModelDef { id: "evomap-glm-5.1", name: "GLM 5.1", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "evomap-gpt-5.5", name: "GPT 5.5", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "evomap-kimi-k2.6", name: "Kimi K2.6", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
