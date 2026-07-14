#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub backend_model: &'static str,
    pub max_tokens: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
}

pub const PROVIDER_ID: &str = "kl";
pub const PROVIDER_NAME: &str = "Kilo Code";
pub const PROVIDER_FULL_NAME: &str = "kilocode";

pub const API_BASE_URL: &str = "https://api.kilo.ai";
pub const CHAT_URL: &str = "https://api.kilo.ai/api/openrouter/chat/completions";
pub const INITIATE_URL: &str = "https://api.kilo.ai/api/device-auth/codes";
pub const POLL_URL_BASE: &str = "https://api.kilo.ai/api/device-auth/codes";
pub const PROFILE_URL: &str = "https://api.kilo.ai/api/profile";

pub const CATEGORY: &str = "oauth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 120;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 120;
pub const POLL_INTERVAL_MS: u64 = 3000;
pub const POLL_TIMEOUT_SECS: u64 = 300;

pub const SUPPORTS_STREAMING: bool = true;
pub const SUPPORTS_VISION: bool = false;
pub const SUPPORTS_TOOLS: bool = true;

pub const COLOR: &str = "#FF6B35";
pub const ICON: &str = "code";
pub const ICON_URL: &str = "/public/providers/kl.png";
pub const WEBSITE: &str = "https://kilocode.ai";

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "nvidia-nemotron-3-ultra-550b-a55b-free", name: "NVIDIA Nemotron 3 Ultra 550B", backend_model: "nvidia/nemotron-3-ultra-550b-a55b:free", max_tokens: 128000, supports_vision: false, supports_tools: false },
    ModelDef { id: "tencent-hy3-free", name: "Tencent Hunyuan 3", backend_model: "tencent/hy3:free", max_tokens: 128000, supports_vision: false, supports_tools: false },
    ModelDef { id: "stepfun-step-3-7-flash-free", name: "StepFun Step 3.7 Flash", backend_model: "stepfun/step-3.7-flash:free", max_tokens: 128000, supports_vision: false, supports_tools: false },
];
