use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "ocf";
pub const PROVIDER_NAME: &str = "OpenCode Free";
pub const MODEL_PREFIX: &str = "ocf";
pub const BASE_URL: &str = "https://opencode.ai/zen";
pub const VALIDATE_URL: &str = "https://opencode.ai/zen/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#E87040";
pub const ICON_NAME: &str = "ocf.webp";
pub const DOCS_URL: &str = "https://opencode.ai";
pub const API_KEY_URL: &str = "https://opencode.ai";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "north-mini-code-free", name: "North Mini Code Free", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "nemotron-3-ultra-free", name: "Nemotron 3 Ultra Free", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5-free", name: "MiMo V2.5 Free", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash-free", name: "DeepSeek V4 Flash Free", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "hy3-free", name: "Tencent Hy3 Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
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
