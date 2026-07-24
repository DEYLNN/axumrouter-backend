pub const PROVIDER_ID: &str = "fb";
pub const PROVIDER_NAME: &str = "FreeBuff";

pub const API_BASE_URL: &str = "https://www.codebuff.com";
pub const VALIDATE_URL: &str = "https://www.codebuff.com/api/v1/chat/completions";

pub const CATEGORY: &str = "oauth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

pub const COLOR: &str = "#4F7CFF";
pub const ICON_NAME: &str = "fb.png";

pub const FREEBUFF_MAX_MESSAGES: usize = 24;
pub const FREEBUFF_DEEPSEEK_MAX_MESSAGES: usize = 24;
pub const FREEBUFF_DEFAULT_MAX_TOKENS: u32 = 4000;

pub const CONTEXT_PRUNER_AGENT_ID: &str = "context-pruner";

pub const AGENT_BY_MODEL: &[(&str, &str)] = &[
    ("deepseek/deepseek-v4-flash", "base2-free-deepseek-flash"),
    ("deepseek/deepseek-v4-pro", "base2-free-deepseek"),
    ("minimax/minimax-m3", "base2-free-minimax-m3"),
    ("mimo/mimo-v2.5", "base2-free-mimo"),
    ("mimo/mimo-v2.5-pro", "base2-free-mimo-pro"),
];

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub backend_model: &'static str,
    pub max_tokens: u32,
}

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "deepseek-v4-flash", backend_model: "deepseek/deepseek-v4-flash", max_tokens: 1000000 },
    ModelDef { id: "deepseek-v4-pro", backend_model: "deepseek/deepseek-v4-pro", max_tokens: 1000000 },
    ModelDef { id: "minimax-m3", backend_model: "minimax/minimax-m3", max_tokens: 1000000 },
    ModelDef { id: "mimo-v2.5", backend_model: "mimo/mimo-v2.5", max_tokens: 1000000 },
    ModelDef { id: "mimo-v2.5-pro", backend_model: "mimo/mimo-v2.5-pro", max_tokens: 1000000 },
];

/// Only fields actually consulted by body builder.
pub struct AgenticProfile {
    pub max_messages: usize,
    pub strip_reasoning_params: bool,
}

pub const BASE_AGENTIC_PROFILE: AgenticProfile = AgenticProfile {
    max_messages: FREEBUFF_MAX_MESSAGES,
    strip_reasoning_params: true,
};

pub const DEEPSEEK_V4_AGENTIC_PROFILE: AgenticProfile = AgenticProfile {
    max_messages: FREEBUFF_DEEPSEEK_MAX_MESSAGES,
    strip_reasoning_params: false,
};

pub fn agentic_profile_for_backend(backend_model: &str) -> &'static AgenticProfile {
    if backend_model.contains("deepseek") {
        &DEEPSEEK_V4_AGENTIC_PROFILE
    } else {
        &BASE_AGENTIC_PROFILE
    }
}
