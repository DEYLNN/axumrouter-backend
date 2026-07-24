#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub backend_model: &'static str,
    pub max_tokens: Option<u32>,
    pub context_length: u32,
}

pub const PROVIDER_ID: &str = "kc";
pub const PROVIDER_NAME: &str = "Kilo Code";

pub const API_BASE_URL: &str = "https://api.kilo.ai";
pub const CHAT_URL: &str = "https://api.kilo.ai/api/openrouter/chat/completions";

pub const CATEGORY: &str = "oauth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 120;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 120;

pub const COLOR: &str = "#FF6B35";
pub const ICON_NAME: &str = "kc.png";

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "nvidia-nemotron-3-ultra-550b-a55b-free", backend_model: "nvidia/nemotron-3-ultra-550b-a55b:free", max_tokens: None, context_length: 1000000 },
    ModelDef { id: "stepfun-step-3-7-flash-free", backend_model: "stepfun/step-3.7-flash:free", max_tokens: None, context_length: 256000 },
    ModelDef { id: "inclusionai-ling-3-0-flash-free", backend_model: "inclusionai/ling-3.0-flash:free", max_tokens: None, context_length: 262144 },
    ModelDef { id: "poolside-laguna-s-2-1-free", backend_model: "poolside/laguna-s-2.1:free", max_tokens: None, context_length: 262144 },
];
