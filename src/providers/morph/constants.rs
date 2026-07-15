use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "mrph";
pub const PROVIDER_NAME: &str = "Morph LLM";
pub const MODEL_PREFIX: &str = "mrph";
pub const BASE_URL: &str = "https://api.morphllm.com";
pub const VALIDATE_URL: &str = "https://api.morphllm.com/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#6B4FBB";
pub const ICON_NAME: &str = "morph.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub fn config() -> crate::engine::openai_compat::config::OpenAIConfig {
    crate::engine::openai_compat::config::OpenAIConfig {
        provider_id: PROVIDER_ID,
        provider_name: PROVIDER_NAME,
        model_prefix: MODEL_PREFIX,
        base_url: BASE_URL,
        validate_url: VALIDATE_URL,
        docs_url: "https://api.morphllm.com",
        api_key_url: "https://api.morphllm.com",
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
    ModelDef { id: "morph-minimax3-428b", name: "Morph MiniMax3 428B", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "morph-dsv4flash", name: "Morph DeepSeek V4 Flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "morph-qwen35-397b", name: "Morph Qwen3.5 397B", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "morph-qwen36-27b", name: "Morph Qwen3.6 27B", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
