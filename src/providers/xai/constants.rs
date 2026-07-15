pub const PROVIDER_ID: &str = "xai";
pub const PROVIDER_NAME: &str = "xAI";
pub const CATEGORY: &str = "oauth";
pub const COLOR: &str = "#1DA1F2";
pub const ICON_NAME: &str = "xai.png";

// OpenAI-compatible
pub const BASE_URL: &str = "https://api.x.ai/v1/chat/completions";
pub const MODELS_URL: &str = "https://api.x.ai/v1/models";

// OAuth
pub const OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
pub const OAUTH_ISSUER: &str = "https://auth.x.ai";
pub const OAUTH_AUTH_ENDPOINT: &str = "https://auth.x.ai/oauth2/authorize";
pub const OAUTH_TOKEN_ENDPOINT: &str = "https://auth.x.ai/oauth2/token";
pub const OAUTH_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
pub const OAUTH_REDIRECT_PORT: u16 = 56121;
pub const OAUTH_REDIRECT_PATH: &str = "/callback";
pub const OAUTH_PKCE_VERIFIER_BYTES: usize = 96;

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 60;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 120;

pub const USER_AGENT: &str = "grok-cli/axumrouter";

pub fn provider_spec() -> crate::providers::spec::ProviderSpec {
    crate::providers::spec::ProviderSpec {
        id: PROVIDER_ID,
        name: PROVIDER_NAME,
        full_name: "xai",
        category: CATEGORY,
        base_url: BASE_URL,
        validate_url: MODELS_URL,
        compatible_api: "openai-chat",
        supports_streaming: true,
        supports_tools: false,
        supports_vision: false,
        color: COLOR,
        icon_name: ICON_NAME,
        usage_url: None,
        quirks: Default::default(),
    }
}

#[derive(Debug, Clone)]
pub struct ModelDef { pub id: &'static str, pub name: &'static str, pub max_tokens: u32, pub supports_vision: bool, pub supports_tools: bool }

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "grok-4.5", name: "Grok 4.5", max_tokens: 500000, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-4.3", name: "Grok 4.3", max_tokens: 1000000, supports_vision: false, supports_tools: true },
    ModelDef { id: "grok-build-0.1", name: "Grok Build 0.1", max_tokens: 262144, supports_vision: false, supports_tools: true },
];
