/// Per-provider static config for OpenAI-compatible providers.
pub struct OpenAIConfig {
    pub provider_id: &'static str,
    pub provider_name: &'static str,
    pub model_prefix: &'static str,
    pub base_url: &'static str,
    pub validate_url: &'static str,
    pub docs_url: &'static str,
    pub api_key_url: &'static str,
    pub category: &'static str,
    pub color: &'static str,
    pub icon_name: &'static str,
    pub default_timeout_secs: u64,
    /// Timeout (seconds) waiting for the first SSE chunk (model loading/prompt prefill).
    /// Mirrors 9router's STREAM_FIRST_CHUNK_TIMEOUT_MS / 1000.
    pub stream_first_chunk_timeout_secs: u64,
    /// Timeout (seconds) between subsequent SSE chunks (model reasoning).
    /// Mirrors 9router's STREAM_STALL_TIMEOUT_MS / 1000.
    pub stream_stall_timeout_secs: u64,
    pub models: &'static [ModelDef],
    pub quirks: crate::providers::spec::ProviderQuirks,
}

pub struct ModelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub max_tokens: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
}
