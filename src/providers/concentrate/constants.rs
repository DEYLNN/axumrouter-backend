use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "conc";
pub const PROVIDER_NAME: &str = "Concentrate AI";
pub const MODEL_PREFIX: &str = "conc";
pub const BASE_URL: &str = "https://api.concentrate.ai";
pub const VALIDATE_URL: &str = "https://api.concentrate.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#3B82F6";
pub const ICON_NAME: &str = "concentrate.png";
pub const DOCS_URL: &str = "https://concentrate.ai";
pub const API_KEY_URL: &str = "https://concentrate.ai";
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
    ModelDef { id: "hy3", name: "Tencent Hy3", max_tokens: 260000, supports_vision: false, supports_tools: true },
    ModelDef { id: "gpt-oss-20b", name: "GPT OSS 20B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "gpt-oss-120b", name: "GPT OSS 120B", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", max_tokens: 1048576, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", max_tokens: 1040000, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax-m3", name: "MiniMax M3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5", name: "MiMo V2.5", max_tokens: 1000000, supports_vision: true, supports_tools: true },
    ModelDef { id: "claude-sonnet-5", name: "Claude Sonnet 5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-haiku-4-5", name: "Claude Haiku 4.5", max_tokens: 200000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-opus-4-5", name: "Claude Opus 4.5", max_tokens: 200000, supports_vision: false, supports_tools: true },
    ModelDef { id: "claude-fable-5", name: "Claude Fable 5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "gpt-5.6-sol", name: "GPT 5.6 Sol", max_tokens: 1050000, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-4.5", name: "Grok 4.5", max_tokens: 500000, supports_vision: false, supports_tools: true },
    ModelDef { id: "gemini-3.5-flash", name: "Gemini 3.5 Flash", max_tokens: 1048576, supports_vision: true, supports_tools: true },
];
