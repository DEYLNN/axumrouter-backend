use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "or";
pub const PROVIDER_NAME: &str = "OpenRouter";
pub const MODEL_PREFIX: &str = "or";
pub const BASE_URL: &str = "https://openrouter.ai/api";
pub const VALIDATE_URL: &str = "https://openrouter.ai/api/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#F97316";
pub const ICON_NAME: &str = "openrouter.png";
pub const DOCS_URL: &str = "https://openrouter.ai";
pub const API_KEY_URL: &str = "https://openrouter.ai/settings/keys";
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
    ModelDef { id: "poolside/laguna-m.1:free", name: "Poolside Laguna M.1 (Free)", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "openai/gpt-oss-120b:free", name: "OpenAI GPT OSS 120B (Free)", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "nvidia/nemotron-3-super-120b-a12b:free", name: "NVIDIA Nemotron 3 Super (Free)", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax/minimax-m2.5:free", name: "MiniMax M2.5 (Free)", max_tokens: 204800, supports_vision: false, supports_tools: true },
];
