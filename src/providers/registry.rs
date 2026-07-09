use std::collections::HashMap;

use crate::db::models::ApiKey;
use crate::providers::traits::Provider;

/// Maps provider name → constructor function.
pub struct ProviderRegistry {
    builders: HashMap<String, Box<dyn Fn(Vec<ApiKey>) -> anyhow::Result<Box<dyn Provider>> + Send + Sync>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            builders: HashMap::new(),
        };

        // OpenAI-compatible (API key + Bearer/XApiKey auth):
        registry.register("mst", |keys| {
            Ok(Box::new(crate::providers::mistral::new_with_keys(keys)))
        });
        registry.register("ocg", |keys| {
            Ok(Box::new(crate::providers::opencode_go::new_with_keys(keys)))
        });
        registry.register("ocf", |keys| {
            Ok(Box::new(crate::providers::opencode_free::new_with_keys(keys)))
        });
        registry.register("tbay", |keys| {
            Ok(Box::new(crate::providers::tokenbay::new_with_keys(keys)))
        });
        registry.register("cl", |keys| {
            Ok(Box::new(crate::providers::cline::new_with_keys(keys)))
        });
        
        // Custom providers:
        registry.register("cf", |keys| {
            Ok(Box::new(crate::providers::cloudflare::provider::CfProvider::new_with_keys(keys)))
        });
        registry.register("fb", |keys| {
            Ok(Box::new(crate::providers::freebuff::provider::FbProvider::new_with_keys(keys)))
        });
        registry.register("cx", |keys| {
            Ok(Box::new(crate::providers::openai_codex::provider::CxProvider::new_with_keys(keys)))
        });
        registry.register("xai", |keys| {
            Ok(Box::new(crate::providers::xai::provider::XaiProvider::new_with_keys(keys)))
        });
        // API-key providers (openai_compat):
        registry.register("xak", |keys| {
            Ok(Box::new(crate::providers::xai_api_key::new_with_keys(keys)))
        });

        registry
    }

    pub fn register<F>(&mut self, id: &str, builder: F)
    where
        F: Fn(Vec<ApiKey>) -> anyhow::Result<Box<dyn Provider>> + Send + Sync + 'static,
    {
        self.builders.insert(id.to_string(), Box::new(builder));
    }

    pub fn build(
        &self,
        id: &str,
        keys: Vec<ApiKey>,
    ) -> Option<Box<dyn Provider>> {
        self.builders.get(id).and_then(|b| b(keys).ok())
    }

    pub fn provider_ids(&self) -> Vec<&str> {
        self.builders.keys().map(|s| s.as_str()).collect()
    }
}
