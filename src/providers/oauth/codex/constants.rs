#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub backend_model: &'static str,
    pub max_tokens: Option<u32>,
    pub context_length: u32,
}

pub const PROVIDER_ID: &str = "cx";
pub const PROVIDER_NAME: &str = "OpenAI Codex";

/// OAuth — authorization_code + PKCE (S256), same client as official codex_cli_rs.
pub const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const OAUTH_SCOPE: &str = "openid profile email offline_access";
pub const REDIRECT_URL: &str = "http://localhost:1455/auth/callback";
pub const OAUTH_EXTRA_PARAMS: &str =
    "id_token_add_organizations=true&codex_cli_simplified_flow=true&originator=codex_cli_rs";

pub const CATEGORY: &str = "oauth";
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;
pub const STREAM_FIRST_CHUNK_TIMEOUT_SECS: u64 = 120;
pub const STREAM_STALL_TIMEOUT_SECS: u64 = 360;

/// Refresh lead time — 9router uses 5 days for codex (refreshLeadMs).
pub const REFRESH_LEAD_SECS: i64 = 5 * 24 * 3600;

pub const COLOR: &str = "#10A37F";
pub const ICON_NAME: &str = "cx.png";

/// OpenAI Codex Responses API endpoint. NOT /chat/completions.
pub const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
pub const VALIDATE_URL: &str = "https://chatgpt.com/codex";
/// WHAM usage endpoint for plan/rate-limit info.
/// Wired to FE Usage page later (same as legacy); kept for now so the wire
/// values aren't lost.
#[allow(dead_code)]
pub const WHAM_USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

pub const USER_AGENT: &str = "codex_cli_rs/0.136.0";
pub const ORIGINATOR: &str = "codex_cli_rs";

/// Reasoning effort options (9router `thinkingConfig`). Values valid for the
/// Codex backend. Default `high` — FE has no effort picker yet, and GPT-5.6
/// reasoning models are designed to run at high effort.
pub const THINKING_OPTIONS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];
pub const DEFAULT_REASONING_EFFORT: &str = "high";

pub const MODELS: &[ModelDef] = &[
    ModelDef { id: "gpt-5.6-sol", backend_model: "gpt-5.6-sol", max_tokens: None, context_length: 1000000 },
    ModelDef { id: "gpt-5.6-terra", backend_model: "gpt-5.6-terra", max_tokens: None, context_length: 1000000 },
    ModelDef { id: "gpt-5.6-luna", backend_model: "gpt-5.6-luna", max_tokens: None, context_length: 1000000 },
    ModelDef { id: "gpt-5.5", backend_model: "gpt-5.5", max_tokens: None, context_length: 1000000 },
    ModelDef { id: "gpt-5.4", backend_model: "gpt-5.4", max_tokens: None, context_length: 1000000 },
    ModelDef { id: "gpt-5.4-mini", backend_model: "gpt-5.4-mini", max_tokens: None, context_length: 400000 },
];