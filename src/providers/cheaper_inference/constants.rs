use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "chi";
pub const PROVIDER_NAME: &str = "Cheaper Inference";
pub const MODEL_PREFIX: &str = "chi";
pub const BASE_URL: &str = "https://api.cheaperinference.com";
pub const VALIDATE_URL: &str = "https://api.cheaperinference.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#10B981";
pub const ICON_NAME: &str = "cheaper_inference.png";
pub const DOCS_URL: &str = "https://cheaperinference.com";
pub const API_KEY_URL: &str = "https://cheaperinference.com";
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
    ModelDef { id: "hy3", name: "Tencent Hy3", max_tokens: 260000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-sonnet-5", name: "Claude Sonnet 5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-5.2", name: "GLM 5.2", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "gpt-5.6-sol", name: "GPT 5.6 Sol", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-opus-4.8", name: "Claude Opus 4.8", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-fable-5", name: "Claude Fable 5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
