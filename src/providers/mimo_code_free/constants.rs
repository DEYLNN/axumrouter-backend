use crate::engine::openai_compat::config::ModelDef;

pub const PROVIDER_ID: &str = "mcf";
pub const PROVIDER_NAME: &str = "MiMo Code Free";
pub const MODEL_PREFIX: &str = "mcf";
pub const BASE_URL: &str = "https://api.xiaomimimo.com/api/free-ai/openai/chat";
pub const BOOTSTRAP_URL: &str = "https://api.xiaomimimo.com/api/free-ai/bootstrap";
pub const CATEGORY: &str = "apikey";
pub const COLOR: &str = "#FF6900";
pub const ICON_URL: &str = "/public/providers/mcf.png";
pub const DEFAULT_TIMEOUT_SECS: u64 = 90;

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "mimo-auto", name: "MiMo Auto", max_tokens: 128000, supports_vision: false, supports_tools: true },
];
