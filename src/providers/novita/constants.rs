use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "nov";
pub const PROVIDER_NAME: &str = "Novita AI";
pub const MODEL_PREFIX: &str = "nov";
pub const BASE_URL: &str = "https://api.novita.ai/openai";
pub const VALIDATE_URL: &str = "https://api.novita.ai/openai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#8B5CF6";
pub const ICON_NAME: &str = "novita.png";
pub const DOCS_URL: &str = "https://novita.ai";
pub const API_KEY_URL: &str = "https://novita.ai";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

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
    ModelDef { id: "tencent/hy3", name: "Tencent Hy3", max_tokens: 260000, supports_vision: false, supports_tools: true },
    ModelDef { id: "moonshotai/kimi-k3", name: "Moonshotai Kimi K3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "zai-org/glm-5.2", name: "ZAI GLM 5.2", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "moonshotai/kimi-k2.7-code", name: "Moonshotai Kimi K2.7 Code", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek/deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek/deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax/minimax-m3", name: "MiniMax M3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
