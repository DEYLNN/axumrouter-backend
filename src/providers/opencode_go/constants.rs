use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "ocg";
pub const PROVIDER_NAME: &str = "OpenCode Go";
pub const MODEL_PREFIX: &str = "ocg";
pub const BASE_URL: &str = "https://opencode.ai/zen/go";
pub const VALIDATE_URL: &str = "https://opencode.ai/auth";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#E87040";
pub const ICON_URL: &str = "/public/providers/ocg.webp";
pub const DOCS_URL: &str = "https://opencode.ai/auth";
pub const API_KEY_URL: &str = "https://opencode.ai/auth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 180;

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "glm-5.2", name: "GLM 5.2", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax-m3", name: "MiniMax M3", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "kimi-k2.7-code", name: "Kimi K2.7 Code", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "kimi-k2.6", name: "Kimi K2.6", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5", name: "Mimo V2.5", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5-pro", name: "Mimo V2.5 Pro", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "qwq-plus", name: "QwQ Plus", max_tokens: 128000, supports_vision: false, supports_tools: true },
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
