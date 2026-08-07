/// Grok Build — OAuth device_code provider via xAI's cli-chat-proxy.
///
/// Distinct from xAI's regular API (`api.x.ai`). This provider wires the
/// Grok Build subscription through `cli-chat-proxy.grok.com` using the
/// OpenAI Responses API (`/v1/responses`).
///
/// Wire capture source: @xai-official/grok CLI 0.2.99.
#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub backend_model: &'static str,
    pub max_tokens: Option<u32>,
    pub context_length: u32,
}

pub const PROVIDER_ID: &str = "gb";
pub const PROVIDER_NAME: &str = "Grok Build";

/// OAuth client_id is shared with the official Grok CLI / xAI OAuth.
pub const OAUTH_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";

pub const OAUTH_DEVICE_CODE_URL: &str = "https://auth.x.ai/oauth2/device/code";
pub const OAUTH_TOKEN_URL: &str = "https://auth.x.ai/oauth2/token";

pub const CATEGORY: &str = "oauth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 120;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 120;

pub const COLOR: &str = "#1DA1F2";
pub const ICON_NAME: &str = "gb.png";

/// Wire values from official Grok CLI. Proxy requires them on every call.
pub const GROK_CLI_VERSION: &str = "0.2.99";
pub const GROK_CLI_CLIENT_IDENTIFIER: &str = "grok-shell";
pub const GROK_CLI_USER_AGENT: &str = "grok-shell/0.2.99 (linux; x86_64)";

pub const API_BASE_URL: &str = "https://cli-chat-proxy.grok.com/v1";
/// OpenAI Responses API endpoint. NOT `/chat/completions`.
pub const RESPONSES_URL: &str = "https://cli-chat-proxy.grok.com/v1/responses";
pub const VALIDATE_URL: &str = "https://cli-chat-proxy.grok.com/v1/models";

/// Default reasoning effort for all models. Override per-request when needed.
/// `high` eats the whole output budget on reasoning (response comes back
/// truncated, tool calls never emitted). Reference CLI prox options default
/// to low / "concise" summary — match that so the visible text + tool calls
/// actually reach the client.
pub const DEFAULT_REASONING_EFFORT: &str = "low";
pub const DEFAULT_REASONING_SUMMARY: &str = "concise";

/// OAuth scopes — HAR capture from official CLI. Includes conversations R/W
/// beyond the plain api-only `xai` scope.
pub const OAUTH_SCOPES: &str =
    "openid profile email offline_access grok-cli:access api:access conversations:read conversations:write";

/// Poll interval (seconds) for device_code token polling.
pub const OAUTH_POLL_INTERVAL_SECS: u64 = 3;

pub const MODELS: &[ModelDef] = &[
    ModelDef {
        id: "grok-build",
        backend_model: "grok-build",
        max_tokens: Some(64000),
        context_length: 500000,
    },
    ModelDef {
        id: "grok-4.5",
        backend_model: "grok-4.5",
        max_tokens: Some(64000),
        context_length: 500000,
    },
];
