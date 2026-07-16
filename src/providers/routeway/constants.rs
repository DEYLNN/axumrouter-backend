use crate::engine::openai_compat::config::ModelDef;
use crate::providers::spec::ProviderQuirks;

pub const PROVIDER_ID: &str = "rwy";
pub const PROVIDER_NAME: &str = "Routeway AI";
pub const MODEL_PREFIX: &str = "rwy";
pub const BASE_URL: &str = "https://api.routeway.ai";
pub const VALIDATE_URL: &str = "https://api.routeway.ai/v1/models";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#6366F1";
pub const ICON_NAME: &str = "routeway.png";
pub const DOCS_URL: &str = "https://routeway.ai";
pub const API_KEY_URL: &str = "https://routeway.ai";
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
    ModelDef { id: "ling-2.6-flash:free", name: "Ling 2.6 Flash Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "gemma-4-31b-it:free", name: "Gemma 4 31B IT Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "step-3.5-flash:free", name: "Step 3.5 Flash Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "nemotron-3-nano-30b-a3b:free", name: "Nemotron 3 Nano 30B A3B Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax-m2:free", name: "MiniMax M2 Free", max_tokens: 204800, supports_vision: false, supports_tools: true },
    ModelDef { id: "laguna-xs.2:free", name: "Laguna XS.2 Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "laguna-m.1:free", name: "Laguna M.1 Free", max_tokens: 262144, supports_vision: false, supports_tools: true },
    ModelDef { id: "nemotron-nano-9b-v2:free", name: "Nemotron Nano 9B V2 Free", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "gpt-oss-120b:free", name: "GPT OSS 120B Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "glm-4.5-air:free", name: "GLM 4.5 Air Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.2-3b-instruct:free", name: "Llama 3.2 3B Instruct Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.2-1b-instruct:free", name: "Llama 3.2 1B Instruct Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.1-8b-instruct:free", name: "Llama 3.1 8B Instruct Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "llama-3.3-70b-instruct:free", name: "Llama 3.3 70B Instruct Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
    ModelDef { id: "mistral-nemo-instruct:free", name: "Mistral Nemo Instruct Free", max_tokens: 131072, supports_vision: false, supports_tools: true },
];
