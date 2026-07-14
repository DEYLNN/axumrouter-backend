use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "mst";
pub const PROVIDER_NAME: &str = "Mistral AI";
pub const MODEL_PREFIX: &str = "mst";
pub const BASE_URL: &str = "https://api.mistral.ai";
pub const VALIDATE_URL: &str = "https://api.mistral.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF7000";
pub const ICON_URL: &str = "/public/providers/mistral.png";
pub const DOCS_URL: &str = "https://docs.mistral.ai";
pub const API_KEY_URL: &str = "https://console.mistral.ai/api-keys";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "mistral-large-latest", name: "Mistral Large 3", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "mistral-medium-latest", name: "Mistral Medium 3", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "mistral-small-latest", name: "Mistral Small 3", max_tokens: 262144, supports_vision: false, supports_tools: true },
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
