use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "cwv";
pub const PROVIDER_NAME: &str = "CanopyWave";
pub const MODEL_PREFIX: &str = "cwv";
pub const BASE_URL: &str = "https://inference.canopywave.io";
pub const VALIDATE_URL: &str = "https://inference.canopywave.io/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#15B8A6";
pub const ICON_NAME: &str = "canopywave.png";
pub const DOCS_URL: &str = "https://canopywave.io";
pub const API_KEY_URL: &str = "https://canopywave.io";
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
    ModelDef { id: "minimax/minimax-m2.5", name: "MiniMax M2.5", max_tokens: 204800, supports_vision: false, supports_tools: true },
    ModelDef { id: "moonshotai/kimi-k2.6", name: "Kimi K2.6", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "xiaomimimo/mimo-v2.5", name: "MiMo V2.5", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
