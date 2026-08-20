#[derive(Clone)]
pub struct AnthropicConfig {
    pub provider_id: String,
    pub provider_name: String,
    pub model_prefix: String,
    pub base_url: String,
    pub validate_url: String,
    pub category: String,
    pub color: String,
    pub icon_name: String,
    pub default_timeout_secs: u64,
    pub stream_first_chunk_timeout_secs: u64,
    pub stream_stall_timeout_secs: u64,
    pub models: Vec<ModelDef>,
    pub quirks: crate::providers::spec::ProviderQuirks,
}

#[derive(Clone)]
pub struct ModelDef {
    pub id: String,
    pub name: String,
    pub max_tokens: Option<u32>,
    pub context_length: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub reasoning: bool,
    /// If set, strip `<tag>…</tag>` blocks (plus `<|tag|>…<|/tag|>` chatml
    /// variant) from response content. Used by providers whose upstream
    /// emits chain-of-thought as literal text instead of a separate field.
    /// Empty slice → no stripping. See `services::thinking_filter`.
    pub thinking_tags: Option<Vec<String>>,
}

impl ModelDef {
    pub fn new(id: &str, name: &str, context_length: u32, supports_vision: bool, supports_tools: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            max_tokens: None,
            context_length,
            supports_vision,
            supports_tools,
            reasoning: false,
            thinking_tags: None,
        }
    }
}