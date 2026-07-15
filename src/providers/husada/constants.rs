use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "hus";
pub const PROVIDER_NAME: &str = "Husada";
pub const MODEL_PREFIX: &str = "hus";
pub const BASE_URL: &str = "https://husada.net";
pub const VALIDATE_URL: &str = "https://husada.net/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#10B981";
pub const ICON_NAME: &str = "husada.jpg";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://husada.net",
        api_key_url: "https://husada.net",
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
    ModelDef { id: "gemini-3.5-flash-extra-low", name: "Gemini 3.5 Flash Extra Low", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "gemini-3.5-flash-low", name: "Gemini 3.5 Flash Low", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "gemini-3.1-pro-low", name: "Gemini 3.1 Pro Low", max_tokens: 2097152, supports_vision: false, supports_tools: true },
];
