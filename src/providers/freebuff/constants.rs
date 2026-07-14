pub const PROVIDER_ID: &str = "fb";
pub const PROVIDER_NAME: &str = "FreeBuff";
pub const PROVIDER_FULL_NAME: &str = "freebuff";

pub const API_BASE_URL: &str = "https://www.codebuff.com";
pub const CLI_CODE_URL: &str = "https://www.codebuff.com/api/auth/cli/code";
pub const CLI_STATUS_URL: &str = "https://www.codebuff.com/api/auth/cli/status";
pub const VALIDATE_URL: &str = "https://www.codebuff.com/api/v1/chat/completions";

pub const CATEGORY: &str = "oauth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 200;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;
pub const POLL_INTERVAL_MS: u64 = 4000;
pub const POLL_TIMEOUT_SECS: u64 = 600;

pub const SUPPORTS_STREAMING: bool = true;
pub const SUPPORTS_VISION: bool = false;
pub const SUPPORTS_TOOLS: bool = true;

pub const COLOR: &str = "#4F7CFF";
pub const ICON: &str = "bolt";
pub const ICON_URL: &str = "/public/providers/fb.png";
pub const WEBSITE: &str = "https://freebuff.com";

pub const FREEBUFF_MAX_MESSAGES: usize = 24;
pub const FREEBUFF_DEEPSEEK_MAX_MESSAGES: usize = 24;
pub const FREEBUFF_MAX_MESSAGE_CHARS: usize = 16000;
pub const FREEBUFF_MAX_TOOL_CHARS: usize = 8000;
pub const FREEBUFF_MAX_TOOL_ARG_CHARS: usize = 6000;
pub const FREEBUFF_CONTEXT_PRUNER_AGENT: &str = "context-pruner";
pub const FREEBUFF_DEFAULT_MAX_TOKENS: u32 = 400;

pub const CONTEXT_PRUNER_AGENT_ID: &str = "context-pruner";

pub const AGENT_BY_MODEL: &[(&str, &str)] = &[
    ("deepseek/deepseek-v4-flash", "base2-free-deepseek-flash"),
    ("deepseek/deepseek-v4-pro", "base2-free-deepseek"),
    ("moonshotai/kimi-k2.6", "base2-free-kimi"),
    ("minimax/minimax-m2.7", "base2-free"),
    ("minimax/minimax-m3", "base2-free-minimax-m3"),
    ("mimo/mimo-v2.5", "base2-free-mimo"),
    ("mimo/mimo-v2.5-pro", "base2-free-mimo-pro"),
];

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: PROVIDER_FULL_NAME,
        category: CATEGORY,
        base_url: API_BASE_URL,
        validate_url: VALIDATE_URL,
        compatible_api: "openai",
        supports_streaming: SUPPORTS_STREAMING,
        supports_tools: SUPPORTS_TOOLS,
        supports_vision: SUPPORTS_VISION,
        color: COLOR,
        icon_url: ICON_URL,
        usage_url: None,
        quirks: crate::providers::spec::ProviderQuirks {
            drop_stream_options: false,
            drop_tools: false,
            drop_tool_choice: false,
            supports_stream_usage: true,
            ..Default::default()
        },
    }
}

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub backend_model: &'static str,
    pub max_tokens: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
}

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", backend_model: "deepseek/deepseek-v4-flash", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", backend_model: "deepseek/deepseek-v4-pro", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "kimi-k2.6", name: "Kimi K2.6", backend_model: "moonshotai/kimi-k2.6", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax-m2.7", name: "MiniMax M2.7", backend_model: "minimax/minimax-m2.7", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "minimax-m3", name: "MiniMax M3", backend_model: "minimax/minimax-m3", max_tokens: 128000, supports_vision: false, supports_tools: true },
    ModelDef { id: "mimo-v2.5", name: "MiMo V2.5", backend_model: "mimo/mimo-v2.5", max_tokens: 1000000, supports_vision: true, supports_tools: true },
    ModelDef { id: "mimo-v2.5-pro", name: "MiMo V2.5 Pro", backend_model: "mimo/mimo-v2.5-pro", max_tokens: 1000000, supports_vision: true, supports_tools: true },
];

pub struct AgenticProfile {
    pub native_tools: bool,
    pub forward_tool_choice: bool,
    pub forward_parallel_tool_calls: bool,
    pub inject_reasoning_content: bool,
    pub max_messages: usize,
    pub max_message_chars: usize,
    pub max_tool_chars: usize,
    pub strip_reasoning_params: bool,
    pub forward_thinking: bool,
}

pub const BASE_AGENTIC_PROFILE: AgenticProfile = AgenticProfile {
    native_tools: true,
    forward_tool_choice: true,
    forward_parallel_tool_calls: true,
    inject_reasoning_content: true,
    max_messages: FREEBUFF_MAX_MESSAGES,
    max_message_chars: FREEBUFF_MAX_MESSAGE_CHARS,
    max_tool_chars: FREEBUFF_MAX_TOOL_CHARS,
    strip_reasoning_params: true,
    forward_thinking: false,
};

pub const DEEPSEEK_V4_AGENTIC_PROFILE: AgenticProfile = AgenticProfile {
    native_tools: true,
    forward_tool_choice: true,
    forward_parallel_tool_calls: true,
    inject_reasoning_content: true,
    max_messages: FREEBUFF_DEEPSEEK_MAX_MESSAGES,
    max_message_chars: FREEBUFF_MAX_MESSAGE_CHARS,
    max_tool_chars: FREEBUFF_MAX_TOOL_CHARS,
    strip_reasoning_params: false,
    forward_thinking: true,
};

pub fn agentic_profile_for_backend(backend_model: &str) -> &'static AgenticProfile {
    if backend_model.contains("deepseek") {
        &DEEPSEEK_V4_AGENTIC_PROFILE
    } else {
        &BASE_AGENTIC_PROFILE
    }
}
