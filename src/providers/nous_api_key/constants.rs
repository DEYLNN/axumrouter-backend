use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "nrak";
pub const PROVIDER_NAME: &str = "Nous Research";
pub const MODEL_PREFIX: &str = "nrak";
pub const BASE_URL: &str = "https://inference-api.nousresearch.com/v1";
pub const VALIDATE_URL: &str = "https://inference-api.nousresearch.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#2563EB";
pub const ICON_NAME: &str = "nrak.png";
pub const DOCS_URL: &str = "https://portal.nousresearch.com/help";
pub const API_KEY_URL: &str = "https://portal.nousresearch.com";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "Hermes-4-70B", name: "Hermes 4 70B", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "Hermes-3-Llama-3.1-70B", name: "Hermes 3 70B", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "Hermes-3-Llama-3.1-8B", name: "Hermes 3 8B", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "Hermes-3-Llama-3.2-3B", name: "Hermes 3 3B", max_tokens: 128000, supports_vision: false, supports_tools: true },
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
