use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "ocz";
pub const PROVIDER_NAME: &str = "Ocenza";
pub const MODEL_PREFIX: &str = "ocz";
pub const BASE_URL: &str = "https://ocenza.com";
pub const VALIDATE_URL: &str = "https://ocenza.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#0EA5E9";
pub const ICON_NAME: &str = "ocenza.jpg";
pub const DOCS_URL: &str = "https://ocenza.com";
pub const API_KEY_URL: &str = "https://ocenza.com";
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

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "gpt-oss-120b", name: "GPT OSS 120B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "step-3.5-flash-2603", name: "Step 3.5 Flash", max_tokens: 262144, supports_vision: false, supports_tools: true },
];
