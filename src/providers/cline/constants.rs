use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "cl";
pub const PROVIDER_NAME: &str = "Cline";
pub const MODEL_PREFIX: &str = "cl";
pub const BASE_URL: &str = "https://api.cline.bot/api";
pub const VALIDATE_URL: &str = "https://api.cline.bot/api/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#5B9BD5";
pub const ICON_NAME: &str = "cl.png";
pub const DOCS_URL: &str = "https://cline.bot";
pub const API_KEY_URL: &str = "https://cline.bot";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

// Models fetched dynamically via /api/v1/models (not available)
// Added manually from known Cline API models
pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "tencent/hy3", name: "Tencent Hy3", max_tokens: 128000, supports_vision: false, supports_tools: true },
];

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
        quirks: ProviderQuirks {
            drop_stream_options: false,
            drop_tools: false,
            drop_tool_choice: false,
            supports_stream_usage: true,
            ..Default::default()
        },
    }
}
