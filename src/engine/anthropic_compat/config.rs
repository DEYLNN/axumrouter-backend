pub struct AnthropicConfig {
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
    pub stream_first_chunk_timeout_secs: u64,
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
