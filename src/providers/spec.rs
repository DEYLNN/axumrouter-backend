#![allow(dead_code)]
/// Provider metadata and behavior flags used to keep provider implementations stable.
/// Inspired by 9router provider registry + quirks model.
#[derive(Debug, Clone)]
pub struct ProviderSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub full_name: &'static str,
    pub category: &'static str,
    pub base_url: &'static str,
    pub validate_url: &'static str,
    pub compatible_api: &'static str,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub color: &'static str,
    pub icon_name: &'static str,
    pub usage_url: Option<&'static str>,
    pub quirks: ProviderQuirks,
}

#[derive(Debug, Clone)]
pub struct ProviderQuirks {
    /// Provider rejects OpenAI stream_options.include_usage.
    pub drop_stream_options: bool,
    /// Provider rejects tools array.
    pub drop_tools: bool,
    /// Provider rejects tool_choice.
    pub drop_tool_choice: bool,
    /// Provider expects x-api-key instead of bearer auth header.
    pub auth_header: AuthHeader,
    /// Provider uses max_completion_tokens instead of max_tokens.
    pub max_tokens_field: MaxTokensField,
    /// Provider supports final usage chunk during SSE stream.
    pub supports_stream_usage: bool,
    /// Default temperature when user doesn't specify. None = omit field.
    pub default_temperature: Option<f64>,
    /// Force temperature — always use this value, ignore user request.
    pub force_temperature: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthHeader {
    Bearer,
    XApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxTokensField {
    MaxTokens,
    MaxCompletionTokens,
}

impl Default for ProviderQuirks {
    fn default() -> Self {
        Self {
            drop_stream_options: false,
            drop_tools: false,
            drop_tool_choice: false,
            auth_header: AuthHeader::Bearer,
            max_tokens_field: MaxTokensField::MaxTokens,
            supports_stream_usage: true,
            default_temperature: None,
            force_temperature: None,
        }
    }
}
