use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "volc";
pub const PROVIDER_NAME: &str = "Volcengine Ark";
pub const MODEL_PREFIX: &str = "volc";
pub const BASE_URL: &str = "https://ark.cn-beijing.volces.com/api/coding/v3";
pub const VALIDATE_URL: &str = "https://ark.cn-beijing.volces.com/api/coding/v3/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#1677FF";
pub const ICON_NAME: &str = "volcengine.png";
pub const DOCS_URL: &str = "";
pub const API_KEY_URL: &str = "";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

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
        quirks: ProviderQuirks::default(),
    }
}

pub const MODELS: &[ModelDef] = &[];
