pub const PROVIDER_ID: &str = "gb";
pub const PROVIDER_NAME: &str = "Grok Build";
pub const CATEGORY: &str = "oauth";
pub const COLOR: &str = "#1DA1F2";
pub const ICON_NAME: &str = "grok-cli.png";

// Grok CLI — custom Responses API (bukan OpenAI chat)
pub const BASE_URL: &str = "https://cli-chat-proxy.grok.com/v1/responses";
pub const MODELS_URL: &str = "https://cli-chat-proxy.grok.com/v1/models";

// OAuth — same xAI OAuth but with grok-cli scope
pub const OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
pub const OAUTH_TOKEN_ENDPOINT: &str = "https://auth.x.ai/oauth2/token";
pub const OAUTH_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access conversations:read conversations:write";
pub const OAUTH_REFERRER: &str = "grok-build";

pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 200;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;

pub const USER_AGENT: &str = "grok-shell/0.2.99 (linux; x86_64)";
pub const CLIENT_IDENTIFIER: &str = "grok-shell";
pub const CLIENT_VERSION: &str = "0.2.99";

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "grok-cli",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: MODELS_URL,
        compatible_api: "openai-responses",
        supports_streaming: true,
        supports_tools: true,
        supports_vision: false,
        color: COLOR,
        icon_name: ICON_NAME,
        usage_url: Some("https://cli-chat-proxy.grok.com/v1/billing?format=credits"),
        quirks: Default::default(),
    }
}

#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub max_tokens: Option<u32>,
    pub context_length: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
}

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "grok-build", name: "Grok Build", max_tokens: None, context_length: 500000, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-4.5", name: "Grok 4.5", max_tokens: None, context_length: 500000, supports_vision: false, supports_tools: true },
];
