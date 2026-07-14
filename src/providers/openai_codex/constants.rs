pub const PROVIDER_ID: &str = "cx";
pub const PROVIDER_NAME: &str = "OpenAI Codex";
pub const PROVIDER_FULL_NAME: &str = "codex";
pub const BASE_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
pub const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CATEGORY: &str = "oauth";
pub const COLOR: &str = "#10A37F";
pub const ICON_URL: &str = "/public/providers/openai.webp";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 200;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;
pub const USER_AGENT: &str = "codex_cli_rs/0.136.0";

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: PROVIDER_FULL_NAME,
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: "https://chatgpt.com/codex",
        compatible_api: "openai-responses",
        supports_streaming: true,
        supports_tools: false,
        supports_vision: false,
        color: COLOR,
        icon_url: ICON_URL,
        usage_url: Some("https://chatgpt.com/backend-api/wham/usage"),
        quirks: Default::default(),
    }
}

#[derive(Debug, Clone)]
pub struct ModelDef { pub id: &'static str, pub name: &'static str, pub max_tokens: u32, pub supports_vision: bool, pub supports_tools: bool }

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "gpt-5.5", name: "GPT 5.5", max_tokens: 1000000, supports_vision: false, supports_tools: true },
];
