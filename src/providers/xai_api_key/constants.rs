use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "xak";
pub const PROVIDER_NAME: &str = "xAI";
pub const MODEL_PREFIX: &str = "xak";
pub const BASE_URL: &str = "https://api.x.ai";
pub const VALIDATE_URL: &str = "https://api.x.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#1DA1F2";
pub const ICON_NAME: &str = "xai.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://console.x.ai",
        api_key_url: "https://console.x.ai",
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

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "grok-4.5", name: "Grok 4.5", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-4.5-fast-reasoning", name: "Grok 4.5 Fast Reasoning", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-4", name: "Grok 4", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-4-fast-reasoning", name: "Grok 4 Fast Reasoning", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-code-fast-1", name: "Grok Code Fast", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-3", name: "Grok 3", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
