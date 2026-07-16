use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "mimo";
pub const PROVIDER_NAME: &str = "Xiaomi MiMo";
pub const MODEL_PREFIX: &str = "mimo";
pub const BASE_URL: &str = "https://api.xiaomimimo.com";
pub const VALIDATE_URL: &str = "https://api.xiaomimimo.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF6900";
pub const ICON_NAME: &str = "xiaomi-mimo.png";
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

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "mimo-v2.5-pro-ultraspeed", name: "MiMo V2.5 Pro Ultrasped", max_tokens: 131000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5-pro", name: "MiMo V2.5 Pro", max_tokens: 131000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5", name: "MiMo V2.5", max_tokens: 131000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2-omni", name: "MiMo V2 Omni", max_tokens: 131000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2-flash", name: "MiMo V2 Flash", max_tokens: 131000, supports_vision: false, supports_tools: true },
];
