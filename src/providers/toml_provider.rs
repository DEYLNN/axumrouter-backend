/// TOML data schema for simple API-key providers.
/// Loaded at compile time via include_str!("../../providers.toml").
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProviderList {
    pub providers: Vec<ProviderDef>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderDef {
    pub id: String,
    pub name: String,
    pub category: String,
    pub color: String,
    pub icon: String,
    pub base_url: String,
    pub validate_url: String,
    pub api_type: String,

    // Optional
    pub timeout: Option<u64>,
    pub first_chunk_timeout: Option<u64>,
    pub stall_timeout: Option<u64>,

    // Quirks (per-provider edge cases)
    pub quirks: Option<QuirksDef>,

    // Models
    #[serde(default)]
    pub models: Vec<ModelDef>,
}

#[derive(Debug, Deserialize)]
pub struct ModelDef {
    pub id: String,
    pub name: String,
    pub ctx: u64,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub tools: bool,
}

#[derive(Debug, Deserialize)]
pub struct QuirksDef {
    /// "bearer" (default) or "x-api-key"
    pub auth_header: Option<String>,
    /// "max_tokens" (default) or "max_completion_tokens"
    pub max_tokens_field: Option<String>,
    pub drop_stream_options: Option<bool>,
    pub drop_tools: Option<bool>,
    pub drop_tool_choice: Option<bool>,
    pub supports_stream_usage: Option<bool>,
    pub default_temperature: Option<f64>,
    pub force_temperature: Option<f64>,
}

use crate::engine::openai_compat::config::{OpenAIConfig, ModelDef as EngineModelDef};
use crate::engine::anthropic_compat::config::{AnthropicConfig, ModelDef as AnthropicModelDef};
use crate::providers::spec::{AuthHeader, MaxTokensField, ProviderQuirks};

fn build_quirks(p: &ProviderDef) -> ProviderQuirks {
    let q = p.quirks.as_ref();
    ProviderQuirks {
        drop_stream_options: q.and_then(|x| x.drop_stream_options).unwrap_or(false),
        drop_tools: q.and_then(|x| x.drop_tools).unwrap_or(false),
        drop_tool_choice: q.and_then(|x| x.drop_tool_choice).unwrap_or(false),
        auth_header: match q.and_then(|x| x.auth_header.as_deref()) {
            Some("x-api-key") => AuthHeader::XApiKey,
            _ => AuthHeader::Bearer,
        },
        max_tokens_field: match q.and_then(|x| x.max_tokens_field.as_deref()) {
            Some("max_completion_tokens") => MaxTokensField::MaxCompletionTokens,
            _ => MaxTokensField::MaxTokens,
        },
        supports_stream_usage: q.and_then(|x| x.supports_stream_usage).unwrap_or(true),
        default_temperature: q.and_then(|x| x.default_temperature),
        force_temperature: q.and_then(|x| x.force_temperature),
    }
}

fn build_models(p: &ProviderDef) -> Vec<EngineModelDef> {
    p.models.iter().map(|m| EngineModelDef {
        id: m.id.clone(), name: m.name.clone(), max_tokens: m.ctx as u32,
        supports_vision: m.vision, supports_tools: m.tools,
    }).collect()
}

/// Convert TOML ProviderDef → engine OpenAIConfig for registration.
pub fn build_openai_config(p: &ProviderDef) -> OpenAIConfig {
    OpenAIConfig {
        provider_id: p.id.clone(),
        provider_name: p.name.clone(),
        model_prefix: p.id.clone(),
        base_url: p.base_url.clone(),
        validate_url: p.validate_url.clone(),
        category: p.category.clone(),
        color: p.color.clone(),
        icon_name: p.icon.clone(),
        default_timeout_secs: p.timeout.unwrap_or(120),
        stream_first_chunk_timeout_secs: p.first_chunk_timeout.unwrap_or(200),
        stream_stall_timeout_secs: p.stall_timeout.unwrap_or(360),
        models: build_models(p),
        quirks: build_quirks(p),
    }
}

/// Convert TOML ProviderDef → engine AnthropicConfig for registration.
pub fn build_anthropic_config(p: &ProviderDef) -> AnthropicConfig {
    AnthropicConfig {
        provider_id: p.id.clone(),
        provider_name: p.name.clone(),
        model_prefix: p.id.clone(),
        base_url: p.base_url.clone(),
        validate_url: p.validate_url.clone(),
        category: p.category.clone(),
        color: p.color.clone(),
        icon_name: p.icon.clone(),
        default_timeout_secs: p.timeout.unwrap_or(120),
        stream_first_chunk_timeout_secs: p.first_chunk_timeout.unwrap_or(200),
        stream_stall_timeout_secs: p.stall_timeout.unwrap_or(360),
        models: p.models.iter().map(|m| AnthropicModelDef {
            id: m.id.clone(), name: m.name.clone(), max_tokens: m.ctx as u32,
            supports_vision: m.vision, supports_tools: m.tools,
        }).collect(),
        quirks: build_quirks(p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_providers_toml() {
        let data = include_str!("../../providers.toml");
        let list: ProviderList = toml::from_str(data).expect("Failed to parse providers.toml");
        assert!(!list.providers.is_empty(), "Should have at least 1 provider");
        for p in &list.providers {
            if p.models.is_empty() {
                println!("{} → 0 models (skipped)", p.id);
            } else {
                println!("{} → {} models", p.id, p.models.len());
            }
        }
        println!("\nTotal: {} providers", list.providers.len());
    }
}
