use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "ags";
pub const PROVIDER_NAME: &str = "Agnes AI";
pub const MODEL_PREFIX: &str = "ags";
pub const BASE_URL: &str = "https://apihub.agnes-ai.com";
pub const VALIDATE_URL: &str = "https://apihub.agnes-ai.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#8B5CF6";
pub const ICON_URL: &str = "/public/providers/ags.png";
pub const DOCS_URL: &str = "https://agnesi.ai";
pub const API_KEY_URL: &str = "https://agnesi.ai";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "agnes-2.0-flash", name: "Agnes 2.0 Flash", max_tokens: 500000, supports_vision: false, supports_tools: true },
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
        icon_url: ICON_URL,
        default_timeout_secs: DEFAULT_TIMEOUT_SECS,
        stream_first_chunk_timeout_secs: 200,
        stream_stall_timeout_secs: 360,
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
