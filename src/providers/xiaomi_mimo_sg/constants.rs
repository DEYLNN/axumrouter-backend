use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "mimosg";
pub const PROVIDER_NAME: &str = "Xiaomi MiMo SG";
pub const MODEL_PREFIX: &str = "mimosg";
pub const BASE_URL: &str = "https://token-plan-sgp.xiaomimimo.com";
pub const VALIDATE_URL: &str = "https://token-plan-sgp.xiaomimimo.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#2563EB";
pub const ICON_NAME: &str = "mimo-sg.png";
pub const DOCS_URL: &str = "https://xiaomimimo.com";
pub const API_KEY_URL: &str = "https://platform.xiaomimimo.com/token-plan";
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
    ModelDef { id: "mimo-v2.5-pro", name: "MiMo V2.5 Pro", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5", name: "MiMo V2.5", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2-pro", name: "MiMo V2 Pro", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2-omni", name: "MiMo V2 Omni", max_tokens: 131072, supports_vision: true, supports_tools: true },
    ModelDef { id: "mimo-v2-flash", name: "MiMo V2 Flash", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
