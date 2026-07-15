use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "0g";
pub const PROVIDER_NAME: &str = "0G Labs";
pub const MODEL_PREFIX: &str = "0g";
pub const BASE_URL: &str = "https://router-api.0g.ai";
pub const VALIDATE_URL: &str = "https://router-api.0g.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#111827";
pub const ICON_NAME: &str = "0g-labs.jpg";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://router-api.0g.ai",
        api_key_url: "https://router-api.0g.ai",
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
    ModelDef { id: "minimax-m3", name: "MiniMax M3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-fable-5", name: "Claude Fable 5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
