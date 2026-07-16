use crate::engine::anthropic_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "mmxcn";
pub const PROVIDER_NAME: &str = "MiniMax China";
pub const MODEL_PREFIX: &str = "mmxcn";
pub const BASE_URL: &str = "https://api.minimaxi.com/anthropic";
pub const VALIDATE_URL: &str = "https://api.minimaxi.com/anthropic/v1/messages";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#DC2626";
pub const ICON_NAME: &str = "minimax-cn.png";
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
    ModelDef { id: "MiniMax-M2.7", name: "MiniMax M2.7", max_tokens: 204800, supports_vision: false, supports_tools: true },
    ModelDef { id: "MiniMax-M2.5", name: "MiniMax M2.5", max_tokens: 204800, supports_vision: false, supports_tools: true },
    ModelDef { id: "MiniMax-M2.1", name: "MiniMax M2.1", max_tokens: 204800, supports_vision: false, supports_tools: true },
];
