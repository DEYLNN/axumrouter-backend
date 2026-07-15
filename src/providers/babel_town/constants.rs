use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "bt";
pub const PROVIDER_NAME: &str = "Babel Town";
pub const MODEL_PREFIX: &str = "bt";
pub const BASE_URL: &str = "https://api.babel.town";
pub const VALIDATE_URL: &str = "https://api.babel.town/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#8B5CF6";
pub const ICON_NAME: &str = "";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://api.babel.town",
        api_key_url: "https://api.babel.town",
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

pub const MODELS: &[ModelDef] = &[];
