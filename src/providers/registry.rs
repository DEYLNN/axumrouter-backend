use std::collections::HashMap;

use crate::db::models::ApiKey;
use sqlx::SqlitePool;
use std::sync::Arc;
use crate::providers::traits::Provider;

/// Maps provider name → constructor function.
pub struct ProviderRegistry {
    builders: HashMap<String, Box<dyn Fn(Vec<ApiKey>, Arc<sqlx::SqlitePool>) -> anyhow::Result<Box<dyn Provider>> + Send + Sync>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            builders: HashMap::new(),
        };

        // OpenAI-compatible (API key + Bearer/XApiKey auth):
        registry.register("mst", |keys, db| {
            Ok(Box::new(crate::providers::mistral::new_with_keys(keys)))
        });
        registry.register("ocg", |keys, db| {
            Ok(Box::new(crate::providers::opencode_go::new_with_keys(keys)))
        });
        registry.register("ocf", |keys, db| {
            Ok(Box::new(crate::providers::opencode_free::new_with_keys(keys)))
        });
        registry.register("tbay", |keys, db| {
            Ok(Box::new(crate::providers::tokenbay::new_with_keys(keys)))
        });
        registry.register("nrak", |keys, db| {
            Ok(Box::new(crate::providers::nous_api_key::new_with_keys(keys)))
        });
        registry.register("cl", |keys, db| {
            Ok(Box::new(crate::providers::cline::new_with_keys(keys)))
        });
        
        // Custom providers:
        registry.register("cf", |keys, db| {
            Ok(Box::new(crate::providers::cloudflare::provider::CfProvider::new_with_keys(keys)))
        });
        registry.register("fb", |keys, db| {
            Ok(Box::new(crate::providers::freebuff::provider::FbProvider::new_with_keys(keys)))
        });
        registry.register("mcf", |keys, db| {
            Ok(Box::new(crate::providers::mimo_code_free::provider::McfProvider::new_with_keys(keys)))
        });
        registry.register("np", |keys, db| {
            Ok(Box::new(crate::providers::nous_portal::provider::NpProvider::new_with_keys(keys, db)))
        });
        registry.register("cx", |keys, db| {
            Ok(Box::new(crate::providers::openai_codex::provider::CxProvider::new_with_keys(keys)))
        });
        registry.register("xai", |keys, db| {
            Ok(Box::new(crate::providers::xai::provider::XaiProvider::new_with_keys(keys)))
        });
        // API-key providers (openai_compat):
        registry.register("xak", |keys, db| {
            Ok(Box::new(crate::providers::xai_api_key::new_with_keys(keys)))
        });

        registry
    }

    pub fn register<F>(&mut self, id: &str, builder: F)
    where
        F: Fn(Vec<ApiKey>, Arc<SqlitePool>) -> anyhow::Result<Box<dyn Provider>> + Send + Sync + 'static,
    {
        self.builders.insert(id.to_string(), Box::new(builder));
    }

    pub fn build(
        &self,
        id: &str,
        keys: Vec<ApiKey>,
        db: Arc<SqlitePool>,
    ) -> Option<Box<dyn Provider>> {
        self.builders.get(id).and_then(|b| b(keys, db).ok())
    }

    pub fn provider_ids(&self) -> Vec<&str> {
        self.builders.keys().map(|s| s.as_str()).collect()
    }
}
