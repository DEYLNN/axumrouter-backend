use crate::engine::anthropic_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "kimi";
pub const PROVIDER_NAME: &str = "Kimi";
pub const MODEL_PREFIX: &str = "kimi";
pub const BASE_URL: &str = "https://api.kimi.com/coding";
pub const VALIDATE_URL: &str = "https://api.kimi.com/coding/v1/messages";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#1E3A8A";
pub const ICON_NAME: &str = "kimi.png";
pub const DOCS_URL: &str = "";
pub const API_KEY_URL: &str = "";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::anthropic_compat::config::AnthropicConfig {
    crate::engine::anthropic_compat::config::AnthropicConfig {
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
    ModelDef { id: "kimi-k2.6", name: "Kimi K2.6", max_tokens: 262000, supports_vision: false, supports_tools: true },
    ModelDef { id: "kimi-k2.5", name: "Kimi K2.5", max_tokens: 262000, supports_vision: false, supports_tools: true },
];
